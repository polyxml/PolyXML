---
title: Why PolyXML
description: An architectural comparison of PolyXML against legacy XML binding toolchains (JAXB, CodeSynthesis, xsdata, xgen, xsd.exe) and performance benchmarks.
---

# Why PolyXML? The Architecture of Modern XML

XML and W3C XML Schema (XSD) underpin the critical transactional infrastructure of global commerce, governance, and industry: **interbank messaging (ISO 20022)**, **aviation telematics (FIXM, AIXM)**, **defense command and control**, and **healthcare interchange (HL7)** all depend strictly on complex, deeply constrained XML schemas.

Yet, for over twenty years, the developer tooling landscape for XML has suffered from **chronic stagnation**. While binary serialization ecosystems like Protocol Buffers (`protoc`) and FlatBuffers (`flatc`) evolved unified cross-platform compilers with zero-cost abstractions, XML data binding remained trapped in fragmented language silos, crippled by legacy paradigms and prohibitive performance penalties.

**PolyXML was created to solve this stagnation.**

---

## 🥊 The Competitive Landscape: Legacy Tools vs. PolyXML

| Ecosystem | Legacy / Competitor Tool | Architecture & Runtime | Modern Idiom Alignment | Critical Operational Friction | PolyXML Modern Approach |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Java** | **Jakarta JAXB (`xjc`)** | JAXP / StAX with reflection | ❌ **Low**: Mutable JavaBeans, no-arg constructors, getters/setters | Reflection overhead; no XSD 1.1 support; lacks standard Bean Validation annotations from facets; cannot emit immutable records natively | ✅ **Java 22+ Records & Sealed Interfaces**: Exhaustive switch pattern matching, direct streaming codecs without reflection, Jakarta validation constraints, Panama FFI |
| **Java** | **Apache XMLBeans** | In-memory XML store maintaining full Infoset | ❌ **Very Low**: Classes extending `XmlObject` | Materialized XML store and pointer traversal on field access | ✅ **Streaming Core**: No intermediate DOM in the Rust parser |
| **C#** | **`XmlSchemaClassGenerator` / `xsd.exe`** | Reflection `System.Xml.Serialization` | 🟡 **Moderate**: Partial classes, nullability | Splits unbounded `xs:choice` into separate lists (destroying document order); unions degrade to strings; no source generator | ✅ **C# 12 / .NET 8+ Records**: Primary constructors, polymorphic choice unions preserving order, `System.Text.Json` source generator contexts, `IValidatableObject` validation |
| **C++** | **CodeSynthesis XSD** | Hard dependency on **Apache Xerces-C++** | ❌ **Low**: Pre-C++11 raw pointers, `auto_ptr`, Boost wrappers | Massive binary footprint; expensive **UTF-8 ↔ UTF-16 (`XMLCh`) transcoding**; punitive **GPL v2 / commercial dual-license** | ✅ **Modern C++20/C++23**: `std::variant`, `std::optional`, `std::string_view`, concepts, zero Xerces dependency, **permissive MIT license** |
| **C++** | **gSOAP (`soapcpp2`)** | Custom low-level C parser with macro tables | ❌ **Very Low**: Procedural C/C++ | Global state variables; namespace collisions; fragile memory ownership; GPL/commercial dual-license | ✅ **Thread-Safe Modern Value Types**: RAII memory management, CMake/Meson module export |
| **Python** | **`xsdata`** | Pure Python over `lxml` or `xml.etree` | 🟡 **High**: Emits `@dataclass` and Pydantic v2 | [10.0x slower to read and 23.5x slower to write](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md) on the 10,000-item catalog workload | ✅ **Rust PyO3 Engine**: Typed models, in-memory XML entity handling, and PEP 695 type aliases |
| **Python** | **`generateDS`** | Monolithic Python script with string matching | ❌ **Very Low**: Legacy procedural classes | Monolithic un-typed files; fails on substitution groups and circular definitions | ✅ **Pydantic v2 & `@dataclass(slots=True)`**: Complete restriction facet validation and IDE autocomplete |
| **Rust** | **`xsd-parser-rs`** | quick-xml / serde-xml-rs derive attributes | 🟡 **Moderate**: Rust structs with serde | **Panics on enterprise schemas** (ISO 20022); cross-namespace type name collisions; serde impedance mismatch on duplicate element sequences | ✅ **Pure-Rust Compiler & Zero-Copy Codecs**: Tarjan SCC cycle-cutting (`Box<T>`), streaming `Cow<'a, str>`, zero Serde mismatch |
| **Go** | **`xuri/xgen` / `goxsd`** | Direct SAX mapping to `encoding/xml` | 🟡 **Moderate**: Standard Go structs | **Collapses `xs:choice` into optional pointers** (losing mutual exclusivity); drops structs on multi-file schemas; no facet validation | ✅ **Go 1.22+ Structs with Choice Validation**: Custom `UnmarshalXML` enforcing mutual exclusivity, pointer cycle cuts, canonical initialisms (`ID`, `URL`) |
| **TypeScript** | **`cxsd`** | JSON-like intermediate mapping | ❌ **Low**: Ambient `.d.ts` classes | **Abandoned project**; no ES Module support; loses choice element ordering; no runtime facet validation | ✅ **TypeScript 5+ & Runtime Zod Schemas**: Discriminated unions, `as const` enums, circular reference handling via `z.lazy()` |

