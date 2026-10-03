pub mod codegen;
pub mod converters;
pub mod error;
pub mod integer;
pub mod ir;
pub mod json;
pub mod parser;
mod pattern;
pub mod schema;
pub mod schema_parser;
pub mod serializer;
pub mod transcoder;
pub mod value;

pub use error::{PolyXmlError, Result};
pub use json::{deserialize_json, serialize_json};
pub use parser::{XmlDeserializer, XmlItemStream};
pub use schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
pub use serializer::XmlSerializer;
pub use transcoder::{json_to_xml, xml_to_json};
pub use value::PolyValue;

use std::sync::Arc;

/// High-level function to deserialize XML bytes into a dynamic PolyValue using a ModelSchema.
pub fn deserialize(xml_bytes: &[u8], schema: Arc<ModelSchema>) -> Result<PolyValue> {
    XmlDeserializer::deserialize(xml_bytes, schema)
}

/// High-level function to deserialize XML bytes with a custom maximum recursion depth limit.
pub fn deserialize_with_limit(
    xml_bytes: &[u8],
    schema: Arc<ModelSchema>,
    max_depth: usize,
) -> Result<PolyValue> {
    XmlDeserializer::deserialize_with_limit(xml_bytes, schema, max_depth)
}

/// High-level function to serialize a PolyValue into XML bytes using a ModelSchema.
pub fn serialize(
    root_name: &str,
    value: &PolyValue,
    schema: &ModelSchema,
    indent: Option<usize>,
) -> Result<Vec<u8>> {
    XmlSerializer::serialize(root_name, value, schema, indent)
}

