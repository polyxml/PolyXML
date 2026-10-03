use std::collections::HashMap;
use std::sync::Arc;

use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize, PolyValue};

fn object(name: &str, value: PolyValue) -> PolyValue {
    PolyValue::Object(HashMap::from([(name.into(), value)]))
}

fn words(value: &str) -> PolyValue {
    PolyValue::List(vec![PolyValue::String(value.into())])
}

#[test]
fn repeated_lexical_lists_keep_whitespace_validation() {
    let schema = ModelSchema::builder("Root")
        .field(FieldSchema::new(
            "words",
            b"Words",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Scalar(ScalarType::List(Box::new(
                ScalarType::String,
            ))))),
        ))
        .build();
    let valid = object(
        "words",
        PolyValue::List(vec![words("alpha"), words("beta")]),
    );
    let output = serialize("Root", &valid, &schema, None).unwrap();
    assert_eq!(valid, deserialize(&output, Arc::clone(&schema)).unwrap());
    for invalid in [
        "two words",
        "",
        "line\nbreak",
        "tab\tbreak",
        "carriage\rreturn",
    ] {
        let value = object(
            "words",
            PolyValue::List(vec![words("alpha"), words(invalid)]),
        );
        assert!(serialize("Root", &value, &schema, None)
            .unwrap_err()
            .to_string()
            .contains("whitespace-separated list item"));
    }
}

#[test]
fn repeated_nested_records_validate_their_own_lexical_lists() {
    let child = ModelSchema::builder("Child")
        .field(FieldSchema::new(
            "words",
            b"Words",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::List(Box::new(ScalarType::String))),
        ))
        .build();
    let schema = ModelSchema::builder("Root")
        .field(FieldSchema::new(
            "children",
            b"Child",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Nested(child))),
        ))
        .build();
    let valid = object(
        "children",
        PolyValue::List(vec![object("words", words("alpha"))]),
    );
    let output = serialize("Root", &valid, &schema, None).unwrap();
    assert_eq!(valid, deserialize(&output, Arc::clone(&schema)).unwrap());
    let invalid = object(
        "children",
        PolyValue::List(vec![object("words", words("two words"))]),
    );
    assert!(serialize("Root", &invalid, &schema, None)
        .unwrap_err()
        .to_string()
        .contains("whitespace-separated list item"));
}

#[test]
fn indexed_scalar_state_preserves_enum_pattern_and_split_text() {
    let mut schema = ModelSchema::builder("Root")
        .field(FieldSchema::new(
            "states",
            b"State",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Scalar(ScalarType::Enum(vec![
                "R&D".into(),
                "Ready".into(),
            ])))),
        ))
        .field(FieldSchema::new(
            "code",
            b"Code",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Pattern(
                Box::new(ScalarType::String),
                vec!["[A-Z]&[A-Z]".into()],
            )),
        ))
        .build();
    let xml = b"<Root><State>R&amp;<![CDATA[D]]></State><State>Ready</State><Code>A&amp;<![CDATA[B]]></Code></Root>";
    let value = deserialize(xml, Arc::clone(&schema)).unwrap();
    assert_eq!(
        value.get("states"),
        Some(&PolyValue::List(vec![
            PolyValue::String("R&D".into()),
            PolyValue::String("Ready".into())
        ]))
    );
    assert_eq!(value.get("code").and_then(PolyValue::as_str), Some("A&B"));
    assert!(deserialize(b"<Root><State>invalid</State></Root>", Arc::clone(&schema)).is_err());
    assert!(deserialize(b"<Root><Code>invalid</Code></Root>", Arc::clone(&schema)).is_err());
    drop(value);
    // Public schema metadata remains mutable before sharing the Arc. No stale plan.
    Arc::get_mut(&mut schema).unwrap().fields[0].val_type =
        ValueType::List(Box::new(ValueType::Scalar(ScalarType::Enum(vec![
            "Done".into()
        ]))));
    assert!(deserialize(b"<Root><State>Ready</State></Root>", Arc::clone(&schema)).is_err());
    assert!(deserialize(b"<Root><State>Done</State></Root>", schema).is_ok());
}
