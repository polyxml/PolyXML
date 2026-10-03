---
name: polyxml-runtime-investigation
description: Diagnose and experimentally optimize regressions in PolyXML's dynamic Rust XML runtime using allocation counts, instruction profiles, and same-host latency comparisons.
---

# Dynamic XML runtime investigation

Read [the investigation toolkit](../../../benchmarks/rust-runtime-investigation/README.md)
for exact commands and diagnostic boundaries. Keep generated XML/Serde consumers
separate from the schema-driven `PolyValue` runtime.

- Start with an explicit detached baseline and a committed candidate on a branch.
  Keep core sources stable while consumers build and run. Retain small experiments
  independently so a failed hypothesis can be backed out without losing evidence.
- Use the counting allocator to distinguish allocation growth from CPU work.
  Requested bytes are cumulative requests, not peak or retained memory. Larger
  schema types can grow a fixed frame reserve without increasing allocation count.
- Use Callgrind's `measured_region` toggle for a fixed operation count. Its simulated
  instruction count identifies candidates; verify changes with uninstrumented
  Criterion consumers. Extracting Valgrind locally is an option when installation
  would unnecessarily change the host.
- Preflight companion tooling before timing. Even `uv run runner.py --help` can
  synchronize and rebuild an editable binding. After preparing its environment,
  use `uv run --no-sync` for companion runs. If a build overlaps timing, retain
  and mark that attempt as contaminated, then repeat it.
  Use revision-owned CLI/native-binding build targets too: a shared debug
  directory can retain another worktree's same-named top-level artifact despite
  a fresh Cargo result. Probe a fixture that distinguishes the revisions and
  record artifact hashes before using companion results as a comparison.
- Preflight the exact PATH used by the quality gate as well as timing tools.
  A locally installed TypeScript compiler needs its node_modules/.bin on PATH;
  having Node available does not make tsc available. Record missing-tool failures
  and rerun the complete gate with the prepared toolchain environment.
- Run heavy work serially, with one Cargo worker and the repository memory cap.
  Short comparisons screen hypotheses. Repeat retained improvements with longer
  measurements, alternating revision order and preserving every raw sample.
- Anchor Criterion filters: `^serialization/` means writes only; `serialization`
  also matches deserialization. Remove stale results only in the runner's owned
  scratch directory before measurement, not from retained raw evidence.
- Check rich enums, patterns, lists and mixed branches alongside plain records.
  Warm pattern-cache gains do not establish cold-start, churn or concurrent
  throughput. Entry/key bounds do not constitute a compiled-regex byte budget.
- Preserve validation, errors, split Text/CData/GeneralRef handling, nil reads,
  nesting and metadata mutation semantics. Borrow scalar metadata from the schema
  owned by a frame instead of cloning rich scalar definitions per element.
  Profile empty `xsi:type` registry checks too: a shared flag maintained by the
  registration setter can avoid locks without caching mutable field metadata.
  Compare an empty registry with actual populated dispatch using `variants.rs`;
  do not assume a plain-record gain also proves polymorphic throughput.
- Include the 17-pattern cycle when changing eviction: a 16-entry FIFO has
  systematic misses, while salted victim selection preserves reuse at the same
  capacity. Confirm hot cases too; pressure improvements alone do not establish
  universal throughput gains.
- Publish positive and negative experiments with exact source revisions, raw
  evidence and remaining regressions. Run the full quality gate before pushing
  the experiment branch; a branch request does not authorize a merge to main.
