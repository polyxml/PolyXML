use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize,serialize,PolyValue};
use std::{hint::black_box,sync::Arc};
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
    (schema, xml)
}


fn main() {
 for branches in [1,16,64,256] { for count in [1,1000] {
  let (schema,xml)=fixture(branches,count);
  let value=deserialize(&xml,Arc::clone(&schema)).unwrap();
  for _ in 0..3 { black_box(serialize("Root",&value,&schema,None).unwrap()); }
  ALLOCATIONS.store(0,Ordering::Relaxed);REALLOCATIONS.store(0,Ordering::Relaxed);BYTES.store(0,Ordering::Relaxed);
  COUNTING.store(true,Ordering::Relaxed);
  let output=serialize("Root",&value,&schema,None).unwrap();
  COUNTING.store(false,Ordering::Relaxed);
  println!("branches={branches},items={count},allocations={},reallocations={},requested_bytes={},output_bytes={}",ALLOCATIONS.load(Ordering::Relaxed),REALLOCATIONS.load(Ordering::Relaxed),BYTES.load(Ordering::Relaxed),output.len());
 } }
}
