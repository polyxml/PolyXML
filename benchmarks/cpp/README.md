# C++ XML benchmarks

The **native binding lane** exercises PolyXML's actual C++ API over the Rust
C ABI. Run `./benchmarks/cpp/run_runtime.sh` after installing a C++20 compiler
and Rust. It builds `polyxml-c`, creates a scalar `Sensor` schema once, checks
an XML round trip, and measures read and write for the 50-byte sensor message
extracted from the [shared one-sensor fixture](../workloads/sensor-batch/README.md).
Five repeated timings are printed per operation. The native API currently
supports scalar schema fields through `SchemaBuilder`; this lane does not
materialize generated C++ models or the 1,000-sensor shared batch.

Use `BENCH_ITERATIONS=2 ./benchmarks/cpp/run_runtime.sh` for a smoke run. Keep
native-binding numbers separate from the generated-model adapter below.

## Generated-model adapter lane

Run `./benchmarks/cpp/run.sh` from the repository root after
`cargo build -p polyxml-cli`. Requires a C++20 compiler.

The C++ generator currently emits model types and validation, but no XML
read/write codec. This suite measures a deliberately narrow XML adapter that
reads and writes the simple `sensor.xsd` fixture into a generated model and a
handwritten equivalent. It verifies both round trips before timing. The
adapter only handles this fixed fixture and does not escape arbitrary XML text;
the fixture uses safe alphanumeric values. It measures batches of 1 and 1,000
sensors. **Do not interpret these timings as PolyXML C++
XML codec throughput or as a general XML parser comparison.**
Reads use the [shared byte-for-byte XML fixtures](../workloads/sensor-batch/README.md).

Use `BENCH_ITERATIONS=2 ./benchmarks/cpp/run.sh` for smoke. For reportable
adapter measurements, repeat on an idle host and record raw output, Git
revision, OS/CPU, and compiler version. Generated code stays in `target/`.
