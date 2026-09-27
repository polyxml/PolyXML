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
| C++ native binding + generated models | 1.784 µs | 1.079 ms | [samples](data/2026-09-26/cpp-native.txt) |
| Python generated dataclasses + native extension | 5.013 µs | 2.470 ms | [samples](data/2026-09-26/shared-python.txt) |
| Go generated structs + `encoding/xml` | 4.401 µs | 3.190 ms | [samples](data/2026-09-26/go.txt) |
| Java generated POJOs + direct StAX codec | 7.421 µs | 0.511 ms | [samples](data/2026-09-26/shared-java.txt) |
| TypeScript/Wasm XML-to-JSON object | 5.497 µs | 1.753 ms | [samples](data/2026-09-26/shared-wasm.txt) |
| C# generated classes + `XmlSerializer` | 7.213 µs | 0.556 ms | [samples](data/2026-09-26/csharp.txt) |
| *C++ narrow string-search adapter (non-conforming)* | *0.124 µs* | *0.106 ms* | [*samples*](data/2026-09-26/cpp-adapter.txt) |

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
| 1 sensor (65 B XML) | Read | 7.213 µs | 7.025 µs |
| 1 sensor | Write | 6.353 µs | 6.630 µs |
| 1,000 sensors (53,795 B XML) | Read | 0.556 ms | 0.557 ms |
| 1,000 sensors | Write | 0.442 ms | 0.438 ms |

The previously observed 1,000-sensor read gap (1.386 ms vs 0.535 ms) was isolated
to a .NET 8 JIT warmup artifact: `XmlSerializer` generates dynamic IL methods that start
in Tier 0. With 100 warmups, whichever model executed first in the process ran while
Tier 0 execution and background Tier 1 compilation occurred. Increasing warmup iterations
to 500 demonstrated identical steady-state execution (~0.556 ms, within 0.2%). The
[raw C# output](data/2026-09-26/csharp.txt) includes all runs and allocations.

## C++: native C-ABI binding and model adapter

The [native C++ binding](https://github.com/polyxml/PolyXML/tree/main/benchmarks/cpp)
exercises PolyXML's full C++20 API (`polyxml.hpp`) backed by the Rust core engine via
`polyxml-c`. With native C-ABI support for nested records and lists (`add_list_nested`),
the harness deserializes shared XML into `polyxml::Value` and converts it into generated
modern C++20 models (`polyxml::generated::Batch`), as well as serializing typed models back to XML:

| Batch | Operation | Native C++ binding median | Non-conforming adapter |
| :--- | :--- | ---: | ---: |
| 1 sensor (65 B XML) | Read | 1.784 µs | 0.124 µs |
| 1 sensor | Write | 0.773 µs | 0.070 µs |
| 1,000 sensors (53,795 B XML) | Read | 1.079 ms | 0.106 ms |
| 1,000 sensors | Write | 0.382 ms | 0.051 ms |

All runs assert exact round-trip fidelity, entity escaping (`&amp;`, `&lt;`, `&gt;`), and strict
syntax error handling. See the [raw native C++ output](data/2026-09-26/cpp-native.txt)
and the [raw adapter output](data/2026-09-26/cpp-adapter.txt). The narrow string-search adapter
is retained strictly as an adapter sanity check and must not be cited as XML parser throughput.

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

## Representative moderate workload: trade-order feed

To evaluate performance on realistic, attribute-heavy documents with optional fields and nested
structures (depth 4), we introduced the [trade-order workload](https://github.com/polyxml/PolyXML/tree/main/benchmarks/workloads/trade-order).
It features 6 complex types, 12 XML elements, 7 XML attributes, and a documented PRNG-seeded distribution
of optional elements and attributes:

- **Single order message (`order-single.xml`)**: 383 bytes (1 order, 2 items), measuring small-message per-call overhead.
- **Moderate batch feed (`order-feed-50.xml`)**: 21,330 bytes (50 orders, 117 items), measuring moderate-scale repeated-record throughput with realistic field sparsity.
- **Documented optional field density**: `Order.priority` (60% present), `Party.TaxId` (40% present), `Item.Discount` (35% present), `Item.Notes` (25% present), `Item.category` (50% present), and `Order.Settlement` (70% present).

All fixtures are generated deterministically by `generate_fixtures.py`.

## What remains

The shared sensor batch input is now executed across all seven targets with tightened bounds
and first/middle/last assertions verifying semantic correctness. With C++ native C-ABI nested schema
support, C++ now has a real, conforming XML codec path linked to generated C++20 models. The
[benchmark guide](index.md) catalogs the broader Python, Rust, Java, C++, and Wasm studies
by workload.
