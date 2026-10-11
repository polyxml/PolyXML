//! Matched XML/JSON consumers. Timings and allocation diagnostics use separate builds.
use polyxml::{schema::ModelSchema, schema_parser::XsdParser, PolyValue};
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "allocations"))]
use std::time::{Duration, Instant};
use std::{borrow::Cow, hint::black_box, sync::Arc};
#[path = "borrowed.rs"]
mod borrowed;
#[path = "owned.rs"]
mod owned;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Batch")]
struct HandBatch {
    #[serde(rename = "Sensor")]
    sensor: Vec<HandSensor>,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct HandSensor {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Value")]
    value: i32,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Batch")]
struct BorrowBatch<'a> {
    #[serde(rename = "Sensor", borrow)]
    sensor: Vec<BorrowSensor<'a>>,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct BorrowSensor<'a> {
    #[serde(rename = "Id", borrow)]
    id: Cow<'a, str>,
    #[serde(rename = "Value")]
    value: i32,
}

#[cfg(feature = "allocations")]
mod allocation {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
    static ENABLED: AtomicBool = AtomicBool::new(false);
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static BYTES: AtomicUsize = AtomicUsize::new(0);
    pub struct Counter;
    unsafe impl GlobalAlloc for Counter {
        unsafe fn alloc(&self, l: Layout) -> *mut u8 {
            if ENABLED.load(Relaxed) {
                CALLS.fetch_add(1, Relaxed);
                BYTES.fetch_add(l.size(), Relaxed);
            }
            System.alloc(l)
        }
        unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
            if ENABLED.load(Relaxed) {
                CALLS.fetch_add(1, Relaxed);
                BYTES.fetch_add(l.size(), Relaxed);
            }
            System.alloc_zeroed(l)
        }
        unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
            if ENABLED.load(Relaxed) {
                CALLS.fetch_add(1, Relaxed);
                BYTES.fetch_add(n, Relaxed);
            }
            System.realloc(p, l, n)
        }
        unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
            System.dealloc(p, l)
        }
    }
    pub fn count(mut f: impl FnMut()) -> (usize, usize) {
        f();
        CALLS.store(0, Relaxed);
        BYTES.store(0, Relaxed);
        ENABLED.store(true, Relaxed);
        f();
        ENABLED.store(false, Relaxed);
        (CALLS.load(Relaxed), BYTES.load(Relaxed))
    }
}
#[cfg(feature = "allocations")]
#[global_allocator]
static ALLOCATOR: allocation::Counter = allocation::Counter;

