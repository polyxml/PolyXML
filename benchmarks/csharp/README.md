# C# XML benchmark

Run `./benchmarks/csharp/run.sh` from the repository root after
`cargo build -p polyxml-cli`. Requires .NET 8 SDK.

The suite generates mutable C# models from `sensor.xsd` and compares them with a
handwritten equivalent through the same `XmlSerializer`. The serializer is
constructed outside timing, and each path is round-trip checked first. The
small and large cases use batches of 1 and 1,000 sensors, respectively.
Reads use the [shared byte-for-byte XML fixtures](../workloads/sensor-batch/README.md).
The tool prints read/write ns/op, allocated bytes/op, XML bytes, runtime, and OS.
This compares model shape and attributes with the same .NET serializer, not a
native PolyXML codec.

Use `BENCH_ITERATIONS=2 ./benchmarks/csharp/run.sh` for a smoke run. Repeat full
runs on an idle host, retain raw output, and record Git revision, CPU, and
`dotnet --version` before publishing numbers. Generated code stays in `target/`.
