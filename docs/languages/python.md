---
title: Python
description: High-throughput XML serialization and deserialization into Python dataclasses and Pydantic v2 models with zero boilerplate.
---

# Python

PolyXML is a native XML data-binding engine for Python `>=3.12`, compiled in Rust via PyO3 with `abi3` stability. On the [10,000-item catalog benchmark](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md), it reads **10.0x faster** and writes **23.5x faster** than `xsdata`.

---

## 📦 Installation

```bash
pip install polyxml
```

PolyXML distributes pre-compiled binary wheels with forward compatibility across **Python 3.12, 3.13, 3.14, and 3.15**.

---

## 1. Standard Python Dataclasses

PolyXML parses XML directly into standard Python `@dataclass` models without requiring base class inheritance. You control XML mapping using standard `dataclasses.field(metadata={...})`.

```python
from dataclasses import dataclass, field
import polyxml

@dataclass
class Aircraft:
    # Mapped as an XML attribute (<Aircraft tail="N1024A">)
    tail_number: str = field(metadata={"type": "Attribute", "name": "tail"})
    
    # Mapped as a child element (<Aircraft><heading>270.5</heading></Aircraft>)
    heading: float = field(metadata={"type": "Element"})
    
    # Defaults to Element if metadata is omitted
    in_flight: bool = True
    speed_knots: int = 250

xml = b"""
<Aircraft tail="N1024A">
    <heading>270.5</heading>
    <in_flight>true</in_flight>
    <speed_knots>320</speed_knots>
</Aircraft>
"""

# 1. Deserialize XML bytes directly into the dataclass
plane = polyxml.deserialize(xml, Aircraft)
print(f"Tail: {plane.tail_number}, Heading: {plane.heading}°, Speed: {plane.speed_knots}kt")
# Output: Tail: N1024A, Heading: 270.5°, Speed: 320kt

# 2. Serialize dataclass back to formatted XML
output = polyxml.serialize(plane, indent=2)
print(output.decode("utf-8"))
```

---

## 2. Nested Dataclasses & Repeated Collections (`list[T]`)

Hierarchical XML structures map naturally to nested dataclasses and `list[T]` collections.

```python
from dataclasses import dataclass, field
import polyxml

@dataclass
class Waypoint:
    name: str = field(metadata={"type": "Attribute"})
    altitude_ft: float = field(metadata={"type": "Element"})

@dataclass
class FlightPlan:
    callsign: str = field(metadata={"type": "Attribute"})
    origin: str = field(metadata={"type": "Element"})
    destination: str = field(metadata={"type": "Element"})
    waypoints: list[Waypoint] = field(metadata={"type": "Element", "name": "Waypoint"}, default_factory=list)

xml = b"""
<FlightPlan callsign="UAL412">
    <origin>KSFO</origin>
    <destination>KORD</destination>
    <Waypoint name="MVA"><altitude_ft>18000.0</altitude_ft></Waypoint>
    <Waypoint name="OBH"><altitude_ft>33000.0</altitude_ft></Waypoint>
    <Waypoint name="IOW"><altitude_ft>24000.0</altitude_ft></Waypoint>
</FlightPlan>
"""

plan = polyxml.deserialize(xml, FlightPlan)
print(f"Flight {plan.callsign}: {plan.origin} -> {plan.destination}")
for wp in plan.waypoints:
    print(f"  - Waypoint {wp.name}: {wp.altitude_ft} ft")
```

---

## 3. Pydantic v2 Models & Runtime Validation

PolyXML fully supports Pydantic v2 `BaseModel` classes using `json_schema_extra` for XML metadata. PolyXML populates the Pydantic model directly, preserving all Pydantic validators, coercions, and schema generation.

