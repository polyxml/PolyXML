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