fn measure(lane: &str, case: &str, op: &str, input: usize, output: usize, mut f: impl FnMut()) {
    #[cfg(feature = "allocations")]
    {
        let (calls, bytes) = allocation::count(&mut f);
        println!("allocation,{lane},{case},{op},0,1,,{calls},{bytes},{input},{output}");
    }
    #[cfg(not(feature = "allocations"))]
    {
        for _ in 0..64 {
            f();
        }
        let pilot = Instant::now();
        for _ in 0..10 {
            f();
        }
        let cost = pilot.elapsed().as_nanos() / 10;
        let chunk = (10_000_000 / cost.max(1)).clamp(1, 100) as usize;
        let samples: usize = std::env::var("BENCH_SAMPLES")
            .unwrap_or("9".into())
            .parse()
            .unwrap();
        let millis: u64 = std::env::var("BENCH_MILLIS")
            .unwrap_or("250".into())
            .parse()
            .unwrap();
        for sample in 0..samples {
            let start = Instant::now();
            let mut iterations = 0;
            while start.elapsed() < Duration::from_millis(millis) {
                for _ in 0..chunk {
                    f();
                }
                iterations += chunk;
            }
            println!(
                "sample,{lane},{case},{op},{sample},{iterations},{:.3},,,{input},{output}",
                start.elapsed().as_nanos() as f64 / iterations as f64
            );
        }
    }
}
fn expected(index: usize, escaped: bool) -> String {
    if escaped {
        format!("sensor-{index} & <é> \"quoted\"")
    } else {
        format!("sensor-{index}")
    }
}
fn check(values: impl Iterator<Item = (impl AsRef<str>, i32)>, count: usize, escaped: bool) {
    let mut seen = 0;
    for (index, (id, value)) in values.enumerate() {
        assert_eq!(id.as_ref(), expected(index, escaped));
        assert_eq!(value, index as i32);
        seen += 1;
    }
    assert_eq!(seen, count);
}
fn check_dynamic(value: &PolyValue, count: usize, escaped: bool) {
    let Some(PolyValue::List(items)) = value.get("sensor") else {
        panic!("missing Sensor list")
    };
    check(
        items.iter().map(|s| {
            (
                s.get("id").unwrap().as_str().unwrap(),
                match s.get("value").unwrap() {
                    PolyValue::Int(i) => *i as i32,
                    _ => panic!("integer required"),
                },
            )
        }),
        count,
        escaped,
    );
}
fn export(lane: &str, case: &str, bytes: &[u8]) {
    if let Ok(directory) = std::env::var("BENCH_EXPORT") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(format!("{directory}/rust-{lane}-{case}.xml"), bytes).unwrap();
    }
}
fn main() {
    let directory = std::env::args().nth(1).expect("fixture directory");
    let ir = XsdParser::new()
        .parse_str(include_str!("schema.xsd"))
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Batch")).unwrap();
    println!("kind,lane,case,op,sample,iterations,ns_per_op,allocations,requested_bytes,input_bytes,output_bytes");
    let order: usize = std::env::var("BENCH_ORDER")
        .unwrap_or("0".into())
        .parse()
        .unwrap();
    let cases = [
        ("sensor_1", 1, false),
        ("sensor_1000", 1000, false),
        ("escaped_1000", 1000, true),
    ];
    for offset in 0..cases.len() {
        let (case, count, escaped) = cases[(offset + order) % cases.len()];
        let xml = std::fs::read_to_string(format!("{directory}/{case}.xml")).unwrap();
        let own = owned::BatchType::from_xml(&xml).unwrap();
        check(own.sensor.iter().map(|s| (&s.id, s.value)), count, escaped);
        let borrow = borrowed::BatchType::from_xml(&xml).unwrap();
        check(
            borrow.sensor.iter().map(|s| (&s.id, s.value)),
            count,
            escaped,
        );
        let hand: HandBatch = quick_xml::de::from_str(&xml).unwrap();
        check(hand.sensor.iter().map(|s| (&s.id, s.value)), count, escaped);
        let cow: BorrowBatch = quick_xml::de::from_str(&xml).unwrap();
        check(cow.sensor.iter().map(|s| (&s.id, s.value)), count, escaped);
        let rs: HandBatch = serde_xml_rs::from_str(&xml).unwrap();
        assert_eq!(rs, hand);
        let dynamic = polyxml::deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
        check_dynamic(&dynamic, count, escaped);
        if !escaped {
            assert!(borrow
                .sensor
                .iter()
                .all(|s| matches!(s.id, Cow::Borrowed(_))));
            assert!(cow.sensor.iter().all(|s| matches!(s.id, Cow::Borrowed(_))));
        }
        let json = serde_json::to_string(&hand).unwrap();
        assert_eq!(serde_json::to_string(&own).unwrap(), json);
        assert_eq!(serde_json::to_string(&borrow).unwrap(), json);
        check(
            serde_json::from_str::<owned::BatchType>(&json)
                .unwrap()
                .sensor
                .iter()
                .map(|s| (&s.id, s.value)),
            count,
            escaped,
        );
        check(
            serde_json::from_str::<borrowed::BatchType>(&json)
                .unwrap()
                .sensor
                .iter()
                .map(|s| (&s.id, s.value)),
            count,
            escaped,
        );
        for position in 0..6 {
            let lane = (position + order) % 6;
            match lane {
                0 => {
                    let mut writer = quick_xml::Writer::new(Vec::new());
                    own.encode_xml(&mut writer, Some("Batch")).unwrap();
                    let out = writer.into_inner();
                    assert_eq!(
                        quick_xml::de::from_reader::<_, HandBatch>(out.as_slice()).unwrap(),
                        hand
                    );
                    export("polyxml_generated_owned", case, &out);
                    measure(
                        "polyxml_generated_owned",
                        case,
                        "xml_read",
                        xml.len(),
                        0,
                        || {
                            black_box(owned::BatchType::from_xml(black_box(&xml)).unwrap());
                        },
                    );
                    measure(
                        "polyxml_generated_owned",
                        case,
                        "xml_write",
                        xml.len(),
                        out.len(),
                        || {
                            let mut writer = quick_xml::Writer::new(Vec::new());
                            black_box(&own)
                                .encode_xml(&mut writer, Some("Batch"))
                                .unwrap();
                            black_box(writer.into_inner());
                        },
                    );
                }
                1 => {
                    let mut writer = quick_xml::Writer::new(Vec::new());
                    borrow.encode_xml(&mut writer, Some("Batch")).unwrap();
                    let out = writer.into_inner();
                    assert_eq!(
                        quick_xml::de::from_reader::<_, HandBatch>(out.as_slice()).unwrap(),
                        hand
                    );
                    export("polyxml_generated_borrowed", case, &out);
                    measure(
                        "polyxml_generated_borrowed",
                        case,
                        "xml_read",
                        xml.len(),
                        0,
                        || {
                            black_box(borrowed::BatchType::from_xml(black_box(&xml)).unwrap());
                        },
                    );
                    measure(
                        "polyxml_generated_borrowed",
                        case,
                        "xml_write",
                        xml.len(),
                        out.len(),
                        || {
                            let mut writer = quick_xml::Writer::new(Vec::new());
                            black_box(&borrow)
                                .encode_xml(&mut writer, Some("Batch"))
                                .unwrap();
                            black_box(writer.into_inner());
                        },
                    );
                }
                2 => {
                    let out = quick_xml::se::to_string(&hand).unwrap();
                    assert_eq!(quick_xml::de::from_str::<HandBatch>(&out).unwrap(), hand);
                    export("quick_xml_serde_owned", case, out.as_bytes());
                    measure(
                        "quick_xml_serde_owned",
                        case,
                        "xml_read",
                        xml.len(),
                        0,
                        || {
                            black_box(
                                quick_xml::de::from_str::<HandBatch>(black_box(&xml)).unwrap(),
                            );
                        },
                    );
                    measure(
                        "quick_xml_serde_owned",
                        case,
                        "xml_write",
                        xml.len(),
                        out.len(),
                        || {
                            black_box(
                                quick_xml::se::to_string(black_box(&hand))
                                    .unwrap()
                                    .into_bytes(),
                            );
                        },
                    );
                }
                3 => {
                    let out = quick_xml::se::to_string(&cow).unwrap();
                    assert_eq!(quick_xml::de::from_str::<HandBatch>(&out).unwrap(), hand);
                    export("quick_xml_serde_borrowed", case, out.as_bytes());
                    measure(
                        "quick_xml_serde_borrowed",
                        case,
                        "xml_read",
                        xml.len(),
                        0,
                        || {
                            black_box(
                                quick_xml::de::from_str::<BorrowBatch>(black_box(&xml)).unwrap(),
                            );
                        },
                    );
                    measure(
                        "quick_xml_serde_borrowed",
                        case,
                        "xml_write",
                        xml.len(),
                        out.len(),
                        || {
                            black_box(
                                quick_xml::se::to_string(black_box(&cow))
                                    .unwrap()
                                    .into_bytes(),
                            );
                        },
                    );
                }
                4 => {
                    let out = serde_xml_rs::to_string(&hand).unwrap();
                    assert_eq!(quick_xml::de::from_str::<HandBatch>(&out).unwrap(), hand);
                    export("serde_xml_rs_owned", case, out.as_bytes());
                    measure("serde_xml_rs_owned", case, "xml_read", xml.len(), 0, || {
                        black_box(serde_xml_rs::from_str::<HandBatch>(black_box(&xml)).unwrap());
                    });
                    measure(
                        "serde_xml_rs_owned",
                        case,
                        "xml_write",
                        xml.len(),
                        out.len(),
                        || {
                            black_box(
                                serde_xml_rs::to_string(black_box(&hand))
                                    .unwrap()
                                    .into_bytes(),
                            );
                        },
                    );
                }
                5 => {
                    let out = polyxml::serialize("Batch", &dynamic, &schema, None).unwrap();
                    assert_eq!(
                        quick_xml::de::from_reader::<_, HandBatch>(out.as_slice()).unwrap(),
                        hand
                    );
                    export("polyxml_dynamic", case, &out);
                    measure("polyxml_dynamic", case, "xml_read", xml.len(), 0, || {
                        black_box(
                            polyxml::deserialize(black_box(xml.as_bytes()), Arc::clone(&schema))
                                .unwrap(),
                        );
                    });
                    measure(
                        "polyxml_dynamic",
                        case,
                        "xml_write",
                        xml.len(),
                        out.len(),
                        || {
                            black_box(
                                polyxml::serialize("Batch", black_box(&dynamic), &schema, None)
                                    .unwrap(),
                            );
                        },
                    );
                }
                _ => unreachable!(),
            }
        }
        for position in 0..3 {
            match (position + order) % 3 {
                0 => {
                    measure(
                        "polyxml_generated_owned",
                        case,
                        "json_read",
                        json.len(),
                        0,
                        || {
                            black_box(
                                serde_json::from_str::<owned::BatchType>(black_box(&json)).unwrap(),
                            );
                        },
                    );
                    measure(
                        "polyxml_generated_owned",
                        case,
                        "json_write",
                        json.len(),
                        json.len(),
                        || {
                            black_box(serde_json::to_vec(black_box(&own)).unwrap());
                        },
                    );
                }
                1 => {
                    measure(
                        "polyxml_generated_borrowed",
                        case,
                        "json_read",
                        json.len(),
                        0,
                        || {
                            black_box(
                                serde_json::from_str::<borrowed::BatchType>(black_box(&json))
                                    .unwrap(),
                            );
                        },
                    );
                    measure(
                        "polyxml_generated_borrowed",
                        case,
                        "json_write",
                        json.len(),
                        json.len(),
                        || {
                            black_box(serde_json::to_vec(black_box(&borrow)).unwrap());
                        },
                    );
                }
                2 => {
                    measure(
                        "serde_json_handwritten",
                        case,
                        "json_read",
                        json.len(),
                        0,
                        || {
                            black_box(serde_json::from_str::<HandBatch>(black_box(&json)).unwrap());
                        },
                    );
                    measure(
                        "serde_json_handwritten",
                        case,
                        "json_write",
                        json.len(),
                        json.len(),
                        || {
                            black_box(serde_json::to_vec(black_box(&hand)).unwrap());
                        },
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}
