---
title: Performance & Benchmarks
description: Reproducible performance benchmarks comparing PolyXML against native and pure-language XML engines.
---

# Performance & Benchmarks

The [WebAssembly vs JavaScript benchmark](wasm-vs-js.md) compares the Wasm
runtime with JavaScript parsers in Node, Bun, and Chromium.
The [Python AOT vs Dataclass benchmark](python-aot-vs-dataclass.md) details the throughput and memory advantages of Ahead-of-Time compiled PyO3 native extensions (`--backend aot`).

PolyXML is engineered to process gigabytes of XML per second by leveraging Rust's zero-cost abstractions, `quick-xml` streaming events, and `lexical-core` numeric conversions.

The repository includes reusable benchmark suites for [Rust, Python, Java, Go, C++, C#, and TypeScript/Wasm](https://github.com/polyxml/PolyXML/blob/main/benchmarks/README.md). The Go and C# suites compare generated models with handwritten equivalents using the same standard XML serializer. The C++ suite measures a narrow XML adapter into generated models because the C++ generator does not yet emit an XML codec; its numbers must not be presented as PolyXML C++ codec throughput. The [suite catalog](https://github.com/polyxml/PolyXML/blob/main/benchmarks/README.md) gives each entry point and methodology.

> 🚀 **Looking for architectural comparisons with legacy compilers?**
> Check out **[Why PolyXML? (The Architecture of Modern XML)](../why-polyxml.md)** for in-depth comparisons against JAXB, CodeSynthesis, xsdata, xgen, and xsd.exe.

---

## 1. Python Deserialization & Serialization Throughput

Benchmarks conducted using Python 3.12 (`abi3-py312`) across 10,000-element streaming payloads (~724 KB XML), micro sensor payloads (~100B), and enterprise orders:

> ⚡ **AOT Native Compilation**: In addition to standard dataclasses, PolyXML can compile schemas directly into native C-extensions via `--backend aot`. See the full [Python AOT Native Extension vs Dataclass Benchmark](python-aot-vs-dataclass.md) (3.4x faster throughput, 60.3% less memory).

### Batch Catalog Workload (10,000 items, ~724 KB XML)

| Engine | Paradigm / Category | Implementation | Deserialization Latency | Deserialization Throughput | Serialization Latency | Serialization Throughput | Peak RAM |
| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **PolyXML** | **Typed Dataclass** | **Rust + PyO3** | **24.0 ms** | **29.8 MB/s** | **12.2 ms** | **58.1 MB/s** | **2.0 MB** |
| `lxml.objectify` | Dynamic C Object | C / Cython (`libxml2`) | 10.2 ms | 70.8 MB/s | 4.1 ms | 173.5 MB/s | 0.2 MB |
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

Statistical benchmarks measured with Criterion.rs:

| Workload / Target | Operation | Latency | Throughput | Zero Allocations |
| :--- | :--- | :---: | :---: | :---: |
| **Sensor Micro (130B)** | Deserialization | **1.19 μs** | **75.0 MiB/s** | Direct scalar parse |
| **Sensor Micro (130B)** | Serialization | **456 ns** | **196.5 MiB/s** | Zero intermediate DOM |
| **Catalog (1,000 items, ~70 KB)** | Deserialization | **1.00 ms** | **60.6 MiB/s** | Zero intermediate DOM |
| **Catalog (1,000 items, ~70 KB)** | Serialization | **305 μs** | **198.3 MiB/s** | Streaming buffer |
| **Catalog (10,000 items, ~724 KB)** | Deserialization | **10.54 ms** | **60.5 MiB/s** | Streaming buffer |
| **Catalog (10,000 items, ~724 KB)** | Serialization | **3.20 ms** | **197.8 MiB/s** | Streaming buffer |

---

## 3. Key-Value Storage & Binary IPC Throughput (`polyxml.dumps_binary` / `loads_binary`)

When storing XML dataclasses and Pydantic models in embedded transactional key-value databases (`libmdbx`, `LMDB`, `RocksDB`) or communicating over Unix Domain Sockets/multiprocessing queues, Python's traditional `cloudpickle` and standard `pickle` encounter severe GIL and GC bottlenecks.

`polyxml.dumps_binary()` and `polyxml.loads_binary()` provide high-speed MessagePack encoding with universal leaf type hooks (`XmlDate`, `XmlDateTime`, `XmlDuration`, `XmlTime`, `Decimal`, `QName`, `Enum`, `Path`, and Pydantic models):

### 10,000 Complex XML Entities in Real MDBX Pipeline

| Serializer Pipeline | Dumps Ops/s | Dumps Latency | Loads Ops/s | Avg Payload Size | MDBX Write Ops/s |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **`CloudPickle + LZ4` (Legacy)** | 20,609 ops/s | 48.5 μs | 49,322 ops/s | 547 B | 18,287 ops/s |
| `Pickle 5 + LZ4` (Stdlib C) | 71,954 ops/s | 13.9 μs | 50,092 ops/s | 539 B | — |
| **`PolyXML Binary + LZ4`** | **163,192 ops/s** | **6.1 μs** | **64,781 ops/s** | **252 B** | **58,781 ops/s** |
| **`PolyXML Binary (Direct, No LZ4)`** | **213,003 ops/s** | **4.7 μs** | **84,673 ops/s** | **327 B** | **63,236 ops/s** |

### Key Takeaways:
- **7.9x Faster Serialization**: Slashes per-object serialization from 48.5 μs down to 6.1 μs.
- **53.9% Storage Space Reduction**: Cuts stored byte size from 547 bytes to 252 bytes per entity.
- **3.2x MDBX Transaction Speedup**: Real database writes into `libmdbx` jump from 18,287 ops/s to 58,781 ops/s.
- **100% Fidelity Guarantee**: Round-trips preserve exact dataclass types, field metadata, and XML primitive representations with `assert loads(dumps(x)) == x`.

---

## 4. How to Reproduce Benchmarks

The benchmark suite is reusable and version-controlled.

> 🛡️ **Memory safety**: the suite entry points below run under
> [`scripts/memcap.sh`](https://github.com/polyxml/PolyXML/blob/main/scripts/memcap.sh),
> which caps each run at 60% of available RAM in an isolated cgroup (kernel
> OOM-kills only the runaway build; the host stays responsive — `ulimit`
> fallback where systemd is unavailable). Wrap any ad-hoc
> `cargo bench --release` the same way; tune with `POLYXML_MEMCAP_PCT`, opt
> out with `POLYXML_MEMCAP_DISABLE=1`.

### Run All Benchmarks (Rust + Python)

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
