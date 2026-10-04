use polyxml::ir::{FieldKind, PrimitiveType, QName, TypeDef, TypeRef};
use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize, PolyValue};
use std::sync::Arc;

const FIXTURE: &str = include_str!("../../../research/fixtures/inherited_simple_content.xsd");

#[test]
fn inherited_text_resolves_terminal_type_without_losing_struct_bases_or_aliases() {
    let ir = XsdParser::new().parse_str(FIXTURE).unwrap();
    for (name, base, expected) in [
        ("Leaf", "Middle", TypeRef::Primitive(PrimitiveType::Int)),
        ("Middle", "Base", TypeRef::Primitive(PrimitiveType::Int)),
        ("CodeLeaf", "CodeBase", TypeRef::Named(QName::local("Code"))),
        (
            "BoundedLeaf",
            "BoundedBase",
            TypeRef::Named(QName::local("Number")),
        ),
    ] {
        let TypeDef::Struct(value) = &ir.types[&QName::local(name)] else {
            panic!("expected struct")
        };
        assert_eq!(value.base_type, Some(QName::local(base)));
        let text = value
            .fields
            .iter()
            .find(|field| field.kind == FieldKind::Text)
            .unwrap();
        assert_eq!(text.type_ref, expected, "{name}");
    }
}

#[test]
fn inherited_text_round_trips_and_keeps_scalar_constraints() {
    let ir = XsdParser::new().parse_str(FIXTURE).unwrap();
    for (root, xml, expected) in [
        (
            "Root",
            "<Root unit='m' rank='4' tag='T'>7</Root>",
            PolyValue::Int(7),
        ),
        (
            "CodeRoot",
            "<CodeRoot label='code'>AB</CodeRoot>",
            PolyValue::String("AB".into()),
        ),
        (
            "BoundedRoot",
            "<BoundedRoot>7</BoundedRoot>",
            PolyValue::Int(7),
        ),
    ] {
        let schema = ModelSchema::from_ir(&ir, Some(root)).unwrap();
        assert_eq!(
            schema
                .fields
                .iter()
                .filter(|f| f.kind == polyxml::schema::FieldKind::Text)
                .count(),
            1
        );
        let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
        assert_eq!(value.get("value"), Some(&expected), "{root}");
        if root == "Root" {
            assert_eq!(value.get("unit").and_then(PolyValue::as_str), Some("m"));
            assert_eq!(value.get("rank"), Some(&PolyValue::Int(4)));
            assert_eq!(value.get("tag").and_then(PolyValue::as_str), Some("T"));
        }
        let output = serialize(root, &value, &schema, None).unwrap();
        assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    }
    for (root, text) in [("Root", "bad"), ("CodeRoot", "X"), ("BoundedRoot", "0")] {
        let schema = ModelSchema::from_ir(&ir, Some(root)).unwrap();
        assert!(deserialize(format!("<{root}>{text}</{root}>").as_bytes(), schema).is_err());
    }
}

#[test]
fn imported_simple_content_and_parser_cache_keep_terminal_text_types() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("base.xsd"), r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:base"><xs:complexType name="Base"><xs:simpleContent><xs:extension base="xs:int"><xs:attribute name="unit" type="xs:string"/></xs:extension></xs:simpleContent></xs:complexType></xs:schema>"#).unwrap();
    let schema_path = directory.path().join("derived.xsd");
    std::fs::write(&schema_path, r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:b="urn:base" xmlns:t="urn:derived" targetNamespace="urn:derived"><xs:import namespace="urn:base" schemaLocation="base.xsd"/><xs:complexType name="Derived"><xs:simpleContent><xs:extension base="b:Base"/></xs:simpleContent></xs:complexType><xs:element name="Root" type="t:Derived"/></xs:schema>"#).unwrap();
    let mut parser = XsdParser::new();
    let first = parser.parse_file(&schema_path).unwrap();
    let second = parser.parse_file(&schema_path).unwrap();
    assert_eq!(first.types, second.types);
    let TypeDef::Struct(value) = &first.types[&QName::new(Some("urn:derived"), "Derived")] else {
        panic!("expected struct")
    };
    assert_eq!(
        value.fields[0].type_ref,
        TypeRef::Primitive(PrimitiveType::Int)
    );
    let schema = ModelSchema::from_ir(&first, Some("Root")).unwrap();
    let value = deserialize(b"<Root xmlns='urn:derived' unit='m'>7</Root>", schema).unwrap();
    assert_eq!(value.get("value"), Some(&PolyValue::Int(7)));
}

#[test]
fn cyclic_simple_content_is_a_resolution_error() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="A"><xs:simpleContent><xs:extension base="B"/></xs:simpleContent></xs:complexType><xs:complexType name="B"><xs:simpleContent><xs:extension base="A"/></xs:simpleContent></xs:complexType></xs:schema>"#;
    let error = XsdParser::new().parse_str(xsd).unwrap_err();
    assert!(error.to_string().contains("simpleContent"), "{error}");
}

#[test]
fn inherited_text_preserves_enum_union_and_list_names() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="Choice"><xs:restriction base="xs:string"><xs:enumeration value="A"/></xs:restriction></xs:simpleType>
      <xs:simpleType name="Either"><xs:union memberTypes="xs:int xs:string"/></xs:simpleType>
      <xs:simpleType name="Numbers"><xs:list itemType="xs:int"/></xs:simpleType>
      <xs:complexType name="EnumBase"><xs:simpleContent><xs:extension base="Choice"/></xs:simpleContent></xs:complexType>
      <xs:complexType name="EnumLeaf"><xs:simpleContent><xs:extension base="EnumBase"/></xs:simpleContent></xs:complexType>
      <xs:complexType name="UnionBase"><xs:simpleContent><xs:extension base="Either"/></xs:simpleContent></xs:complexType>
      <xs:complexType name="UnionLeaf"><xs:simpleContent><xs:extension base="UnionBase"/></xs:simpleContent></xs:complexType>
      <xs:complexType name="ListBase"><xs:simpleContent><xs:extension base="Numbers"/></xs:simpleContent></xs:complexType>
      <xs:complexType name="ListLeaf"><xs:simpleContent><xs:extension base="ListBase"/></xs:simpleContent></xs:complexType>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    for (leaf, terminal) in [
        ("EnumLeaf", "Choice"),
        ("UnionLeaf", "Either"),
        ("ListLeaf", "Numbers"),
    ] {
        let TypeDef::Struct(value) = &ir.types[&QName::local(leaf)] else {
            panic!("expected struct")
        };
        assert_eq!(
            value.fields[0].type_ref,
            TypeRef::Named(QName::local(terminal))
        );
    }
}