```python
from pydantic import BaseModel, Field, field_validator
import polyxml

class SystemMetric(BaseModel):
    sensor_id: int = Field(..., json_schema_extra={"type": "Attribute", "name": "id"})
    label: str = Field(..., json_schema_extra={"type": "Element"})
    cpu_percent: float = Field(..., json_schema_extra={"type": "Element"})

    @field_validator("cpu_percent")
    @classmethod
    def validate_cpu(cls, v: float) -> float:
        if not (0.0 <= v <= 100.0):
            raise ValueError(f"CPU usage must be between 0 and 100, got {v}")
        return v

xml = b'<SystemMetric id="1"><label>EdgeNode-01</label><cpu_percent>42.8</cpu_percent></SystemMetric>'

metric = polyxml.deserialize(xml, SystemMetric)
print(f"Node {metric.sensor_id} ({metric.label}): {metric.cpu_percent}% CPU")

# Serialization preserves Pydantic models
serialized = polyxml.serialize(metric, indent=2)
print(serialized.decode("utf-8"))
```

---

## 4. XML Namespaces & Prefix Mapping

PolyXML includes high-performance, W3C-compliant XML namespace support during both serialization and deserialization.

### Declaring Namespaces in Models

Define namespaces using an inner `class Meta` on models and `namespace` metadata on fields (standard in `dataclasses` and `pydantic` via `pyxsdata`):

```python
from dataclasses import dataclass, field
import polyxml

@dataclass
class Item:
    class Meta:
        name = "item"
        namespace = "http://example.com/catalog"

    title: str = field(metadata={"type": "Element", "namespace": "http://example.com/catalog"})
    sku: str = field(metadata={"type": "Attribute", "namespace": "http://example.com/inv"})
    price: float = field(metadata={"type": "Attribute"})  # Unprefixed attribute

item = Item(title="High Performance Rust", sku="ISBN-999", price=49.99)
```

### Auto-Generated Prefixes (Default)

By default, PolyXML automatically detects namespaces, assigns clean sequential prefixes (`ns0`, `ns1`, ...), and hoists `xmlns` declarations to the root element:

```python
xml_bytes = polyxml.serialize(item, indent=2)
print(xml_bytes.decode("utf-8"))
```

Output:
```xml
<ns0:item xmlns:ns0="http://example.com/catalog" xmlns:ns1="http://example.com/inv" ns1:sku="ISBN-999" price="49.99">
  <ns0:title>High Performance Rust</ns0:title>
</ns0:item>
```

### Custom Prefix Mapping (`ns_map`)

You can control prefix assignments or define a default namespace (`xmlns="..."`) using `ns_map`:

```python
ns_map = {
    None: "http://example.com/catalog",  # Default namespace
    "inv": "http://example.com/inv",     # Custom prefix
}
xml_bytes = polyxml.serialize(item, indent=2, ns_map=ns_map)
print(xml_bytes.decode("utf-8"))
```

Output:
```xml
<item xmlns="http://example.com/catalog" xmlns:inv="http://example.com/inv" inv:sku="ISBN-999" price="49.99">
  <title>High Performance Rust</title>
</item>
```

### Zero-Overhead Fast Path (`namespaces=False`)

If your payload does not use namespaces or you want raw throughput without prefix resolution:

```python
raw_bytes = polyxml.serialize(item, namespaces=False)
```

---

## 5. Constant-Memory Streaming with `polyxml.iterparse`

For large documents (hundreds of megabytes to gigabytes), parsing the entire document at once can cause out-of-memory errors. `polyxml.iterparse` yields typed objects one by one in constant $O(1)$ memory:

```python
from dataclasses import dataclass, field
import polyxml

@dataclass
class LogEntry:
    id: int = field(metadata={"type": "Attribute"})
    level: str = field(metadata={"type": "Element"})
    message: str = field(metadata={"type": "Element"})

# Large streaming XML document
large_xml = b"""
<ServerLog>
    <LogEntry id="1"><level>INFO</level><message>Server initialized</message></LogEntry>
    <LogEntry id="2"><level>WARN</level><message>High latency detected</message></LogEntry>
    <LogEntry id="3"><level>ERROR</level><message>Database timeout</message></LogEntry>
</ServerLog>
"""

# Stream records matching tag "LogEntry" with O(1) memory overhead
error_count = 0
for entry in polyxml.iterparse(large_xml, LogEntry, tag="LogEntry"):
    if entry.level == "ERROR":
        print(f"[ALERT] Entry #{entry.id}: {entry.message}")
        error_count += 1

print(f"Processing complete. Errors found: {error_count}")
```

