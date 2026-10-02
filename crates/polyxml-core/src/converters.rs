use crate::error::{PolyXmlError, Result};
use crate::ir::PrimitiveType;
use crate::schema::ScalarType;
use crate::value::PolyValue;

#[inline(always)]
pub fn trim_bytes(mut b: &[u8]) -> &[u8] {
    while let Some((first, rest)) = b.split_first() {
        if first.is_ascii_whitespace() {
            b = rest;
        } else {
            break;
        }
    }
    while let Some((last, rest)) = b.split_last() {
        if last.is_ascii_whitespace() {
            b = rest;
        } else {
            break;
        }
    }
    b
}

pub struct ValueConverter;

impl ValueConverter {
    pub fn parse_scalar(
        scalar_type: &ScalarType,
        bytes: &[u8],
        field_name: &str,
    ) -> Result<PolyValue> {
        match scalar_type {
            ScalarType::Restricted(base, facets) => {
                let value = Self::parse_scalar(base, bytes, field_name)?;
                let text = std::str::from_utf8(bytes)?;
                let length = match &value {
                    PolyValue::List(items) => items.len(),
                    _ => text.chars().count(),
                };
                let mut valid = facets.length.is_none_or(|n| length == n)
                    && facets.min_length.is_none_or(|n| length >= n)
                    && facets.max_length.is_none_or(|n| length <= n);
                for pattern in &facets.patterns {
                    valid &= regex::Regex::new(&format!("\\A(?:{pattern})\\z"))
                        .map(|r| r.is_match(text))
                        .unwrap_or(false);
                }
                for (bound, inclusive, minimum) in [
                    (&facets.min_inclusive, true, true),
                    (&facets.min_exclusive, false, true),
                    (&facets.max_inclusive, true, false),
                    (&facets.max_exclusive, false, false),
                ] {
                    if let Some(bound) = bound {
                        let ordering = if let PolyValue::Int(number) = value {
                            bound
                                .parse::<i128>()
                                .ok()
                                .map(|bound| (number as i128).cmp(&bound))
                        } else if scalar_is_integer(base) {
                            crate::integer::compare(text, bound)
                        } else {
                            text.parse::<f64>()
                                .ok()
                                .zip(bound.parse::<f64>().ok())
                                .and_then(|(value, bound)| value.partial_cmp(&bound))
                        };
                        valid &= ordering.is_some_and(|ordering| {
                            if minimum {
                                ordering.is_gt() || (inclusive && ordering.is_eq())
                            } else {
                                ordering.is_lt() || (inclusive && ordering.is_eq())
                            }
                        });
                    }
                }
                if !valid {
                    return Err(PolyXmlError::ScalarParseError {
                        field: field_name.into(),
                        expected: "restriction facets",
                        value: text.into(),
                    });
                }
                Ok(value)
            }
            ScalarType::List(inner) => {
                let text = std::str::from_utf8(bytes)?;
                let values = text
                    .split([' ', '\t', '\r', '\n'])
                    .filter(|token| !token.is_empty())
                    .map(|token| Self::parse_scalar(inner, token.as_bytes(), field_name))
                    .collect::<Result<Vec<_>>>()?;
                Ok(PolyValue::List(values))
            }
            ScalarType::Union(members) => {
                for member in members {
                    if let Ok(value) = Self::parse_scalar(member, bytes, field_name) {
                        return Ok(value);
                    }
                }
                Err(PolyXmlError::ScalarParseError {
                    field: field_name.to_string(),
                    expected: "union member",
                    value: String::from_utf8_lossy(bytes).to_string(),
                })
            }
            ScalarType::Enum(values) => {
                let value = std::str::from_utf8(trim_bytes(bytes))?;
                if values.iter().any(|candidate| candidate == value) {
                    Ok(PolyValue::String(value.to_string()))
                } else {
                    Err(PolyXmlError::ScalarParseError {
                        field: field_name.to_string(),
                        expected: "enumeration",
                        value: value.to_string(),
                    })
                }
            }
            ScalarType::Pattern(base, patterns) => {
                let value = std::str::from_utf8(trim_bytes(bytes))?;
                for pattern in patterns {
                    let anchored = format!(r"\A(?:{pattern})\z");
                    if !regex::Regex::new(&anchored)
                        .map(|compiled| compiled.is_match(value))
                        .unwrap_or(false)
                    {
                        return Err(PolyXmlError::ScalarParseError {
                            field: field_name.to_string(),
                            expected: "pattern",
                            value: value.to_string(),
                        });
                    }
                }
                Self::parse_scalar(base, bytes, field_name)
            }
            ScalarType::String => {
                let s = std::str::from_utf8(bytes)?;
                Ok(PolyValue::String(s.to_string()))
            }
            ScalarType::Integer(primitive) => {
                let text = std::str::from_utf8(bytes)?.trim_matches([' ', '\t', '\r', '\n']);
                if !crate::integer::validate(text, *primitive) {
                    return Err(PolyXmlError::ScalarParseError {
                        field: field_name.into(),
                        expected: "XML Schema integer",
                        value: text.into(),
                    });
                }
                Ok(PolyValue::String(text.into()))
            }
            ScalarType::Int => {
                let trimmed = trim_bytes(bytes);
                let val: i64 =
                    lexical_core::parse(trimmed).map_err(|_| PolyXmlError::ScalarParseError {
                        field: field_name.to_string(),
                        expected: "integer",
                        value: String::from_utf8_lossy(bytes).to_string(),
                    })?;
                Ok(PolyValue::Int(val))
            }
            ScalarType::Float => {
                let trimmed = trim_bytes(bytes);
                let val: f64 =
                    lexical_core::parse(trimmed).map_err(|_| PolyXmlError::ScalarParseError {
                        field: field_name.to_string(),
                        expected: "float",
                        value: String::from_utf8_lossy(bytes).to_string(),
                    })?;
                Ok(PolyValue::Float(val))
            }
            ScalarType::Bool => {
                let trimmed = trim_bytes(bytes);
                match trimmed {
                    b"true" | b"1" => Ok(PolyValue::Bool(true)),
                    b"false" | b"0" => Ok(PolyValue::Bool(false)),
                    _ => Err(PolyXmlError::ScalarParseError {
                        field: field_name.to_string(),
                        expected: "boolean ('true', 'false', '1', '0')",
                        value: String::from_utf8_lossy(bytes).to_string(),
                    }),
                }
            }
            ScalarType::Decimal
            | ScalarType::XmlDateTime
            | ScalarType::XmlTime
            | ScalarType::XmlDuration
            | ScalarType::Any => {
                let s = std::str::from_utf8(trim_bytes(bytes))?;
                Ok(PolyValue::String(s.to_string()))
            }
            ScalarType::XmlDate => {
                let value = std::str::from_utf8(trim_bytes(bytes))?;
                if !valid_xml_date(value.as_bytes()) {
                    return Err(PolyXmlError::ScalarParseError {
                        field: field_name.to_string(),
                        expected: "XML Schema date",
                        value: value.to_string(),
                    });
                }
                Ok(PolyValue::String(value.to_string()))
            }
            ScalarType::XmlGregorian(kind) => {
                let value = std::str::from_utf8(trim_bytes(bytes))?;
                if !valid_gregorian(*kind, value.as_bytes()) {
                    return Err(PolyXmlError::ScalarParseError {
                        field: field_name.to_string(),
                        expected: "XML Schema Gregorian partial date",
                        value: value.to_string(),
                    });
                }
                Ok(PolyValue::String(value.to_string()))
            }
        }
    }
}

