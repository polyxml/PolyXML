use std::collections::HashMap;
use std::sync::Arc;

use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::serializer::XmlSerializer;
use polyxml::{deserialize, serialize, PolyValue};

fn schema(namespace: Option<&str>) -> Arc<ModelSchema> {
    let declaration = namespace
        .map(|ns| format!("targetNamespace='{ns}' xmlns:t='{ns}' elementFormDefault='qualified'"))
        .unwrap_or_default();
    let xsd = format!("<xs:schema xmlns:xs='http://www.w3.org/2001/XMLSchema' {declaration}>
      <xs:element name='Root'><xs:complexType mixed='true'><xs:choice minOccurs='0' maxOccurs='unbounded'>
        <xs:element name='Text' type='xs:string' nillable='true'/>
        <xs:element name='Number' type='xs:int' nillable='true'/>
        <xs:element name='Child' nillable='true'><xs:complexType><xs:sequence>
          <xs:element name='Value' type='xs:string'/>
        </xs:sequence></xs:complexType></xs:element>
      </xs:choice></xs:complexType></xs:element></xs:schema>");
    let ir = XsdParser::new().parse_str(&xsd).unwrap();
    ModelSchema::from_ir(&ir, Some("Root")).unwrap()
}

#[test]
fn nil_scalar_and_nested_items_keep_order_and_differ_from_empty_strings() {
    let schema = schema(None);
    for nil in [
        "<Text i:nil='true'/>",
        "<Text i:nil='1'></Text>",
        "<Number i:nil='true'/>",
        "<Child i:nil='true'/>",
    ] {
        let xml = format!("<Root xmlns:i='http://www.w3.org/2001/XMLSchema-instance'>before{nil}<Text/><Child><Value>nested</Value></Child>{nil}<Text>after</Text></Root>");
        let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
        let items_field = &schema.fields[schema.mixed_content.as_ref().unwrap().items_index].name;
        let items = value.get(items_field).and_then(PolyValue::as_list).unwrap();
        assert!(items[1].get("value").unwrap().is_null());
        assert_eq!(items[2].get("value").and_then(PolyValue::as_str), Some(""));
        assert!(items[4].get("value").unwrap().is_null());
        let output = serialize("Root", &value, &schema, None).unwrap();
        assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    }
}

#[test]
fn nil_instance_prefix_does_not_rebind_the_element_namespace() {
    let schema = schema(Some("urn:document"));
    let xml = b"<Root xmlns='urn:document' xmlns:i='http://www.w3.org/2001/XMLSchema-instance'><Text i:nil='true'/><Number i:nil='true'/><Child i:nil='true'/><Text>after</Text></Root>";
    let value = deserialize(xml, Arc::clone(&schema)).unwrap();
    for prefix in ["", "doc", "xsi", "xsi1"] {
        let ns_map = HashMap::from([(prefix.to_owned(), "urn:document".to_owned())]);
        let output = XmlSerializer::serialize_with_options(
            "Root",
            &value,
            &schema,
            None,
            Some(true),
            Some(&ns_map),
        )
        .unwrap();
        let mut reader = quick_xml::NsReader::from_reader(output.as_slice());
        let mut nils = 0;
        loop {
            let (namespace, event) = reader.read_resolved_event().unwrap();
            match event {
                quick_xml::events::Event::Empty(element) => {
                    assert!(
                        matches!(namespace, quick_xml::name::ResolveResult::Bound(uri) if uri.as_ref() == "urn:document")
                    );
                    for attribute in element.attributes() {
                        let attribute = attribute.unwrap();
                        if attribute.key.local_name().as_ref() == "nil" {
                            let (namespace, _) = reader.resolver().resolve_attribute(attribute.key);
                            assert!(
                                matches!(namespace, quick_xml::name::ResolveResult::Bound(uri) if uri.as_ref() == "http://www.w3.org/2001/XMLSchema-instance")
                            );
                            nils += 1;
                        }
                    }
                }
                quick_xml::events::Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(nils, 3);
        assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    }
    let output =
        XmlSerializer::serialize_with_options("Root", &value, &schema, None, Some(false), None)
            .unwrap();
    assert_eq!(value, deserialize(&output, schema).unwrap());
}
