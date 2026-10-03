use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize};
use std::{hint::black_box, sync::Arc};

// Cycle through more than the admitted cache capacity to measure FIFO misses.
// Schema construction and correctness checks remain outside measurement.
fn fixture(patterns: usize) -> (Arc<ModelSchema>, Vec<u8>) {
    let mut builder = ModelSchema::builder("Root");
    for i in 0..patterns {
        builder = builder.field(FieldSchema::new(
            format!("values{i}"),
            format!("Value{i}").as_bytes(),
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Scalar(ScalarType::Pattern(
                Box::new(ScalarType::String),
                vec![format!("P{i}-[0-9]{{3}}")],
            )))),
        ));
    }
    let schema = builder.build();
    let mut xml = String::from("<Root>");
    for _ in 0..8 {
        for i in 0..patterns {
            xml.push_str(&format!("<Value{i}>P{i}-123</Value{i}>"));
        }
    }
    xml.push_str("</Root>");
    let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
    for i in 0..patterns {
        let values = value.get(&format!("values{i}")).unwrap().as_list().unwrap();
        assert_eq!(values.len(), 8);
        assert!(values
            .iter()
            .all(|value| value.as_str() == Some(format!("P{i}-123").as_str())));
    }
    let output = serialize("Root", &value, &schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    (schema, xml.into_bytes())
}

fn benchmarks(c: &mut Criterion) {
    for operation in ["cache_read", "cache_write"] {
        let mut group = c.benchmark_group(operation);
        group.sampling_mode(SamplingMode::Flat);
        for patterns in [1, 16, 17, 64] {
            let (schema, xml) = fixture(patterns);
            let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
            group.bench_with_input(
                BenchmarkId::new("active_patterns", patterns),
                &patterns,
                |b, _| {
                    b.iter(|| {
                        if operation == "cache_read" {
                            black_box(deserialize(black_box(&xml), Arc::clone(&schema)).unwrap());
                        } else {
                            black_box(serialize("Root", black_box(&value), &schema, None).unwrap());
                        }
                    });
                },
            );
        }
        group.finish();
    }
}
criterion_group!(benches, benchmarks);
criterion_main!(benches);
