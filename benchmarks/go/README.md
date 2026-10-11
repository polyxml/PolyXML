# Go XML benchmark

Run `./benchmarks/go/run.sh` from the repository root after building the local
CLI with `cargo build -p polyxml-cli`. Requires Go 1.22+.

The runner generates Go structs from `sensor.xsd`, then uses Go's `encoding/xml`
for both generated and handwritten structs. It checks decoding before timing and
measures read/write for one sensor and a 1,000-sensor batch. Both paths read
the [shared XML fixtures](../workloads/sensor-batch/README.md). The comparison measures generated model tags and layout, **not**
a separate PolyXML XML engine. Go reports ns/op, B/op, allocations/op, and MB/s.

For a smoke run use `BENCH_TIME=1x ./benchmarks/go/run.sh`. For reportable
measurements, run on an idle host with `BENCH_COUNT=5 BENCH_TIME=3s`, retain the
raw output, and record the Git revision, OS/CPU, and `go version`. Generated
sources are ignored under `target/`.

Every sensor field is checked outside timing; timed reads only unmarshal.
Set `BENCH_BASELINE_FIRST=1` in alternate fresh processes to rotate model
order. For a repeated comparison, run five separate processes with
`BENCH_COUNT=1 BENCH_TIME=2s` and retain every output, including `B/op` and
`allocs/op`. Both models use the same native `encoding/xml` implementation.
