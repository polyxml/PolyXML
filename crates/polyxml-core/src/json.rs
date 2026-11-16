use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::error::{PolyXmlError, Result};
use crate::schema::{FieldSchema, ModelSchema, ScalarType, ValueType};
use crate::value::PolyValue;

/// Convert a `PolyValue` into a `serde_json::Value` dynamically without a schema.
impl From<&PolyValue> for JsonValue {
    fn from(val: &PolyValue) -> Self {
        match val {
            PolyValue::Null => JsonValue::Null,
            PolyValue::Bool(b) => JsonValue::Bool(*b),
            PolyValue::Int(i) => JsonValue::Number((*i).into()),
            PolyValue::Float(f) => serde_json::Number::from_f64(*f)
                .map(JsonValue::Number)
                .unwrap_or(JsonValue::Null),
            PolyValue::String(s) => JsonValue::String(s.clone()),
            PolyValue::List(l) => JsonValue::Array(l.iter().map(JsonValue::from).collect()),
            PolyValue::Object(m) => {
                let mut map = serde_json::Map::with_capacity(m.len());
                for (k, v) in m {
                    map.insert(k.clone(), JsonValue::from(v));
                }
                JsonValue::Object(map)
            }
            PolyValue::Record { schema, values } => {
                let mut map = serde_json::Map::with_capacity(schema.fields.len());
                for (idx, field) in schema.fields.iter().enumerate() {
                    if let Some(Some(v)) = values.get(idx) {
                        map.insert(field.name.clone(), JsonValue::from(v));
                    }
                }
                JsonValue::Object(map)
            }
        }
    }
}

/// Convert a `serde_json::Value` into a `PolyValue` dynamically without a schema.
impl From<JsonValue> for PolyValue {
    fn from(val: JsonValue) -> Self {
        match val {
            JsonValue::Null => PolyValue::Null,
            JsonValue::Bool(b) => PolyValue::Bool(b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    PolyValue::Int(i)
                } else if let Some(u) = n.as_u64() {
                    PolyValue::Int(u as i64)
                } else if let Some(f) = n.as_f64() {
                    PolyValue::Float(f)
                } else {
                    PolyValue::String(n.to_string())
                }
            }
            JsonValue::String(s) => PolyValue::String(s),
            JsonValue::Array(a) => PolyValue::List(a.into_iter().map(PolyValue::from).collect()),
            JsonValue::Object(o) => {
                let mut map = HashMap::with_capacity(o.len());
                for (k, v) in o {
                    map.insert(k, PolyValue::from(v));
                }
                PolyValue::Object(map)
            }
        }
    }
}

/// Convert a `PolyValue` to `serde_json::Value` guided by a `ModelSchema`.
pub fn poly_value_to_json_value(
    val: &PolyValue,
    schema: &ModelSchema,
    by_alias: bool,
) -> JsonValue {
    // xsi:type dispatch: a Record parsed as a concrete
    // derivation iterates its own schema so variant-only fields survive
    // transcoding, and records "@xsi:type" when it is a registered variant.
    let (schema, xsi_type) = match val {
        PolyValue::Record { schema: rec, .. } if rec.name != schema.name => {
            let xsi = schema
                .matches_variant(rec)
                .then(|| String::from_utf8_lossy(&rec.xml_name).into_owned());
            (rec.as_ref(), xsi)
        }
        _ => (schema, None),
    };

    let get_field = |idx: usize, name: &str| -> Option<&PolyValue> {
        match val {
            PolyValue::Record { values, .. } => values.get(idx).and_then(|v| v.as_ref()),
            PolyValue::Object(map) => map.get(name),
            _ => None,
        }
    };

    match val {
        PolyValue::Record { .. } | PolyValue::Object(_) => {
            let mut json_map = serde_json::Map::with_capacity(
                schema.fields.len() + usize::from(xsi_type.is_some()),
            );
            if let Some(xsi) = xsi_type {
                json_map.insert("@xsi:type".to_string(), JsonValue::String(xsi));
            }
            for (idx, field) in schema.fields.iter().enumerate() {
                let key = if by_alias {
                    std::str::from_utf8(&field.xml_name).unwrap_or(field.name.as_str())
                } else {
                    field.name.as_str()
                };

                if let Some(field_val) = get_field(idx, &field.name) {
                    let converted = match &field.val_type {
                        ValueType::Nested(nested_schema) => {
                            if field_val.is_null() {
                                JsonValue::Null
                            } else {
                                poly_value_to_json_value(field_val, nested_schema, by_alias)
                            }
                        }
                        ValueType::List(inner) => {
                            if let PolyValue::List(items) = field_val {
                                if let ValueType::Nested(nested_schema) = inner.as_ref() {
                                    JsonValue::Array(
                                        items
                                            .iter()
                                            .map(|item| {
                                                poly_value_to_json_value(
                                                    item,
                                                    nested_schema,
                                                    by_alias,
                                                )
                                            })
                                            .collect(),
                                    )
                                } else {
                                    JsonValue::Array(items.iter().map(JsonValue::from).collect())
                                }
                            } else if field_val.is_null() {
                                JsonValue::Null
                            } else {
                                JsonValue::Array(vec![JsonValue::from(field_val)])
                            }
                        }
                        ValueType::Scalar(_) => JsonValue::from(field_val),
                    };
                    json_map.insert(key.to_string(), converted);
                }
            }
            JsonValue::Object(json_map)
        }
        other => JsonValue::from(other),
    }
}

