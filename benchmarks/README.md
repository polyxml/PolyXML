# PolyXML Benchmark Suites

Benchmarking tooling for PolyXML's Rust core and its language bindings. The
directory is organized as **one self-contained directory per language** plus a
cross-language orchestrator, so new suites never crowd the root.

## Layout

```
benchmarks/
├── README.md        # This index: catalog, methodology, conventions
├── run_all.sh       # Orchestrator: builds the extension, runs Rust + Python suites
├── __init__.py      # Package marker for `python -m benchmarks.python`
├── cli/             # CLI startup and argument validation (hyperfine)
├── python/          # Python bindings vs the Python XML ecosystem
├── java/            # Java bindings vs JAXB, Jackson, and Panama (JMH)
├── go/              # Go generated models vs handwritten structs
├── cpp/             # C++ generated-model XML adapter
├── csharp/          # C# generated models vs handwritten classes
├── rust-phf-e2e/    # Generated Rust match vs phf XML decoding
├── workloads/       # Shared XML input fixtures for cross-language runs
└── typescript-wasm/ # Node, Bun, and browser Wasm comparisons
```

Each suite directory owns its `README.md`, runner, dependencies, and generated
results; generated output stays in the suite's gitignored `target/` (or
equivalent) and never at this root.

## Catalog

| Suite | Location | Tooling | How to run |
| :--- | :--- | :--- | :--- |
| Rust core engine | [`crates/polyxml-core/benches/`](../crates/polyxml-core/benches/) | [Criterion.rs](https://github.com/bheisler/criterion.rs) | `cargo bench --bench core_benchmarks` |
| Rust tag dispatch | [`crates/polyxml-core/benches/`](../crates/polyxml-core/benches/) | [Criterion.rs](https://github.com/bheisler/criterion.rs) + `perf stat` | `cargo bench --bench tag_dispatch` ([results](../docs/benchmarks/rust-phf-dispatch.md)) |
| Rust end-to-end dispatch | [`rust-phf-e2e/`](rust-phf-e2e/README.md) | Generated decoders | `./benchmarks/rust-phf-e2e/run.sh` |
| Python comparative | [`python/`](python/README.md) | Custom CLI suite | `python -m benchmarks.python` or `./benchmarks/run_all.sh` |
| CLI startup | [`cli/`](cli/README.md) | hyperfine | `./benchmarks/cli/benchmark.sh` |
| Java four-runtime | [`java/`](java/README.md) | JMH (Maven) | See [`java/README.md`](java/README.md) |
| Go models | [`go/`](go/README.md) | `go test -bench` | `./benchmarks/go/run.sh` |
| C++ native binding + XML adapter | [`cpp/`](cpp/README.md) | C++20 chrono | `./benchmarks/cpp/run_runtime.sh` and `./benchmarks/cpp/run.sh` |
| C# models | [`csharp/`](csharp/README.md) | .NET Stopwatch | `./benchmarks/csharp/run.sh` |
| TypeScript/Wasm | [`typescript-wasm/`](typescript-wasm/README.md) | Node, Bun, Chromium | See [`typescript-wasm/README.md`](typescript-wasm/README.md) and [methodology](../docs/benchmarks/wasm-vs-js.md) |

Readers in all seven targets can use the [same byte-for-byte sensor XML fixtures](workloads/sensor-batch/README.md).
Their different return values, runtimes, and serializers still require separate interpretation.

The Rust Criterion suite lives inside its crate because `cargo bench` requires
`benches/` next to the crate manifest — it is the one deliberate exception to the
one-directory-per-suite rule. Filter a benchmark and open the HTML report:

```bash
cargo bench --bench core_benchmarks -- deserialization
# Report: target/criterion/report/index.html
```

### Run the unified Rust and Python suites

```bash
./benchmarks/run_all.sh
```

Requires Rust 1.80+ and Python 3.12+ (or `uv`). The script compiles
`polyxml-python` in `--release`, runs Criterion, then runs the Python suite and
writes `benchmarks/python/results.md` / `results.json`. The Java, Go, C++, C#,
TypeScript/Wasm, and CLI suites have separate entry points shown in the catalog;
their runtime dependencies and workloads differ substantially.

---

## Shared methodology rules

Suite READMEs document their own tooling; these rules apply to all of them:

1. **Smoke runs prove execution, not speed.** Short iterations and single forks
   are execution checks only — never quote them as performance evidence.
2. **Measure on an otherwise idle host** and record the toolchain, OS/CPU, and
   repository revision next to the numbers.
3. **Inspect confidence intervals and repeat** before making any throughput
   claim; a single run on a shared machine is noise.
4. Published figures live in [`docs/benchmarks/index.md`](../docs/benchmarks/index.md) —
   update them from a full, idle-host run, not from a smoke.
5. **Run under a memory cap on shared/small hosts.** Heavy builds and bench
   runs go through [`scripts/memcap.sh`](../scripts/memcap.sh), which caps
   the run at 60% of available RAM in an isolated cgroup (kernel OOM-kills
   only the runaway process instead of freezing the host; `ulimit`
   fallback where systemd is unavailable). The suite entry points
   (`run_all.sh`, `cli/benchmark.sh`, `rust-phf-e2e/run.sh`, `scripts/perf_stat.sh`) already
   re-exec through it; wrap any ad-hoc `cargo bench`/`--release` build the
   same way. Tune with `POLYXML_MEMCAP_PCT`, opt out with
   `POLYXML_MEMCAP_DISABLE=1`.

## Adding a suite

1. Create `benchmarks/<language>/` and copy [`java/README.md`](java/README.md) as
   a template: it covers build, run commands, workloads, interpretation caveats,
   and profiler usage.
2. Keep the suite self-contained — build files, generators, tests, and results all
   live under the suite directory, with generated output gitignored.
3. Do not add loose files to this root; it holds only this index, the
   orchestrator, and the package marker.
4. If `run_all.sh` should drive the suite, gate it on toolchain detection so
   machines without that toolchain still succeed.
5. Add the suite to the catalog above and link it from
   [`docs/benchmarks/index.md`](../docs/benchmarks/index.md).

---

Published methodology, result tables, and reproduction commands:
[`docs/benchmarks/index.md`](../docs/benchmarks/index.md).
