use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize, PolyValue};
use std::{hint::black_box, sync::Arc};

fn model(name: &str, namespace: Option<&str>) -> Arc<ModelSchema> {
    let mut builder = ModelSchema::builder(name).field(FieldSchema::new(
        "value",
        b"Value",
        FieldKind::Element,
        ValueType::Scalar(ScalarType::Int),
    ));
    if let Some(namespace) = namespace {
        builder = builder.namespace(namespace);
    }
    builder.build()
}

fn fixture(registered: bool, count: usize) -> (Arc<ModelSchema>, Vec<u8>) {
    let base = model("Base", None);
    if registered {
        base.set_variants(vec![model("Derived", Some("urn:derived"))]);
    }
    let schema = ModelSchema::builder("Root")
        .field(FieldSchema::new(
            "items",
            b"Item",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Nested(base))),
        ))
        .build();
    let mut xml = String::from(
        "<Root xmlns:i='http://www.w3.org/2001/XMLSchema-instance' xmlns:d='urn:derived'>",
    );
    for index in 0..count {
        let selector = if registered {
            " i:type='d:Derived'"
        } else {
            ""
        };
        xml.push_str(&format!("<Item{selector}><Value>{index}</Value></Item>"));
    }
    xml.push_str("</Root>");
    let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
    let items = value.get("items").unwrap().as_list().unwrap();
    assert_eq!(items.len(), count);
    for (index, item) in items.iter().enumerate() {
        assert_eq!(item.get("value"), Some(&PolyValue::Int(index as i64)));
        let PolyValue::Record { schema, .. } = item else {
            panic!("expected record")
        };
        assert_eq!(schema.name, if registered { "Derived" } else { "Base" });
    }
    let output = serialize("Root", &value, &schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    (schema, xml.into_bytes())
}

fn benchmarks(c: &mut Criterion) {
    for operation in ["variant_read", "variant_write"] {
        let mut group = c.benchmark_group(operation);
        group.sampling_mode(SamplingMode::Flat);
        for (kind, registered) in [("empty_registry", false), ("registered", true)] {
            let (schema, xml) = fixture(registered, 1000);
            let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
            group.throughput(Throughput::Bytes(xml.len() as u64));
            group.bench_function(BenchmarkId::new(kind, 1000), |b| {
                if operation == "variant_read" {
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
