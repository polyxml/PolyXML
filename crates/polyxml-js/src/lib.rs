use std::collections::HashMap;
use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi::JsUnknown;
use napi_derive::napi;

use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::value::PolyValue;

#[napi(object)]
pub struct JsFieldDef {
    pub name: String,
    pub xml_name: String,
    pub kind: String,
    pub scalar_type: Option<String>,
    pub namespace: Option<String>,
    pub nested_schema: Option<JsModelSchema>,
    pub is_list: Option<bool>,
    pub required: Option<bool>,
}

#[napi(object)]
pub struct JsModelSchema {
    pub name: String,
    pub xml_name: Option<String>,
    pub fields: Vec<JsFieldDef>,
    pub namespace: Option<String>,
    pub is_abstract: Option<bool>,
    pub variants: Option<Vec<JsModelSchema>>,
}

fn convert_js_schema(schema: &JsModelSchema) -> Arc<ModelSchema> {
    let mut builder = ModelSchema::builder(&schema.name);
    if let Some(xml_name) = &schema.xml_name {
        builder = builder.xml_name(xml_name.as_bytes());
    }
    if let Some(is_abstract) = schema.is_abstract {
        builder = builder.is_abstract(is_abstract);
    }
    if let Some(ns) = &schema.namespace {
        builder = builder.namespace(ns);
    }
    for f in &schema.fields {
        let kind = match f.kind.as_str() {
            "attribute" => FieldKind::Attribute,
            "text" => FieldKind::Text,
            _ => FieldKind::Element,
        };
        let base_val_type = if let Some(nested) = &f.nested_schema {
            ValueType::Nested(convert_js_schema(nested))
        } else {
            let sc = match f.scalar_type.as_deref().unwrap_or("string") {
                "int" => ScalarType::Int,
                "float" => ScalarType::Float,
                "bool" => ScalarType::Bool,
                "decimal" => ScalarType::Decimal,
                "xml_date" => ScalarType::XmlDate,
                "xml_datetime" => ScalarType::XmlDateTime,
                _ => ScalarType::String,
            };
            ValueType::Scalar(sc)
        };
        let val_type = if f.is_list.unwrap_or(false) {
            ValueType::List(Box::new(base_val_type))
        } else {
            base_val_type
        };
        let mut field = FieldSchema::new(&f.name, f.xml_name.as_bytes(), kind, val_type);
        if let Some(ns) = &f.namespace {
            field = field.namespace(ns);
        }
        if f.required.unwrap_or(false) {
            field = field.required();
        }
        builder = builder.field(field);
    }
    let built = builder.build();
    if let Some(variants) = &schema.variants {
        built.set_variants(variants.iter().map(convert_js_schema).collect());
    }
    built
}

fn field_nested_schema(field: &FieldSchema) -> Option<&ModelSchema> {
    match &field.val_type {
        ValueType::Nested(s) => Some(s.as_ref()),
        ValueType::List(inner) => match inner.as_ref() {
            ValueType::Nested(s) => Some(s.as_ref()),
            _ => None,
        },
        ValueType::Scalar(_) => None,
    }
}

fn poly_value_to_js(
    env: &Env,
    val: &PolyValue,
    expected_schema: Option<&ModelSchema>,
) -> Result<JsUnknown> {
    match val {
        PolyValue::Null => Ok(env.get_null()?.into_unknown()),
        PolyValue::Bool(b) => Ok(env.get_boolean(*b)?.into_unknown()),
        PolyValue::Int(i) => Ok(env.create_int64(*i)?.into_unknown()),
        PolyValue::Float(f) => Ok(env.create_double(*f)?.into_unknown()),
        PolyValue::String(s) => Ok(env.create_string(s)?.into_unknown()),
        PolyValue::List(list) => {
            let mut arr = env.create_array(list.len() as u32)?;
            for (idx, item) in list.iter().enumerate() {
                let js_item = poly_value_to_js(env, item, expected_schema)?;
                arr.set(idx as u32, js_item)?;
            }
            Ok(arr.coerce_to_object()?.into_unknown())
        }
        PolyValue::Object(map) => {
            let mut obj = env.create_object()?;
            for (k, v) in map {
                let child_expected = expected_schema
                    .and_then(|exp| exp.fields.iter().find(|f| f.name == *k))
                    .and_then(field_nested_schema);
                let js_val = poly_value_to_js(env, v, child_expected)?;
                obj.set(k.as_str(), js_val)?;
            }
            Ok(obj.into_unknown())
        }
        PolyValue::Record { schema, values } => {
            let mut obj = env.create_object()?;
            if expected_schema
                .is_some_and(|exp| schema.name != exp.name && exp.matches_variant(schema))
            {
                let xsi_type = String::from_utf8_lossy(&schema.xml_name);
                obj.set("@xsi:type", xsi_type.as_ref())?;
            }
            for (idx, field) in schema.fields.iter().enumerate() {
                if let Some(Some(item)) = values.get(idx) {
                    let js_val = poly_value_to_js(env, item, field_nested_schema(field))?;
                    obj.set(field.name.as_str(), js_val)?;
                }
            }
            Ok(obj.into_unknown())
        }
    }
}