fn two_digits(bytes: &[u8]) -> Option<u8> {
    if bytes.len() == 2 && bytes.iter().all(u8::is_ascii_digit) {
        Some((bytes[0] - b'0') * 10 + bytes[1] - b'0')
    } else {
        None
    }
}

fn valid_xml_date(bytes: &[u8]) -> bool {
    let date = if bytes.ends_with(b"Z")
        || (bytes.len() >= 6
            && matches!(bytes[bytes.len() - 6], b'+' | b'-')
            && bytes[bytes.len() - 3] == b':')
    {
        let Some(date) = valid_timezone(bytes) else {
            return false;
        };
        date
    } else {
        bytes
    };
    let (negative, date) = if let Some(rest) = date.strip_prefix(b"-") {
        (true, rest)
    } else {
        (false, date)
    };
    let mut parts = date.split(|byte| *byte == b'-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if year.len() < 4 || !year.iter().all(u8::is_ascii_digit) || (year.len() > 4 && year[0] == b'0')
    {
        return false;
    }
    let Some(month) = two_digits(month) else {
        return false;
    };
    let Some(day) = two_digits(day) else {
        return false;
    };
    let Ok(year) = std::str::from_utf8(year).unwrap_or("").parse::<i64>() else {
        return false;
    };
    if year == 0 && negative {
        return false;
    }
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => return false,
    };
    day >= 1 && day <= max_day
}

