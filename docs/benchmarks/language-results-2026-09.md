---
title: Language Benchmark Runs (September 2026)
description: Shared XML reads across seven targets, plus repeated Java, Go, C#, and C++ measurements with raw output.
---

# Language benchmark runs — September 2026

These are measurements from one lightly loaded Linux/WSL2 host, **not a ranking
of programming languages**. The suites differ in serializer, model shape, and
the values returned. Readers in all seven target ecosystems use the exact same
[one-sensor and 1,000-sensor XML inputs](https://github.com/polyxml/PolyXML/tree/main/benchmarks/workloads/sensor-batch);
the Go, C#, and C++ adapter writers start from equivalent values but can emit
different bytes.

## Environment and method

| Item | Value |
| :--- | :--- |
| Run date | 2026-09-26 (America/Chicago) |
| Host | AMD Ryzen 5 4500, 12 logical CPUs, 7.7 GiB RAM |
| OS | Ubuntu 26.04.1 under Linux 6.18.33.2 WSL2, x86-64 |
| Source | PolyXML 0.24.0, benchmark source at `8ec122f` (timed loops unchanged by later correctness and version checks) |
| Toolchains | Rust 1.98.1; Python 3.12.14 with PolyXML 0.24.0; Go 1.26.0; GCC 15.2.0; OpenJDK 25.0.4.1; Node 26.8.2 with rebuilt `@polyxml/wasm` 0.24.0; .NET SDK 8.0.425/runtime 8.0.31 |

We checked round trips before timing. Go used five `go test -bench` repetitions
at 2 seconds each. C# used five fresh processes with 1,000 iterations per
operation. The C++ adapter used five fresh processes with 10,000 iterations;
the native C++ binding used five repetitions of 100,000 iterations in one
process. Medians below are from those repetitions. Each suite warmed up
before measuring. For exact samples and allocations, see the raw output links.

## Shared sensor readers

Every row below decoded the same 65-byte or 53,795-byte XML file and checked
the last sensor. These are **within-runtime regression reference points, not
an all-language speed ranking**: C++ uses a fixture-specific adapter, Wasm
returns generic JavaScript values, and the other rows materialize different
generated model types. Setup and code generation are outside timing.

| Target and read path | 1 sensor median | 1,000 sensors median | Raw output |
| :--- | ---: | ---: | :--- |
| Rust generated decoder | 0.524 µs | 0.355 ms | [samples](data/2026-09-26/shared-rust.txt) |
| Python generated dataclasses + native extension | 5.013 µs | 2.470 ms | [samples](data/2026-09-26/shared-python.txt) |
| Go generated structs + `encoding/xml` | 4.401 µs | 3.190 ms | [samples](data/2026-09-26/go.txt) |
| C++ generated models + fixed-fixture adapter | 0.124 µs | 0.106 ms | [samples](data/2026-09-26/cpp-adapter.txt) |
| Java generated POJOs + direct StAX codec | 7.421 µs | 0.511 ms | [samples](data/2026-09-26/shared-java.txt) |
| TypeScript/Wasm XML-to-JSON object | 5.497 µs | 1.753 ms | [samples](data/2026-09-26/shared-wasm.txt) |
| C# generated classes + `XmlSerializer` | 10.088 µs | 1.386 ms | [samples](data/2026-09-26/csharp.txt) |

The Rust, Python, Java, and Wasm lanes have their own five-repeat loops; Java
used 100,000 small-document and 1,000 large-document iterations per repeat
after warmup. The other three use the counts in their source runners. This
shared read workload is separate from the Java JMH suite below and from the
published Python and Rust catalog workloads.

## Go: generated model vs handwritten model

Both paths use `encoding/xml`, so this measures the generated model's tags and
layout rather than a separate PolyXML XML runtime.

| Batch | Operation | Generated median | Handwritten median |
| :--- | :--- | ---: | ---: |
| 1 sensor (65 B XML) | Read | 4.401 µs | 4.419 µs |
| 1 sensor | Write | 2.943 µs | 2.955 µs |
| 1,000 sensors (53,795 B XML) | Read | 3.190 ms | 3.177 ms |
| 1,000 sensors | Write | 0.706 ms | 0.699 ms |

These differences are small relative to the run-to-run spread. The
[raw Go output](data/2026-09-26/go.txt) includes all five samples, B/op, and
allocations/op.

## C#: generated model vs handwritten model

Both paths use `XmlSerializer`. The generated mutable class inherits a base
model; the handwritten baseline is a flat class. The serializer is constructed
before timing, so startup cost is excluded.

| Batch | Operation | Generated median | Handwritten median |
| :--- | :--- | ---: | ---: |
| 1 sensor (65 B XML) | Read | 10.088 µs | 7.213 µs |
| 1 sensor | Write | 6.353 µs | 6.630 µs |
| 1,000 sensors (53,795 B XML) | Read | 1.386 ms | 0.535 ms |
| 1,000 sensors | Write | 0.540 ms | 0.438 ms |

The large-batch read gap merits profiling before attributing it to one
generated feature. The [raw C# output](data/2026-09-26/csharp.txt) includes
every run and per-operation managed allocations.

## C++: native binding and model adapter

The [native C++ binding](https://github.com/polyxml/PolyXML/tree/main/benchmarks/cpp)
uses PolyXML's Rust C ABI for a 50-byte scalar sensor extracted from the shared
one-sensor fixture. Its median was **974 ns/read** and **333 ns/write** across
five repetitions. This includes dynamic value construction on read and FFI
overhead; schema setup was outside timing. See the [raw native output](data/2026-09-26/cpp-native.txt).

The generated C++ model still has no XML codec. A separate, fixture-specific
adapter measured the 1,000-sensor read at **106 µs** for both generated and
handwritten models; the numbers are an adapter sanity check and are **not**
PolyXML C++ codec throughput. See the [raw adapter output](data/2026-09-26/cpp-adapter.txt).
The native binding currently supports scalar fields through its public schema
builder, so we did not label its scalar result as a 1,000-sensor batch result.

## Java: JMH binding comparison

The Java suite uses its own 80-field synthetic settlement and telemetry
projections. It is **not** the shared sensor fixture above. Each JMH operation
processes 1,000 messages; figures below are **batches per second**. We ran
three warmup iterations and five measurement iterations of one second in each
of two forks, with `-prof gc`. The `±` figure is JMH's reported score error.

| Workload | Operation | Direct StAX POJO | Jackson | JAXB | Panama native |
| :--- | :--- | ---: | ---: | ---: | ---: |
| Settlement | Read | 227 ± 31 | 243 ± 2 | 115 ± 4 | 119 ± 4 |
| Settlement | Write | 158 ± 4 | 139 ± 26 | 91 ± 2 | 181 ± 2 |
| Telemetry | Read | 381 ± 78 | 404 ± 9 | 156 ± 4 | 129 ± 4 |
| Telemetry | Write | 451 ± 5 | 345 ± 6 | 230 ± 19 | 230 ± 8 |

The Panama reader returns sampled native field values rather than creating a
Java POJO, and Java heap allocation profiling excludes native Rust memory.
Output sizes can differ. Treat that column as a separate integration path,
not a like-for-like model comparison. The [raw JMH JSON](data/2026-09-26/java-jmh.json)
contains all measured scores, errors, forks, and allocation metrics. These
figures do not include serializer/schema setup or JVM startup.

The remaining JMH paths used the same workloads, forks, warmup, measurement
duration, and profiler. The figures below are also batches per second:

| Workload | Direct read reuse | Direct write reuse | Record read | Record write | POJO mutation | Record mutation |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| Settlement | 228 ± 25 | 157 ± 4 | 116 ± 3 | 148 ± 24 | 84 ± 18 | 64 ± 5 |
| Telemetry | 360 ± 97 | 418 ± 59 | 149 ± 4 | 400 ± 17 | 194 ± 35 | 98 ± 4 |

The reuse cases share StAX factories but still create readers and writers per
message. Mutation includes parse, status update, and write, so it is not
comparable to a read-only or write-only operation. The
[additional JMH JSON](data/2026-09-26/java-jmh-additional.json) contains the
full errors and allocation measurements for these paths.

## What remains

The input is now shared across all seven targets. A defensible cross-language
speedup claim would still require equivalent return values, consistent
read/write scope, and comparable harnesses. In particular, generated C++
models have no XML codec, the native C++ schema builder cannot represent the
batch, and the Wasm lane performs XML-to-JSON object transcoding. The
[benchmark guide](index.md) keeps the broader Python, Rust, Java, and Wasm
studies separate by workload.