---

## 5. Serialization & Pretty-Printing

PolyXML provides high-throughput serialization with configurable formatting:

```python
from dataclasses import dataclass, field
import polyxml

@dataclass
class Config:
    env: str = field(metadata={"type": "Attribute"})
    debug: bool = field(metadata={"type": "Element"})
    max_retries: int = field(metadata={"type": "Element"})

cfg = Config(env="production", debug=False, max_retries=5)

# Compact output (indent=None or indent=0)
compact = polyxml.serialize(cfg)
print("Compact:", compact.decode("utf-8"))

# Pretty-printed with 2-space indentation
pretty = polyxml.serialize(cfg, indent=2)
print("Pretty:\n" + pretty.decode("utf-8"))
```

---

## 6. Migrating from `xsdata` and `ElementTree`

This migration example shows the corresponding APIs. Measured speedups depend on workload; see the [committed XML benchmark results](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md).

### Comparison: Deserialization

=== "PolyXML (Native Rust)"
    ```python
    import polyxml
    # Parse XML into typed dataclasses
    result = polyxml.deserialize(xml_bytes, Catalog)
    ```

=== "xsdata (Pure Python)"
    ```python
    from xsdata.formats.dataclass.parsers import XmlParser
    # Equivalent xsdata API
    parser = XmlParser()
    result = parser.from_bytes(xml_bytes, Catalog)
    ```

=== "xml.etree.ElementTree"
    ```python
    import xml.etree.ElementTree as ET
    # Parsing plus manual conversion into typed objects
    root = ET.fromstring(xml_bytes)
    items = [
        CatalogItem(
            id=int(el.attrib["id"]),
            name=el.findtext("name"),
            price=float(el.findtext("price"))
        )
        for el in root.findall("item")
    ]
    ```

### Key Migration Benefits:
- **No Parser Contexts Needed**: `polyxml.deserialize` is a pure function.
- **Fast Constructor Calling**: PolyXML uses positional tuples `cls(*args)` internally, eliminating dictionary allocations.
- **Zero Schema Compilation**: Works directly with standard `@dataclass` and Pydantic models.

---

## 7. Zero-GIL Binary Serialization (`dumps_binary` & `loads_binary`)

When caching parsed models in transactional key-value stores (such as `libmdbx`, `LMDB`, or `Redis`) or passing objects across multiprocessing workers, re-serializing to XML or using Python's standard `pickle`/`cloudpickle` creates severe CPU and GIL bottlenecks.

`polyxml.dumps_binary` and `polyxml.loads_binary` provide high-throughput MessagePack binary serialization:

```bash
pip install "polyxml[msgpack]"
```

### Usage Example

```python
from dataclasses import dataclass
from decimal import Decimal
from enum import Enum
import pathlib
from xml.etree.ElementTree import QName
from pyxsdata.models.datatype import XmlDate, XmlDateTime, XmlDuration, XmlTime
import polyxml

class RouteMode(str, Enum):
    BUS = "bus"
    RAIL = "rail"

@dataclass
class ServiceJourney:
    id: str
    mode: RouteMode
    fare: Decimal
    service_date: XmlDate
    departure: XmlTime
    timestamp: XmlDateTime
    duration: XmlDuration
    schema_type: QName
    source_file: pathlib.Path

journey = ServiceJourney(
    id="SJ-402",
    mode=RouteMode.BUS,
    fare=Decimal("4.50"),
    service_date=XmlDate(2026, 9, 12),
    departure=XmlTime(14, 30, 0),
    timestamp=XmlDateTime(2026, 9, 12, 14, 30, 0),
    duration=XmlDuration("PT45M"),
    schema_type=QName("http://www.netex.org.uk/netex", "ServiceJourney"),
    source_file=pathlib.Path("/data/netex/timetable.xml"),
)

# 1. Direct typed binary encoding & decoding
blob = polyxml.dumps_binary(journey)
restored = polyxml.loads_binary(blob, ServiceJourney)
assert restored == journey

# 2. Self-describing tagged envelopes (tag_class=True)
# Encodes (module:qualname, payload) so untyped loads_binary() reconstructs the exact class
tagged_blob = polyxml.dumps_binary(journey, tag_class=True)
dynamic_obj = polyxml.loads_binary(tagged_blob)
assert isinstance(dynamic_obj, ServiceJourney)
assert dynamic_obj.fare == Decimal("4.50")
assert dynamic_obj.duration == XmlDuration("PT45M")

# 3. Full Pydantic v2 support
from pydantic import BaseModel

class UserProfile(BaseModel):
    username: str
    balance: Decimal

user = UserProfile(username="alex", balance=Decimal("125.75"))
pydantic_blob = polyxml.dumps_binary(user)
restored_user = polyxml.loads_binary(pydantic_blob, UserProfile)
assert restored_user.balance == Decimal("125.75")
```

