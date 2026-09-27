# C++ XML benchmarks

## Native C++ binding lane (`runtime.cpp` / `run_runtime.sh`)

The **native binding lane** exercises PolyXML's full modern C++20 API (`polyxml.hpp`) backed by the Rust core engine via the `polyxml-c` C-ABI. Run:

```bash
./benchmarks/cpp/run_runtime.sh
```

Prerequisites: A modern C++20 compiler (`g++` >= 11 or `clang++` >= 13) and Rust.

### Workload and Methodology
1. **Schema Definition**: Compiles schemas dynamically with `polyxml::SchemaBuilder` supporting nested records and lists (`add_list_nested`, `add_nested`, `add_element`).
2. **Generated Models**: Generates modern C++20 models (`batch.hpp`) from the shared `batch.xsd` schema using `polyxml generate --lang cpp`.
3. **End-to-End Typed Processing**:
   - **Read**: Deserializes raw XML into `polyxml::Value` and extracts strongly typed `polyxml::generated::Batch` models (`std::vector<SensorType>`).
   - **Write**: Maps `polyxml::generated::Batch` into `polyxml::Value` records and serializes to conforming XML with `polyxml::serialize`.
4. **Shared Fixtures**: Evaluates both the 1-sensor (65 bytes) and 1,000-sensor (53,795 bytes) fixtures from `benchmarks/workloads/sensor-batch/`.
5. **Correctness Verifications**:
   - Asserts exact 1-sensor round trip.
   - Asserts 1,000-sensor batch bounds: size 1,000, first item (`sensor-0`, `0`), middle item (`sensor-500`, `500`), and last item (`sensor-999`, `999`).
   - Asserts entity escaping and unescaping (`&amp;`, `&lt;`, `&gt;`).
   - Asserts strict syntax validation (malformed XML properly throws `polyxml::Exception`).
6. **Repetitions**: Runs 5 repeated iterations on steady-state execution reporting `ns/op`.

Use `BENCH_ITERATIONS_1=1000 BENCH_ITERATIONS_1000=50 ./benchmarks/cpp/run_runtime.sh` for quick testing.

## Narrow string search adapter lane (`bench.cpp` / `run.sh`)

Run `./benchmarks/cpp/run.sh` from the repository root after `cargo build -p polyxml-cli`.

> [!WARNING]
> This suite uses a hand-rolled `std::string::find` search adapter, **NOT** a conforming XML parser. It was historically used as an adapter sanity check for generated models before full C-ABI nested schema support was added.
> Output lines are prefixed with `adapter_` (`adapter_generated`, `adapter_baseline`). **Do not cite these timings as PolyXML C++ XML parser throughput.** Refer to the Native C++ binding lane above for true PolyXML C++ benchmarks.
