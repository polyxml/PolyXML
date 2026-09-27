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
            ScalarType::String => {
                let s = std::str::from_utf8(bytes)?;
                Ok(PolyValue::String(s.to_string()))
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
            | ScalarType::XmlDate
            | ScalarType::XmlDateTime
            | ScalarType::XmlTime
            | ScalarType::XmlDuration
            | ScalarType::Any => {
                let s = std::str::from_utf8(trim_bytes(bytes))?;
                Ok(PolyValue::String(s.to_string()))
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
