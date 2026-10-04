use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize, PolyValue};
use std::{hint::black_box, sync::Arc};

fn fixture(branches: usize, count: usize) -> (Arc<ModelSchema>, Vec<u8>) {
    let declarations = (0..branches)
        .map(|i| format!(r#"<xs:element name="B{i}" type="xs:int"/>"#))
        .collect::<String>();
    let xsd = format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="Root"><xs:complexType mixed="true"><xs:choice minOccurs="0" maxOccurs="unbounded">{declarations}</xs:choice></xs:complexType></xs:element></xs:schema>"#
    );
    let ir = XsdParser::new().parse_str(&xsd).unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let body = (0..count)
        .map(|i| format!("<B{}>{i}</B{}>", i % branches, i % branches))
        .collect::<String>();
    let xml = format!("<Root>{body}</Root>").into_bytes();
    let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
    let mixed = schema.mixed_content.as_ref().unwrap();
    let field = &schema.fields[mixed.items_index].name;
    let items = value.get(field).and_then(PolyValue::as_list).unwrap();
    assert_eq!(items.len(), count);
    for (i, item) in items.iter().enumerate() {
        assert_eq!(item.get("value"), Some(&PolyValue::Int(i as i64)));
        let kind = item.get("kind").and_then(PolyValue::as_str).unwrap();
        let branch = mixed
            .branches
            .iter()
            .find(|b| b.variant_name == kind)
            .unwrap();
        assert_eq!(branch.xml_name, format!("B{}", i % branches).as_bytes());
    }
    let output = serialize("Root", &value, &schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
    let output = serialize("Sensor", &value, &schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    (schema, xml)
}

fn benchmarks(c: &mut Criterion) {
    for operation in ["branch_read", "branch_write"] {
        let mut group = c.benchmark_group(operation);
        group.sampling_mode(SamplingMode::Flat);
        for branches in [64, 256] {
            for count in [1024] {
                let (schema, xml) = fixture(branches, count);
                let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
                group.throughput(Throughput::Bytes(xml.len() as u64));
                group.bench_with_input(
                    BenchmarkId::new(format!("branches_{branches}"), count),
                    &count,
                    |b, _| {
                        b.iter(|| {
                            if operation == "branch_read" {
                                black_box(
                                    deserialize(black_box(&xml), Arc::clone(&schema)).unwrap(),
                                );
                            } else {
                                black_box(
                                    serialize("Root", black_box(&value), &schema, None).unwrap(),
                                );
                            }
                        });
                    },
                );
            }
        }
        group.finish();
    }
}
criterion_group!(benches, benchmarks);
criterion_main!(benches);
