//! Regression tests for issue #162: escaped XSD enumeration values and
//! `xsi:nil` scalar children inside mixed-content complex types.

use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize};
use std::sync::Arc;

#[test]
fn escaped_enumeration_value_round_trips() {
    let ir = XsdParser::new()
        .parse_str(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
              <xs:simpleType name="StateType"><xs:restriction base="xs:string">
                <xs:enumeration value="R&amp;D"/><xs:enumeration value="Ready"/>
              </xs:restriction></xs:simpleType>
              <xs:element name="Root" type="StateType"/>
            </xs:schema>"#,
        )
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let value = deserialize(b"<Root>R&amp;D</Root>", Arc::clone(&schema))
        .expect("escaped enumeration value must be accepted");
    let xml = serialize("Root", &value, &schema, None).unwrap();
    assert!(
        String::from_utf8_lossy(&xml).contains("R&amp;D"),
        "serialized output must re-escape the value, got: {}",
        String::from_utf8_lossy(&xml)
    );
}

#[test]
fn nil_scalar_in_mixed_content_serializes() {
    let ir = XsdParser::new()
        .parse_str(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
              <xs:element name="Root"><xs:complexType mixed="true">
                <xs:choice minOccurs="0" maxOccurs="unbounded">
                  <xs:element name="Text" type="xs:string" nillable="true"/>
                  <xs:element name="State" type="xs:string"/>
                </xs:choice>
              </xs:complexType></xs:element>
            </xs:schema>"#,
        )
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let value = deserialize(
        b"<Root xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance'><Text xsi:nil='true'/><State>Ready</State></Root>",
        Arc::clone(&schema),
    )
    .unwrap();
    let xml = serialize("Root", &value, &schema, None).expect("nil mixed scalar must serialize");
    let text = String::from_utf8_lossy(&xml);
    assert!(text.contains("nil"), "expected xsi:nil in output: {text}");
    assert!(text.contains("<State>Ready</State>"), "got: {text}");
}
