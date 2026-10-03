// Fixture definitions are extracted verbatim from core_benchmarks.rs by run.py.
include!("fixtures.rs");

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct CountingAllocator;
static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static REALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

// Diagnostic instrumentation only. The benchmark consumers use System directly.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            REALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(new_size, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let operation = args.get(1).expect("read or write");
    let count: usize = args
        .get(2)
        .expect("0 for sensor, or catalog count")
        .parse()
        .unwrap();
    let iterations: usize = args.get(3).expect("iteration count").parse().unwrap();
    let mode = args.get(4).map(String::as_str).unwrap_or("allocations");
    let (schema, xml) = if count == 0 {
        build_sensor_fixture()
    } else {
        build_catalog_fixture(count)
    };
    let value = deserialize(&xml, Arc::clone(&schema)).unwrap();
    let root = if count == 0 { "Sensor" } else { "Catalog" };
    let output = serialize(root, &value, &schema, None).unwrap();
    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());
    for _ in 0..10 {
        if operation == "read" {
            black_box(deserialize(&xml, Arc::clone(&schema)).unwrap());
        } else {
            black_box(serialize(root, &value, &schema, None).unwrap());
        }
    }
    COUNTING.store(mode == "allocations", Ordering::Relaxed);
    measured_region(operation, iterations, &xml, &value, root, &schema);
    COUNTING.store(false, Ordering::Relaxed);
    println!("operation={operation},count={count},iterations={iterations},allocations={},reallocations={},requested_bytes={},input_bytes={},output_bytes={},field_schema_bytes={},scalar_type_bytes={},value_type_bytes={},poly_value_bytes={}",
        ALLOCATIONS.load(Ordering::Relaxed), REALLOCATIONS.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed), xml.len(), output.len(),
        std::mem::size_of::<FieldSchema>(), std::mem::size_of::<ScalarType>(), std::mem::size_of::<ValueType>(), std::mem::size_of::<polyxml::PolyValue>());
}

#[inline(never)]
#[no_mangle]
fn measured_region(
    operation: &str,
    iterations: usize,
    xml: &[u8],
    value: &polyxml::PolyValue,
    root: &str,
    schema: &Arc<ModelSchema>,
) {
    for _ in 0..iterations {
        if operation == "read" {
            black_box(deserialize(black_box(xml), Arc::clone(schema)).unwrap());
        } else {
            black_box(serialize(root, black_box(value), schema, None).unwrap());
        }
    }
}
