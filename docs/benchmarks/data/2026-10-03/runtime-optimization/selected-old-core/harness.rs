use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
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
    let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
    let output = serialize("Sensor", &value, &schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
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

    let xml = xml.into_bytes();
    let value = deserialize(&xml, Arc::clone(&catalog_schema)).unwrap();
    assert_eq!(value.get("items").unwrap().as_list().unwrap().len(), count);
    let output = serialize("Catalog", &value, &catalog_schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&catalog_schema)).unwrap());
    (catalog_schema, xml)
}

fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialization");

    // 1. Sensor Telemetry (Micro)
    {
        let (schema, xml) = build_sensor_fixture();
        group.throughput(Throughput::Bytes(xml.len() as u64));
        group.bench_function("sensor_micro", |b| {
            b.iter(|| {
                let res = deserialize(black_box(&xml), Arc::clone(&schema)).unwrap();
                black_box(res);
            });
        });
    }

    // 2. Catalog Batch (1,000 and 10,000 items)
    for &count in &[1_000, 10_000] {
        let (schema, xml) = build_catalog_fixture(count);
        group.throughput(Throughput::Bytes(xml.len() as u64));
        group.bench_with_input(BenchmarkId::new("catalog_items", count), &count, |b, _| {
            b.iter(|| {
                let res = deserialize(black_box(&xml), Arc::clone(&schema)).unwrap();
                black_box(res);
            });
        });
    }

    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    // 1. Sensor Telemetry (Micro)
    {
        let (schema, xml) = build_sensor_fixture();
        let val = deserialize(&xml, Arc::clone(&schema)).unwrap();
        let expected_bytes = serialize("Sensor", &val, &schema, None).unwrap();
        group.throughput(Throughput::Bytes(expected_bytes.len() as u64));
        group.bench_function("sensor_micro", |b| {
            b.iter(|| {
                let res = serialize(
                    black_box("Sensor"),
                    black_box(&val),
                    black_box(&schema),
                    black_box(None),
                )
                .unwrap();
                black_box(res);
            });
        });
    }

    // 2. Catalog Batch (1,000 and 10,000 items)
    for &count in &[1_000, 10_000] {
        let (schema, xml) = build_catalog_fixture(count);
        let val = deserialize(&xml, Arc::clone(&schema)).unwrap();
        let expected_bytes = serialize("Catalog", &val, &schema, None).unwrap();
        group.throughput(Throughput::Bytes(expected_bytes.len() as u64));
        group.bench_with_input(BenchmarkId::new("catalog_items", count), &count, |b, _| {
            b.iter(|| {
                let res = serialize(
                    black_box("Catalog"),
                    black_box(&val),
                    black_box(&schema),
                    black_box(None),
                )
                .unwrap();
                black_box(res);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_deserialization, bench_serialization);
criterion_main!(benches);
