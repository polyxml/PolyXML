---
title: Rust
description: Using PolyXML's native Rust engine directly in Rust applications for high-throughput, zero-allocation XML processing.
---

# Rust

The `polyxml` crate is the pure, idiomatic Rust core engine driving the entire PolyXML ecosystem. It provides ultra-fast streaming serialization and deserialization with zero FFI overhead, zero C dependencies, and no intermediate DOM allocations.

---

## 📦 Cargo Installation

Add `polyxml` to your `Cargo.toml`:

```toml
[dependencies]
polyxml = "0.1"
```

---

## 1. Basic Models: Attributes vs. Elements

PolyXML distinguishes between XML attributes, child elements, and text content through `FieldKind`.

```rust
use std::sync::Arc;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define the schema programmatically
    let schema = ModelSchema::builder("Telemetry")
        .field(FieldSchema::new("device_id", b"id", FieldKind::Attribute, ValueType::Scalar(ScalarType::Int)).required())
        .field(FieldSchema::new("altitude", b"altitude", FieldKind::Element, ValueType::Scalar(ScalarType::Float)))
        .field(FieldSchema::new("armed", b"armed", FieldKind::Element, ValueType::Scalar(ScalarType::Bool)))
        .field(FieldSchema::new("callsign", b"callsign", FieldKind::Element, ValueType::Scalar(ScalarType::String)))
        .build();

    let xml = br#"<Telemetry id="402">
        <altitude>31500.75</altitude>
        <armed>true</armed>
        <callsign>GHOST-1</callsign>
    </Telemetry>"#;

    // 2. Deserialize XML into a PolyValue map
    let val = deserialize(xml, Arc::clone(&schema))?;
    
    // 3. Extract strongly-typed scalar values
    let device_id = val.get("device_id").and_then(|v| v.as_i64()).unwrap_or(0);
    let altitude = val.get("altitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let armed = val.get("armed").and_then(|v| v.as_bool()).unwrap_or(false);
    let callsign = val.get("callsign").and_then(|v| v.as_str()).unwrap_or("");

    println!("Telemetry: Device {device_id} ({callsign}) at {altitude}ft, Armed: {armed}");

    // 4. Serialize back to formatted XML bytes
    let output = serialize("Telemetry", &val, &schema, Some(2))?;
    println!("Serialized XML:\n{}", std::str::from_utf8(&output)?);

    Ok(())
}
```

---

## 2. Nested Structures & Repeated Collections (`List`)

PolyXML natively supports complex hierarchical data models, including nested sub-objects (`ValueType::Nested`) and repeated collections (`ValueType::List`).

```rust
use std::sync::Arc;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize, PolyValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define the child schema (Item)
    let item_schema = ModelSchema::builder("Item")
        .field(FieldSchema::new("sku", b"sku", FieldKind::Attribute, ValueType::Scalar(ScalarType::String)))
        .field(FieldSchema::new("qty", b"qty", FieldKind::Element, ValueType::Scalar(ScalarType::Int)))
        .field(FieldSchema::new("price", b"price", FieldKind::Element, ValueType::Scalar(ScalarType::Float)))
        .build();

    // Define the parent schema (Order) containing a list of Items
    let order_schema = ModelSchema::builder("Order")
        .field(FieldSchema::new("order_id", b"id", FieldKind::Attribute, ValueType::Scalar(ScalarType::Int)))
        .field(FieldSchema::new(
            "items",
            b"Item",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Nested(Arc::clone(&item_schema)))),
        ))
        .build();

    let xml = br#"
    <Order id="9801">
        <Item sku="PART-A"><qty>5</qty><price>19.99</price></Item>
        <Item sku="PART-B"><qty>2</qty><price>45.50</price></Item>
    </Order>
    "#;

    let order = deserialize(xml, Arc::clone(&order_schema))?;
    
    // Inspect repeated elements
    if let Some(PolyValue::List(items)) = order.get("items") {
        for item in items {
            let sku = item.get("sku").and_then(|v| v.as_str()).unwrap_or("");
            let qty = item.get("qty").and_then(|v| v.as_i64()).unwrap_or(0);
            let price = item.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            println!("- Item: SKU={sku}, Qty={qty}, Total=${:.2}", (qty as f64) * price);
        }
    }

    Ok(())
}
```

---

## 3. Constant-Memory Streaming with `XmlItemStream`

When parsing large XML feeds (e.g. PubMed, Wikipedia, SEC EDGAR, ISO 20022), loading the full document into memory is prohibitive. `XmlItemStream` yields individual record subtrees, so memory depends on the largest record rather than the complete document.

