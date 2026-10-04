<h1 align="center">
  <img src="docs/assets/brand/logo_polyxml_banner.png" alt="PolyXML" width="1000">
</h1>

<p align="center">
  <strong>The "protoc for XML" — Modern XSD-to-Code Generator & High-Performance Streaming Runtime</strong><br>
  <em>Python • Rust • C++20 • Java 22+ • TypeScript • Go • C# 12</em>
</p>

<p align="center">
  <a href="https://github.com/polyxml/PolyXML/actions"><img src="https://img.shields.io/github/actions/workflow/status/polyxml/PolyXML/ci.yml?branch=main&label=CI&logo=github" alt="CI"></a>
  <a href="https://polyxml.github.io/PolyXML/"><img src="https://img.shields.io/badge/docs-zensical-blue.svg?logo=gitbook" alt="Docs"></a>
  <a href="https://github.com/polyxml/polyxml-w3c-tests"><img src="https://img.shields.io/badge/W3C%20XSTS-99.8%25%20Passed-brightgreen.svg" alt="W3C XSTS Conformance"></a>
  <a href="https://github.com/polyxml/PolyXML/blob/main/crates/polyxml-python/pyproject.toml"><img src="https://img.shields.io/badge/coverage-100%25-brightgreen.svg?logo=pytest" alt="Coverage: 100%"></a>
  <a href="https://github.com/astral-sh/ruff"><img src="https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/astral-sh/ruff/main/assets/badge/v2.json" alt="Ruff"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</p>

<p align="center">
  <a href="https://crates.io/crates/polyxml"><img src="https://img.shields.io/crates/v/polyxml.svg?logo=rust&label=crates.io" alt="crates.io: polyxml"></a>
  <a href="https://crates.io/crates/polyxml-cli"><img src="https://img.shields.io/crates/v/polyxml-cli.svg?logo=rust&label=polyxml-cli" alt="crates.io: polyxml-cli"></a>
  <a href="https://pypi.org/project/polyxml/"><img src="https://img.shields.io/pypi/v/polyxml.svg?logo=pypi&label=PyPI" alt="PyPI: polyxml"></a>
  <a href="https://www.npmjs.com/package/@polyxml/node"><img src="https://img.shields.io/npm/v/@polyxml/node.svg?logo=npm&color=CB3837&label=npm" alt="npm: @polyxml/node"></a>
  <a href="https://central.sonatype.com/artifact/io.github.polyxml/polyxml"><img src="https://img.shields.io/maven-central/v/io.github.polyxml/polyxml.svg?logo=apache-maven&color=C71A36&label=Maven" alt="Maven Central"></a>
  <a href="https://pkg.go.dev/github.com/polyxml/PolyXML/bindings/go"><img src="https://pkg.go.dev/badge/github.com/polyxml/PolyXML/bindings/go.svg" alt="Go Reference"></a>
  <a href="https://github.com/polyxml/homebrew-polyxml"><img src="https://img.shields.io/badge/Homebrew-polyxml-FBB040.svg?logo=homebrew&logoColor=black" alt="Homebrew"></a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.80%2B-orange.svg?logo=rust" alt="Rust: 1.80+"></a>
  <a href="https://www.python.org"><img src="https://img.shields.io/badge/Python-3.12%20%7C%203.13%20%7C%203.14%20%7C%203.15-3776AB.svg?logo=python&logoColor=white" alt="Python: 3.12 | 3.13 | 3.14 | 3.15"></a>
  <a href="https://nodejs.org"><img src="https://img.shields.io/badge/Node.js-20%20%7C%2022-339933.svg?logo=node.js&logoColor=white" alt="Node.js: 20 | 22"></a>
  <a href="https://www.typescriptlang.org"><img src="https://img.shields.io/badge/TypeScript-5.0%2B-3178C6.svg?logo=typescript&logoColor=white" alt="TypeScript: 5.0+"></a>
  <a href="https://openjdk.org/projects/panama/"><img src="https://img.shields.io/badge/Java-22%2B%20Panama-ED8B00.svg?logo=openjdk&logoColor=white" alt="Java: 22+ Panama"></a>
  <a href="https://go.dev"><img src="https://img.shields.io/badge/Go-1.22%2B-00ADD8.svg?logo=go&logoColor=white" alt="Go: 1.22+"></a>
  <a href="https://en.cppreference.com/w/cpp/20"><img src="https://img.shields.io/badge/C%2B%2B-20-00599C.svg?logo=c%2B%2B" alt="C++: 20"></a>
  <a href="https://dotnet.microsoft.com"><img src="https://img.shields.io/badge/.NET-8.0%2B-512BD4.svg?logo=dotnet&logoColor=white" alt=".NET: 8.0+"></a>