#[napi]
pub fn deserialize(
    env: Env,
    xml: Either<String, Buffer>,
    schema: JsModelSchema,
) -> Result<JsUnknown> {
    let xml_bytes: &[u8] = match &xml {
        Either::A(s) => s.as_bytes(),
        Either::B(b) => b.as_ref(),
    };

    let model_schema = convert_js_schema(&schema);
    let val = polyxml::deserialize(xml_bytes, Arc::clone(&model_schema))
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;

    poly_value_to_js(&env, &val, Some(model_schema.as_ref()))
}

fn js_to_scalar(val: JsUnknown, scalar_type: &ScalarType) -> Result<PolyValue> {
    let val_type = val.get_type()?;
    if val_type == napi::ValueType::Null || val_type == napi::ValueType::Undefined {
        return Ok(PolyValue::Null);
    }
    match scalar_type {
        ScalarType::Int => {
            let num = val.coerce_to_number()?;
            Ok(PolyValue::Int(num.get_int64()?))
        }
        ScalarType::Float => {
            let num = val.coerce_to_number()?;
            Ok(PolyValue::Float(num.get_double()?))
        }
        ScalarType::Bool => {
            let b = val.coerce_to_bool()?;
            Ok(PolyValue::Bool(b.get_value()?))
        }
        _ => {
            let s = val.coerce_to_string()?;
            let utf8 = s.into_utf8()?;
            Ok(PolyValue::String(utf8.as_str()?.to_string()))
        }
    }
}

fn js_to_field_poly_value(env: &Env, val: JsUnknown, val_type: &ValueType) -> Result<PolyValue> {
    let js_type = val.get_type()?;
    if js_type == napi::ValueType::Null || js_type == napi::ValueType::Undefined {
        return Ok(PolyValue::Null);
    }
    match val_type {
        ValueType::Scalar(st) => js_to_scalar(val, st),
        ValueType::Nested(nested) => js_to_poly_value(env, val, nested),
        ValueType::List(inner) => {
            let obj = val.coerce_to_object()?;
            if obj.is_array()? {
                let len = obj.get_array_length()?;
                let mut items = Vec::with_capacity(len as usize);
                for idx in 0..len {
                    let elem: JsUnknown = obj.get_element(idx)?;
                    items.push(js_to_field_poly_value(env, elem, inner)?);
                }
                Ok(PolyValue::List(items))
            } else {
                Ok(PolyValue::List(vec![js_to_field_poly_value(
                    env,
                    obj.into_unknown(),
                    inner,
                )?]))
            }
        }
    }
}

fn read_optional_string_prop(obj: &napi::JsObject, name: &str) -> Result<Option<String>> {
    if !obj.has_named_property(name)? {
        return Ok(None);
    }
    let prop: JsUnknown = obj.get_named_property(name)?;
    if matches!(
        prop.get_type()?,
        napi::ValueType::Null | napi::ValueType::Undefined
    ) {
        return Ok(None);
    }
    let s = prop.coerce_to_string()?;
    let utf8 = s.into_utf8()?;
    Ok(Some(utf8.as_str()?.to_string()))
}