---

## 🔍 In-Depth Ecosystem Comparison & Migration Guide

### ☕ Java: Moving Beyond JAXB (`xjc`) & XMLBeans

For over two decades, **JAXB / Jakarta XML Binding (`xjc`)** has been the industry standard for Java. While highly mature with an extensive Maven/Gradle plugin ecosystem, it was architected during the Java 1.4/5 era and carries substantial technical debt for modern cloud-native systems:
- **No XSD 1.1 Support**: Issues requesting XSD 1.1 support ([eclipse-ee4j/jaxb-ri#1176](https://github.com/eclipse-ee4j/jaxb-ri/issues/1176)) have remained unresolved for years, preventing the use of modern schema assertions (`<xs:assert>`) and `openContent`.
- **Missing Schema Facet Annotations**: JAXB does not translate XSD facets (`minLength`, `maxLength`, `pattern`, `minInclusive`) into standard Jakarta Bean Validation annotations ([eclipse-ee4j/jaxb-ri#917](https://github.com/eclipse-ee4j/jaxb-ri/issues/917)).
- **Reflection & Classloader Overhead**: Dynamic reflection injection ([eclipse-ee4j/jaxb-ri#564](https://github.com/eclipse-ee4j/jaxb-ri/issues/564)) and circular class hierarchy deadlocks ([#312](https://github.com/eclipse-ee4j/jaxb-ri/issues/312)) create friction with GraalVM native compilation and modular Java runtimes.

**How PolyXML Compares**:
- **Modern Java 22+ Constructs**: Emits immutable `record` types with `sealed interface` choice variants for exhaustive switch pattern matching.
- **Direct Zero-Reflection StAX Codecs**: Generates direct `XMLStreamWriter` / `XMLStreamReader` codecs that bypass runtime reflection entirely, providing sub-microsecond throughput and instant GraalVM native image compatibility.
- **Backward-Compatible Drop-In**: If your existing codebase expects classic JavaBeans, `--style pojo --feature builder,direct-codec` emits mutable POJOs with fluent builders that integrate directly with legacy frameworks.

---

### 🔷 C# / .NET: Modernizing from `XmlSchemaClassGenerator` & `xsd.exe`

In the .NET ecosystem, Microsoft's legacy `xsd.exe` generated archaic C# 2.0 code with raw arrays. **`XmlSchemaClassGenerator`** emerged as the modern open-source standard, significantly improving upon `xsd.exe` by supporting nullable reference types and `List<T>`. However, real-world enterprise deployments encounter several key limitations:
- **Choice Sequence Splitting**: In unbounded choice groups (`<xs:choice maxOccurs="unbounded">`), `XmlSchemaClassGenerator` splits choices into separate lists (`itemsA: List<ItemA>`, `itemsB: List<ItemB>`), destroying interleaved document order upon serialization ([mganss/XmlSchemaClassGenerator#616](https://github.com/mganss/XmlSchemaClassGenerator/issues/616)).
- **Untyped Unions**: XML simple type unions (`<xs:union>`) default to untyped string serialization ([#397](https://github.com/mganss/XmlSchemaClassGenerator/issues/397)).
- **Runtime Reflection Dependency**: Operates against classic reflection `XmlSerializer`, lacking AOT source generators ([#277](https://github.com/mganss/XmlSchemaClassGenerator/issues/277)).

**How PolyXML Compares**:
- **C# 12 Primary Constructor Records**: Generates modern immutable records with primary constructors.
- **Document Order Preservation**: Generates polymorphic choice unions with `[XmlElement("itemA", typeof(...))]` on an ordered collection, guaranteeing that interleaved sequence order is preserved during serialization.
- **Dual JSON & XML Serialization**: Simultaneously generates `[JsonPropertyName]` and `[JsonSerializable]` contexts for native .NET 8+ `System.Text.Json` source generator pipelines.

---

### 🐍 Python: Upgrading from `xsdata` & `generateDS`

In Python, **`xsdata`** is the leading modern standard, producing clean Python standard dataclasses and Pydantic models. However:
- **Pure-Python Performance Ceiling**: Because `xsdata` parses XML using pure Python tree traversal, its parsing throughput is bounded by Python interpreter overhead. In our published [10,000-item catalog benchmark](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md), PolyXML's typed binding is **10.0x faster to read and 23.5x faster to write**.
- **Memory Overhead on Massive Schemas**: Complex enterprise schemas with thousands of types (such as USAF UCI with 5,558 types) can consume gigabytes of memory or crash during pure-Python generation.

**How PolyXML Compares**:
- **Blazing Native Engine**: A safe, pre-compiled Rust PyO3 engine executes event streaming and scalar conversion in compiled native code.
- **AOT Native Extensions**: Generates Ahead-of-Time native C-extension / PyO3 bindings (`--backend aot`) that deliver up to **134,000 ops/sec at 7.4 µs latency**.
- **Modern Python Typing**: Strict adherence to Python 3.12+ PEP 695 type aliases (`type Sku = str`) and PEP 604 union types (`TypeA | TypeB`).

---

### 🐹 Go: Structured Contracts Beyond `xuri/xgen`

**`xuri/xgen`** is a versatile multi-language parser for compiling XSD schemas into Go structs. However:
- **Multi-File Instability**: Frequently drops structs or elements when compiling complex multi-file industry schemas ([xuri/xgen#80](https://github.com/xuri/xgen/issues/80), [#93](https://github.com/xuri/xgen/issues/93)).
- **Scalar Mapping Discrepancies**: Lacks custom parsers for Gregorian date types like `xs:gDay` ([#13](https://github.com/xuri/xgen/issues/13)) and maps `xs:byte` to `byte` (uint8) instead of signed `int8` ([#58](https://github.com/xuri/xgen/issues/58)).
- **Choice Exclusivity**: Collapses `<xs:choice>` into bare pointers without enforcing mutual exclusivity during unmarshaling.

**How PolyXML Compares**:
- **Strict Choice Exclusivity**: Emits custom `UnmarshalXML` and `Validate()` methods that verify only one branch of a choice is populated.
- **Unbounded Choice Ordering**: Emits `Items []ContainerChoice \`xml:",any"\`` to preserve interleaved document order.
- **Idiomatic Go Conventions**: Automatically normalizes acronyms (`ID`, `URL`, `UUID`, `HTTP`) according to Go naming conventions.

---

### 🌐 TypeScript: Type Safety Beyond `cxsd`

**`cxsd`** pioneered streaming XSD-to-TypeScript generation, but has been unmaintained for several years:
- **Ordering Loss**: Choice sequences lose element ordering ([charto/cxsd#29](https://github.com/charto/cxsd/issues/29)).
- **No Runtime Validation**: Generates ambient `.d.ts` declaration files, providing zero runtime schema validation in modern Node.js or browser environments.

**How PolyXML Compares**:
- **TypeScript 5+ Discriminated Unions**: Emits tagged unions (`kind: "itemA"`) and `as const` object dictionaries.
- **Runtime Validation Integration**: Simultaneously emits validation schemas for **Zod** (`z.discriminatedUnion`), **Valibot**, or **TypeBox**, giving frontend and backend TypeScript applications end-to-end type safety and validation.

---

### 🦀 Rust: Enterprise Schema Resilience Beyond `xsd-parser-rs`

Existing Rust XSD generators like `xsd-parser-rs` ([Bergmann89/xsd-parser](https://github.com/Bergmann89/xsd-parser)) provide helpful initial steps towards serde-annotated structs, but encounter key friction points on enterprise schemas:
- **Cross-Namespace Collisions**: Referencing the same type name across different target namespaces causes identifier collisions in generated Rust modules ([Bergmann89/xsd-parser#150](https://github.com/Bergmann89/xsd-parser/issues/150)).
- **Choice Crashes**: Crashes on `<xs:choice maxOccurs > 1>` ([#137](https://github.com/Bergmann89/xsd-parser/issues/137)).
- **Namespace Qualification Issues**: Deserialization fails with `elementFormDefault="qualified"` prefixing ([#124](https://github.com/Bergmann89/xsd-parser/issues/124)).

**How PolyXML Compares**:
- **Tarjan SCC Cycle-Cutting**: Automatically identifies recursive type cycles and applies minimal `Box<T>` boxing.
- **Zero-Copy Streaming**: Leverages `Cow<'a, str>` and custom `quick-xml` state machines, eliminating Serde impedance mismatches on complex schema constructs.

---

### ⚡ C++: Escaping CodeSynthesis XSD & gSOAP Licensing Traps

Historical C++ tools like **CodeSynthesis XSD** and **gSOAP** provide high execution performance, but impose significant organizational barriers:
- **Punitive Dual-Licensing**: Enforce strict commercial paywalls or copyleft GPL v2 licensing, exposing commercial products to licensing contamination.
- **Legacy Dependencies**: Hard dependencies on **Apache Xerces-C++** require heavy runtime dynamic libraries and expensive UTF-8 $\leftrightarrow$ UTF-16 (`XMLCh`) string transcoding on every text node.

**How PolyXML Compares**:
- **Permissive MIT License**: 100% open source with zero commercial fees or licensing traps.
- **Modern C++20/C++23**: Generates clean value types with `std::variant`, `std::optional`, `std::string_view`, and C++20 concepts with zero Apache Xerces dependency.

---

---

## ⚡ The 6 Pillars of PolyXML

### 1. The `protoc` of XML: Unified Intermediate Representation (`SchemaIR`)
Legacy XML tools treated code generation as a local script within each programming language. When an enterprise schema failed in Python, teams had to write bespoke monkey-patches; when it failed in C++, teams bought expensive commercial licenses.

PolyXML operates as a **single, unified compiler frontend** written in safe, high-performance Rust:
- Ingests W3C XSD 1.0 and 1.1 schemas, resolving multi-namespace imports, transitive includes, and schema component redefinitions (`<xs:redefine>`).
- Lowers schema components into a language-agnostic Intermediate Representation (**PolyXML-IR**).
- Computes **Tarjan's Strongly Connected Components (SCC)** algorithm across type dependency graphs to identify and cut recursive cycles (`Box<T>`, pointers, `std::unique_ptr`, `z.lazy`).
- Guarantees that **all 7 target languages** receive structurally identical, bug-free data contracts from the exact same schema.

### 2. Zero-Allocation Streaming Runtime vs. Intermediate DOM Memory Bloat
Some XML data-binding libraries construct an intermediate Document Object Model (DOM) tree before populating user objects. This can increase peak memory and garbage collection pressure; the actual cost depends on the document and library.

PolyXML eliminates intermediate DOM allocations entirely:
- **Direct Event Streaming**: Feeds raw bytes directly through a monomorphized `quick-xml` event state machine.
- **Slice Conversions with `lexical-core`**: Converts numeric and boolean scalars directly from ASCII byte slices into native integers and floats without intermediate heap string allocations.
- **Borrowed Text Where Supported**: Rust generated models can use `Cow<'a, str>`; check each target's ownership model before assuming a zero-copy parse.

### 3. Modern Language Idioms (2024–2026) vs. 20-Year-Old Code Generation
Most legacy compilers were architected during the Java 5 / C++98 era. They generate sprawling boilerplate:
- **Immutable by default, mutable on request**: PolyXML generates immutable Java 22+ `record` types and `sealed interface` choice models that support compiler-enforced pattern matching without default branches. When a legacy framework requires JavaBeans, `--style pojo --feature builder` emits no-arg classes with getters/setters and fluent builders instead — same facets, same codecs.
- **No more raw pointers or Xerces**: PolyXML generates clean C++20 value types, `std::variant`, and C++20 concepts with zero external runtime dependencies.
- **No more untyped Python bags**: PolyXML generates `@dataclass(slots=True, kw_only=True)` and Pydantic v2 models leveraging Python 3.12 PEP 695 type aliases (`type Sku = ...`) and PEP 604 union syntax (`TypeA | TypeB`).

### 4. Permissive Open Source (MIT) vs. Commercial Paywalls
Historical C++ tools like CodeSynthesis XSD and gSOAP enforce strict **GPL v2 / commercial dual-licensing**. Incorporating them into proprietary cloud microservices, aerospace avionics, or banking applications forces enterprises to pay thousands of dollars in per-seat or per-server licensing fees, or risk GPL license contamination.

PolyXML is **100% permissively licensed under the MIT License**, with zero runtime licensing fees, zero commercial paywalls, and zero legal restrictions on proprietary distribution.

### 5. Dual-Format Polyglot Architecture & XML/JSON Transcoding (`polyxml transcode`)
Enterprise engineering rarely lives in an XML-only silo. Interbank rails (ISO 20022), aviation telemetry (FIXM), and healthcare networks (HL7) mandate strict XML Schema contracts, but modern cloud services, microservices, and frontends operate on JSON.

Historically, bridging this divide forced engineering teams into painful trade-offs:
- **Untyped Parser Mapping**: Ad-hoc scripts need explicit rules for XML attributes and repeated elements. The [Python XML benchmark](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md) measures `xmltodict` on specific inputs.
- **Duplicate Schema Maintenance**: Manually writing and synchronizing separate XSD and OpenAPI/JSON schemas across teams inevitably leads to silent drift and catastrophic production outages.

PolyXML breaks this dichotomy through a **natively dual-format architecture**:
- **Whole-Document Transcoder (`polyxml transcode`)**: A Rust-powered CLI and runtime converter for XML ↔ JSON. It accepts stdin/stdout pipes, but buffers each complete input and output document; schema-free conversion builds an intermediate JSON value tree.
- **Schema-Directed Precision**: Use `--schema schema.xsd` to ensure numeric types, booleans, and arrays in JSON match the exact XSD type definitions rather than ambiguous strings.
- **Dynamic Schema-Less Fallback**: Automatically preserves XML attributes (`@attr`) and text content (`#text`) in pure JSON when no schema is present.
- **Natively Dual-Annotated Generated Models**:
  - **Go**: Generated structs include both `xml:"..."` and `json:"..."` tags, with `json:"-"` on `XMLName`, allowing identical structs to marshal to both formats with Go's standard libraries.
  - **C#**: Emits `[property: JsonPropertyName("...")]` on primary constructor records and `[JsonConverter(typeof(JsonStringEnumConverter))]` on enums for native .NET `System.Text.Json` serialization.
  - **Rust**: Inherent zero-copy `.to_json_string()`, `.to_json_vec()`, `.from_json_str()`, and `.from_json_slice()` methods alongside XML codecs, with Serde rename support.
  - **Python**: Inherent `.to_json()` and `@classmethod from_json()` on generated models, compatible `JsonSerializer` / `JsonParser` APIs, and direct `polyxml.xml_to_json()` / `polyxml.json_to_xml()`.

### 6. Controlled XML Entity Handling

PolyXML's native streaming runtime uses `quick-xml` events and resolves standard
XML entities and numeric character references in memory. Unknown named
references are treated as text; the runtime does not fetch external entity
URLs or local files while parsing XML. The schema compiler is a separate path
that reads local XSD includes and imports. Generated codecs use their target
language's XML libraries, so their entity behavior should be assessed in the
consuming application.

---

## 📊 Performance Benchmarks: Head-to-Head

### 1. Python Deserialization & Serialization Throughput
*Workload: 10,000 complex business items (~724 KB XML) measured with Python 3.12 (`abi3-py312`)*

[Committed benchmark results and runner](https://github.com/polyxml/PolyXML/tree/main/benchmarks/python) support this table and chart. Different parsers return different object shapes.

```
Deserialization Throughput (Higher is Better)
PolyXML (Typed Dataclass)    ████████████████████████████████ 29.8 MB/s  (10.0x faster)
ElementTree (Untyped DOM)    ████████████████████████████████████ 53.0 MB/s
xmltodict (Untyped Dict)     ████████ 12.5 MB/s
xsdata (Typed Dataclass)     ██ 3.0 MB/s

Serialization Throughput (Higher is Better)
PolyXML (Typed Dataclass)    ████████████████████████████████████ 58.1 MB/s  (23.5x faster)
xmltodict (Untyped Dict)     █████ 8.9 MB/s
xsdata (Typed Dataclass)     █ 2.5 MB/s
```

| Engine | Data Model | Deserialization Latency | Deserialization Speedup | Serialization Latency | Serialization Speedup | Peak RAM |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **PolyXML** | **Typed Dataclass** | **24.0 ms** | **10.0x** | **12.2 ms** | **23.5x** | **2.0 MB** |
| `lxml.objectify` | Dynamic C Proxy | 11.8 ms | 20.4x | 4.1 ms | 70.1x | 0.2 MB |
| `ElementTree` | Untyped DOM | 13.6 ms | 17.8x | — | — | 7.1 MB |
| `defusedxml` | Secure DOM | 29.5 ms | 8.2x | — | — | 7.1 MB |
| `xmltodict` | Untyped Dict | 57.5 ms | 4.2x | 80.0 ms | 3.6x | 4.8 MB |
| `xsdata` | Typed Dataclass | 241.9 ms | 1.0x (Ref) | 298.8 ms | 1.0x (Ref) | 3.3 MB |

### 2. Real-Time Micro Telemetry (Sensor ~100B, UCI Telemetry)
*Workload: High-frequency telemetry packets in avionics, robotics, and financial feeds*

Source: [Python sensor benchmark results](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md).

| Engine | Paradigm | Deserialization Latency | Speedup vs Standard Python |
| :--- | :--- | :---: | :---: |
| **PolyXML** | **Typed Dataclass** | **3.2 μs** | **13.9x** |
| **PolyXML (Pydantic)** | **Typed Pydantic v2** | **3.8 μs** | **11.8x** |
| `lxml.etree` | Untyped DOM | 3.3 μs | 13.4x |
| `ElementTree` | Untyped DOM | 5.3 μs | 8.4x |
| `defusedxml` | Secure DOM | 9.1 μs | 4.9x |
| `xmltodict` | Untyped Dict | 10.6 μs | 4.2x |
| `pydantic-xml` | Typed Pydantic v2 | 17.1 μs | 2.6x |
| `xsdata` | Typed Dataclass | 44.5 μs | 1.0x (Ref) |

> **Telemetry Benchmark Summary**: PolyXML deserializes packets in **3.2 microseconds**—neck-and-neck with raw C-based DOM parsers (`lxml` at 3.3 μs) while delivering fully typed, validated dataclasses.

---

### 3. Native JSON Data-Binding
PolyXML offers Rust-backed JSON serialization and deserialization with compatible `JsonParser` and `JsonSerializer` APIs. Run a like-for-like benchmark on your model before making a JSON speedup claim.

---

### 4. Pure Rust Core Throughput (`crates/polyxml-core`)
*Statistical benchmarks measured using Criterion.rs; [benchmark source](https://github.com/polyxml/PolyXML/blob/main/crates/polyxml-core/benches/core_benchmarks.rs). These historical figures have no committed raw Criterion report; rerun before using them for a new performance claim.*

| Workload | Operation | Latency | Throughput | Allocation Strategy |
| :--- | :--- | :---: | :---: | :--- |
| **Sensor Micro (130B)** | Deserialization | **1.19 μs** | **75.1 MiB/s** | Direct scalar parse, 0 DOM |
| **Sensor Micro (130B)** | Serialization | **479 ns** | **187.2 MiB/s** | Zero allocation |
| **Catalog (1,000 items, ~70 KB)** | Deserialization | **1.01 ms** | **60.2 MiB/s** | Streaming buffer |
| **Catalog (1,000 items, ~70 KB)** | Serialization | **358 μs** | **169.1 MiB/s** | Streaming buffer |
| **Catalog (10,000 items, ~724 KB)** | Deserialization | **10.18 ms** | **62.7 MiB/s** | Streaming buffer |
| **Catalog (10,000 items, ~724 KB)** | Serialization | **3.61 ms** | **175.2 MiB/s** | Streaming buffer |

---

## 🏛️ Official W3C XSTS Conformance Tested

Unlike experimental open-source compilers that panic when encountering complex enterprise schemas, PolyXML is continuously validated against the **official W3C XML Schema 1.0 / 1.1 Test Suite (XSTS)** using our dedicated testing repository, **[polyxml-w3c-tests](https://github.com/polyxml/polyxml-w3c-tests)**.

Across more than 600 official test groups from Sun Microsystems, Microsoft, and NIST:
- **Schema Compilation Pass Rate**: **635 / 636 groups passed (99.8%)**
- **Instance Validation & Round-Trip Pass Rate**: **489 / 507 instances passed (96.4%)**
- Full handling of anonymous types, unbounded compositor propagation, recursive inheritance cycle-cutting, and substitution groups.

---

## 🚀 The Bottom Line

| If you are using... | PolyXML gives you... |
| :--- | :--- |
| **JAXB / `xjc` in Java** | Immutable Java 22+ records, sealed interface choices, zero reflection overhead, and Project Panama FFI — or drop-in JavaBeans with fluent builders and zero-reflection StAX codecs (`--style pojo --feature builder,direct-codec`) for existing codebases. |
| **CodeSynthesis in C++** | Modern C++20 value types, `std::variant`, zero Apache Xerces dependency, zero UTF-16 transcoding overhead, and a permissive MIT license. |
| **`xsdata` in Python** | [10.0x faster XML reading and 23.5x faster XML writing](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md) on the 10,000-item catalog, compatible JSON parser and serializer APIs, and direct XML/JSON transcoding. |
| **`xsd-parser` in Rust** | A battle-tested compiler that doesn't panic on complex schemas, with automatic Tarjan `Box<T>` cycle breaks, inherent streaming XML codecs, and native `.to_json_string()` codecs. |
| **`xgen` in Go** | Dual `xml:"..."` and `json:"..."` struct tags on every model, true `xs:choice` mutual exclusivity validation, pointer cycle breaks, and canonical Go initialism normalization. |
| **`xsd.exe` in .NET** | Modern C# 12 records with primary constructors, dual `XmlSerializer` and `System.Text.Json` attributes (`[JsonPropertyName]`, `[JsonConverter]`), and standard `IValidatableObject` integration. |
| **Ad-hoc XML ↔ JSON Scripts** | CLI document conversion (`polyxml transcode`) with schema-directed precision or dynamic `@attr` preservation. |

**Ready to modernize your XML infrastructure?**
👉 **[Get Started with the 5-Minute Quickstart →](quickstart.md)**
👉 **[Read the XSD-to-Code Generator & CLI Guide →](guides/compiler.md)**