</p>

---

## Overview

**PolyXML turns W3C XML Schemas (`.xsd`) into production-ready, type-safe data models with built-in streaming parsers and serializers.**

If you have ever used `xjc` (JAXB), `CodeSynthesis XSD`, or `xsdata`, PolyXML is their modern, safe-Rust replacement. It compiles your schema once and generates idiomatic code across **7 languages simultaneously**. In the published 10,000-item Python catalog benchmark, PolyXML's typed binding is **10.0x faster to read and 23.5x faster to write than `xsdata`** ([benchmark results](benchmarks/python/results.md)); other runtimes and parsers have different results.

Just as Protocol Buffers (`protoc`) and FlatBuffers (`flatc`) modernized binary serialization, **PolyXML brings modern software engineering to XML**:

1. **🛠️ Universal XSD-to-Code Generator (`polyxml`)**: Ingests W3C XSD 1.0 and 1.1 schemas, resolves cyclic types with Tarjan's SCC algorithm, and compiles production-ready, strongly-typed data contracts across **7 modern ecosystems** simultaneously (**Python**, **Rust**, **C++**, **Java**, **TypeScript**, **Go**, and **C#**).
2. **⚡ Ultra-Fast Streaming Runtime**: The Rust core deserializes and serializes directly with `quick-xml` and `lexical-core`, without an intermediate DOM. See the [benchmark methods and per-language coverage](docs/benchmarks/index.md) for measured comparisons.
3. **🏛️ Official W3C XSTS Conformance Tested**: Validated against the official W3C XML Schema Test Suite with a **>99.8% schema compilation pass rate** and **>96% round-trip validation rate** via [polyxml-w3c-tests](https://github.com/polyxml/polyxml-w3c-tests).
4. **📦 Permissive MIT License**: 100% open source with zero commercial licensing fees, eliminating the GPL dual-licensing traps of legacy C++ tools.
5. **🛡️ Controlled XML Entity Handling**: The native streaming runtime resolves standard and numeric character references in memory and does not fetch external entities while parsing XML. Schema compilation separately reads local XSD includes and imports.
6. **🧩 Scalable Enterprise Architecture**: Scales effortlessly to massive schemas (such as USAF UCI v2.5 with 5,558 types or HL7 FHIR) using Tarjan SCC cycle condensation, topological DAG chunking, and root-element graph selection—preventing downstream compiler OOMs, eliminating Python circular import deadlocks, and keeping toolchains responsive.

### Why PolyXML Across Ecosystems

Instead of relying on single-language generators with divergent capabilities, PolyXML unifies your data contracts while generating idiomatic, high-performance code tailored to each target:

- **☕ Java (22+)**: Immutable records with sealed interface choices and pattern matching, direct streaming codecs without runtime reflection overhead, optional Jakarta validation (`@Size`, `@Pattern`, `@Min`, `@Max`), and Jackson 2/3 backends for Spring Boot 3/4.
- **🐍 Python (3.12+)**: Modern `@dataclass(slots=True)` and Pydantic V2 models, up to **10.0x faster read / 23.5x write** than `xsdata`, with optional Ahead-of-Time native PyO3 compiled extensions.
- **🔷 C# / .NET (8+)**: Modern primary constructor `record` types, polymorphic choice unions preserving document order, and native `System.Text.Json` source-gen compatibility.
- **🐹 Go (1.22+)**: Idiomatic structs with `xml:",any"` document order preservation, strict choice exclusivity validation, and simultaneous JSON annotations.
- **🌐 TypeScript (5+)**: Discriminated unions, `as const` enums, and optional runtime Zod, Valibot, or TypeBox validation schemas.
- **🦀 Rust**: Zero-copy borrowed streaming (`Cow<'a, str>`), Tarjan SCC cycle boxing (`Box<T>`), and fast `quick-xml` codecs.
- **⚡ C++ (20/23)**: Modern value types (`std::variant`, `std::optional`, concepts), CMake/Meson export, zero Apache Xerces dependency, and a **100% permissive MIT license** (eliminating commercial/GPL dual-licensing traps).

> 📖 **Evaluating PolyXML against your current toolchain?**  
> Read our in-depth **[Architectural Comparison & Migration Guide](docs/why-polyxml.md)** for a detailed, transparent breakdown and benchmarks across JAXB (`xjc`), `XmlSchemaClassGenerator`, `xsdata`, `xuri/xgen`, `cxsd`, and `CodeSynthesis XSD`.

---

## ⚡ Quick Start: From XSD to Code in Seconds

Install the PolyXML CLI in seconds on Linux and macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/polyxml/PolyXML/main/scripts/install.sh | bash
```

Generate strongly-typed code for all 7 languages from any W3C XML Schema in a single command:

```bash
# 1. Generate all targets with their defaults
polyxml generate schemas/pain.001.001.09.xsd \
  --lang python --lang rust --lang csharp --lang java \
  --lang typescript --lang go --lang cpp --out ./generated

# Java 25 / Spring Boot 4 with Jackson 3
polyxml generate schemas/pain.001.001.09.xsd --lang java \
  --backend jackson3 --style pojo --feature builder --feature validation \
  --package com.enterprise.banking --out ./generated/java

# Filter massive schemas to selected root elements and reachable types
polyxml generate schemas/uci.xsd --lang rust \
  --root-element Entity --root-element PositionReport --out ./generated/uci

# 2. Or build an entire enterprise project declaratively
polyxml build --config polyxml.toml
```

All 7 targets are declared in a single [`polyxml.toml` workspace manifest](docs/guides/compiler.md).

### Consume the Generated Models Instantly

```python
# Generated by polyxml generate --lang python
from generated.python import Customer
import polyxml

customer = Customer.from_xml(xml_bytes)   # streaming XML → typed dataclass
xml_output = customer.to_xml(indent=2)    # typed dataclass → formatted XML
json_bytes = customer.to_json(indent=2)   # native JSON on the same model
```

👉 **[See all 7 languages in the Quickstart →](docs/quickstart.md#consume-the-generated-models-instantly)**

---

## 🔄 Dual-Format XML ↔ JSON Transcoding (`polyxml transcode`)

Bridge legacy enterprise XML (ISO 20022 banking, HL7 healthcare, FIXM aviation) and modern JSON microservices through stdin/stdout CLI pipes or the `polyxml.xml_to_json` / `polyxml.json_to_xml` Python APIs. Each call reads and converts a complete document in memory. **[Full reference →](docs/guides/compiler.md)**

---

## 🚀 Performance Benchmarks

Every published metric is backed by committed raw execution logs, deterministic fixtures, and automated reproduction harnesses ([full methodology & index →](docs/benchmarks/index.md)):

### ⚡ Python XML Data-Binding
- **10.0x faster reads & 23.5x faster writes** vs `xsdata` on a 10,000-item catalog ([raw results](benchmarks/python/results.md), [methodology](benchmarks/python/README.md)).
- **3.2 μs per telemetry packet** (13.9x faster vs `xsdata`) while returning fully-typed Python dataclasses.

### ⚡ Ahead-of-Time (AOT) PyO3 Native Extensions (`--backend aot`)
- **8.4x faster reads & 10.3x faster writes** on real-world USAF UCI mission telemetry: parses XML in **7.44 μs (134,336 ops/sec, 193.5 MB/s)** vs 62.5 μs in standard dataclasses ([AOT benchmark report](docs/benchmarks/python-json-binary-aot-2026-09.md)).
- **60.3% memory reduction**: Retaining 10,000 telemetry objects consumes only 1.02 MB under AOT vs 2.57 MB with slotted dataclasses ([AOT architecture breakdown](docs/benchmarks/python-aot-vs-dataclass.md)).

### ⚡ Built-in Dual JSON & Binary Bindings
- **9.2x to 15.5x faster JSON deserialization** than `xsdata` on identical Python dataclass models (12.8 μs vs 156.5 μs for sensor data; 1.3 ms vs 20.5 ms for 100-item batches).
- **2.3x faster binary decoding** than standard library `pickle` on sensor packets with 36% smaller wire payloads (118 B vs 185 B).

### ⚡ Multi-Language Streaming Core
- Deserializing 1,000-sensor XML batches across native targets: **0.355 ms** in Rust, **0.511 ms** in Java (StAX direct codec), **0.556 ms** in C# (.NET 8), and **1.079 ms** in native C++20 ([7-target language results](docs/benchmarks/language-results-2026-09.md)).

---

## 🌐 Real-World Industry Showcases & Polyglot Bridges

All three production showcase repositories demonstrate core PolyXML capabilities across **all 7 languages** (Rust, Python, Go, C++20, Java 22+, TypeScript 5+, C# 12) with dual XML ↔ JSON data-binding, CLI streaming transcoding, and WebAssembly (`@polyxml/wasm`) integration. In addition, each repository is purpose-built to highlight distinct advanced compiler flags, backends, and schema features:

| Showcase Repository | Domain & Schemas | Distinct PolyXML Features Highlighted |
| :--- | :--- | :--- |
| **[🛸 Defense & Aerospace](https://github.com/polyxml/polyxml-defense-examples)**<br>`polyxml-defense-examples` | **USAF UCI v2.5** ([8.3 MB schema, 5,558 types](https://github.com/polyxml/polyxml-defense-examples#readme))<br>↔ **Anduril Lattice SDK** (Protobuf/JSON) | • **`backend = "aot"`**: Ahead-of-Time compiled PyO3 native extension in the [telemetry bridge example](https://github.com/polyxml/polyxml-defense-examples/blob/main/examples/python/bridge_aot.py)<br>• **`features = ["rkyv"]`**: Opt-in zero-copy binary serialization in Rust for telemetry & tactical radio links<br>• **`xsd:extension` Inlining**: Base headers (`SecurityInformation`, `MessageHeader`) inlined into derived commands<br>• **Massive Schema Validation**: `polyxml validate` on the [Open-Arsenal schema](https://github.com/polyxml/polyxml-defense-examples#readme)<br>• **Standard Library Java 22+**: Zero-dependency immutable records with `java.time.Instant`<br>• **Edge C2 Streaming**: Browser/Node WebAssembly streaming drone swarm telemetry via `parseStream` |
| **[💳 Global Finance & Banking](https://github.com/polyxml/polyxml-finance-examples)**<br>`polyxml-finance-examples` | **ISO 20022 `pacs.008`** (Interbank XML)<br>↔ **FinTech Intents** (FedNow/Stripe JSON) | • **`backend = "jackson"`**: Jackson 2 XML/JSON annotations for Spring Boot 3 / Jakarta EE banking<br>• **`backend = "source-gen"`**: C# 12 / .NET 8 `System.Text.Json` source generation for Native AOT<br>• **Strict Facets & Attributes**: XML attributes on simple content (`Ccy="USD"`) & `xs:pattern` regexes (UETR, IBAN)<br>• **Batch Payment Streaming**: WebAssembly `parseStream` consuming high-volume `<Document>` payment batches |
| **[🚍 Smart Cities & Transit](https://github.com/polyxml/polyxml-transit-examples)**<br>`polyxml-transit-examples` | **CEN SIRI v2.0 & NeTEx** (European Norm)<br>↔ **Google GTFS-RT** (Protobuf/JSON) | • **`backend = "sonic"`**: ByteDance's JIT/AVX-accelerated JSON engine for high-throughput Go microservices<br>• **Deeply Nested Collections**: Hierarchical arrays (`VehicleActivity[]`, `MonitoredCall[]`)<br>• **C++20 Concepts & Value Equality**: `XmlModel` concept verification and `operator==` structural comparisons<br>• **Client-Side Map Streaming**: WebAssembly streaming transit vehicle deliveries into passenger maps |

---

## Language Ecosystem & Packages

| Ecosystem / Language | Registry | Installation | Interop Tech |
| :--- | :--- | :--- | :--- |
| **Rust (Core)** | [crates.io](https://crates.io/crates/polyxml) | `cargo add polyxml` | Native Zero-Copy |
| **Rust (CLI)** | [crates.io](https://crates.io/crates/polyxml-cli) | `cargo install polyxml-cli` | Native CLI Compiler |
| **Rust (C-ABI)** | [crates.io](https://crates.io/crates/polyxml-c) | `cargo add polyxml-c` | C-ABI Shared Lib |
| **Python** | [PyPI](https://pypi.org/project/polyxml/) | `pip install polyxml` | PyO3 (`abi3-py312`) |
| **TypeScript / Node** | [npm](https://www.npmjs.com/package/@polyxml/node) | `npm install @polyxml/node` | `napi-rs` Native Addon |
| **WebAssembly** | [npm](https://www.npmjs.com/package/@polyxml/wasm) | `npm install @polyxml/wasm` | Browser, Node.js & Bun |
| **Java** | [Maven Central](https://central.sonatype.com/artifact/io.github.polyxml/polyxml) | `<groupId>io.github.polyxml</groupId>` · `<artifactId>polyxml</artifactId>` | Java 22+ Panama FFI |
| **Go** | [Go Reference](https://pkg.go.dev/github.com/polyxml/PolyXML/bindings/go) | `go get github.com/polyxml/PolyXML/bindings/go` | Cgo (`polyxml.h`) |
| **Modern C++20 / C** | [Conan](conan/) / [vcpkg](packaging/vcpkg/) (`polyxml`) | `conan install` / `vcpkg install polyxml` | Header-Only C++20 & Native Lib |
| **C# / .NET 8+** | NuGet / Native | `dotnet add package PolyXML` | C# 12 Records & `System.Xml` |
| **macOS & Linux** | [Homebrew Tap](https://github.com/polyxml/homebrew-polyxml) | `brew install polyxml/polyxml/polyxml` | Native Headers & Dynamic Lib |
| **Universal Installer** | [GitHub Releases](https://github.com/polyxml/PolyXML/releases) | `curl -fsSL .../install.sh \| bash` | Pre-compiled Standalone Binary |
| **Debian & Fedora** | [GitHub Releases](https://github.com/polyxml/PolyXML/releases) | `dpkg -i *.deb` / `dnf install *.rpm` | Native `.deb` & `.rpm` Packages |

---

## Documentation & Learning

- **[Multi-Language Quickstart](https://polyxml.github.io/PolyXML/quickstart/)**: 5-minute setup across all 7 target ecosystems.
- **[XSD-to-Code & CLI Guide](https://polyxml.github.io/PolyXML/guides/compiler/)**: Full reference for `polyxml generate`, `build`, `validate`, and `polyxml.toml`.
- **[Language Guides](https://polyxml.github.io/PolyXML/languages/)**: Rust, Python, C++20, Go, TypeScript/Node.js, Java, and C# examples.
- **[Why PolyXML? Architectural Breakdown](https://polyxml.github.io/PolyXML/why-polyxml/)**: Deep comparison against JAXB, CodeSynthesis, xsdata, xgen, and xsd.exe.
- **[Architecture & Streaming Pipeline](https://polyxml.github.io/PolyXML/architecture/)**: Detailed breakdown of our zero-copy reader, frame stack, and Tarjan cycle-cutting.
- **[Performance Benchmarks](https://polyxml.github.io/PolyXML/benchmarks/)**: Reproducible benchmarks and throughput charts.
- **[Real-World Industry Showcases](#-real-world-industry-showcases--polyglot-bridges)**: Production repositories for Defense (USAF UCI), Finance (ISO 20022), and Transit (CEN SIRI/NeTEx).

---

## License

Licensed under the [MIT License](LICENSE).
