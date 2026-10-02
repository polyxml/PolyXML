use std::sync::Arc;

use polyxml::error::PolyXmlError;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::value::PolyValue;
use polyxml::{deserialize, deserialize_with_limit, serialize};

#[test]
fn test_xml_entity_roundtrip_escaping() {
    let schema = ModelSchema::builder("Article")
        .field(FieldSchema::new(
            "title",
            b"title",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "category",
            b"category",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    // 1. Input XML with escaped entities
    let xml = br#"<Article category="Tech &amp; Science"><title>Rust &lt;2024&gt; &quot;Edition&quot; &amp; 'Tips'</title></Article>"#;

    let val = deserialize(xml, Arc::clone(&schema)).expect("Failed to deserialize escaped XML");
    assert_eq!(
        val.get("category"),
        Some(&PolyValue::String("Tech & Science".into()))
    );
    assert_eq!(
        val.get("title"),
        Some(&PolyValue::String(
            r#"Rust <2024> "Edition" & 'Tips'"#.into()
        ))
    );

    // 2. Serialize back and verify entities are escaped in the output
    let out = serialize("Article", &val, &schema, None).expect("Serialization failed");
    let out_str = std::str::from_utf8(&out).unwrap();
    assert!(out_str.contains("Tech &amp; Science"));
    assert!(out_str.contains("&lt;2024&gt;"));
    assert!(out_str.contains("&amp;"));

    // 3. Re-deserialize serialized output to confirm perfect roundtrip
    let val2 = deserialize(&out, Arc::clone(&schema)).expect("Re-deserialization failed");
    assert_eq!(val, val2);
}

#[test]
fn test_cdata_section_handling() {
    let schema = ModelSchema::builder("Payload")
        .field(FieldSchema::new(
            "code",
            b"code",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    let xml = br#"
    <Payload>
        <code><![CDATA[function test() { if (x < 10 && y > 20) return "ok"; }]]></code>
    </Payload>
    "#;

    let val = deserialize(xml, Arc::clone(&schema)).expect("Failed to parse CDATA");
    let code_str = val.get("code").and_then(|v| v.as_str()).unwrap();
    assert_eq!(
        code_str,
        r#"function test() { if (x < 10 && y > 20) return "ok"; }"#
    );
}

#[test]
fn test_max_depth_recursion_limit() {
    // Construct 20 levels of nested schemas
    let mut current_schema = ModelSchema::builder("Leaf")
        .field(FieldSchema::new(
            "name",
            b"name",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    for _ in 1..=20 {
        current_schema = ModelSchema::builder("Node")
            .field(FieldSchema::new(
                "name",
                b"name",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::String),
            ))
            .field(FieldSchema::new(
                "child",
                b"Node",
                FieldKind::Element,
                ValueType::Nested(current_schema),
            ))
            .build();
    }
    let node_schema = current_schema;

    // Generate XML nested 20 levels deep
    let mut nested_xml = String::from(r#"<Node name="root">"#);
    for i in 1..=20 {
        nested_xml.push_str(&format!(r#"<Node name="lvl{}">"#, i));
    }
    for _ in 1..=20 {
        nested_xml.push_str("</Node>");
    }
    nested_xml.push_str("</Node>");

    // Parsing with max_depth = 10 should fail with MaxDepthExceeded
    let err = deserialize_with_limit(nested_xml.as_bytes(), Arc::clone(&node_schema), 10)
        .expect_err("Should have exceeded max depth limit");

    match err {
        PolyXmlError::MaxDepthExceeded { max_depth, current } => {
            assert_eq!(max_depth, 10);
            assert!(current >= 10);
        }
        other => panic!("Expected MaxDepthExceeded, got: {:?}", other),
    }

    // Parsing with max_depth = 50 should succeed
    let ok = deserialize_with_limit(nested_xml.as_bytes(), Arc::clone(&node_schema), 50);
    assert!(ok.is_ok());
}

#[test]
fn test_numeric_character_references() {
    let schema = ModelSchema::builder("TextRef")
        .field(FieldSchema::new(
            "val",
            b"val",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    // Decimal &#65; is 'A', hex &#x42; is 'B', &#x26; is '&'
    let xml = br#"<TextRef><val>&#65;&#x42;&#x26;</val></TextRef>"#;
    let val = deserialize(xml, Arc::clone(&schema)).expect("Failed to parse char refs");
    assert_eq!(val.get("val"), Some(&PolyValue::String("AB&".into())));
}

#[test]
fn test_malformed_unclosed_cdata_rejection() {
    let schema = ModelSchema::builder("Doc")
        .field(FieldSchema::new(
            "body",
            b"body",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    let bad_xml = br#"<Doc><body><![CDATA[unclosed cdata</body></Doc>"#;
    assert!(deserialize(bad_xml, schema).is_err());
}

#[test]
fn unsupported_general_entities_are_rejected() {
    let schema = ModelSchema::builder("Document")
        .field(FieldSchema::new(
            "payload",
            b"Payload",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();
    for xml in [
        "<Document><Payload>&audit;</Payload></Document>",
        "<!DOCTYPE Document [<!ENTITY audit 'EXPECTED'>]><Document><Payload>&audit;</Payload></Document>",
        "<!DOCTYPE Document [<!ENTITY audit SYSTEM 'file:///nonexistent/sentinel'>]><Document><Payload>&audit;</Payload></Document>",
        "<Document><Unknown>&audit;</Unknown></Document>",
    ] {
        let error = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap_err();
        assert!(error.to_string().contains("Unsupported general entity reference"), "{error}");
    }
    let value = deserialize(
        b"<Document><Payload>&amp;&#65;&#x42;</Payload></Document>",
        schema,
    )
    .unwrap();
    assert_eq!(value.get("payload"), Some(&PolyValue::String("&AB".into())));
}
