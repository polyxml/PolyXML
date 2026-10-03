# Generated Rust XML and Serde regression runner

Builds the same sensor XSD in borrowed and owned modes with a baseline checkout
and the current checkout, then checks every decoded field in both XML and JSON
round trips before timing. XML writes use the declared `Batch` root explicitly.
The consumers use an identical measurement harness and shared dependency lock
resolution; retained lockfiles allow auditing differences.

```bash
# Choose the baseline release or commit explicitly.
git worktree add --detach ../polyxml-benchmark-baseline <baseline-commit>
python3 benchmarks/rust-xml-regression/run.py \
  --baseline ../polyxml-benchmark-baseline \
  --output docs/benchmarks/data/<date>/generated --rounds 2
```

Run on an idle host. Builds and runs are serial, one Cargo worker, through the
required systemd memory cap with swap disabled. Each operation warms up for
100 calls, then records seven samples of at least 250ms each. Each sample includes
model/output destruction. Reads include allocations; writes allocate a fresh
output buffer from a preconstructed model. JSON serialization uses `to_vec`,
JSON parsing uses `from_str`. Sizes are one and 1,000 sensors. Alternating revision
order across process rounds helps identify drift; increase rounds for uncertain
results. No cross-language ranking or allocation-count claim follows from this
suite. Output retains all samples, generated sources, dependency locks, build
logs, source revisions and toolchain/host metadata. Builds are excluded from timing.

## Dynamic core comparison

```bash
python3 benchmarks/rust-xml-regression/run_core.py \
  --baseline ../polyxml-benchmark-baseline \
  --output docs/benchmarks/data/<date>/core --rounds 2
```

Uses the current core Criterion harness for both core revisions, adding list-count
and XML round-trip equality checks before timing. Measures micro telemetry and
1,000/10,000-item catalogs with 100 samples, three-second warmup, five-second
measurement targets. Retains every Criterion sample and estimate JSON separately
for each process round; standalone consumers share dependency resolutions. These
results return dynamic PolyValue records and belong in a separate table from
XSD-generated models. Dependencies and harness code are fixed per comparison;
this is a source regression check, not a reproduction of the old release toolchain.

After both runners finish, use `python3 benchmarks/rust-xml-regression/summarize.py
<results-directory>`. The summary uses each process's median for generated
consumers and each process's Criterion mean for the core, then the median across
processes. It retains individual process values and Criterion confidence
intervals rather than implying a confidence interval from two process runs.
