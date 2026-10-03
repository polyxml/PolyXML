# Dynamic Rust XML runtime investigation

These tools diagnose the schema-driven `PolyValue` runtime. Generated Rust XML
and JSON/Serde consumers have separate suites in
[`rust-xml-regression`](../rust-xml-regression/README.md).

Start from a committed core implementation and an explicit detached baseline.
Run builds, profiles and latency measurements serially through `scripts/memcap.sh`.
The runners use one Cargo worker, systemd memory limits and disabled swap. Keep
source files stable until a comparison finishes; source revisions describe HEAD,
so an uncommitted core edit makes that identification incomplete.

## Latency comparisons

For plain records, use the current core harness for both revisions:

```bash
python3 benchmarks/rust-xml-regression/run_core.py \
  --baseline ../polyxml-perf-control \
  --output benchmarks/rust-runtime-investigation/target/evidence/final-core \
  --rounds 3 --samples 100 --warmup 3 --measurement 5
```

For scalar constraints and content-model validation, use the alternate harness:

```bash
python3 benchmarks/rust-xml-regression/run_core.py \
  --baseline ../polyxml-perf-control \
  --harness benchmarks/rust-runtime-investigation/constraints.rs \
  --output benchmarks/rust-runtime-investigation/target/evidence/final-constraints \
  --rounds 3 --samples 100 --warmup 3 --measurement 5
python3 benchmarks/rust-runtime-investigation/summarize.py \
  benchmarks/rust-runtime-investigation/target/evidence/final-constraints
```

Both harnesses check decoded values and XML round trips before timing. The
constraint harness measures 1,000 repeated values: a 16-member enum, one scalar
pattern, a string restriction with a pattern and length, lexical integer lists,
and a content regex. Pattern cases measure **warm reuse**, including value
validation but excluding the first compilation. They do not characterize cold
patterns, cache churn, many-pattern schemas or multithreaded throughput.

Criterion filters are regexes: use `--filter '^serialization/'` for writes only.
The unanchored filter `serialization` also matches `deserialization`. The runner
clears its own Criterion directory before each process, retains separate results
and alternates baseline/candidate order. Short runs (`--rounds 2 --samples 50
--warmup 1 --measurement 2`) screen hypotheses; confirm useful changes with longer
runs. The summarizer takes the median of independent process mean estimates and
retains each process's confidence interval and delta. It does not manufacture a
confidence interval across process repetitions.

## Allocation diagnostics

```bash
python3 benchmarks/rust-runtime-investigation/run.py \
  --repo ../polyxml-perf-control --label control \
  --output benchmarks/rust-runtime-investigation/target/evidence/control
python3 benchmarks/rust-runtime-investigation/run.py \
  --repo . --label candidate \
  --output benchmarks/rust-runtime-investigation/target/evidence/candidate
```

The standalone release consumer enables a counting allocator only around the
measured operation. It counts allocations, reallocations and requested bytes;
requested bytes are cumulative requests, **not peak RSS or retained memory**.
Schema construction, ten warmup operations, and correctness checks are outside
the counted region. The returned value or byte buffer is destroyed in that
region. Use count 0 for the sensor and 1,000/10,000 for catalogs. Diagnostic
binaries contain instrumentation even when counting is disabled; never use
these binaries for latency claims. Public schema/scalar/value sizes are recorded
to distinguish per-frame growth from per-value heap allocation.

## Instruction profiles

With Valgrind installed, profile the same consumer and fixed number of operations:

```bash
scripts/memcap.sh valgrind --tool=callgrind --collect-atstart=no \
  --toggle-collect=measured_region \
  --callgrind-out-file=target/read.callgrind \
  --log-file=target/read-valgrind.txt --error-exitcode=93 \
  benchmarks/rust-runtime-investigation/target/candidate/target/release/xml-diagnostics \
  read 1000 25 instructions
callgrind_annotate --auto=no --inclusive=no --threshold=95 \
  target/read.callgrind > target/read-annotated.txt
```

`measured_region` is deliberately not inlined. Collection excludes startup and
warmup; the release consumer retains line tables for attribution. Callgrind
counts executed instructions in a simulation; those counts explain hypotheses,
but do not measure hardware cycles or prove a latency improvement. If Valgrind
is extracted into an isolated directory, set `VALGRIND_LIB` to its matching
`usr/libexec/valgrind` directory and invoke its executable by absolute path.

Retain baseline and candidate profiles, source harnesses, lockfiles, build logs,
metadata, every Criterion sample and verification logs under a dated
`docs/benchmarks/data/` directory. Link the resulting report from the benchmark
index. Include experiments that failed to improve performance, and state what
remains slower than the historical baseline.

`cache_boundaries.rs` is a second alternate harness for the cache's working-set
boundary: 1, 16, 17 and 64 distinct patterns, eight occurrences per pattern.
Reads cycle pattern keys to force FIFO misses above capacity. Writes visit each
field's eight occurrences consecutively, so they can still reuse each compiled
regex locally even when the full schema exceeds capacity. Run it with the same
`--harness` option and retain both cases; field order affects reuse.