### Key Performance & Architecture Advantages:
- **Native MessagePack support**: `dumps_binary()` and `loads_binary()` work with typed XML models. Measure size and throughput with your own records and storage engine.
- **Universal Schema Leaf Support**: Out-of-the-box lossless handling of `XmlDate`, `XmlDateTime`, `XmlDuration`, `XmlTime`, `Decimal`, `QName`, `Enum`, `Path`, `UserString`, and any object implementing `.from_string()`.
- **Zero Schema Compilation**: Introspects dataclasses dynamically in C with zero manual boilerplate or per-class serializer generation.

---

## 7. Native JSON Serialization & Deserialization (`xsdata` Replacement)

PolyXML provides high-throughput native JSON serialization and deserialization directly in Rust via `serde_json`. You can now **completely ditch `xsdata`** for both XML and JSON data-binding.

### Functional API

```python
import polyxml
from dataclasses import dataclass, field

@dataclass
class User:
    user_id: int = field(metadata={"name": "userId", "type": "Attribute"})
    full_name: str = field(metadata={"name": "fullName", "type": "Element"})

user = User(user_id=42, full_name="Ada Lovelace")

# 1. Serialize to JSON bytes (by_alias=True by default for XML schema names)
json_bytes = polyxml.serialize_json(user, indent=2)

# 2. Or serialize directly to a JSON string
json_str = polyxml.dumps_json(user, indent=2)

# 3. Deserialize JSON back into typed dataclasses or Pydantic models
restored = polyxml.deserialize_json(json_bytes, User)
restored_from_str = polyxml.loads_json(json_str, User)
```

### Dual-Key Matching Resilience

Unlike `xsdata`—which fails with `"Unknown property User.user_id"` if incoming JSON uses Python snake_case attribute names instead of camelCase schema names—PolyXML seamlessly accepts **both**:
- Schema alias names (`{"userId": 42, "fullName": "Ada Lovelace"}`)
- Python field names (`{"user_id": 42, "full_name": "Ada Lovelace"}`)

### 100% Drop-In Compatibility with `xsdata`

Migrating an existing codebase from `xsdata` requires **zero code changes**—simply swap your imports:

```python
# Before (xsdata):
# from xsdata.formats.dataclass.serializers import JsonSerializer, XmlSerializer
# from xsdata.formats.dataclass.parsers import JsonParser, XmlParser

# After (PolyXML drop-in replacement):
from polyxml import JsonSerializer, JsonParser, XmlSerializer, XmlParser
# Or:
from polyxml.compat.xsdata import JsonSerializer, JsonParser, XmlSerializer, XmlParser

# Existing parser and serializer calls retain the same shape:
serializer = JsonSerializer(indent=2)
json_str = serializer.render(user)

parser = JsonParser()
user = parser.from_string(json_str, User)
user = parser.parse("data.json", User)
```

## 8. High-Performance XML ↔ JSON Transcoding (`polyxml.xml_to_json` & `polyxml.json_to_xml`)

PolyXML provides Rust-backed functions to convert complete XML and JSON documents without a Python-level parsing loop. Each call holds the input and output in memory; schema-free conversion also builds an intermediate JSON value tree.

### Schema-Directed Transcoding

Passing an XSD schema guarantees that scalar types (integers, floats, booleans) and list elements in JSON conform precisely to your XML schema definition:

