---
title: Rust XML and Serde regression check — October 2026
description: Same-host comparison of generated Rust models and the dynamic XML core, with retained samples and dependency locks.
---

# Rust XML and Serde regression check — October 2026

The generated Rust sensor models show no material XML or JSON regression.
The dynamic core has a repeatable slowdown: approximately **8–10% on XML reads**
and **12–14% on XML writes** compared with version 0.27.0. These are separate
execution paths; generated XML methods call quick-xml directly rather than
building dynamic PolyValue records.

This comparison ran on October 3, 2026, between
[`cef575c` (0.27.0, September 27)](https://github.com/polyxml/PolyXML/commit/cef575c11cc4f05e877ced157036f7ff53b8687b)
and [`bbbd087` (0.34.6)](https://github.com/polyxml/PolyXML/commit/bbbd087a9a8e9838a32912973cd4029c30f29c08).
It rebuilds both with the same current toolchain, rather than comparing historical
results from another host. No runtime or generator code was changed for this study.

## Method and environment

- Intel Core i5-11600K, WSL2 Linux 6.18.40.1, rustc 1.99.0 / LLVM 23.1.1.
- Serial release builds with one Cargo worker; systemd memory caps at 60% of
  available RAM, swap disabled inside each scope. No OOM occurred.
- Two processes per revision and mode. Revision order reverses on the second
  round: baseline/current, then current/baseline. IDE background processes were
  present; no other local build or benchmark jobs ran during timing. CPU affinity
  and clocks were not pinned.
- Existing dependency versions are identical between revisions. Core Criterion
  locks have identical dependency names/versions except the local PolyXML version.
  Generated consumers additionally acquire the current core's four regex packages.
- XML reads allocate and destroy the decoded model. XML/JSON writes use an
  existing model, allocate a fresh output buffer and destroy it within timing.
  File I/O, generation, builds and correctness checks are excluded.

[Host metadata and fixture/binary hashes](data/2026-10-03/environment.json),
[generated settings](data/2026-10-03/generated/metadata.json),
[core settings](data/2026-10-03/core/metadata.json), and
[summary with all process values and confidence intervals](data/2026-10-03/summary.json)
are retained. The [raw result directory](https://github.com/polyxml/PolyXML/tree/main/docs/benchmarks/data/2026-10-03)
contains build logs, generated sources, dependency locks, every CSV sample and
Criterion JSON sample/estimate files.

## Generated models: XML and Serde JSON

Uses the shared sensor XSD and byte-identical one/1,000-sensor fixtures.
Each consumer checks every ID and numeric value after initial XML parsing,
XML writing/reparsing and Serde JSON writing/reparsing. XML writes explicitly
use the declared `Batch` element name. Both borrowed and owned modes compile
and pass these checks.

Each operation has 100 warmup calls followed by seven samples of at least 250ms.
The table uses the median of each process's sample medians, in microseconds.
Two processes do not justify a tight overall confidence interval. Positive
changes mean slower operation. [Raw CSV samples and emitted models](https://github.com/polyxml/PolyXML/tree/main/docs/benchmarks/data/2026-10-03/generated)
support the following table.

| Mode | Sensors | Operation | 0.27.0 µs | 0.34.6 µs | Change |
| --- | ---: | --- | ---: | ---: | ---: |
| borrowed | 1 | json read | 0.0817 | 0.0816 | -0.1% |
| borrowed | 1000 | json read | 61.0611 | 60.1965 | -1.4% |
| borrowed | 1 | json write | 0.0393 | 0.0397 | +0.9% |
| borrowed | 1000 | json write | 26.8802 | 26.8247 | -0.2% |
| borrowed | 1 | xml read | 0.2999 | 0.3029 | +1.0% |
| borrowed | 1000 | xml read | 197.0706 | 196.5701 | -0.3% |
| borrowed | 1 | xml write | 0.1722 | 0.1731 | +0.5% |
| borrowed | 1000 | xml write | 54.3188 | 54.6977 | +0.7% |
| owned | 1 | json read | 0.0816 | 0.0811 | -0.7% |
| owned | 1000 | json read | 59.2340 | 59.5526 | +0.5% |
| owned | 1 | json write | 0.0397 | 0.0370 | -6.6% |
| owned | 1000 | json write | 24.2637 | 24.5694 | +1.3% |
| owned | 1 | xml read | 0.3230 | 0.3155 | -2.3% |
| owned | 1000 | xml read | 234.5747 | 235.6331 | +0.5% |
| owned | 1 | xml write | 0.1799 | 0.1753 | -2.6% |
| owned | 1000 | xml write | 80.2948 | 81.3253 | +1.3% |

XML changes fall between −2.6% and +1.3% across these aggregated cases.
JSON has no consistent slowdown; the larger apparent improvement for the owned
single-sensor JSON writer varies by process, so it is not a reliable speedup claim.
This fixture exercises repeated string/integer elements, not namespaces, choices,
attributes, escaping, mixed content or inheritance.

Borrowed mode refers to the generated Cow string representation and XML borrowing.
The current Serde derives lack `serde(borrow)` on string fields; JSON parsing into
those Cow fields allocates owned strings. These timings do not establish an
allocation count or zero-copy JSON capability.

## Dynamic core: XML read and write

Uses the same current Criterion harness for both revisions, with XML round-trip
equality and catalog list-count checks before timing. The fixtures contain a
94-byte scalar sensor and 1,000/10,000-item catalogs. They use manually constructed
ModelSchema values without fixed values, content patterns or lexical xs:list
fields. These operations return dynamic PolyValue records, not generated structs.

Each process uses 100 Criterion samples, three-second warmups and five-second
measurement targets. The table is the median of the two process mean estimates.
The individual 95% confidence intervals are retained in the summary; they are
not confidence intervals across machines or process repetitions.
[Raw Criterion samples and logs](https://github.com/polyxml/PolyXML/tree/main/docs/benchmarks/data/2026-10-03/core)
support the following table.

| Workload | Operation | 0.27.0 µs | 0.34.6 µs | Change |
| --- | --- | ---: | ---: | ---: |
| Catalog 1,000 items | XML read | 751.5526 | 810.3993 | +7.8% |
| Catalog 10,000 items | XML read | 7517.3653 | 8103.8516 | +7.8% |
| Sensor | XML read | 0.9587 | 1.0552 | +10.1% |
| Catalog 1,000 items | XML write | 196.9401 | 223.8763 | +13.7% |
| Catalog 10,000 items | XML write | 2004.5030 | 2250.6360 | +12.3% |
| Sensor | XML write | 0.2509 | 0.2830 | +12.8% |

Every core case is slower in both process rounds. Baseline and current per-process
95% mean confidence intervals do not overlap for these cases. Reversing revision
order preserves the difference, making the signal stronger than a single run.
This is evidence of a regression on these workloads, not a claim about every schema.

Source inspection identifies plausible contributors: frame completion now checks
attribute defaults and fixed values; serialization runs a new prevalidation pass,
including recursively walking repeated values for lexical-list checks even when
the repeated type is an ordinary nested record. Larger runtime schema/value-type
representations and additional branches may also matter. This study does not
attribute the measured slowdown to any one change; a profile or controlled
ablation is needed for that conclusion.

The next optimization should remove unnecessary work for unconstrained fields
while retaining fixed/default, typed-list and content-model enforcement. In
particular, a type-directed lexical-list fast path could avoid walking repeated
nested records that cannot contain a lexical list at that field. Do not disable
validation globally to improve the benchmark. No validation behavior was altered
as part of this documentation and measurement task.

## Reproduction

From the repository root, with systemd user scopes and Rust available:

```bash
git worktree add --detach ../polyxml-benchmark-baseline cef575c11cc4f05e877ced157036f7ff53b8687b
python3 benchmarks/rust-xml-regression/run.py \
  --baseline ../polyxml-benchmark-baseline \
  --output docs/benchmarks/data/<new-date>/generated --rounds 2
python3 benchmarks/rust-xml-regression/run_core.py \
  --baseline ../polyxml-benchmark-baseline \
  --output docs/benchmarks/data/<new-date>/core --rounds 2
python3 benchmarks/rust-xml-regression/summarize.py docs/benchmarks/data/<new-date>
```

The [runner README](https://github.com/polyxml/PolyXML/tree/main/benchmarks/rust-xml-regression)
describes the measured work and retained outputs. Use additional process rounds
when results are uncertain. This study does not rerun the other six languages,
PHF compile-cost experiments, or real-world corpus compilation.

## Verification

The full repository quality gate passed: Rust formatting, strict Clippy,
workspace tests, Python lint, and 112 Python tests with 100% statement/branch
coverage. The strict documentation build passed. The new Rust generated-model
example compiled and ran; Python's XML/JSON/XML example and TypeScript's strict
compile plus Zod JSON round trip also passed. [Verification evidence](data/2026-10-03/verification/verification.json)
and the accompanying logs are retained. Benchmark assertions passed for all four
generated consumers and both core revisions.
