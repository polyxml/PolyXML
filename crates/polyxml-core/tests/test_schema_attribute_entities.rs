use std::sync::Arc;

use polyxml::ir::{QName, TypeDef, TypeRef};
use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize, PolyValue};

const FIXTURE: &str = include_str!("../../../research/fixtures/schema_attribute_entities.xsd");

#[test]
fn attribute_entities_reach_enum_facets_and_defaults_in_value_space() {
    let ir = XsdParser::new().parse_str(FIXTURE).unwrap();
    let TypeDef::Enum(status) = &ir.types[&QName::local("Status")] else {
        panic!("Status must be an enum");
    };
    assert_eq!(
        status
            .variants
            .iter()
            .map(|v| v.value.as_str())
            .collect::<Vec<_>>(),
        [
            "R&D",
            "\"quoted\" 'value' <tag>",
            "café 😀",
            "literal &amp;"
        ]
    );
    let TypeDef::Simple(code) = &ir.types[&QName::local("Code")] else {
        panic!("Code must retain its pattern");
    };
    assert_eq!(code.facets.patterns, ["[A-Z]&[A-Z]"]);
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    for status in [
        "R&amp;D",
        "&quot;quoted&quot; 'value' &lt;tag&gt;",
        "café 😀",
        "literal &amp;amp;",
    ] {
        let xml =
            format!("<Root><Status>{status}</Status><Code>A&amp;B</Code><Fixed/><Default/></Root>");
        let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
        for (name, expected) in [
            ("fixed", "R&D"),
            ("default", "café"),
            ("label", "<label>"),
            ("mode", "R&D"),
            ("code", "A&B"),
        ] {
            assert_eq!(value.get(name).and_then(PolyValue::as_str), Some(expected));
        }
        let output = serialize("Root", &value, &schema, None).unwrap();
        assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    }
    for content in [
        "<Fixed>wrong</Fixed>",
        "<Code>wrong</Code>",
        "<Status>wrong</Status>",
    ] {
        assert!(deserialize(
            format!("<Root>{content}</Root>").as_bytes(),
            Arc::clone(&schema)
        )
        .is_err());
    }
}

#[test]
fn namespace_and_qname_attributes_are_normalized_consistently() {
    let xml = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSche&#109;a"
        xmlns:t="urn:example:a&amp;b" targetNamespace="urn:example:a&#38;b">
      <xs:simpleType name="Text"><xs:restriction base="xs:str&#105;ng"/></xs:simpleType>
      <xs:simpleType name="Either"><xs:union memberTypes="t:Te&#120;t&#32;xs:int"/></xs:simpleType>
      <xs:complexType name="RootType"><xs:sequence>
        <xs:element name="Value" type="t:Te&#120;t"/>
      </xs:sequence></xs:complexType>
      <xs:element name="Root" type="t:RootType"/>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xml).unwrap();
    assert_eq!(ir.target_namespace.as_deref(), Some("urn:example:a&b"));
    let TypeDef::Union(union) = &ir.types[&QName::new(Some("urn:example:a&b"), "Either")] else {
        panic!("Either must be a union");
    };
    assert_eq!(union.branches.len(), 2);
    assert_eq!(
        union.branches[0].type_ref,
        TypeRef::Named(QName::new(Some("urn:example:a&b"), "Text"))
    );
    ModelSchema::from_ir(&ir, Some("Root")).unwrap();
}

#[test]
fn literal_attribute_whitespace_is_normalized_but_character_references_survive() {
    let xml = "<xs:schema xmlns:xs='http://www.w3.org/2001/XMLSchema'><xs:simpleType name='Text'><xs:restriction base='xs:string'><xs:enumeration value='a\tb\r\nc\nd'/><xs:enumeration value='a&#9;b&#10;c&#13;d'/></xs:restriction></xs:simpleType></xs:schema>";
    let ir = XsdParser::new().parse_str(xml).unwrap();
    let TypeDef::Enum(text) = &ir.types[&QName::local("Text")] else {
        panic!("Text must be an enum")
    };
    assert_eq!(text.variants[0].value, "a b c d");
    assert_eq!(text.variants[1].value, "a\tb\nc\rd");
}

#[test]
fn malformed_attributes_and_undefined_entities_are_errors_even_in_skipped_content() {
    for fragment in [
        "<xs:simpleType name='Text'><xs:restriction base='xs:string'><xs:enumeration value='&missing;'/></xs:restriction></xs:simpleType>",
        "<xs:complexType name='Root'><xs:attribute name='mode' fixed='&#xNOTHEX;'/></xs:complexType>",
        "<xs:annotation><xs:appinfo value='&missing;'/></xs:annotation>",
        "<xs:element name='Root' type='xs:string' type='xs:int'/>",
    ] {
        let xml = format!("<xs:schema xmlns:xs='http://www.w3.org/2001/XMLSchema'>{fragment}</xs:schema>");
        assert!(XsdParser::new().parse_str(&xml).is_err(), "accepted {fragment}");
    }
}