```rust
use std::sync::Arc;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::XmlItemStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let item_schema = ModelSchema::builder("Product")
        .field(FieldSchema::new("id", b"id", FieldKind::Attribute, ValueType::Scalar(ScalarType::Int)))
        .field(FieldSchema::new("title", b"title", FieldKind::Element, ValueType::Scalar(ScalarType::String)))
        .field(FieldSchema::new("price", b"price", FieldKind::Element, ValueType::Scalar(ScalarType::Float)))
        .build();

    let large_xml_stream = br#"
    <Catalog>
        <Product id="1"><title>Sensor Module A</title><price>12.50</price></Product>
        <Product id="2"><title>Actuator Hub B</title><price>89.00</price></Product>
        <Product id="3"><title>Telemetry Unit C</title><price>245.99</price></Product>
    </Catalog>
    "#;

    // Stream records matching tag "Product" without materializing the <Catalog> DOM
    let mut stream = XmlItemStream::new(&large_xml_stream[..], b"Product", Arc::clone(&item_schema));

    let mut count = 0;
    while let Some(result) = stream.next() {
        let product = result?;
        let title = product.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let price = product.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        println!("Streamed product #{}: {} (${:.2})", count + 1, title, price);
        count += 1;
    }

    println!("Total products processed: {count}");
    Ok(())
}
```

---

## 4. Nullability, Empty Elements, and `xsi:nil`

In production XML feeds, elements may be empty, omitted, or explicitly marked null via XML Schema Instance attributes (`xsi:nil="true"`). PolyXML natively handles all three scenarios:

```rust
use std::sync::Arc;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, PolyValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = ModelSchema::builder("Record")
        .field(FieldSchema::new("id", b"id", FieldKind::Element, ValueType::Scalar(ScalarType::Int)))
        .field(FieldSchema::new("notes", b"notes", FieldKind::Element, ValueType::Scalar(ScalarType::String)))
        .field(FieldSchema::new("status", b"status", FieldKind::Element, ValueType::Scalar(ScalarType::String)))
        .build();

    let xml = br#"
    <Record xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
        <id>100</id>
        <notes xsi:nil="true"/>
        <status></status>
    </Record>
    "#;

    let record = deserialize(xml, schema)?;

    // notes is parsed as PolyValue::Null due to xsi:nil="true"
    assert_eq!(record.get("notes"), Some(&PolyValue::Null));

    // empty <status></status> parses as empty string
    assert_eq!(record.get("status").and_then(|v| v.as_str()), Some(""));

    Ok(())
}
```

---

## 5. CDATA Sections & XML Entity Escaping

PolyXML automatically unescapes standard XML entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`) as well as numeric character references (`&#65;`, `&#x41;`), and extracts raw text within `<![CDATA[...]]>` blocks:

```rust
use std::sync::Arc;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::deserialize;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = ModelSchema::builder("Payload")
        .field(FieldSchema::new("script", b"script", FieldKind::Element, ValueType::Scalar(ScalarType::String)))
        .field(FieldSchema::new("desc", b"desc", FieldKind::Element, ValueType::Scalar(ScalarType::String)))
        .build();

    let xml = br#"
    <Payload>
        <script><![CDATA[if (a < 10 && b > 20) { return true; }]]></script>
        <desc>Tom &amp; Jerry &quot;Classic&quot;</desc>
    </Payload>
    "#;

    let val = deserialize(xml, schema)?;
    assert_eq!(val.get("script").unwrap().as_str().unwrap(), "if (a < 10 && b > 20) { return true; }");
    assert_eq!(val.get("desc").unwrap().as_str().unwrap(), "Tom & Jerry \"Classic\"");

    Ok(())
}
```

---

## 6. XML Namespaces & Prefix Mapping

`polyxml-core` natively supports W3C XML namespaces for both serialization and deserialization with zero allocation overhead when disabled.

### Schema Definition with Namespaces