fn js_to_poly_value(env: &Env, val: JsUnknown, schema: &ModelSchema) -> Result<PolyValue> {
    let val_type = val.get_type()?;
    if val_type == napi::ValueType::Null || val_type == napi::ValueType::Undefined {
        return Ok(PolyValue::Null);
    }
    if val_type != napi::ValueType::Object {
        return Err(Error::new(
            Status::InvalidArg,
            format!("Expected object for schema '{}'", schema.name),
        ));
    }
    let obj = val.coerce_to_object()?;
    let resolved_variant = if schema.has_variants() || schema.is_abstract {
        let raw_type = read_optional_string_prop(&obj, "@xsi:type")?
            .or(read_optional_string_prop(&obj, "$type")?);
        match raw_type {
            Some(raw) => {
                let local = raw.rsplit(':').next().unwrap_or(&raw);
                match schema
                    .find_variant(local.as_bytes())
                    .filter(|v| !v.is_abstract)
                {
                    Some(v) => Some(v),
                    None if schema.is_abstract => {
                        return Err(Error::new(
                            Status::InvalidArg,
                            format!(
                                "Cannot instantiate abstract schema '{}' with unknown @xsi:type '{}'",
                                schema.name, raw
                            ),
                        ));
                    }
                    None => None,
                }
            }
            None if schema.is_abstract => {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!(
                        "Cannot instantiate abstract schema '{}' without a valid @xsi:type variant",
                        schema.name
                    ),
                ));
            }
            None => None,
        }
    } else {
        None
    };

    if let Some(variant_schema) = resolved_variant {
        let mut values = Vec::with_capacity(variant_schema.fields.len());
        for f in &variant_schema.fields {
            if obj.has_named_property(&f.name)? {
                let prop: JsUnknown = obj.get_named_property(&f.name)?;
                values.push(Some(js_to_field_poly_value(env, prop, &f.val_type)?));
            } else {
                values.push(None);
            }
        }
        return Ok(PolyValue::Record {
            schema: variant_schema,
            values: values.into_boxed_slice(),
        });
    }

    let mut map = HashMap::new();
    for f in &schema.fields {
        if obj.has_named_property(&f.name)? {
            let prop: JsUnknown = obj.get_named_property(&f.name)?;
            let pv = js_to_field_poly_value(env, prop, &f.val_type)?;
            map.insert(f.name.clone(), pv);
        }
    }
    Ok(PolyValue::Object(map))
}

fn poly_value_has_variant_record(val: &PolyValue, schema: &ModelSchema) -> bool {
    match val {
        PolyValue::Record { schema: rec, .. } if schema.matches_variant(rec) => true,
        PolyValue::Record {
            schema: rec,
            values,
        } => rec.fields.iter().zip(values.iter()).any(|(f, v)| {
            v.as_ref().is_some_and(|pv| {
                field_nested_schema(f)
                    .is_some_and(|nested| poly_value_has_variant_record(pv, nested))
            })
        }),
        PolyValue::Object(map) => schema.fields.iter().any(|f| {
            map.get(&f.name).is_some_and(|pv| {
                field_nested_schema(f)
                    .is_some_and(|nested| poly_value_has_variant_record(pv, nested))
            })
        }),
        PolyValue::List(items) => items
            .iter()
            .any(|pv| poly_value_has_variant_record(pv, schema)),
        _ => false,
    }
}

#[napi]
pub fn serialize(
    env: Env,
    root_name: String,
    value: napi::JsObject,
    schema: JsModelSchema,
    indent: Option<u32>,
    namespaces: Option<bool>,
    ns_map: Option<HashMap<String, String>>,
) -> Result<Buffer> {
    let model_schema = convert_js_schema(&schema);
    let poly_val = js_to_poly_value(&env, value.into_unknown(), &model_schema)?;
    let effective_namespaces =
        if namespaces.is_none() && poly_value_has_variant_record(&poly_val, &model_schema) {
            Some(true)
        } else {
            namespaces
        };
    let indent_opt = indent.map(|i| i as usize);
    let bytes = polyxml::serialize_with_options(
        &root_name,
        &poly_val,
        &model_schema,
        indent_opt,
        effective_namespaces,
        ns_map.as_ref(),
    )
    .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))?;
    Ok(bytes.into())
}

