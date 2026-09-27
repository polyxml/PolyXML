# C++ generated-model XML adapter benchmark

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

Use `BENCH_ITERATIONS=2 ./benchmarks/cpp/run.sh` for smoke. For reportable
adapter measurements, repeat on an idle host and record raw output, Git
revision, OS/CPU, and compiler version. Generated code stays in `target/`.
