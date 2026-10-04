use std::hint::black_box;

use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize};
use std::sync::Arc;

fn build_sensor_fixture() -> (Arc<ModelSchema>, Vec<u8>) {
    let schema = ModelSchema::builder("Sensor")
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::Int),
        ))
        .field(FieldSchema::new(
            "temp",
            b"temp",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Float),
        ))
        .field(FieldSchema::new(
            "status",
            b"status",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "active",
            b"active",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Bool),
        ))
        .build();

    let xml = br#"<Sensor id="1001"><temp>23.75</temp><status>OPERATIONAL</status><active>true</active></Sensor>"#.to_vec();
    (schema, xml)
}

fn build_catalog_fixture(count: usize) -> (Arc<ModelSchema>, Vec<u8>) {
    let item_schema = ModelSchema::builder("Item")
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
            "price",
            b"price",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Float),
        ))
        .build();

    let catalog_schema = ModelSchema::builder("Catalog")
        .field(FieldSchema::new(
            "items",
            b"item",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Nested(item_schema))),
        ))
        .build();

    let mut xml = String::with_capacity(count * 64 + 32);
    xml.push_str("<Catalog>");
    for i in 0..count {
        use std::fmt::Write;
        write!(
            xml,
            r#"<item id="{i}"><name>Part-{i}</name><price>{:.2}</price></item>"#,
            (i as f64) * 1.25
        )
        .unwrap();
    }
    xml.push_str("</Catalog>");

    (catalog_schema, xml.into_bytes())
}

