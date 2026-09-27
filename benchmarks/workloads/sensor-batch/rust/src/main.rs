use std::hint::black_box;
use std::time::Instant;

#[path = "../../target/rust/batch.rs"]
mod batch;

fn measure(count: usize, xml: &str, iterations: usize) {
    let parsed = batch::BatchType::from_xml(xml).unwrap();
    assert_eq!(parsed.sensor.len(), count);
    assert_eq!(parsed.sensor[count - 1].id, format!("sensor-{}", count - 1));
    assert_eq!(parsed.sensor[count - 1].value, (count - 1) as i32);
    for _ in 0..100 {
        black_box(batch::BatchType::from_xml(xml).unwrap());
    }
    for repeat in 0..5 {
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(batch::BatchType::from_xml(black_box(xml)).unwrap());
        }
        let ns = start.elapsed().as_nanos() as f64 / iterations as f64;
        println!(
            "rust,size={count},repeat={repeat},ns/op={ns:.1},xml_bytes={}",
            xml.len()
        );
    }
}

fn main() {
    measure(1, include_str!("../../sensor-1.xml"), 100_000);
    measure(1000, include_str!("../../sensor-1000.xml"), 1_000);
}
