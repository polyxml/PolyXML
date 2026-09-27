---
title: Benchmarks
description: Benchmark coverage, published results, methods, and reproduction commands for every PolyXML target language.
---

# Benchmarks

PolyXML has benchmark suites for all seven target languages, plus WebAssembly
and CLI startup. The suites answer different questions, so compare numbers
**within the same workload and runtime**. A quick smoke run confirms that a
suite works; it is not a performance result.

## Choose a benchmark

| Target | What it measures | Suite and instructions | Published results |
| :--- | :--- | :--- | :--- |
| Rust | Core XML read/write; tag dispatch and generated decoder check | [Criterion suites](https://github.com/polyxml/PolyXML/tree/main/crates/polyxml-core/benches), [generated decoder runner](https://github.com/polyxml/PolyXML/tree/main/benchmarks/rust-phf-e2e) | [Core results](#2-pure-rust-core-throughput-cratespolyxml-core), [dispatch study](rust-phf-dispatch.md) |
| Python | Typed binding vs Python XML libraries; AOT vs dataclasses | [Python suite](https://github.com/polyxml/PolyXML/tree/main/benchmarks/python) | [Comparative results](#1-python-deserialization-serialization-throughput), [AOT study](python-aot-vs-dataclass.md) |
| Java | Generated POJOs, JAXB, Jackson, and Panama | [JMH suite](https://github.com/polyxml/PolyXML/tree/main/benchmarks/java) | [September 2026 measurements](language-results-2026-09.md#java-jmh-binding-comparison) |
| Go | Generated structs vs equivalent handwritten structs, both using `encoding/xml` | [Go suite](https://github.com/polyxml/PolyXML/tree/main/benchmarks/go) | [September 2026 measurements](language-results-2026-09.md#go-generated-model-vs-handwritten-model) |
| C++ | Native binding plus generated-model XML adapter | [C++ suite](https://github.com/polyxml/PolyXML/tree/main/benchmarks/cpp) | [September 2026 measurements](language-results-2026-09.md#c-native-binding-and-model-adapter) |
| C# | Generated vs handwritten classes, both using `XmlSerializer` | [C# suite](https://github.com/polyxml/PolyXML/tree/main/benchmarks/csharp) | [September 2026 measurements](language-results-2026-09.md#c-generated-model-vs-handwritten-model) |
| TypeScript & Wasm | Wasm vs JavaScript parsers in Node, Bun, and Chromium | [TypeScript/Wasm suite](https://github.com/polyxml/PolyXML/tree/main/benchmarks/typescript-wasm) | [Wasm vs JavaScript study](wasm-vs-js.md), [shared sensor run](language-results-2026-09.md#shared-sensor-readers) |

The [shared sensor workload](https://github.com/polyxml/PolyXML/tree/main/benchmarks/workloads/sensor-batch)
feeds the **same XML bytes** to readers in all seven targets. Its
[September 2026 results](language-results-2026-09.md#shared-sensor-readers)
identify each binding layer and explain why a shared input alone does not
justify a cross-language speed ranking.

The [CLI startup suite](https://github.com/polyxml/PolyXML/tree/main/benchmarks/cli)
measures command startup and argument validation separately from language
bindings. The [repository suite catalog](https://github.com/polyxml/PolyXML/blob/main/benchmarks/README.md)
lists every entry point.

!!! note "Reading the C++ numbers"
    The C++ generator emits model types but does not yet emit an XML codec. Its
    generated-model lane measures a fixture-specific adapter, while the native
    lane measures the actual C++ binding over the Rust C ABI. Keep their
    timings separate; the adapter is **not** PolyXML C++ codec throughput.

## Published studies

- [September 2026 language runs](language-results-2026-09.md): repeated Java,
  Go, C#, and C++ measurements with raw output and workload limits.
- [WebAssembly vs JavaScript](wasm-vs-js.md): Node, Bun, and Chromium workloads.
- [Python AOT vs dataclasses](python-aot-vs-dataclass.md): throughput and memory.
- [Rust tag dispatch](rust-phf-dispatch.md): dispatch strategies and their tradeoffs.

The older Python and Rust result tables below preserve their original workloads.
Use each linked method and environment when interpreting those figures.

> 🚀 **Looking for architectural comparisons with legacy compilers?**
> Check out **[Why PolyXML? (The Architecture of Modern XML)](../why-polyxml.md)** for in-depth comparisons against JAXB, CodeSynthesis, xsdata, xgen, and xsd.exe.

---

## 1. Python Deserialization & Serialization Throughput

Benchmarks conducted using Python 3.12 (`abi3-py312`) across 10,000-element streaming payloads (~724 KB XML), micro sensor payloads (~100B), and enterprise orders. The [committed results](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md) and [runner](https://github.com/polyxml/PolyXML/tree/main/benchmarks/python) support the tables below.

> ⚡ **AOT Native Compilation**: In addition to standard dataclasses, PolyXML can compile schemas directly into native C-extensions via `--backend aot`. See the full [Python AOT Native Extension vs Dataclass Benchmark](python-aot-vs-dataclass.md) (3.4x faster throughput, 60.3% less memory).

### Batch Catalog Workload (10,000 items, ~724 KB XML)

| Engine | Paradigm / Category | Implementation | Deserialization Latency | Deserialization Throughput | Serialization Latency | Serialization Throughput | Peak RAM |
| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **PolyXML** | **Typed Dataclass** | **Rust + PyO3** | **24.0 ms** | **29.8 MB/s** | **12.2 ms** | **58.1 MB/s** | **2.0 MB** |
| `lxml.objectify` | Dynamic C Object | C / Cython (`libxml2`) | 11.8 ms | 61.0 MB/s | 4.1 ms | 173.5 MB/s | 0.2 MB |
| `lxml.etree` | Untyped DOM | C / Cython (`libxml2`) | 11.8 ms | 61.1 MB/s | — | — | <0.1 MB |
| `ElementTree` | Untyped DOM | Python Stdlib C/Python | 13.6 ms | 53.0 MB/s | — | — | 7.1 MB |
| `defusedxml` | Secure DOM | Python Defused | 29.5 ms | 24.2 MB/s | — | — | 7.1 MB |
| `xmltodict` | Untyped Dict | C (`pyexpat`) | 57.5 ms | 12.5 MB/s | 80.0 ms | 8.9 MB/s | 4.8 MB |
| `xsdata` | Typed Dataclass | Pure Python | 241.9 ms | 3.0 MB/s | 298.8 ms | 2.5 MB/s | 3.3 MB |

> **Key Takeaway**: 
> - **Vs Typed Dataclasses (`xsdata`)**: PolyXML is **10.0x faster** at deserialization and **23.5x faster** at serialization, while saving over 1.3 MB of memory.
> - **Vs Dict Parsers (`xmltodict`)**: PolyXML is **4.2x faster** on deserialization and **6.5x faster** on serialization, while instantiating strongly-typed dataclasses instead of unstructured string dictionaries.
> - **Vs C Proxies (`lxml.objectify`)**: PolyXML executes within ~2x of raw C dynamic proxy trees, while returning genuine, fully typed Python dataclasses with IDE autocomplete and type safety.

---

### Real-Time Micro Telemetry Workload (Sensor ~100B, UCI Telemetry)

| Engine | Category | Deserialization Latency | Serialization Latency | Speedup vs Pure Python |
| :--- | :--- | :---: | :---: | :---: |
| **PolyXML** | **Typed Dataclass** | **3.2 μs** | **1.8 μs** | **13.9x** |
| **PolyXML (Pydantic)** | **Typed Pydantic v2** | **3.8 μs** | **1.9 μs** | **11.8x** |
| `lxml.etree` | Untyped DOM | 3.3 μs | — | 13.4x |
| `lxml.objectify` | C Dynamic Object | 3.3 μs | 1.4 μs | 13.1x |
| `ElementTree` | Untyped DOM | 5.3 μs | — | 8.4x |
| `defusedxml` | Secure DOM | 9.1 μs | — | 4.9x |
| `declxml` | Declarative Dict | 10.2 μs | 23.9 μs | 4.2x |
| `xmltodict` | Untyped Dict | 10.6 μs | 15.4 μs | 4.2x |
| `pydantic-xml` | Typed Pydantic v2 | 17.1 μs | 15.5 μs | 2.6x |
| `untangle` | Dynamic Object | 19.5 μs | — | 2.3x |
| `xsdata` | Typed Dataclass | 44.5 μs | 45.5 μs | 1.0x (Ref) |

Critical telemetry commands and sensor packets deserialize in **3.2 microseconds**, neck-and-neck with C-based DOM parsers (`lxml` at 3.3 μs).

---

## 2. Pure Rust Core Throughput (`crates/polyxml-core`)

Statistical benchmarks measured with Criterion.rs; [benchmark source](https://github.com/polyxml/PolyXML/blob/main/crates/polyxml-core/benches/core_benchmarks.rs) and [reproduction command](#run-pure-rust-criterion-benchmarks). These historical figures have no committed raw Criterion report, so rerun before using them for a new performance claim:

| Workload / Target | Operation | Latency | Throughput | Zero Allocations |
| :--- | :--- | :---: | :---: | :---: |
| **Sensor Micro (130B)** | Deserialization | **1.19 μs** | **75.0 MiB/s** | Direct scalar parse |
| **Sensor Micro (130B)** | Serialization | **456 ns** | **196.5 MiB/s** | Zero intermediate DOM |
| **Catalog (1,000 items, ~70 KB)** | Deserialization | **1.00 ms** | **60.6 MiB/s** | Zero intermediate DOM |
| **Catalog (1,000 items, ~70 KB)** | Serialization | **305 μs** | **198.3 MiB/s** | Streaming buffer |
| **Catalog (10,000 items, ~724 KB)** | Deserialization | **10.54 ms** | **60.5 MiB/s** | Streaming buffer |
| **Catalog (10,000 items, ~724 KB)** | Serialization | **3.20 ms** | **197.8 MiB/s** | Streaming buffer |

---

## 3. How to Reproduce Benchmarks

The benchmark suite is reusable and version-controlled.

> 🛡️ **Memory safety**: the suite entry points below run under
> [`scripts/memcap.sh`](https://github.com/polyxml/PolyXML/blob/main/scripts/memcap.sh),
> which caps each run at 60% of available RAM in an isolated cgroup (kernel
> OOM-kills only the runaway build; the host stays responsive — `ulimit`
> fallback where systemd is unavailable). Wrap any ad-hoc
> `cargo bench --release` the same way; tune with `POLYXML_MEMCAP_PCT`, opt
> out with `POLYXML_MEMCAP_DISABLE=1`.

### Run the unified Rust and Python suites

```bash
./benchmarks/run_all.sh
```

### Run Pure Rust Criterion Benchmarks

```bash
cargo bench --bench core_benchmarks
cargo bench --bench tag_dispatch   # dispatch strategy suite: docs/benchmarks/rust-phf-dispatch.md
```

### Run Python Comparative Benchmarks CLI

```bash
python -m benchmarks.python --workload all --catalog-sizes 1000 10000 --iterations 25 --output-md benchmarks/python/results.md --output-json benchmarks/python/results.json
```

### Run the Java Four-Runtime JMH Suite

```bash
mvn -f benchmarks/java/pom.xml clean package
```

See the [Java benchmark README](https://github.com/polyxml/PolyXML/tree/main/benchmarks/java) for the Panama profile, workload definitions, and interpretation caveats.

### Run the Go, C++, and C# suites

Build the local CLI once, then run each suite from the repository root:

```bash
cargo build -p polyxml-cli
./benchmarks/go/run.sh
./benchmarks/cpp/run.sh
./benchmarks/csharp/run.sh
```

Each runner generates its model into an ignored `target/` directory, checks a
round trip, and measures small and 1,000-sensor batches. See the linked suite
READMEs above for toolchain requirements and interpretation limits. Use their
short-iteration settings only to check that they execute.

### Run the TypeScript/Wasm suite

The [TypeScript/Wasm README](https://github.com/polyxml/PolyXML/tree/main/benchmarks/typescript-wasm)
has separate commands for Node, Bun, and headless Chromium, plus instructions
for the larger fixture. These lanes are intentionally separate from the unified
Rust/Python runner.

For publishable measurements, run on an otherwise idle host, repeat the full
suite, and keep raw output with the Git revision, CPU/OS, and toolchain versions.
Compare read with read and write with write on the same payload; generated
models, DOM parsers, and transcoding pipelines can return different values.