fn valid_timezone(bytes: &[u8]) -> Option<&[u8]> {
    if let Some(value) = bytes.strip_suffix(b"Z") {
        return Some(value);
    }
    let split = bytes
        .iter()
        .rposition(|byte| *byte == b'+' || *byte == b'-')?;
    let suffix = &bytes[split..];
    if suffix.len() != 6 || suffix[3] != b':' {
        return None;
    }
    let hours = two_digits(&suffix[1..3])?;
    let minutes = two_digits(&suffix[4..6])?;
    if hours > 14 || minutes > 59 || (hours == 14 && minutes != 0) {
        return None;
    }
    Some(&bytes[..split])
}

fn valid_gregorian(kind: PrimitiveType, bytes: &[u8]) -> bool {
    let has_timezone = bytes.ends_with(b"Z")
        || bytes.contains(&b'+')
        || (bytes.len() >= 6 && bytes[bytes.len() - 6] == b'-' && bytes[bytes.len() - 3] == b':');
    let value = if has_timezone {
        match valid_timezone(bytes) {
            Some(value) => value,
            None => return false,
        }
    } else {
        bytes
    };
    match kind {
        PrimitiveType::GDay => {
            value.len() == 5
                && value.starts_with(b"---")
                && two_digits(&value[3..]).is_some_and(|day| (1..=31).contains(&day))
        }
        PrimitiveType::GMonth => {
            value.len() == 4
                && value.starts_with(b"--")
                && two_digits(&value[2..]).is_some_and(|month| (1..=12).contains(&month))
        }
        PrimitiveType::GMonthDay => {
            if value.len() != 7 || !value.starts_with(b"--") || value[4] != b'-' {
                return false;
            }
            let (Some(month), Some(day)) = (two_digits(&value[2..4]), two_digits(&value[5..7]))
            else {
                return false;
            };
            let max_day = match month {
                2 => 29,
                4 | 6 | 9 | 11 => 30,
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                _ => return false,
            };
            day >= 1 && day <= max_day
        }
        PrimitiveType::GYear => valid_year(value),
        PrimitiveType::GYearMonth => {
            if value.len() < 7 || value[value.len() - 3] != b'-' {
                return false;
            }
            valid_year(&value[..value.len() - 3])
                && two_digits(&value[value.len() - 2..])
                    .is_some_and(|month| (1..=12).contains(&month))
        }
        _ => false,
    }
}

fn valid_year(value: &[u8]) -> bool {
    let digits = value.strip_prefix(b"-").unwrap_or(value);
    digits.len() >= 4
        && digits.iter().all(u8::is_ascii_digit)
        && (digits.len() == 4 || digits[0] != b'0')
}

fn scalar_is_integer(scalar: &ScalarType) -> bool {
    match scalar {
        ScalarType::Integer(_) => true,
        ScalarType::Restricted(base, _) | ScalarType::Pattern(base, _) => scalar_is_integer(base),
        _ => false,
    }
}