```python
import polyxml

xml_payload = b"""<Order id="101"><customer>Alice</customer><total>49.99</total></Order>"""

# 1. XML to JSON using XSD schema guidance
json_bytes = polyxml.xml_to_json(
    xml_payload,
    schema_path="schemas/order.xsd",
    indent=2
)
print(json_bytes.decode("utf-8"))
# Output:
# {
#   "@id": 101,
#   "customer": "Alice",
#   "total": 49.99
# }

# 2. JSON to XML with schema guidance and root element
restored_xml = polyxml.json_to_xml(
    json_bytes,
    schema_path="schemas/order.xsd",
    root="Order",
    indent=2
)
print(restored_xml.decode("utf-8"))
```

### Model-Directed Transcoding

You can also pass any generated dataclass or Pydantic model class to guide transcoding:

```python
from generated.models import Order
import polyxml

# Transcode using model metadata
json_output = polyxml.xml_to_json(xml_payload, model=Order, indent=2)
xml_output = polyxml.json_to_xml(json_output, model=Order, indent=2)
```

### Dynamic Schema-Less Transcoding

When no schema or model is available, PolyXML dynamically converts arbitrary XML to JSON while faithfully preserving attributes with the `@` prefix and mixed/text content with `#text`:

```python
arbitrary_xml = b'<Response status="200"><message>OK</message></Response>'

# Schema-less transcoding
dynamic_json = polyxml.xml_to_json(arbitrary_xml, indent=2)
# Output: {"@status": "200", "message": "OK"}

# Convert back to XML
restored_xml = polyxml.json_to_xml(dynamic_json, root="Response")
```

### Universal Input Support

Both functions accept raw `bytes`, `str`, `pathlib.Path`, and open file/stream objects (`IO[bytes]`, `IO[str]`):

```python
from pathlib import Path

# From Path to bytes:
json_bytes = polyxml.xml_to_json(Path("order.xml"), schema_path=Path("order.xsd"))

# Directly with open file streams:
with open("order.xml", "rb") as f:
    json_bytes = polyxml.xml_to_json(f, indent=2)
```

---

## 7. Ahead-of-Time (AOT) Compiled Native Extensions (`--backend aot`)

For ultra-high-throughput pipelines, streaming microservices, or memory-critical environments, PolyXML can compile your XML Schema directly into a standalone, Ahead-of-Time (AOT) compiled Rust PyO3 native extension.

Instead of generating pure-Python `@dataclass` or Pydantic models that parse XML through Python-level reflection, `--backend aot` compiles:
1. Native Rust `struct` representations with `#[pyclass]` annotations.
2. Inlined zero-copy XML and JSON parsers (`quick-xml` & `serde_json`).
3. Complete PEP 561 type stubs (`.pyi` and `py.typed`) for strict IDE autocompletion (Pyright, Mypy).

### Generating & Building an AOT Extension

```bash
# 1. Generate the standalone PyO3 native extension crate
polyxml generate schema.xsd -l python -b aot -p my_extension -o ./generated/my_extension

# 2. Compile into your active Python environment using maturin
cd ./generated/my_extension
maturin develop --release
```

### Usage

```python
import my_extension

# Native parsing directly into a C-extension PyObject:
reading = my_extension.SensorReadingType.from_xml(xml_bytes)
print(f"ID: {reading.sensorId}, Temp: {reading.temperature}")

# Native fast serialization:
xml_bytes = reading.to_xml(indent=2)
json_bytes = reading.to_json(indent=2)
```

### Production Reference & Benchmarks

- **Real-World Showcase**: The **[polyxml-defense-examples](https://github.com/polyxml/polyxml-defense-examples)** repository includes an end-to-end AOT C2 telemetry bridge ([`examples/python/bridge_aot.py`](https://github.com/polyxml/polyxml-defense-examples/blob/main/examples/python/bridge_aot.py)) for USAF UCI v2.5 schemas.
- **Detailed Benchmarks**: See the [Python AOT vs Dataclass Benchmark](../benchmarks/python-aot-vs-dataclass.md) for full metrics showing 3.4x higher throughput and 60.3% lower peak memory.
