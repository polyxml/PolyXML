//! Exact XML Schema integer lexical validation and comparison, without a size cap.
use crate::ir::PrimitiveType;
use std::cmp::Ordering;

fn parts(text: &str) -> Option<(bool, &str)> {
    let text = text.trim_matches([' ', '\t', '\r', '\n']);
    let negative = text.starts_with('-');
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    Some((negative && !digits.is_empty(), digits))
}

pub fn validate(text: &str, primitive: PrimitiveType) -> bool {
    let Some((negative, digits)) = parts(text) else {
        return false;
    };
    match primitive {
        PrimitiveType::Integer => true,
        PrimitiveType::PositiveInteger => !negative && !digits.is_empty(),
        PrimitiveType::NonNegativeInteger => !negative,
        PrimitiveType::NegativeInteger => negative,
        PrimitiveType::NonPositiveInteger => negative || digits.is_empty(),
        _ => false,
    }
}

pub fn compare(left: &str, right: &str) -> Option<Ordering> {
    let (left_negative, left) = parts(left)?;
    let (right_negative, right) = parts(right)?;
    Some(match (left_negative, right_negative) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => {
            let order = left.len().cmp(&right.len()).then_with(|| left.cmp(right));
            if left_negative {
                order.reverse()
            } else {
                order
            }
        }
    })
}

/// Compare integer bounds in the XSD value space without narrowing or rounding.
pub fn within_bounds(
    text: &str,
    min: Option<&str>,
    max: Option<&str>,
    min_exclusive: Option<&str>,
    max_exclusive: Option<&str>,
) -> bool {
    [
        (min, false, true),
        (max, false, false),
        (min_exclusive, true, true),
        (max_exclusive, true, false),
    ]
    .into_iter()
    .all(|(bound, exclusive, minimum)| {
        bound.is_none_or(|bound| {
            compare(text, bound).is_some_and(|order| {
                if minimum {
                    order.is_gt() || (!exclusive && order.is_eq())
                } else {
                    order.is_lt() || (!exclusive && order.is_eq())
                }
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparison_uses_sign_and_magnitude() {
        for (a, b, ordering) in [
            ("+0001", "1", Ordering::Equal),
            ("-0", "+0", Ordering::Equal),
            ("-99", "-100", Ordering::Greater),
            ("100", "99", Ordering::Greater),
            ("-1", "0", Ordering::Less),
            (
                "99999999999999999999999999999999999999999",
                "100000000000000000000000000000000000000000",
                Ordering::Less,
            ),
        ] {
            assert_eq!(compare(a, b), Some(ordering));
            assert_eq!(compare(b, a), Some(ordering.reverse()));
        }
        assert_eq!(compare("1e2", "100"), None);
    }
}
