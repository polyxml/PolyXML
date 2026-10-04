use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize, PolyValue};
use std::{collections::HashMap, sync::Arc};

fn tagged(kind: &str, value: PolyValue) -> PolyValue {
    PolyValue::Object(HashMap::from([
        ("kind".into(), PolyValue::String(kind.into())),
        ("value".into(), value),
    ]))
}

#[test]
fn large_mixed_tables_keep_first_match_metadata_edits_records_and_nil() {
    let declarations = (0..64)
        .map(|i| format!(r#"<xs:element name="B{i}" type="xs:int"/>"#))
        .collect::<String>();
    let xsd = format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="Root"><xs:complexType mixed="true"><xs:choice minOccurs="0" maxOccurs="unbounded">{declarations}</xs:choice></xs:complexType></xs:element></xs:schema>"#
    );
    let ir = XsdParser::new().parse_str(&xsd).unwrap();
    let mut schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let mixed = schema.mixed_content.as_ref().unwrap();
    let items_name = schema.fields[mixed.items_index].name.clone();
    let kinds = (0..64)
        .map(|i| {
            mixed
                .branches
                .iter()
                .find(|b| b.xml_name == format!("B{i}").as_bytes())
                .unwrap()
                .variant_name
                .clone()
        })
        .collect::<Vec<_>>();
    let first_index = mixed
        .branches
        .iter()
        .position(|b| b.xml_name == b"B0")
        .unwrap();
    let old_kind = kinds[0].clone();
    let record_schema = ModelSchema::builder("Tagged")
        .field(FieldSchema::new(
            "value",
            b"value",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Int),
        ))
        .field(FieldSchema::new(
            "kind",
            b"kind",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();
    let mut items = (0..256)
        .map(|i| tagged(&kinds[i % 64], PolyValue::Int(i as i64)))
        .collect::<Vec<_>>();
    items[0] = PolyValue::Record {
        schema: record_schema,
        values: vec![
            Some(PolyValue::Int(0)),
            Some(PolyValue::String(old_kind.clone())),
        ]
        .into_boxed_slice(),
    };
    items.push(tagged(&kinds[63], PolyValue::Null));
    let mut value = PolyValue::Object(HashMap::from([(
        items_name.clone(),
        PolyValue::List(items),
    )]));
    {
        let mixed = Arc::get_mut(&mut schema)
            .unwrap()
            .mixed_content
            .as_mut()
            .unwrap();
        let mut duplicate = mixed.branches[first_index].clone();
        duplicate.xml_name = b"Duplicate".to_vec();
        mixed.branches.push(duplicate);
    }
    let output = serialize("Root", &value, &schema, None).unwrap();
    assert!(std::str::from_utf8(&output).unwrap().contains("<B0>0</B0>"));
    assert!(!std::str::from_utf8(&output).unwrap().contains("<Duplicate"));
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    Arc::get_mut(&mut schema)
        .unwrap()
        .mixed_content
        .as_mut()
        .unwrap()
        .branches[first_index]
        .variant_name = "changed_kind".into();
    let PolyValue::Object(root) = &mut value else {
        unreachable!()
    };
    let PolyValue::List(items) = root.get_mut(&items_name).unwrap() else {
        unreachable!()
    };
    for item in items.iter_mut() {
        if item.get("kind").and_then(PolyValue::as_str) == Some(old_kind.as_str()) {
            match item {
                PolyValue::Object(fields) => {
                    fields.insert("kind".into(), PolyValue::String("changed_kind".into()));
                }
                PolyValue::Record { values, .. } => {
                    values[1] = Some(PolyValue::String("changed_kind".into()))
                }
                _ => unreachable!(),
            }
        }
    }
    let output = serialize("Root", &value, &schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    let PolyValue::Object(root) = &mut value else {
        unreachable!()
    };
    let PolyValue::List(items) = root.get_mut(&items_name).unwrap() else {
        unreachable!()
    };
    items[0] = tagged("unregistered", PolyValue::Int(0));
    assert!(serialize("Root", &value, &schema, None).is_err());
}