/// Coerce a JSON value into a `PolyValue` scalar matching `ScalarType`.
fn coerce_scalar(val: &JsonValue, st: &ScalarType) -> PolyValue {
    match val {
        JsonValue::Null => PolyValue::Null,
        JsonValue::Bool(b) => PolyValue::Bool(*b),
        JsonValue::Number(n) => match st {
            ScalarType::Int => {
                if let Some(i) = n.as_i64() {
                    PolyValue::Int(i)
                } else {
                    PolyValue::Int(n.as_f64().unwrap_or(0.0) as i64)
                }
            }
            ScalarType::Float => PolyValue::Float(n.as_f64().unwrap_or(0.0)),
            ScalarType::Decimal | ScalarType::String => PolyValue::String(n.to_string()),
            _ => {
                if let Some(i) = n.as_i64() {
                    PolyValue::Int(i)
                } else {
                    PolyValue::Float(n.as_f64().unwrap_or(0.0))
                }
            }
        },
        JsonValue::String(s) => match st {
            ScalarType::Int => {
                if let Ok(i) = s.trim().parse::<i64>() {
                    PolyValue::Int(i)
                } else {
                    PolyValue::String(s.clone())
                }
            }
            ScalarType::Float => {
                if let Ok(f) = s.trim().parse::<f64>() {
                    PolyValue::Float(f)
                } else {
                    PolyValue::String(s.clone())
                }
            }
            ScalarType::Bool => {
                let trimmed = s.trim().to_ascii_lowercase();
                if trimmed == "true" || trimmed == "1" {
                    PolyValue::Bool(true)
                } else if trimmed == "false" || trimmed == "0" {
                    PolyValue::Bool(false)
                } else {
                    PolyValue::String(s.clone())
                }
            }
            _ => PolyValue::String(s.clone()),
        },
        other => PolyValue::from(other.clone()),
    }
}

fn convert_list_item(item: &JsonValue, inner: &ValueType) -> Result<PolyValue> {
    match inner {
        ValueType::Nested(nested_schema) => json_value_to_poly_value(item, nested_schema),
        ValueType::Scalar(st) => Ok(coerce_scalar(item, st)),
        ValueType::List(_) => Ok(PolyValue::from(item.clone())),
    }
}

fn convert_json_field_from_map(
    map: &serde_json::Map<String, JsonValue>,
    field: &FieldSchema,
) -> Result<Option<PolyValue>> {
    let xml_name_str = std::str::from_utf8(&field.xml_name).ok();
    // Priority: match schema alias (xml_name) first, fallback to python field.name
    let found_val = xml_name_str
        .and_then(|alias| map.get(alias))
        .or_else(|| map.get(&field.name));
    let Some(json_field_val) = found_val else {
        return Ok(None);
    };
    if json_field_val.is_null() {
        return Ok(Some(PolyValue::Null));
    }
    let converted = match &field.val_type {
        ValueType::Scalar(st) => coerce_scalar(json_field_val, st),
        ValueType::List(inner) => {
            if let JsonValue::Array(items) = json_field_val {
                let mut poly_list = Vec::with_capacity(items.len());
                for item in items {
                    poly_list.push(convert_list_item(item, inner)?);
                }
                PolyValue::List(poly_list)
            } else {
                // Single item into list
                PolyValue::List(vec![convert_list_item(json_field_val, inner)?])
            }
        }
        ValueType::Nested(nested_schema) => {
            json_value_to_poly_value(json_field_val, nested_schema)?
        }
    };
    Ok(Some(converted))
}

