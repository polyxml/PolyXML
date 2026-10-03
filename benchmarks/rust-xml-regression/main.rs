use std::{
    hint::black_box,
    time::{Duration, Instant},
};
mod batch;

fn measure(mut f: impl FnMut(), operation: &str, count: usize, bytes: usize) {
    for _ in 0..100 {
        f();
    }
    for repeat in 0..7 {
        let start = Instant::now();
        let mut iterations = 0;
        while start.elapsed() < Duration::from_millis(250) {
            for _ in 0..100 {
                f();
            }
            iterations += 100;
        }
        println!(
            "{operation},{count},{repeat},{iterations},{:.3},{bytes}",
            start.elapsed().as_nanos() as f64 / iterations as f64
        );
    }
}
fn check(value: &batch::BatchType<'_>, count: usize) {
    assert_eq!(value.sensor.len(), count);
    for (i, sensor) in value.sensor.iter().enumerate() {
        assert_eq!(sensor.id, format!("sensor-{i}"));
        assert_eq!(sensor.value, i as i32);
    }
}
fn main() {
    let fixtures = std::env::args().nth(1).expect("fixture directory");
    println!("operation,count,repeat,iterations,ns_per_op,bytes");
    for count in [1, 1000] {
        let xml = std::fs::read_to_string(format!("{fixtures}/sensor-{count}.xml")).unwrap();
        let parsed = batch::BatchType::from_xml(&xml).unwrap();
        check(&parsed, count);
        let mut writer = quick_xml::Writer::new(Vec::new());
        parsed.encode_xml(&mut writer, Some("Batch")).unwrap();
        let output = writer.into_inner();
        check(&batch::BatchType::from_xml_bytes(&output).unwrap(), count);
        let json = serde_json::to_string(&parsed).unwrap();
        check(&serde_json::from_str(&json).unwrap(), count);
        measure(
            || {
                black_box(batch::BatchType::from_xml(black_box(&xml)).unwrap());
            },
            "xml_read",
            count,
            xml.len(),
        );
        measure(
            || {
                let mut writer = quick_xml::Writer::new(Vec::new());
                black_box(&parsed)
                    .encode_xml(&mut writer, Some("Batch"))
                    .unwrap();
                black_box(writer.into_inner());
            },
            "xml_write",
            count,
            output.len(),
        );
        measure(
            || {
                black_box(serde_json::from_str::<batch::BatchType<'_>>(black_box(&json)).unwrap());
            },
            "json_read",
            count,
            json.len(),
        );
        measure(
            || {
                black_box(serde_json::to_vec(black_box(&parsed)).unwrap());
            },
            "json_write",
            count,
            json.len(),
        );
    }
}
