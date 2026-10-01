use polyxml::ir::{FieldKind, PrimitiveType, TypeDef, TypeRef};
use polyxml::schema_parser::XsdParser;
use polyxml::value::PolyValue;

#[test]
fn test_any_attribute_schema_parser_and_runtime_roundtrip() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:element name="Extensible">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="name" type="xs:string"/>
          </xs:sequence>
          <xs:anyAttribute processContents="lax"/>
        </xs:complexType>
      </xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new()
        .parse_str(xsd)
        .expect("schema parse failed");

    // Verify IR contains Extensible struct with any_attribute field of kind AnyAttribute
    let extensible_def = ir
        .types
        .values()
        .find_map(|t| match t {
            TypeDef::Struct(s) if s.qname.local.contains("Extensible") => Some(s),
            _ => None,
        })
        .expect("Extensible struct definition not found in IR");

    let any_attr_field = extensible_def
        .fields
        .iter()
        .find(|f| f.kind == FieldKind::AnyAttribute)
        .expect("FieldKind::AnyAttribute field not found in Extensible struct");

    assert_eq!(any_attr_field.name, "any_attribute");
    assert_eq!(any_attr_field.xml_name, "*");
    assert_eq!(
        any_attr_field.type_ref,
        TypeRef::Primitive(PrimitiveType::AnyType)
    );

    // Convert to dynamic ModelSchema
    let model_schema =
        polyxml::schema::ModelSchema::from_ir(&ir, Some(&extensible_def.qname.local))
            .expect("ModelSchema::from_ir failed");

    assert!(
        model_schema.any_attribute_field.is_some(),
        "any_attribute_field must be tracked in ModelSchema"
    );
    let any_idx = model_schema.any_attribute_field.unwrap();

    // Deserialize XML with foreign/wildcard attributes
    let input_xml = r#"<Extensible extra="val" status="active"><name>test</name></Extensible>"#;
    let parsed_val = polyxml::deserialize(input_xml.as_bytes(), model_schema.clone())
        .expect("deserialization failed");

    match &parsed_val {
        PolyValue::Record { values, .. } => {
            let any_attr_val = values
                .get(any_idx)
                .and_then(|v| v.as_ref())
                .expect("any_attribute value missing in deserialized record");
            match any_attr_val {
                PolyValue::Object(map) => {
                    assert_eq!(
                        map.get("extra"),
                        Some(&PolyValue::String("val".to_string()))
                    );
                    assert_eq!(
                        map.get("status"),
                        Some(&PolyValue::String("active".to_string()))
                    );
                }
                other => panic!(
                    "Expected PolyValue::Object for any_attribute, got {:?}",
                    other
                ),
            }
        }
        other => panic!("Expected PolyValue::Record, got {:?}", other),
    }

    // Serialize back to XML
    let serialized_xml = polyxml::serialize("Extensible", &parsed_val, &model_schema, None)
        .expect("serialization failed");
    let xml_str = String::from_utf8(serialized_xml).expect("valid utf8");

    assert!(xml_str.contains("extra=\"val\""));
    assert!(xml_str.contains("status=\"active\""));
    assert!(xml_str.contains("<name>test</name>"));
}
