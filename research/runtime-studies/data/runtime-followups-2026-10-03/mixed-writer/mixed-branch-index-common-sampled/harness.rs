use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize, PolyValue};
use std::{hint::black_box, sync::Arc};

fn fixture(kind: &str, count: usize) -> (Arc<ModelSchema>, Vec<u8>) {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="State"><xs:restriction base="xs:string"><xs:enumeration value="Ready"/><xs:enumeration value="Done"/></xs:restriction></xs:simpleType>
      <xs:simpleType name="Code"><xs:restriction base="xs:string"><xs:pattern value="[A-Z]{3}[0-9]{3}"/></xs:restriction></xs:simpleType>
      <xs:element name="Root"><xs:complexType mixed="true"><xs:choice minOccurs="0" maxOccurs="unbounded">
        <xs:element name="Text" type="xs:string"/>
        <xs:element name="Number" type="xs:int"/>
        <xs:element name="State" type="State"/>
        <xs:element name="Code" type="Code"/>
        <xs:element name="Child"><xs:complexType><xs:sequence><xs:element name="Value" type="xs:string"/></xs:sequence></xs:complexType></xs:element>
      </xs:choice></xs:complexType></xs:element>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let mut xml = String::from("<Root>");
    for index in 0..count {
        let part = match kind {
            "text" => format!("text-{index}&amp;<![CDATA[more]]>"),
            "scalar" => format!("<Text>item-{index}&amp;<![CDATA[more]]></Text>"),
            "enum" => "<State>Ready</State>".into(),
            "pattern" => format!("<Code>ABC{:03}</Code>", index % 1000),
            "nested" => format!("<Child><Value>item-{index}</Value></Child>"),
            _ => unreachable!(),
        };
        xml.push_str(&part);
    }
    xml.push_str("</Root>");
    let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
    let items_name = &schema.fields[schema.mixed_content.as_ref().unwrap().items_index].name;
    let items = value.get(items_name).and_then(PolyValue::as_list).unwrap();
    if kind == "text" {
        // Text, GeneralRef and CData become separate ordered items.
        assert_eq!(items.len(), count * 3);
        for (index, items) in items.chunks_exact(3).enumerate() {
            for (item, expected) in
                items
                    .iter()
                    .zip([format!("text-{index}"), "&".into(), "more".into()])
            {
                assert_eq!(item.get("kind").and_then(PolyValue::as_str), Some("#text"));
                assert_eq!(
                    item.get("value").and_then(PolyValue::as_str),
                    Some(expected.as_str())
                );
            }
        }
    } else {
        assert_eq!(items.len(), count);
        for (index, item) in items.iter().enumerate() {
            let value = item.get("value").unwrap();
            match kind {
                "scalar" => assert_eq!(value.as_str(), Some(format!("item-{index}&more").as_str())),
                "enum" => assert_eq!(value.as_str(), Some("Ready")),
                "pattern" => assert_eq!(
                    value.as_str(),
                    Some(format!("ABC{:03}", index % 1000).as_str())
                ),
                "nested" => assert_eq!(
                    value.get("value").and_then(PolyValue::as_str),
                    Some(format!("item-{index}").as_str())
                ),
                _ => unreachable!(),
            }
        }
    }
    let output = serialize("Root", &value, &schema, None).unwrap();
    let reparsed = deserialize(&output, Arc::clone(&schema)).unwrap();
    if kind == "text" {
        // XML writers coalesce adjacent text segments. Compare their text value
        // rather than treating the reader's segmentation as XML semantics.
        let text = |value: &PolyValue| -> String {
            value
                .get(items_name)
                .unwrap()
                .as_list()
                .unwrap()
                .iter()
                .map(|item| item.get("value").unwrap().as_str().unwrap())
                .collect()
        };
        assert_eq!(text(&value), text(&reparsed));
    } else {
        assert_eq!(value, reparsed);
    }
    (schema, xml.into_bytes())
}

fn benchmarks(c: &mut Criterion) {
    for operation in ["mixed_read", "mixed_write"] {
        let mut group = c.benchmark_group(operation);
        group.sampling_mode(SamplingMode::Flat);
        for kind in ["text", "scalar", "enum", "pattern", "nested"] {
            let (schema, xml) = fixture(kind, 1000);
            let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
            group.throughput(Throughput::Bytes(xml.len() as u64));
            group.bench_function(BenchmarkId::new(kind, 1000), |b| {
                if operation == "mixed_read" {
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
