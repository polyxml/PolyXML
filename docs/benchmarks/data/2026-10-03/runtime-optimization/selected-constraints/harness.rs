use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use polyxml::ir::RestrictionFacets;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize, PolyValue};
use std::{hint::black_box, sync::Arc};

fn fixture(kind: &str, count: usize) -> (Arc<ModelSchema>, Vec<u8>) {
    let scalar = match kind {
        "enum" => ScalarType::Enum((0..16).map(|i| format!("STATE{i}")).collect()),
        "pattern" => ScalarType::Pattern(
            Box::new(ScalarType::String),
            vec!["[A-Z]{3}[0-9]{3}".into()],
        ),
        "restricted" => ScalarType::Restricted(
            Box::new(ScalarType::String),
            Box::new(RestrictionFacets {
                patterns: vec!["[A-Z]{3}[0-9]{3}".into()],
                length: Some(6),
                ..Default::default()
            }),
        ),
        "list" => ScalarType::List(Box::new(ScalarType::Int)),
        "content" => ScalarType::String,
        _ => unreachable!(),
    };
    let mut builder = ModelSchema::builder("Root").field(FieldSchema::new(
        "values",
        b"Value",
        FieldKind::Element,
        ValueType::List(Box::new(ValueType::Scalar(scalar))),
    ));
    if kind == "content" {
        builder = builder
            .content_pattern(polyxml::schema::compile_content_pattern("(?:Value;)*").unwrap());
    }
    let schema = builder.build();
    let mut xml = String::from("<Root>");
    for i in 0..count {
        let value = match kind {
            "enum" => format!("STATE{}", i % 16),
            "pattern" | "restricted" => format!("ABC{:03}", i % 1000),
            "list" => format!("{i} {} -1", i + 1),
            _ => format!("record-{i}"),
        };
        xml.push_str(&format!("<Value>{value}</Value>"));
    }
    xml.push_str("</Root>");
    let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
    let values = value.get("values").unwrap().as_list().unwrap();
    assert_eq!(values.len(), count);
    for (i, value) in values.iter().enumerate() {
        match kind {
            "enum" => assert_eq!(value.as_str(), Some(format!("STATE{}", i % 16).as_str())),
            "pattern" | "restricted" => {
                assert_eq!(value.as_str(), Some(format!("ABC{:03}", i % 1000).as_str()))
            }
            "list" => assert_eq!(
                value,
                &PolyValue::List(vec![
                    PolyValue::Int(i as i64),
                    PolyValue::Int(i as i64 + 1),
                    PolyValue::Int(-1)
                ])
            ),
            _ => assert_eq!(value.as_str(), Some(format!("record-{i}").as_str())),
        }
    }
    let output = serialize("Root", &value, &schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    (schema, xml.into_bytes())
}
fn benchmarks(c: &mut Criterion) {
    for operation in ["constraint_read", "constraint_write"] {
        let mut group = c.benchmark_group(operation);
        group.sampling_mode(SamplingMode::Flat);
        for kind in ["enum", "pattern", "restricted", "list", "content"] {
            let (schema, xml) = fixture(kind, 1000);
            let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
            group.throughput(Throughput::Bytes(xml.len() as u64));
            group.bench_function(BenchmarkId::new(kind, 1000), |b| {
                if operation == "constraint_read" {
                    b.iter(|| {
                        black_box(deserialize(black_box(&xml), Arc::clone(&schema)).unwrap())
                    });
                } else {
                    b.iter(|| {
                        black_box(serialize("Root", black_box(&value), &schema, None).unwrap())
                    });
                }
            });
        }
        group.finish();
    }
}
criterion_group!(benches, benchmarks);
criterion_main!(benches);
