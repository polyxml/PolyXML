# Python AOT Native Extension vs Standard Dataclass Benchmark

- **Date**: 2026-09-27T03:05:26Z
- **Python**: 3.12.14
- **Iterations**: 2000 (after 50 warmup runs)
- **Semantic Check**: Verified identical decoded fields between AOT PyO3 models and standard dataclasses.
- **Mathematical Check**: Verified that `ops_per_sec * median_sec == 1.0` exactly.

## 1. Deserialization Throughput & Latency

| Workload | XML Size | Dataclass Latency | Dataclass Ops/s | AOT Latency | AOT Ops/s | AOT Speedup | AOT Throughput |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **USAF UCI v2.5 Telemetry (Real-World)** | 1,510 B | 62.5 μs | 16,003 | **7.44 μs** | **134,336** | **8.39x** | 193.5 MB/s |
| **SensorReading (Synthetic)** | 226 B | 11.3 μs | 88,367 | **2.31 μs** | **433,839** | **4.91x** | 93.5 MB/s |

## 2. Serialization Throughput & Latency

| Workload | XML Size | Dataclass Latency | Dataclass Ops/s | AOT Latency | AOT Ops/s | AOT Speedup | AOT Throughput |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **USAF UCI v2.5 Telemetry (Real-World)** | 1,510 B | 39.8 μs | 25,141 | **3.88 μs** | **257,932** | **10.26x** | 371.4 MB/s |
| **SensorReading (Synthetic)** | 226 B | 7.4 μs | 135,980 | **1.74 μs** | **573,393** | **4.22x** | 123.6 MB/s |