/// Convert a `serde_json::Value` into a `PolyValue` matching a `ModelSchema`.
pub fn json_value_to_poly_value(val: &JsonValue, schema: &ModelSchema) -> Result<PolyValue> {
    match val {
        JsonValue::Object(map) => {
            let resolved_variant = if schema.has_variants() || schema.is_abstract {
                let raw = map
                    .get("@xsi:type")
                    .or_else(|| map.get("$type"))
                    .and_then(|v| v.as_str());
                match raw {
                    Some(raw_type) => {
                        let local = raw_type.rsplit(':').next().unwrap_or(raw_type);
                        match schema
                            .find_variant(local.as_bytes())
                            .filter(|v| !v.is_abstract)
                        {
                            Some(variant) => Some(variant),
                            None if schema.is_abstract => {
                                return Err(PolyXmlError::SchemaError(format!(
                                    "Cannot instantiate abstract schema '{}' with unknown @xsi:type '{}'",
                                    schema.name, raw_type
                                )));
                            }
                            None => None,
                        }
                    }
                    None if schema.is_abstract => {
                        return Err(PolyXmlError::SchemaError(format!(
                            "Cannot instantiate abstract schema '{}' without a valid @xsi:type variant",
                            schema.name
                        )));
                    }
                    None => None,
                }
            } else {
                None
            };

            if let Some(variant_schema) = resolved_variant {
                let mut values = Vec::with_capacity(variant_schema.fields.len());
                for field in &variant_schema.fields {
                    values.push(convert_json_field_from_map(map, field)?);
                }
                return Ok(PolyValue::Record {
                    schema: variant_schema,
                    values: values.into_boxed_slice(),
                });
            }

            let mut obj = HashMap::with_capacity(schema.fields.len());
            for field in &schema.fields {
                if let Some(converted) = convert_json_field_from_map(map, field)? {
                    obj.insert(field.name.clone(), converted);
                }
            }
            Ok(PolyValue::Object(obj))
        }
        other => Ok(PolyValue::from(other.clone())),
    }
}

/// Serialize a `PolyValue` into JSON bytes using a `ModelSchema`.
pub fn serialize_json(
    value: &PolyValue,
    schema: &ModelSchema,
    indent: Option<usize>,
    by_alias: bool,
) -> Result<Vec<u8>> {
    let json_val = poly_value_to_json_value(value, schema, by_alias);
    match indent {
        Some(spaces) if spaces > 0 => {
            let indent_str = " ".repeat(spaces);
            let formatter = serde_json::ser::PrettyFormatter::with_indent(indent_str.as_bytes());
            let mut buf = Vec::new();
            let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
            json_val.serialize(&mut ser)?;
            Ok(buf)
        }
        _ => Ok(serde_json::to_vec(&json_val)?),
    }
}

/// Deserialize JSON bytes into a `PolyValue` using a `ModelSchema`.
pub fn deserialize_json(json_bytes: &[u8], schema: Arc<ModelSchema>) -> Result<PolyValue> {
    let json_val: JsonValue = serde_json::from_slice(json_bytes)?;
    json_value_to_poly_value(&json_val, &schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldKind, FieldSchema};

    #[test]
    fn test_json_roundtrip_primitives() {
        let schema = ModelSchema::builder("User")
            .field(FieldSchema::new(
                "user_id",
                b"userId",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::Int),
            ))
            .field(FieldSchema::new(
                "full_name",
                b"fullName",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::String),
            ))
            .field(FieldSchema::new(
                "score",
                b"score",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::Float),
            ))
            .field(FieldSchema::new(
                "is_admin",
                b"isAdmin",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::Bool),
            ))
            .build();

        let mut map = HashMap::new();
        map.insert("user_id".to_string(), PolyValue::Int(42));
        map.insert(
            "full_name".to_string(),
            PolyValue::String("Ada Lovelace".to_string()),
        );
        map.insert("score".to_string(), PolyValue::Float(99.5));
        map.insert("is_admin".to_string(), PolyValue::Bool(true));
        let poly_val = PolyValue::Object(map);

        // Serialize with by_alias = true
        let json_bytes = serialize_json(&poly_val, &schema, None, true).unwrap();
        let json_str = std::str::from_utf8(&json_bytes).unwrap();
        assert!(json_str.contains("\"userId\":42"));
        assert!(json_str.contains("\"fullName\":\"Ada Lovelace\""));
        assert!(json_str.contains("\"score\":99.5"));
        assert!(json_str.contains("\"isAdmin\":true"));

        // Deserialize back
        let parsed = deserialize_json(&json_bytes, schema).unwrap();
        assert_eq!(parsed.get("user_id"), Some(&PolyValue::Int(42)));
        assert_eq!(
            parsed.get("full_name"),
            Some(&PolyValue::String("Ada Lovelace".to_string()))
        );
        assert_eq!(parsed.get("score"), Some(&PolyValue::Float(99.5)));
        assert_eq!(parsed.get("is_admin"), Some(&PolyValue::Bool(true)));
    }

    #[test]
    fn test_json_deserialization_accepts_python_field_names() {
        let schema = ModelSchema::builder("User")
            .field(FieldSchema::new(
                "user_id",
                b"userId",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::Int),
            ))
            .field(FieldSchema::new(
                "full_name",
                b"fullName",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::String),
            ))
            .build();

        let json = br#"{"user_id": 100, "full_name": "Grace Hopper"}"#;
        let parsed = deserialize_json(json, schema).unwrap();
        assert_eq!(parsed.get("user_id"), Some(&PolyValue::Int(100)));
        assert_eq!(
            parsed.get("full_name"),
            Some(&PolyValue::String("Grace Hopper".to_string()))
        );
    }
}
