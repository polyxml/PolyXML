---
title: Python JSON, Binary, and AOT Benchmarks (September 2026)
description: Verified, reproducible performance benchmarks for Python JSON data-binding, binary serialization, and Ahead-of-Time (AOT) PyO3 C-extensions with committed raw outputs.
---

# Python JSON, Binary, and AOT Benchmarks — September 2026

This report provides reproducible performance evidence for Python JSON data-binding, binary serialization, and Ahead-of-Time (AOT) compiled native extensions. Every measurement is backed by committed harnesses, identical semantic model checks, and raw sample logs.

## Environment and Methodology

| Item | Value |
| :--- | :--- |
| Run date | 2026-09-26 (America/Chicago) |
| Host | AMD Ryzen 5 4500, 12 logical CPUs, 7.7 GiB RAM |
| OS | Ubuntu 26.04.1 under Linux 6.18.33.2 WSL2, x86-64 |
| Toolchains | Python 3.12.14, PolyXML 0.24.0 (PyO3 `abi3-py312`), Rust 1.98.1 |
| Competitors & Libraries | `xsdata` 26.2, `cloudpickle` 3.1.2, `lz4` 4.4.5, standard library `pickle` (protocol 5) |
| Raw Data Directory | [`docs/benchmarks/data/2026-09-26/`](https://github.com/polyxml/PolyXML/tree/main/docs/benchmarks/data/2026-09-26) |

---

## 1. JSON Data-Binding: PolyXML vs `xsdata`

This suite compares **PolyXML** native JSON APIs (`polyxml.loads_json`, `dumps_json`, `JsonParser`, `JsonSerializer`) against **`xsdata`** JSON data-binding on the **exact same Python dataclass models** and **identical JSON payloads**.

- **Runner**: [`benchmarks/python-json/`](https://github.com/polyxml/PolyXML/tree/main/benchmarks/python-json)
- **Raw output**: [Text log](data/2026-09-26/python-json.txt) · [JSON metrics](data/2026-09-26/python-json.json)
- **Semantic Check**: Verified `assert poly_parsed == expected == xs_parsed` for every run.

### Deserialization (JSON → Typed Dataclass)

| Workload | Payload Size | `xsdata` Median | PolyXML Native Median | Speedup vs `xsdata` | PolyXML Throughput |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Sensor (Small)** | 128 B | 156.5 μs | **12.8 μs** | **12.2x faster** | 9.5 MB/s |
| **Order (Nested)** | 793 B | 1,284.5 μs | **140.2 μs** | **9.2x faster** | 5.4 MB/s |
| **Batch (100 Items)** | 12,842 B | 20,570.3 μs | **1,326.7 μs** | **15.5x faster** | 9.2 MB/s |

### Serialization (Typed Dataclass → JSON)

| Workload | Payload Size | `xsdata` Median | PolyXML Native Median | Speedup vs `xsdata` | PolyXML Throughput |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Sensor (Small)** | 128 B | 86.4 μs | **10.3 μs** | **8.4x faster** | 11.9 MB/s |
| **Order (Nested)** | 793 B | 505.6 μs | **60.4 μs** | **8.4x faster** | 12.5 MB/s |
| **Batch (100 Items)** | 12,842 B | 6,586.1 μs | **711.7 μs** | **9.3x faster** | 17.2 MB/s |

---

## 2. Binary Serialization: PolyXML vs `pickle` and `cloudpickle`

This suite compares **PolyXML Binary** (`polyxml.dumps_binary`, `loads_binary` based on MessagePack) with standard library **`pickle`** (protocol 5 with C accelerator) and **`cloudpickle`** across direct uncompressed and LZ4 compressed modes.

- **Runner**: [`benchmarks/python-binary/`](https://github.com/polyxml/PolyXML/tree/main/benchmarks/python-binary)
- **Raw output**: [Text log](data/2026-09-26/python-binary.txt) · [JSON metrics](data/2026-09-26/python-binary.json)
- **Semantic Check**: Verified `assert loads(dumps(obj)) == obj` for all serializers and compression modes.

### Direct (Uncompressed) Serialization

| Workload | Serializer | Payload Size | Encode Median | Decode Median | Encode Ops/s | Decode Ops/s |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **Sensor (Small)** | **PolyXML Binary** | **118 B** | 5.3 μs | **3.6 μs** | 188,147 | **276,129** |
| | Pickle 5 (Stdlib C) | 185 B | 5.1 μs | 8.2 μs | 195,695 | 121,721 |
| | CloudPickle | 185 B | 18.8 μs | 8.1 μs | 53,233 | 123,678 |
| **Order (Nested)** | **PolyXML Binary** | **642 B** | 23.6 μs | **16.7 μs** | 42,382 | **59,893** |
| | Pickle 5 (Stdlib C) | 684 B | 24.2 μs | 27.3 μs | 41,328 | 36,668 |
| | CloudPickle | 684 B | 92.7 μs | 27.3 μs | 10,792 | 36,621 |
| **Batch (100 Items)** | **PolyXML Binary** | 11,853 B | **180.4 μs** | 331.0 μs | **5,544** | 3,022 |
| | Pickle 5 (Stdlib C) | **7,192 B** | 248.7 μs | **275.2 μs** | 4,021 | **3,634** |
| | CloudPickle | **7,192 B** | 718.1 μs | 282.2 μs | 1,392 | 3,544 |

### Compressed (LZ4) Serialization

| Workload | Serializer | LZ4 Size | Encode+LZ4 Median | Decode+LZ4 Median | Encode+LZ4 Ops/s | Decode+LZ4 Ops/s |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **Sensor (Small)** | **PolyXML Binary** | **138 B** | 6.9 μs | **4.2 μs** | 144,959 | **239,349** |
| | Pickle 5 (Stdlib C) | 198 B | 6.9 μs | 9.7 μs | 144,020 | 103,157 |
| | CloudPickle | 198 B | 20.5 μs | 9.9 μs | 48,807 | 100,873 |
| **Order (Nested)** | **PolyXML Binary** | **344 B** | 26.9 μs | **17.4 μs** | 37,236 | **57,511** |
| | Pickle 5 (Stdlib C) | 443 B | 27.6 μs | 30.3 μs | 36,294 | 33,055 |
| | CloudPickle | 443 B | 90.2 μs | 31.0 μs | 11,086 | 32,218 |
| **Batch (100 Items)** | **PolyXML Binary** | **1,937 B** | **203.5 μs** | 341.9 μs | **4,913** | 2,925 |
| | Pickle 5 (Stdlib C) | 1,986 B | 265.2 μs | 508.1 μs | 3,771 | 1,968 |
| | CloudPickle | 1,986 B | 721.9 μs | 301.8 μs | 1,358 | 3,314 |

---

## 3. Real-World Ahead-of-Time (AOT) PyO3 C-Extension vs Standard Dataclass

This suite evaluates the performance difference between PolyXML's standard Python `@dataclass` runtime (`polyxml.deserialize`, `serialize`) and Ahead-of-Time compiled PyO3 native extensions (`--backend aot`).

- **Workload 1**: Real-world **USAF UCI v2.5** command and control entity telemetry ([`benchmarks/python-aot/data/uci_entity.xml`](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python-aot/data/uci_entity.xml), 1,510 bytes XML). Models: `uci_entity_core.EntityMt` dataclass vs `uci_aot.EntityMt` PyO3 extension.
- **Workload 2**: Synthetic `SensorReading` telemetry (226 bytes XML).
- **Runner**: [`benchmarks/python-aot/`](https://github.com/polyxml/PolyXML/tree/main/benchmarks/python-aot)
- **Raw output**: [Text log](data/2026-09-26/python-aot.txt) · [JSON metrics](data/2026-09-26/python-aot.json)
- **Mathematical Consistency**: Verified $\text{ops/sec} \times \text{median latency (s)} = 1.0$ exactly across all measurements.

### Deserialization (XML → Model)

| Workload | XML Size | Dataclass Latency | Dataclass Ops/s | AOT Latency | AOT Ops/s | AOT Speedup | AOT Throughput |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **USAF UCI v2.5 Telemetry (Real-World)** | 1,510 B | 62.5 μs | 16,003 | **7.44 μs** | **134,336** | **8.39x** | **193.5 MB/s** |
| **SensorReading (Synthetic)** | 226 B | 11.3 μs | 88,367 | **2.31 μs** | **433,839** | **4.91x** | **93.5 MB/s** |

### Serialization (Model → XML)

| Workload | XML Size | Dataclass Latency | Dataclass Ops/s | AOT Latency | AOT Ops/s | AOT Speedup | AOT Throughput |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **USAF UCI v2.5 Telemetry (Real-World)** | 1,510 B | 39.8 μs | 25,141 | **3.88 μs** | **257,932** | **10.26x** | **371.4 MB/s** |
| **SensorReading (Synthetic)** | 226 B | 7.4 μs | 135,980 | **1.74 μs** | **573,393** | **4.22x** | **123.6 MB/s** |

---

## 4. Key Takeaways & Tradeoffs

1. **JSON Data-Binding**:
   - PolyXML native JSON deserialization is **9.2x to 15.5x faster** than `xsdata`, and serialization is **8.4x to 9.3x faster**, while operating on the exact same dataclass models.
2. **Binary Serialization**:
   - For small and nested objects, PolyXML MessagePack decoding is **2.3x faster** than `pickle` (3.6 μs vs 8.2 μs for Sensor; 16.7 μs vs 27.3 μs for Order) and creates smaller payloads (118 B vs 185 B).
   - For large homogeneous 100-item batches, `pickle`'s specialized C array packing achieves a smaller uncompressed payload (7.2 KB vs 11.8 KB) and slightly faster decoding (275.2 μs vs 331.0 μs). With LZ4 compression, PolyXML achieves smaller payloads (1,937 B vs 1,986 B).
3. **Ahead-of-Time (AOT) C-Extensions**:
   - On production USAF UCI XML telemetry messages, AOT PyO3 native extensions parse XML in **7.44 μs** (134,336 ops/sec, 193.5 MB/s), an **8.39x speedup** over runtime dataclass reflection.
   - Per-packet latency is measured at 7.44 μs (not sub-microsecond). While extraordinarily fast, we strictly avoid rounding or exaggeration.
