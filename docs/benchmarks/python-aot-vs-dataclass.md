---
title: Python AOT Native Extension vs Dataclass Benchmark
description: Empirical throughput and memory profiling comparison between PolyXML's Python standard dataclass runtime and Ahead-of-Time (AOT) PyO3 native extensions.
---

# Python AOT Native Extension vs Dataclass Benchmark

This document presents empirical performance benchmarks comparing PolyXML's standard Python dataclass runtime (`--backend dataclass`) with Ahead-of-Time compiled PyO3 native extensions (`--backend aot`).

---

## 1. Overview & Architectural Contrast

When generating Python bindings from XML schemas (XSD), PolyXML offers two primary execution models:

| Dimension | Standard Dataclass (`--backend dataclass`) | AOT Native Extension (`--backend aot`) |
| :--- | :--- | :--- |
| **Output** | Pure Python source code (`.py`) | Standalone PyO3 Rust crate (`Cargo.toml`, `src/lib.rs`, `.pyi`) |
| **Compilation** | None (interpreted by CPython) | Ahead-of-time compiled C-extension (`.so` / `.pyd`) via `maturin` |
| **ABI Compatibility** | Universal pure Python | Python 3.12+ Limited API (`abi3-py312`) |
| **Object Representation** | Stdlib `@dataclass(slots=True)` | Native Rust `struct` exposed via `#[pyclass]` |
| **Deserialization Engine** | Dynamic PyO3 runtime (`polyxml.deserialize`) | Compiled Rust parser (`quick-xml` + static field dispatch) |
| **Primitive Storage** | Boxed Python objects (`PyFloatObject`, `PyLongObject`) | Unboxed native primitives (`f64`, `i64`, `bool`) in contiguous memory |

### Why AOT Native Extensions Outperform Pure Dataclasses

1. **Zero Interpreter Crossing During Parsing**:
   With standard dataclasses, even if parsing uses a fast C/Rust parser, the parser must construct Python objects and set attributes across the CPython C-API boundary for every field. With AOT, the XML event loop (`quick-xml`) populates native Rust struct fields directly in unboxed CPU registers and contiguous memory.
2. **Reduced Heap Allocation & GC Overhead**:
   In CPython, every `float` or `int` is a heap-allocated `PyObject` with reference counting and GC tracking overhead. An AOT `#[pyclass]` keeps primitives stored inline inside the Rust struct layout. A batch of 10,000 objects in memory requires only the outer PyObject wrapper, reducing memory allocations and garbage collection cycles.
3. **Optimized Monomorphic Serialization**:
   Serialization from pure Python requires walking Python attribute dicts or slot descriptors. AOT serialization calls monomorphic Rust serializers with static buffer sizing and zero dynamic type lookups.

---

## 2. Empirical Benchmark Results

Measurements conducted on Linux x86_64 using Python 3.12 (`abi3-py312`) and release-compiled binaries (`maturin develop --release`).

### Workload: High-Frequency Sensor Telemetry (~180 bytes XML)

The benchmark simulates real-time IoT / telemetry ingestion (`SensorReading` containing `sensorId`, `temperature`, `humidity`, `pressure`, and `status`).

| Paradigm / Target | Deserialization Throughput | Operations / Second | Per-Message Latency | Peak Memory (10k Objects) |
| :--- | :---: | :---: | :---: | :---: |
| **Standard Dataclass (slots)** | 37.09 MB/s | 172,066 ops/s | 5.81 μs | 2,572.4 KB |
| **AOT Native (PyO3 cdylib)** | **126.25 MB/s** | **585,764 ops/s** | **1.71 μs** | **1,020.7 KB** |
| **Performance Delta** | **+240.4% (3.40x)** | **3.40x faster** | **70.6% lower latency** | **60.3% memory reduction** |

### Key Takeaways:
- **3.40x Higher Ingestion Throughput**: Deserialization jumps from 172,000 to over 585,000 records per second on a single thread.
- **Sub-2-Microsecond Latency**: Per-message parse time drops from 5.81 μs down to 1.71 μs.
- **60.3% Peak Memory Reduction**: Retaining 10,000 active records in memory consumes only ~1.0 MB under AOT vs ~2.57 MB with slotted dataclasses.

---

## 3. How to Reproduce

PolyXML includes an automated, self-contained benchmark script that compiles an AOT extension on-the-fly and measures both throughput and memory:

```bash
# Ensure Python virtual environment with polyxml and maturin is active
source .venv/bin/activate

# Run the comparative benchmark
python crates/polyxml-python/benches/aot_vs_dataclass.py --iterations 20000
```

### Manual Compilation & Usage

To generate and compile an AOT native extension for your own schema:

```bash
# 1. Generate the standalone PyO3 crate
polyxml generate schema.xsd -l python -b aot -p my_extension -o ./my_extension_pkg

# 2. Build the extension into your virtual environment
cd ./my_extension_pkg
maturin develop --release

# 3. Use in Python with full type safety and maximum speed
python -c "
import my_extension

reading = my_extension.SensorReadingType.from_xml('''
<SensorReading xmlns=\"urn:sensors\">
    <sensorId>SN-1004</sensorId>
    <temperature>23.4</temperature>
    <humidity>45.2</humidity>
    <pressure>1012.8</pressure>
    <status>ACTIVE</status>
</SensorReading>
''')

print(f'Sensor: {reading.sensorId}, Temp: {reading.temperature}°C')
xml_out = reading.to_xml()
json_out = reading.to_json()
"
```

---

## 4. Architectural Selection Guide: When to Choose AOT

| Scenario | Recommend Dataclass (`-b dataclass`) | Recommend AOT (`-b aot`) |
| :--- | :---: | :---: |
| Pure-Python portability (no Rust compiler in CI/CD) | ✅ | |
| Rapid prototyping / frequent schema iteration | ✅ | |
| Integration with standard Python tooling (`pydantic`, `dataclasses.asdict`) | ✅ | |
| High-throughput streaming pipelines (Kafka, MQTT, ZeroMQ) | | ✅ |
| Ultra-low latency microservices / telemetry ingestion | | ✅ |
| Memory-constrained environments (containers with <512MB RAM) | | ✅ |
| Cross-language serialization (XML to JSON transcoding at native speed) | | ✅ |
