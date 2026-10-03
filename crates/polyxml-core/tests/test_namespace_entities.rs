use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize, PolyValue, XmlItemStream};
use std::{io::Cursor, sync::Arc};

const NS: &str = "urn:polyxml:A&B";
fn value_field() -> FieldSchema {
    FieldSchema::new(
        "value",
        b"Value",
        FieldKind::Element,
        ValueType::Scalar(ScalarType::Int),
    )
}

#[test]
fn escaped_namespace_bindings_match_strict_root_and_round_trip() {
    let schema = ModelSchema::builder("Root")
        .namespace(NS)
        .strict_root(true)
        .field(value_field())
        .build();
    for xml in [
        "<Root xmlns='urn:polyxml:A&amp;B'><Value>7</Value></Root>",
        "<p:Root xmlns:p='urn:polyxml:A&#38;B'><p:Value>7</p:Value></p:Root>",
        "<p:Root xmlns:p='urn:polyxml:A&#x26;B'><p:Value>7</p:Value></p:Root>",
    ] {
        let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
        assert_eq!(value.get("value"), Some(&PolyValue::Int(7)));
        let output = serialize("Root", &value, &schema, None).unwrap();
        assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    }
}

#[test]
fn namespace_entity_values_are_decoded_once() {
    let schema = ModelSchema::builder("Root")
        .namespace("urn:polyxml:literal&amp;")
        .strict_root(true)
        .build();
    let xml = b"<Root xmlns='urn:polyxml:literal&amp;amp;'/>";
    assert!(deserialize(xml, schema).is_ok());
}

fn abstract_schema() -> Arc<ModelSchema> {
    let base = ModelSchema::builder("Root").is_abstract(true).build();
    let concrete = ModelSchema::builder("Derived")
        .namespace(NS)
        .field(value_field())
        .build();
    base.set_variants(vec![concrete]);
    base
}

#[test]
fn entity_decoding_applies_to_inherited_and_local_xsi_type_bindings() {
    let base = abstract_schema();
    for xml in [
  "<Root xmlns:t='urn:polyxml:A&amp;B' xmlns:i='http://www.w3.org/2001/XMLSchema-instance' i:type='t:Derived'><Value>9</Value></Root>",
  "<Root xmlns:t='urn:polyxml:A&#x26;B' xmlns:i='http://www.w3.org/2001/XMLSchema-instance' i:type='t:Derived'/>",
 ] {
  let value=deserialize(xml.as_bytes(),Arc::clone(&base)).unwrap();
  let PolyValue::Record{schema,..}=value else{panic!("expected concrete record")};
  assert_eq!(schema.name,"Derived");
 }
    let xml="<Envelope xmlns:t='urn:polyxml:A&amp;B' xmlns:i='http://www.w3.org/2001/XMLSchema-instance'><Root i:type='t:Derived'><Value>9</Value></Root><Root i:type='t:Derived'/></Envelope>";
    let mut stream = XmlItemStream::new(Cursor::new(xml.as_bytes()), base, b"Root");
    for _ in 0..2 {
        let value = stream.next_item().unwrap().unwrap();
        let PolyValue::Record { schema, .. } = value else {
            panic!("expected concrete record")
        };
        assert_eq!(schema.name, "Derived");
    }
    assert!(stream.next_item().unwrap().is_none());
}

#[test]
fn undefined_entities_cannot_form_namespace_bindings() {
    let schema = ModelSchema::builder("Root").build();
    for xml in [
        "<Root xmlns='urn:polyxml:&undefined;'/>",
        "<Root xmlns:unused='urn:polyxml:&undefined;'></Root>",
        "<Root><Unknown xmlns:u='urn:&undefined;'></Unknown></Root>",
    ] {
        assert!(
            deserialize(xml.as_bytes(), Arc::clone(&schema)).is_err(),
            "{xml}"
        );
    }
}
