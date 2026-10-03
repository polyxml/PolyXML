//! Bounded process-wide reuse of anchored XML scalar patterns.
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, OnceLock, RwLock};

const MAX_ENTRIES: usize = 16;
const MAX_KEY_BYTES: usize = 4096;

#[derive(Clone)]
enum CompiledPattern {
    Valid(Arc<regex::Regex>),
    Invalid,
}

impl CompiledPattern {
    fn compile(pattern: &str) -> Self {
        match regex::Regex::new(&format!(r"\A(?:{pattern})\z")) {
            Ok(regex) => Self::Valid(Arc::new(regex)),
            Err(_) => Self::Invalid,
        }
    }

    fn matches(&self, value: &str) -> bool {
        match self {
            Self::Valid(regex) => regex.is_match(value),
            Self::Invalid => false,
        }
    }
}

#[derive(Default)]
struct PatternCache {
    entries: HashMap<String, CompiledPattern>,
    order: VecDeque<String>,
}

impl PatternCache {
    fn insert(&mut self, pattern: &str, compiled: CompiledPattern) {
        if self.entries.len() == MAX_ENTRIES {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.order.push_back(pattern.into());
        self.entries.insert(pattern.into(), compiled);
    }
}

static CACHE: OnceLock<RwLock<PatternCache>> = OnceLock::new();

pub(crate) fn matches(pattern: &str, value: &str) -> bool {
    // Bound entry and key retention without changing regex acceptance limits.
    if pattern.len() > MAX_KEY_BYTES {
        return CompiledPattern::compile(pattern).matches(value);
    }
    let cache = CACHE.get_or_init(|| RwLock::new(PatternCache::default()));
    let cached = {
        let guard = cache.read().unwrap_or_else(|poison| poison.into_inner());
        guard.entries.get(pattern).cloned()
    };
    if let Some(compiled) = cached {
        return compiled.matches(value);
    }

    // Compilation and matching happen outside the lock. Concurrent misses can
    // compile twice, but installation rechecks the key and retains one entry.
    let compiled = CompiledPattern::compile(pattern);
    let installed = {
        let mut guard = cache.write().unwrap_or_else(|poison| poison.into_inner());
        if let Some(existing) = guard.entries.get(pattern) {
            existing.clone()
        } else {
            guard.insert(pattern, compiled.clone());
            compiled
        }
    };
    installed.matches(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_cache_evicts_and_retains_invalid_results() {
        let mut cache = PatternCache::default();
        cache.insert("[", CompiledPattern::compile("["));
        assert!(!cache.entries["["].matches("anything"));
        for i in 0..MAX_ENTRIES {
            let pattern = format!("VALUE{i}");
            cache.insert(&pattern, CompiledPattern::compile(&pattern));
        }
        assert_eq!(cache.entries.len(), MAX_ENTRIES);
        assert_eq!(cache.order.len(), MAX_ENTRIES);
        assert!(!cache.entries.contains_key("["));
        assert!(cache.entries["VALUE0"].matches("VALUE0"));
        assert!(!cache.entries["VALUE0"].matches("VALUE00"));
    }

    #[test]
    fn cache_matches_uncached_regex_under_churn_and_concurrency() {
        let threads: Vec<_> = (0..4)
            .map(|thread| {
                std::thread::spawn(move || {
                    for i in 0..64 {
                        let pattern = format!("T{thread}-[0-9]{{{}}}", i % 8 + 1);
                        let text = format!("T{thread}-{}", "1".repeat(i % 8 + 1));
                        assert!(matches(&pattern, &text));
                        assert!(!matches(&pattern, &(text + "x")));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(!matches("[", "anything"));
        assert!(matches("é+", "éé"));
        assert!(!matches("é+", "é\n"));
        assert!(matches(
            &"a".repeat(MAX_KEY_BYTES + 1),
            &"a".repeat(MAX_KEY_BYTES + 1)
        ));
    }
}
