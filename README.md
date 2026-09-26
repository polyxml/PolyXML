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

If you have ever used `xjc` (JAXB), `CodeSynthesis XSD`, or `xsdata`, PolyXML is their modern, safe-Rust replacement. It compiles your schema once and generates idiomatic, zero-overhead code across **7 languages simultaneously**—with direct `.from_xml()` and `.to_xml()` methods running **10x–24x faster** than traditional Python and C DOM parsers.

Just as Protocol Buffers (`protoc`) and FlatBuffers (`flatc`) modernized binary serialization, **PolyXML brings modern software engineering to XML**:

1. **🛠️ Universal XSD-to-Code Generator (`polyxml`)**: Ingests W3C XSD 1.0 and 1.1 schemas, resolves cyclic types with Tarjan's SCC algorithm, and compiles production-ready, strongly-typed data contracts across **7 modern ecosystems** simultaneously (**Python**, **Rust**, **C++**, **Java**, **TypeScript**, **Go**, and **C#**).
2. **⚡ Ultra-Fast Streaming Runtime**: Direct-to-struct deserialization and serialization powered by `quick-xml` and `lexical-core`, executing **10x–24x faster than traditional tools** with **zero intermediate DOM allocations**.
3. **🏛️ Official W3C XSTS Conformance Tested**: Validated against the official W3C XML Schema Test Suite with a **>99.8% schema compilation pass rate** and **>96% round-trip validation rate** via [polyxml-w3c-tests](https://github.com/polyxml/polyxml-w3c-tests).
4. **📦 Permissive MIT License**: 100% open source with zero commercial licensing fees, eliminating the GPL dual-licensing traps of legacy C++ tools.
5. **🛡️ Controlled XML Entity Handling**: The native streaming runtime resolves standard and numeric character references in memory and does not fetch external entities while parsing XML. Schema compilation separately reads local XSD includes and imports.

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

# Target-specific backend, style, and enhancements
polyxml generate schemas/pain.001.001.09.xsd --lang java \
  --backend jackson --style pojo --feature builder --feature direct-codec \
  --package com.enterprise.banking --out ./generated/java

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

Headline numbers ([full methodology & reproduction steps →](docs/benchmarks/index.md)):

- **10x–24x faster** than legacy Python bindings: 10.0x faster deserialization & 23.5x faster serialization than `xsdata` on 10,000-item catalogs.
- **3.2 μs per telemetry packet** (13.9x vs pure Python) — neck-and-neck with the C-based `lxml.etree` while still returning fully typed dataclasses.
- **Binary KV-store & IPC pipelines**: 163k dumps ops/s, 53.9% smaller payloads than CloudPickle+LZ4, and 3.2x faster transactional MDBX writes.

---

## 🌐 Real-World Industry Showcases & Polyglot Bridges

All three production showcase repositories demonstrate core PolyXML capabilities across **all 7 languages** (Rust, Python, Go, C++20, Java 22+, TypeScript 5+, C# 12) with dual XML ↔ JSON data-binding, CLI streaming transcoding, and WebAssembly (`@polyxml/wasm`) integration. In addition, each repository is purpose-built to highlight distinct advanced compiler flags, backends, and schema features:

| Showcase Repository | Domain & Schemas | Distinct PolyXML Features Highlighted |
| :--- | :--- | :--- |
| **[🛸 Defense & Aerospace](https://github.com/polyxml/polyxml-defense-examples)**<br>`polyxml-defense-examples` | **USAF UCI v2.5** (8.3 MB XML)<br>↔ **Anduril Lattice SDK** (Protobuf/JSON) | • **`features = ["rkyv"]`**: Opt-in zero-copy binary serialization in Rust for telemetry & tactical radio links<br>• **`xsd:extension` Inlining**: Base headers (`SecurityInformation`, `MessageHeader`) inlined into derived commands<br>• **Massive Schema Validation**: `polyxml validate` on 8.3 MB, 5,558-type Open-Arsenal schemas<br>• **Standard Library Java 22+**: Zero-dependency immutable records with `java.time.Instant`<br>• **Edge C2 Streaming**: Browser/Node WebAssembly streaming drone swarm telemetry via `parseStream` |
| **[💳 Global Finance & Banking](https://github.com/polyxml/polyxml-finance-examples)**<br>`polyxml-finance-examples` | **ISO 20022 `pacs.008`** (Interbank XML)<br>↔ **FinTech Intents** (FedNow/Stripe JSON) | • **`backend = "jackson"`**: Enterprise Jackson XML/JSON annotations for Spring Boot / Jakarta EE banking<br>• **`backend = "source-gen"`**: C# 12 / .NET 8 `System.Text.Json` source generation for Native AOT<br>• **Strict Facets & Attributes**: XML attributes on simple content (`Ccy="USD"`) & `xs:pattern` regexes (UETR, IBAN)<br>• **Batch Payment Streaming**: WebAssembly `parseStream` consuming high-volume `<Document>` payment batches |
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