/// High-level function to serialize a PolyValue with optional namespace support and custom prefix mapping.
pub fn serialize_with_options(
    root_name: &str,
    value: &PolyValue,
    schema: &ModelSchema,
    indent: Option<usize>,
    enable_namespaces: Option<bool>,
    ns_map: Option<&std::collections::HashMap<String, String>>,
) -> Result<Vec<u8>> {
    XmlSerializer::serialize_with_options(
        root_name,
        value,
        schema,
        indent,
        enable_namespaces,
        ns_map,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_serialization_deserialization() {
        // 1. Build schema
        let schema = ModelSchema::builder("User")
            .field(FieldSchema::new(
                "id",
                b"id",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::Int),
            ))
            .field(FieldSchema::new(
                "name",
                b"name",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::String),
            ))
            .field(FieldSchema::new(
                "score",
                b"score",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::Float),
            ))
            .field(FieldSchema::new(
                "active",
                b"active",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::Bool),
            ))
            .build();

        let xml = br#"<User id="101"><name>Ada Lovelace</name><score>99.5</score><active>true</active></User>"#;

        // 2. Deserialize
        let val = deserialize(xml, Arc::clone(&schema)).expect("deserialization failed");
        assert_eq!(val.get("id"), Some(&PolyValue::Int(101)));
        assert_eq!(
            val.get("name"),
            Some(&PolyValue::String("Ada Lovelace".into()))
        );
        assert_eq!(val.get("score"), Some(&PolyValue::Float(99.5)));
        assert_eq!(val.get("active"), Some(&PolyValue::Bool(true)));

        // 3. Serialize back
        let output_xml = serialize("User", &val, &schema, None).expect("serialization failed");
        let output_str = std::str::from_utf8(&output_xml).unwrap();

        // 4. Verify roundtrip output contains all tags
        assert!(output_str.contains(r#"id="101""#));
        assert!(output_str.contains("<name>Ada Lovelace</name>"));
        assert!(output_str.contains("<score>99.5</score>"));
        assert!(output_str.contains("<active>true</active>"));

        // 5. Re-deserialize from generated XML
        let val2 =
            deserialize(&output_xml, Arc::clone(&schema)).expect("re-deserialization failed");
        assert_eq!(val, val2);
    }

    #[test]
    fn test_nested_elements_and_lists() {
        let item_schema = ModelSchema::builder("Item")
            .field(FieldSchema::new(
                "title",
                b"title",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::String),
            ))
            .field(FieldSchema::new(
                "qty",
                b"qty",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::Int),
            ))
            .build();

        let order_schema = ModelSchema::builder("Order")
            .field(FieldSchema::new(
                "order_id",
                b"id",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::String),
            ))
            .field(FieldSchema::new(
                "items",
                b"item",
                FieldKind::Element,
                ValueType::List(Box::new(ValueType::Nested(Arc::clone(&item_schema)))),
            ))
            .build();

        let xml = br#"
        <Order id="ORD-999">
            <item qty="2"><title>Widget A</title></item>
            <item qty="5"><title>Widget B</title></item>
        </Order>
        "#;

        let val =
            deserialize(xml, Arc::clone(&order_schema)).expect("nested deserialization failed");
        assert_eq!(
            val.get("order_id"),
            Some(&PolyValue::String("ORD-999".into()))
        );

        let items = val.get("items").unwrap().as_list().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].get("title"),
            Some(&PolyValue::String("Widget A".into()))
        );
        assert_eq!(items[0].get("qty"), Some(&PolyValue::Int(2)));
        assert_eq!(
            items[1].get("title"),
            Some(&PolyValue::String("Widget B".into()))
        );
        assert_eq!(items[1].get("qty"), Some(&PolyValue::Int(5)));
    }

    #[test]
    fn test_streaming_xml_item_stream() {
        let item_schema = ModelSchema::builder("Item")
            .field(FieldSchema::new(
                "title",
                b"title",
                FieldKind::Element,
                ValueType::Scalar(ScalarType::String),
            ))
            .field(FieldSchema::new(
                "qty",
                b"qty",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::Int),
            ))
            .build();

        let xml = br#"
        <Inventory>
            <meta><timestamp>12345</timestamp></meta>
            <items>
                <item qty="10"><title>Part A</title></item>
                <item qty="20"><title>Part B</title></item>
                <item qty="30"><title>Part C</title></item>
            </items>
        </Inventory>
        "#;

        let mut stream = XmlItemStream::new(&xml[..], Arc::clone(&item_schema), b"item");
        let item1 = stream.next_item().unwrap().expect("item 1");
        assert_eq!(
            item1.get("title"),
            Some(&PolyValue::String("Part A".into()))
        );
        assert_eq!(item1.get("qty"), Some(&PolyValue::Int(10)));

        let item2 = stream.next_item().unwrap().expect("item 2");
        assert_eq!(
            item2.get("title"),
            Some(&PolyValue::String("Part B".into()))
        );
        assert_eq!(item2.get("qty"), Some(&PolyValue::Int(20)));

        let item3 = stream.next_item().unwrap().expect("item 3");
        assert_eq!(
            item3.get("title"),
            Some(&PolyValue::String("Part C".into()))
        );
        assert_eq!(item3.get("qty"), Some(&PolyValue::Int(30)));

        let item4 = stream.next_item().unwrap();
        assert!(item4.is_none());
    }

    #[test]
    fn test_namespaced_serialization() {
        use std::collections::HashMap;

        let schema = ModelSchema::builder("Item")
            .xml_name(b"item")
            .namespace("http://example.com/ns1")
            .field(
                FieldSchema::new(
                    "title",
                    b"title",
                    FieldKind::Element,
                    ValueType::Scalar(ScalarType::String),
                )
                .namespace("http://example.com/ns1"),
            )
            .field(
                FieldSchema::new(
                    "sku",
                    b"sku",
                    FieldKind::Attribute,
                    ValueType::Scalar(ScalarType::String),
                )
                .namespace("http://example.com/ns2"),
            )
            .field(FieldSchema::new(
                "unprefixed_attr",
                b"unprefixed_attr",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::Int),
            ))
            .build();

        let mut obj = HashMap::new();
        obj.insert("title".into(), PolyValue::String("Laptop".into()));
        obj.insert("sku".into(), PolyValue::String("SKU123".into()));
        obj.insert("unprefixed_attr".into(), PolyValue::Int(42));
        let poly_val = PolyValue::Object(obj);

        // 1. Auto-detected namespaces
        let xml_bytes = serialize("item", &poly_val, &schema, None).unwrap();
        let xml_str = std::str::from_utf8(&xml_bytes).unwrap();

        assert!(xml_str.contains("xmlns:ns0=\"http://example.com/ns1\""));
        assert!(xml_str.contains("xmlns:ns1=\"http://example.com/ns2\""));
        assert!(xml_str.starts_with("<ns0:item"));
        assert!(xml_str.contains("ns1:sku=\"SKU123\""));
        assert!(xml_str.contains("unprefixed_attr=\"42\""));
        assert!(xml_str.contains("<ns0:title>Laptop</ns0:title>"));
        assert!(xml_str.ends_with("</ns0:item>"));

        // 2. Custom ns_map with default namespace for ns1
        let mut custom_map = HashMap::new();
        custom_map.insert("".into(), "http://example.com/ns1".into());
        custom_map.insert("inv".into(), "http://example.com/ns2".into());

        let custom_xml_bytes = serialize_with_options(
            "item",
            &poly_val,
            &schema,
            None,
            Some(true),
            Some(&custom_map),
        )
        .unwrap();
        let custom_str = std::str::from_utf8(&custom_xml_bytes).unwrap();

        assert!(custom_str.contains("xmlns=\"http://example.com/ns1\""));
        assert!(custom_str.contains("xmlns:inv=\"http://example.com/ns2\""));
        assert!(custom_str.starts_with("<item"));
        assert!(custom_str.contains("inv:sku=\"SKU123\""));
        assert!(custom_str.contains("<title>Laptop</title>"));
        assert!(custom_str.ends_with("</item>"));

        // 3. Explicitly disabled namespaces (fast-path)
        let raw_xml_bytes =
            serialize_with_options("item", &poly_val, &schema, None, Some(false), None).unwrap();
        let raw_str = std::str::from_utf8(&raw_xml_bytes).unwrap();
        assert!(!raw_str.contains("xmlns"));
        assert!(raw_str.starts_with("<item"));
        assert!(raw_str.contains("sku=\"SKU123\""));
        assert!(raw_str.contains("<title>Laptop</title>"));

        // 4. Verify deserialization roundtrip ignores xmlns attributes and handles prefixed elements
        let roundtrip_val = deserialize(&xml_bytes, Arc::clone(&schema)).unwrap();
        assert_eq!(
            roundtrip_val.get("title"),
            Some(&PolyValue::String("Laptop".into()))
        );
        assert_eq!(
            roundtrip_val.get("sku"),
            Some(&PolyValue::String("SKU123".into()))
        );
        assert_eq!(
            roundtrip_val.get("unprefixed_attr"),
            Some(&PolyValue::Int(42))
        );
    }

    #[test]
    fn test_deserialization_does_not_confuse_xmlns_with_attributes() {
        // Schema has an attribute named "prefix"
        let schema = ModelSchema::builder("Item")
            .field(FieldSchema::new(
                "prefix",
                b"prefix",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::String),
            ))
            .build();

        // XML defines xmlns:prefix="http://example.com" and an actual attribute prefix="actual_val"
        let xml = br#"<Item xmlns:prefix="http://example.com" prefix="actual_val" />"#;
        let val = deserialize(xml, Arc::clone(&schema)).unwrap();
        assert_eq!(
            val.get("prefix"),
            Some(&PolyValue::String("actual_val".into()))
        );

        // XML defines xmlns:prefix="http://example.com" but NO actual attribute prefix
        let xml2 = br#"<Item xmlns:prefix="http://example.com" />"#;
        let val2 = deserialize(xml2, Arc::clone(&schema)).unwrap();
        assert_eq!(val2.get("prefix"), None);
    }
}