Declare namespaces on models or specific fields using `.with_namespace()`:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::{deserialize, serialize_with_options, PolyValue};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define a schema with model-level and field-level namespaces
    let schema = ModelSchema::builder("Order")
        .with_namespace("https://example.com/orders")
        .field(
            FieldSchema::new("id", b"id", FieldKind::Attribute, ValueType::Scalar(ScalarType::Int))
                .required(),
        )
        .field(
            FieldSchema::new("item", b"item", FieldKind::Element, ValueType::Scalar(ScalarType::String))
                .with_namespace("https://example.com/items"),
        )
        .build();

    let mut record = HashMap::new();
    record.insert("id".to_string(), PolyValue::Int(1001));
    record.insert("item".to_string(), PolyValue::String("Widget Pro".to_string()));
    let val = PolyValue::Object(record);

    // 1. Serialization with automatic namespace prefix generation (ns0, ns1, ...)
    let bytes = serialize_with_options("Order", &val, &schema, Some(2), true, None)?;
    println!("{}", std::str::from_utf8(&bytes)?);
    // Output:
    // <ns0:Order xmlns:ns0="https://example.com/orders" xmlns:ns1="https://example.com/items" id="1001">
    //   <ns1:item>Widget Pro</ns1:item>
    // </ns0:Order>

    // 2. Serialization with custom prefix mapping
    let mut ns_map = HashMap::new();
    ns_map.insert(Some("ord".to_string()), "https://example.com/orders".to_string());
    ns_map.insert(Some("itm".to_string()), "https://example.com/items".to_string());

    let bytes = serialize_with_options("Order", &val, &schema, Some(2), true, Some(&ns_map))?;
    println!("{}", std::str::from_utf8(&bytes)?);
    // Output:
    // <ord:Order xmlns:ord="https://example.com/orders" xmlns:itm="https://example.com/items" id="1001">
    //   <itm:item>Widget Pro</itm:item>
    // </ord:Order>

    // 3. Deserializing namespaced documents (automatically matches local names and prefixes)
    let parsed = deserialize(&bytes, Arc::clone(&schema))?;
    assert_eq!(parsed.get("id").and_then(|v| v.as_i64()), Some(1001));
    assert_eq!(parsed.get("item").and_then(|v| v.as_str()), Some("Widget Pro"));

    Ok(())
}
```

---

## 7. High-Throughput Production Best Practices

To achieve the maximum throughput from `polyxml-core`:

1. **Reuse Schema Instances**: Construct `Arc<ModelSchema>` once at startup or store it in a static `LazyLock`. Passing `Arc::clone(&schema)` is an atomic reference bump that avoids re-indexing fields.
2. **Pre-allocate Buffers**: For serialization loops, pass an existing `Vec<u8>` or reuse serialization buffers across requests to avoid heap allocations.
3. **Use `XmlItemStream` for Files > 5 MB**: For large XML feeds, streaming ensures your process memory remains constant regardless of file size.
4. **Thread Safety**: Both `ModelSchema` and `PolyValue` are fully `Send + Sync`, making them ideal for parallel processing with `rayon` or multi-threaded Tokio runtimes.

---

## 8. Native JSON Codecs & Document Transcoder

### Inherent JSON Methods on Generated Models

Rust models compiled with `polyxml generate --lang rust --codecs` automatically implement inherent, zero-copy JSON codecs backed by `serde_json`:

```rust
// Generated model from schemas/order.xsd
use generated::rust::Order;

// 1. Serialize to JSON string or Vec<u8>
let json_str: String = order.to_json_string()?;
let json_vec: Vec<u8> = order.to_json_vec()?;

// 2. Deserialize from JSON string slice or byte slice
let restored = Order::from_json_str(&json_str)?;
let from_bytes = Order::from_json_slice(&json_vec)?;
```

### Whole-Document Transcoder (`polyxml::transcoder`)

To convert a complete document without compiling Rust structs, use
`polyxml::transcoder`. It accepts byte slices and returns newly allocated
output bytes; schema-free conversion builds an intermediate JSON value tree:

```rust
use polyxml::transcoder::{xml_to_json, json_to_xml};

let xml_input = br#"<Product id="42"><name>Sensor</name><price>19.99</price></Product>"#;

// Transcode XML to JSON with pretty formatting
let json_bytes = xml_to_json(
    xml_input,
    None, // Optional Arc<ModelSchema>
    Some(2), // Indentation width
    true, // Use schema field aliases
)?;

// Transcode JSON back to XML, inferring the root from its single top-level key
let restored_xml = json_to_xml(
    &json_bytes,
    None,
    None,
    Some(2),
    None, // Namespace handling
    None, // Namespace map
)?;
```

---

## 9. rkyv generation status

The CLI accepts `--feature rkyv`, but the generated `#[rkyv(check_bytes)]` attribute is incompatible with rkyv 0.8. Do not enable this option for new projects until the generator is updated and its output is verified against the rkyv version you use. The default Rust output does not require rkyv.

### Binary lexical values

Generated `xs:hexBinary` and `xs:base64Binary` fields preserve their XML lexical
text as `Cow<'a, str>` (or `String` with zero-copy disabled), including through
XML round trips. They are not decoded byte buffers. Decode hex/base64 explicitly
when consuming the payload. This matches the core runtime's lexical storage and
avoids passing byte slices to string-based XML codecs.
