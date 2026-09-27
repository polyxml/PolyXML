use std::hint::black_box;
use std::time::Instant;

#[path = "../target/120/match/record.rs"]
mod match_120;
#[path = "../target/16/match/record.rs"]
mod match_16;
#[path = "../target/120/phf/record.rs"]
mod phf_120;
#[path = "../target/16/phf/record.rs"]
mod phf_16;

fn measure<'a, T>(name: &str, xml: &'a str, iterations: usize, decode: impl Fn(&'a str) -> T) {
    for _ in 0..1000 {
        black_box(decode(black_box(xml)));
    }
    for repeat in 0..5 {
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(decode(black_box(xml)));
        }
        let nanos = start.elapsed().as_nanos() as f64 / iterations as f64;
        let mb_s = xml.len() as f64 / nanos * 1000.0;
        println!(
            "{name},repeat={repeat},ns/op={nanos:.1},MB/s={mb_s:.2},xml_bytes={}",
            xml.len()
        );
    }
}

fn main() {
    let small = include_str!("../target/16/record.xml");
    let medium = include_str!("../target/120/record.xml");
    assert_eq!(
        match_16::Record::from_xml(small).unwrap().field015,
        "value-015"
    );
    assert_eq!(
        phf_16::Record::from_xml(small).unwrap().field015,
        "value-015"
    );
    assert_eq!(
        match_120::Record::from_xml(medium).unwrap().field119,
        "value-119"
    );
    assert_eq!(
        phf_120::Record::from_xml(medium).unwrap().field119,
        "value-119"
    );
    if std::env::var_os("POLYXML_PHF_FIRST").is_some() {
        measure("16/phf", small, 100_000, |xml| {
            phf_16::Record::from_xml(xml).unwrap()
        });
        measure("16/match", small, 100_000, |xml| {
            match_16::Record::from_xml(xml).unwrap()
        });
        measure("120/phf", medium, 10_000, |xml| {
            phf_120::Record::from_xml(xml).unwrap()
        });
        measure("120/match", medium, 10_000, |xml| {
            match_120::Record::from_xml(xml).unwrap()
        });
    } else {
        measure("16/match", small, 100_000, |xml| {
            match_16::Record::from_xml(xml).unwrap()
        });
        measure("16/phf", small, 100_000, |xml| {
            phf_16::Record::from_xml(xml).unwrap()
        });
        measure("120/match", medium, 10_000, |xml| {
            match_120::Record::from_xml(xml).unwrap()
        });
        measure("120/phf", medium, 10_000, |xml| {
            phf_120::Record::from_xml(xml).unwrap()
        });
    }
}
