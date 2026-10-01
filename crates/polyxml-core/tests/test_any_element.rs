use polyxml::ir::{FieldKind, PrimitiveType, TypeDef, TypeRef};
use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::value::PolyValue;

#[test]
fn test_any_element_schema_parser_and_runtime_roundtrip() {
    let xsd = r###"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:audit:content" targetNamespace="urn:audit:content" elementFormDefault="qualified">
      <xs:element name="Body">
        <xs:complexType>
          <xs:choice>
            <xs:element name="Uri" type="xs:string"/>
            <xs:any namespace="##other" processContents="lax"/>
          </xs:choice>
          <xs:attribute name="content" type="xs:string"/>
        </xs:complexType>
      </xs:element>
    </xs:schema>"###;

    let ir = XsdParser::new()
        .parse_str(xsd)
        .expect("schema parse failed");

    // Verify IR contains Body struct with wildcard element
    let body_def = ir
        .types
        .values()
        .find_map(|t| match t {
            TypeDef::Struct(s) if s.qname.local.contains("Body") => Some(s),
            _ => None,
        })
        .expect("Body struct definition not found in IR");

    let any_field = body_def
        .fields
        .iter()
        .find(|f| f.kind == FieldKind::Any)
        .expect("FieldKind::Any field not found in Body struct");

    assert_eq!(any_field.name, "any");
    assert_eq!(any_field.xml_name, "*");
    assert_eq!(any_field.namespace.as_deref(), Some("##other"));
    assert_eq!(
        any_field.type_ref,
        TypeRef::Primitive(PrimitiveType::AnyType)
    );

    // Convert to dynamic ModelSchema
    let model_schema = ModelSchema::from_ir(&ir, Some(&body_def.qname.local))
        .expect("ModelSchema::from_ir failed");

    assert!(
        model_schema.any_element_field.is_some(),
        "any_element_field must be tracked in ModelSchema"
    );

    // Deserialize XML with wildcard element in foreign namespace
    let input_xml = r#"<Body xmlns="urn:audit:content" xmlns:o="urn:other" content="text"><o:Extra>y</o:Extra></Body>"#;
    let parsed_val = polyxml::deserialize(input_xml.as_bytes(), model_schema.clone())
        .expect("deserialization failed");

    let any_idx = model_schema
        .any_element_field
        .expect("any_element_field must be set");

    let PolyValue::Record { values, .. } = &parsed_val else {
        panic!("expected Record PolyValue");
    };

    let any_val = values
        .get(any_idx)
        .and_then(|v| v.as_ref())
        .expect("any field missing in deserialized record");

    let PolyValue::Object(any_obj) = any_val else {
        panic!("expected Object for wildcard element, got {:?}", any_val);
    };

    assert_eq!(
        any_obj.get("qname"),
        Some(&PolyValue::String("{urn:other}Extra".to_string()))
    );
    assert_eq!(
        any_obj.get("text"),
        Some(&PolyValue::String("y".to_string()))
    );

    // Serialize back to XML
    let serialized =
        polyxml::serialize("Body", &parsed_val, &model_schema, None).expect("serialization failed");
    let serialized_str = String::from_utf8(serialized).expect("invalid utf8 in serialized xml");

    assert!(
        serialized_str.contains("Extra"),
        "serialized XML missing Extra element: {serialized_str}"
    );
    assert!(
        serialized_str.contains('y'),
        "serialized XML missing text content 'y': {serialized_str}"
    );

    // Roundtrip verification: deserializing again must match
    let roundtrip_val = polyxml::deserialize(serialized_str.as_bytes(), model_schema)
        .expect("roundtrip deserialization failed");
    assert_eq!(parsed_val, roundtrip_val);
}

#[test]
fn test_any_element_go_codegen() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:element name="Container">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="id" type="xs:string"/>
            <xs:any minOccurs="0" maxOccurs="unbounded" processContents="lax"/>
          </xs:sequence>
        </xs:complexType>
      </xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new()
        .parse_str(xsd)
        .expect("schema parse failed");

    let go_codegen = polyxml::codegen::GoCodegen::new(polyxml::codegen::GoOptions::default());
    let go_code = go_codegen.generate_module(&ir);

    assert!(
        go_code.contains("type AnyElement struct"),
        "Go codegen must emit AnyElement struct definition when FieldKind::Any is present"
    );
    assert!(
        go_code.contains("XMLName xml.Name"),
        "AnyElement must include XMLName"
    );
    assert!(
        go_code.contains("`xml:\",any,attr\""),
        "AnyElement must include ,any,attr tag"
    );
    assert!(
        go_code.contains("`xml:\",innerxml\""),
        "AnyElement must include ,innerxml tag"
    );
    assert!(
        go_code.contains("[]AnyElement"),
        "Container struct must have []AnyElement field"
    );
    assert!(
        go_code.contains("`xml:\",any\""),
        "Container struct must tag wildcard field with xml:\",any\""
    );
}
