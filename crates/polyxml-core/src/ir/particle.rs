use super::QName;
use serde::{Deserialize, Serialize};

/// Source particle boundaries and occurrence constraints, independent of field layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Particle {
    Element(QName),
    Reference(QName),
    Sequence(Vec<Particle>),
    Choice(Vec<Particle>),
    Repeat {
        particle: Box<Particle>,
        min: usize,
        max: Option<usize>,
    },
}
impl Particle {
    pub fn has_choice(&self) -> bool {
        match self {
            Self::Choice(_) => true,
            Self::Sequence(items) => items.iter().any(Self::has_choice),
            Self::Repeat { particle, .. } => particle.has_choice(),
            _ => false,
        }
    }
    pub fn has_repeated_sequence(&self) -> bool {
        match self {
            Self::Repeat { particle, max, .. } => {
                (matches!(particle.as_ref(), Self::Sequence(_)) && max.is_none_or(|n| n > 1))
                    || particle.has_repeated_sequence()
            }
            Self::Choice(items) | Self::Sequence(items) => {
                items.iter().any(Self::has_repeated_sequence)
            }
            _ => false,
        }
    }
    pub fn has_reference(&self) -> bool {
        match self {
            Self::Reference(_) => true,
            Self::Sequence(items) | Self::Choice(items) => items.iter().any(Self::has_reference),
            Self::Repeat { particle, .. } => particle.has_reference(),
            _ => false,
        }
    }
    pub fn has_substitution_reference(
        &self,
        groups: &std::collections::HashMap<QName, Vec<QName>>,
        abstract_elements: &std::collections::BTreeSet<QName>,
    ) -> bool {
        match self {
            Self::Reference(name) => groups.contains_key(name) || abstract_elements.contains(name),
            Self::Sequence(items) | Self::Choice(items) => items
                .iter()
                .any(|item| item.has_substitution_reference(groups, abstract_elements)),
            Self::Repeat { particle, .. } => {
                particle.has_substitution_reference(groups, abstract_elements)
            }
            _ => false,
        }
    }
    pub fn resolve_references(
        &mut self,
        groups: &std::collections::HashMap<QName, Vec<QName>>,
        abstract_elements: &std::collections::BTreeSet<QName>,
    ) {
        match self {
            Self::Reference(name) => {
                let mut pending = vec![name.clone()];
                let mut seen = std::collections::BTreeSet::new();
                let mut alternatives = Vec::new();
                while let Some(name) = pending.pop() {
                    if !seen.insert(name.clone()) {
                        continue;
                    }
                    if !abstract_elements.contains(&name) {
                        alternatives.push(Self::Element(name.clone()));
                    }
                    if let Some(members) = groups.get(&name) {
                        pending.extend(members.iter().cloned());
                    }
                }
                *self = Self::Choice(alternatives);
            }
            Self::Sequence(items) | Self::Choice(items) => {
                for item in items {
                    item.resolve_references(groups, abstract_elements);
                }
            }
            Self::Repeat { particle, .. } => particle.resolve_references(groups, abstract_elements),
            _ => {}
        }
    }
    pub fn adopt_namespace(&mut self, namespace: &str) {
        match self {
            Self::Element(name) | Self::Reference(name) => {
                if name.namespace.is_none() {
                    name.namespace = Some(namespace.into());
                }
            }
            Self::Sequence(items) | Self::Choice(items) => {
                for item in items {
                    item.adopt_namespace(namespace);
                }
            }
            Self::Repeat { particle, .. } => particle.adopt_namespace(namespace),
        }
    }
    pub fn pattern(&self) -> String {
        match self {
            Self::Element(name) | Self::Reference(name) => regex::escape(&format!("{};", name)),
            Self::Sequence(items) => items.iter().map(Self::pattern).collect(),
            Self::Choice(items) if items.is_empty() => "\\x00".into(),
            Self::Choice(items) => format!(
                "(?:{})",
                items
                    .iter()
                    .map(Self::pattern)
                    .collect::<Vec<_>>()
                    .join("|")
            ),
            Self::Repeat { particle, min, max } => format!(
                "(?:{}){{{},{}}}",
                particle.pattern(),
                min,
                max.map(|v| v.to_string()).unwrap_or_default()
            ),
        }
    }
}
