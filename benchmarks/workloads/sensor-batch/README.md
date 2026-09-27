# Shared sensor batch workload

`sensor-1.xml` and `sensor-1000.xml` are byte-identical input fixtures for
benchmark readers in all seven target ecosystems. `batch.xsd` defines the
shared root and sensor fields. Each `<Sensor>` has an `Id` of `sensor-N`
and a numeric `Value` of `N`, from zero to count minus one. Both files have a
`<Batch>` root and no XML declaration, namespace, or whitespace between nodes.

Readers verify the number of sensors and final ID and value before timing.
Writers in the Go, C#, and C++ adapter suites start from equivalent
preconstructed models and may emit runtime-specific namespace declarations;
compare logical values and record output byte length separately.

| Target | Shared-fixture read command | What it returns |
| :--- | :--- | :--- |
| Rust | `./benchmarks/workloads/sensor-batch/run_rust.sh` | Generated borrowed models |
| Python | `./benchmarks/workloads/sensor-batch/run_python.sh` | Generated dataclasses through the local PolyXML extension |
| Java | `./benchmarks/workloads/sensor-batch/run_java.sh` | Generated POJOs through direct StAX codecs (JDK 22+) |
| Go | `./benchmarks/go/run.sh` | Generated structs through `encoding/xml` |
| C++ | `./benchmarks/cpp/run.sh` | Generated models through a fixture-specific adapter |
| C# | `./benchmarks/csharp/run.sh` | Generated classes through `XmlSerializer` |
| TypeScript/Wasm | `./benchmarks/workloads/sensor-batch/run_wasm.sh` | Generic JavaScript objects from Wasm XML-to-JSON transcoding |

Build the local CLI first with `cargo build -p polyxml-cli`. The Python runner
requires a Python 3.12+ environment with the **current local** PolyXML extension
installed; the Java runner requires a JDK 22+ on `PATH`; the Wasm runner
requires `./crates/polyxml-wasm/build.sh` first. Each runner documents its
toolchain in the output or suite README. The C++ native binding additionally
measures the scalar sensor inside `sensor-1.xml` but cannot represent this
batch through its public schema builder.

This common input does **not** make the language numbers directly comparable:
the return types and binding layers differ. Use it to detect regressions and
compare alternatives within a runtime. Keep the full-suite Java JMH numbers
separate: they use 80-field settlement/telemetry projections.
