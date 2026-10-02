use super::QName;
use serde::{Deserialize, Serialize};

/// Source particle boundaries and occurrence constraints, independent of field layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Particle {
    Element(QName),
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
    pub fn adopt_namespace(&mut self, namespace: &str) {
        match self {
            Self::Element(name) => {
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
            Self::Element(name) => regex::escape(&format!("{};", name)),
            Self::Sequence(items) => items.iter().map(Self::pattern).collect(),
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