#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_js_schema_supports_nested_lists_required_and_variants() {
        let address_schema = JsModelSchema {
            name: "Address".to_string(),
            xml_name: None,
            namespace: None,
            is_abstract: None,
            variants: None,
            fields: vec![
                JsFieldDef {
                    name: "city".to_string(),
                    xml_name: "city".to_string(),
                    kind: "element".to_string(),
                    scalar_type: Some("string".to_string()),
                    namespace: None,
                    nested_schema: None,
                    is_list: None,
                    required: Some(true),
                },
                JsFieldDef {
                    name: "zip".to_string(),
                    xml_name: "zip".to_string(),
                    kind: "element".to_string(),
                    scalar_type: Some("int".to_string()),
                    namespace: None,
                    nested_schema: None,
                    is_list: None,
                    required: None,
                },
            ],
        };

        let car_variant = JsModelSchema {
            name: "Car".to_string(),
            xml_name: Some("Car".to_string()),
            namespace: None,
            is_abstract: None,
            variants: None,
            fields: vec![
                JsFieldDef {
                    name: "id".to_string(),
                    xml_name: "id".to_string(),
                    kind: "element".to_string(),
                    scalar_type: Some("string".to_string()),
                    namespace: None,
                    nested_schema: None,
                    is_list: None,
                    required: Some(true),
                },
                JsFieldDef {
                    name: "doors".to_string(),
                    xml_name: "doors".to_string(),
                    kind: "element".to_string(),
                    scalar_type: Some("int".to_string()),
                    namespace: None,
                    nested_schema: None,
                    is_list: None,
                    required: None,
                },
            ],
        };

        let vehicle_schema = JsModelSchema {
            name: "Vehicle".to_string(),
            xml_name: Some("Vehicle".to_string()),
            namespace: None,
            is_abstract: Some(true),
            variants: Some(vec![car_variant]),
            fields: vec![JsFieldDef {
                name: "id".to_string(),
                xml_name: "id".to_string(),
                kind: "element".to_string(),
                scalar_type: Some("string".to_string()),
                namespace: None,
                nested_schema: None,
                is_list: None,
                required: Some(true),
            }],
        };

        let order_schema = JsModelSchema {
            name: "Order".to_string(),
            xml_name: Some("Order".to_string()),
            namespace: None,
            is_abstract: None,
            variants: None,
            fields: vec![
                JsFieldDef {
                    name: "ship_to".to_string(),
                    xml_name: "shipTo".to_string(),
                    kind: "element".to_string(),
                    scalar_type: None,
                    namespace: None,
                    nested_schema: Some(address_schema),
                    is_list: None,
                    required: Some(true),
                },
                JsFieldDef {
                    name: "tags".to_string(),
                    xml_name: "tag".to_string(),
                    kind: "element".to_string(),
                    scalar_type: Some("string".to_string()),
                    namespace: None,
                    nested_schema: None,
                    is_list: Some(true),
                    required: None,
                },
                JsFieldDef {
                    name: "vehicles".to_string(),
                    xml_name: "vehicle".to_string(),
                    kind: "element".to_string(),
                    scalar_type: None,
                    namespace: None,
                    nested_schema: Some(vehicle_schema),
                    is_list: Some(true),
                    required: None,
                },
            ],
        };

        let model = convert_js_schema(&order_schema);
        assert!(model.fields[0].required);
        if let ValueType::Nested(ship_to_schema) = &model.fields[0].val_type {
            assert!(ship_to_schema.fields[0].required);
            assert!(!ship_to_schema.fields[1].required);
        } else {
            panic!("expected ValueType::Nested for ship_to");
        }
        assert!(matches!(model.fields[1].val_type, ValueType::List(_)));

        let xml = br#"<Order xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
            <shipTo><city>Austin</city><zip>78701</zip></shipTo>
            <tag>priority</tag>
            <tag>fragile</tag>
            <vehicle xsi:type="Car"><id>v1</id><doors>4</doors></vehicle>
        </Order>"#;
        let parsed = polyxml::deserialize(xml, Arc::clone(&model)).expect("deserialize order");
        let ship_to = parsed.get("ship_to").expect("ship_to");
        assert_eq!(
            ship_to.get("city"),
            Some(&PolyValue::String("Austin".into()))
        );
        assert_eq!(ship_to.get("zip"), Some(&PolyValue::Int(78701)));

        let tags = match parsed.get("tags") {
            Some(PolyValue::List(items)) => items,
            other => panic!("expected tags list, got {other:?}"),
        };
        assert_eq!(tags.len(), 2);

        let vehicles = match parsed.get("vehicles") {
            Some(PolyValue::List(items)) => items,
            other => panic!("expected vehicles list, got {other:?}"),
        };
        assert_eq!(vehicles[0].get("doors"), Some(&PolyValue::Int(4)));

        let out_xml =
            polyxml::serialize_with_options("Order", &parsed, &model, None, Some(true), None)
                .expect("serialize roundtrip");
        let out_str = std::str::from_utf8(&out_xml).unwrap();
        assert!(out_str.contains("<shipTo><city>Austin</city><zip>78701</zip></shipTo>"));
        assert!(out_str.contains("xsi:type=\"Car\""));
    }
}
