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

#[test]
fn cached_patterns_keep_raw_restriction_and_trimmed_pattern_semantics() {
    use polyxml::converters::ValueConverter;
    use polyxml::ir::RestrictionFacets;
    let pattern = ScalarType::Pattern(Box::new(ScalarType::String), vec!["[A-Z]+".into()]);
    let restriction = ScalarType::Restricted(
        Box::new(ScalarType::String),
        Box::new(RestrictionFacets {
            patterns: vec!["[A-Z]+".into()],
            ..Default::default()
        }),
    );
    for _ in 0..3 {
        assert_eq!(
            ValueConverter::parse_scalar(&pattern, b" AB ", "code").unwrap(),
            PolyValue::String(" AB ".into())
        );
        assert!(ValueConverter::parse_scalar(&restriction, b" AB ", "code").is_err());
        assert!(ValueConverter::parse_scalar(&restriction, b"AB", "code").is_ok());
        assert!(ValueConverter::parse_scalar(&pattern, b"AB\nX", "code").is_err());
        let invalid = ScalarType::Pattern(Box::new(ScalarType::String), vec!["[".into()]);
        assert!(ValueConverter::parse_scalar(&invalid, b"AB", "code").is_err());
    }
    let mut schema = ModelSchema::builder("Root")
        .field(FieldSchema::new(
            "code",
            b"Code",
            FieldKind::Element,
            ValueType::Scalar(pattern),
        ))
        .build();
    assert!(deserialize(b"<Root><Code>AB</Code></Root>", Arc::clone(&schema)).is_ok());
    Arc::get_mut(&mut schema).unwrap().fields[0].val_type = ValueType::Scalar(ScalarType::Pattern(
        Box::new(ScalarType::String),
        vec!["[0-9]+".into()],
    ));
    assert!(deserialize(b"<Root><Code>AB</Code></Root>", Arc::clone(&schema)).is_err());
    let value = deserialize(b"<Root><Code>12</Code></Root>", Arc::clone(&schema)).unwrap();
    serialize("Root", &value, &schema, None).unwrap();
    assert!(serialize(
        "Root",
        &object("code", PolyValue::String("AB".into())),
        &schema,
        None
    )
    .is_err());
}

#[test]
fn lexical_list_formatting_preserves_escaping_empty_lists_and_long_numbers() {
    let schema = ModelSchema::builder("Root")
        .field(FieldSchema::new(
            "words",
            b"Words",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::List(Box::new(ScalarType::String))),
        ))
        .field(FieldSchema::new(
            "numbers",
            b"Numbers",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::List(Box::new(ScalarType::Int))),
        ))
        .build();
    for words in [
        vec![],
        vec![
            PolyValue::String("<&".into()),
            PolyValue::String("\"'".into()),
        ],
    ] {
        let value = PolyValue::Object(HashMap::from([
            ("words".into(), PolyValue::List(words)),
            (
                "numbers".into(),
                PolyValue::List(vec![
                    PolyValue::Int(i64::MIN),
                    PolyValue::Int(0),
                    PolyValue::Int(i64::MAX),
                ]),
            ),
        ]));
        for indent in [None, Some(2)] {
            let output = serialize("Root", &value, &schema, indent).unwrap();
            assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
            assert!(String::from_utf8(output)
                .unwrap()
                .contains("-9223372036854775808 0 9223372036854775807"));
        }
    }
}

#[test]
fn unified_scalar_state_keeps_mixed_branch_order_nil_and_nested_frames() {
    use polyxml::schema_parser::XsdParser;
    let ir = XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
    <xs:simpleType name="StateType"><xs:restriction base="xs:string"><xs:enumeration value="Research"/><xs:enumeration value="Ready"/></xs:restriction></xs:simpleType>
    <xs:element name="Root"><xs:complexType mixed="true"><xs:choice minOccurs="0" maxOccurs="unbounded">
      <xs:element name="State" type="StateType"/>
      <xs:element name="Text" type="xs:string" nillable="true"/>
      <xs:element name="Number" type="xs:int"/>
      <xs:element name="Child"><xs:complexType><xs:sequence><xs:element name="Value" type="xs:string"/></xs:sequence></xs:complexType></xs:element>
    </xs:choice></xs:complexType></xs:element></xs:schema>"#).unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    for document in [
        "<Root>start<State>Re<![CDATA[ady]]></State>mid<Number>-7</Number><Text/><State>Ready</State>end</Root>",
        "<Root><Child><Value>nested</Value></Child><Text>after</Text><Number>0</Number></Root>",
    ] {
        let value = deserialize(document.as_bytes(),Arc::clone(&schema)).unwrap();
        let output = serialize("Root",&value,&schema,None).unwrap();
        assert_eq!(value,deserialize(&output,Arc::clone(&schema)).unwrap());
    }
    let nil_document = b"<Root xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance'><Text xsi:nil='true'/><State>Ready</State></Root>";
    assert!(deserialize(nil_document, Arc::clone(&schema)).is_ok());
    assert!(deserialize(b"<Root><State>invalid</State></Root>", schema).is_err());
}
