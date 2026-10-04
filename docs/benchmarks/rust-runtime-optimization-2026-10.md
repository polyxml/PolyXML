---
title: Rust XML runtime optimization — October 2026
description: Dynamic XML optimization experiments with instruction profiles, allocation counts and retained same-host comparisons.
---

# Rust XML runtime optimization — October 2026

The retained branch improves plain XML writes by **18–20%** and rich-enum reads
by about **45%** versus 0.34.6 on these fixtures. Warm scalar-pattern workloads
improve much more by removing repeated regex compilation. Plain catalog reads
still cost approximately **5% more than 0.27.0**. The tables below link the samples
and process estimates supporting each result.

This follows the [XML/Serde regression audit](rust-xml-serde-regression-2026-10.md).
The optimization changes the dynamic, schema-driven Rust XML engine. Generated
Rust models use their own quick-xml methods and Serde JSON implementations.
Their emitted model sources remain byte-identical in this comparison.

## Retained changes

- Append XML directly to a byte vector, replacing a cursor over that vector.
- Format lexical lists into one string rather than allocating individual token
  strings and collecting a vector before joining. Keep primitive formatting inline.
- Select fixed-value and lexical-list obligations before prevalidation; skip
  recursive list scans for ordinary repeated records. Nested writers retain
  their own validation.
- Store active field/branch indices in the parser and borrow scalar metadata
  from the frame's schema. Avoid cloning enum members and pattern strings for
  every value. Consolidate ordinary/mixed scalar state and move nested mixed
  branch names into the resulting object.
- Reuse exactly anchored scalar regexes through a concurrent cache. It admits
  16 entries with keys up to 4,096 bytes. Larger keys compile uncached; invalid
  expressions retain the previous validation failure. Matching and compilation
  run outside the lock. At capacity, an eviction nonce and the existing randomized
  hasher select a victim, avoiding the FIFO prototype's cyclic misses.

The cache keys actual pattern strings, and parser indices resolve actual field
metadata. Public schema edits before sharing do not leave stale prepared flags.
Fixed values, lexical-list whitespace rejection, content-model validation and
scalar restrictions remain enabled. Cache limits bound entries/key retention,
**not compiled-regex bytes or peak memory**. Concurrent misses can compile the
same expression more than once before one result is retained.

## Method

The measured implementation is
[`7f845ae`](https://github.com/polyxml/PolyXML/commit/7f845ae043b68cdb3166d04ce74eaa99bc9c2b0d).
The control is [`bbbd087` (0.34.6)](https://github.com/polyxml/PolyXML/commit/bbbd087a9a8e9838a32912973cd4029c30f29c08).
The historical comparison uses [`cef575c` (0.27.0)](https://github.com/polyxml/PolyXML/commit/cef575c11cc4f05e877ced157036f7ff53b8687b).
Each comparison records exact revisions and clean core-source status in its
metadata. Subsequent documentation commits do not change the measured engine.

The host is an Intel Core i5-11600K under WSL2, rustc 1.99.0 / LLVM 23.1.1.
Confirmation builds and measurements run serially, with one Cargo worker,
systemd memory caps at 60% of available RAM, and swap disabled inside each scope.
The IDE is present; affinity and CPU frequency are not pinned. No OOM occurred.

All Criterion confirmation suites use three processes per revision, 100 samples,
three-second warmup and five-second measurement targets. Order alternates
baseline/candidate, candidate/baseline, baseline/candidate. Tables use the median
of process mean estimates. Each process's confidence interval and paired delta
remain in the summary; samples within a process are not independent process
repetitions. Small deltas deserve more caution than the large gains.

The core harness checks round-trip equality and catalog counts before timing.
Payloads are 94 bytes for the sensor and 63,911 / 668,911 bytes for catalogs;
written catalogs are 63,411 / 663,911 bytes because numeric spelling normalizes.
The constraint harness checks every value and round trips: 1,000 enum values,
patterned strings, restricted strings, lexical integer lists, or strings with a
content regex. Scalar-pattern cases measure warm reuse, excluding first
compilation. Working-set pressure receives a separate suite; concurrent
throughput and cold-start latency are unmeasured.

The [investigation toolkit](https://github.com/polyxml/PolyXML/tree/7f845ae043b68cdb3166d04ce74eaa99bc9c2b0d/benchmarks/rust-runtime-investigation)
and [comparison runner](https://github.com/polyxml/PolyXML/tree/7f845ae043b68cdb3166d04ce74eaa99bc9c2b0d/benchmarks/rust-xml-regression)
contain reproduction commands. Criterion filters are regexes: `^serialization/`
selects writes; `serialization` also matches deserialization.

## Plain-record confirmation

[Process estimates](data/2026-10-03/runtime-optimization/selected-core/summary.json) and
[all Criterion samples](data/2026-10-03/runtime-optimization/selected-core/samples.tar.gz) support this table.
The first confirmation attempt is retained in the prototype archive and excluded:
a companion `uv run ... --help` unexpectedly rebuilt its editable binding during
the final round. Both the clean repeat and the later retained-policy run are saved.

| Operation | Fixture | 0.34.6 µs | Candidate µs | Change |
| --- | --- | ---: | ---: | ---: |
| read | catalog 1000 | 797.834 | 788.425 | -1.18% |
| read | catalog 10000 | 8,011.487 | 7,932.545 | -0.99% |
| read | sensor | 1.046 | 1.012 | -3.16% |
| write | catalog 1000 | 223.146 | 179.231 | -19.68% |
| write | catalog 10000 | 2,253.219 | 1,800.463 | -20.09% |
| write | sensor | 0.279 | 0.228 | -18.42% |

All write fixtures improve across all three paired processes. Catalog read
changes are small; the sensor gain should not be generalized to larger documents.

## Historical release comparison

[Process estimates](data/2026-10-03/runtime-optimization/selected-old-core/summary.json) and
[raw samples](data/2026-10-03/runtime-optimization/selected-old-core/samples.tar.gz) support this fresh same-host
comparison, rather than a ratio of separate studies.

| Operation | Fixture | 0.27.0 µs | Candidate µs | Change |
| --- | --- | ---: | ---: | ---: |
| read | catalog 1000 | 755.809 | 796.211 | +5.35% |
| read | catalog 10000 | 7,585.809 | 7,963.984 | +4.99% |
| read | sensor | 0.975 | 1.009 | +3.50% |
| write | catalog 1000 | 200.999 | 180.104 | -10.40% |
| write | catalog 10000 | 2,005.150 | 1,806.558 | -9.90% |
| write | sensor | 0.258 | 0.229 | -11.07% |

Writes recover the historical regression and improve further. Catalog reads
still cost approximately 5% more than 0.27.0; full read recovery remains unresolved.

## Constrained scalar workloads

[Process estimates](data/2026-10-03/runtime-optimization/selected-constraints/summary.json) and
[raw samples](data/2026-10-03/runtime-optimization/selected-constraints/samples.tar.gz) support this table.
Times are microseconds per 1,000-value document.

| Operation | Constraint | 0.34.6 µs | Candidate µs | Change |
| --- | --- | ---: | ---: | ---: |
| read | content | 192.784 | 193.025 | +0.13% |
| read | enum | 351.979 | 194.919 | -44.62% |
| read | list | 282.413 | 268.777 | -4.83% |
| read | pattern | 10,165.972 | 230.009 | -97.74% |
| read | restricted | 10,279.245 | 243.834 | -97.63% |
| write | content | 48.653 | 37.219 | -23.50% |
| write | enum | 69.108 | 53.182 | -23.04% |
| write | list | 232.458 | 191.602 | -17.58% |
| write | pattern | 9,603.202 | 92.221 | -99.04% |
| write | restricted | 9,557.319 | 103.450 | -98.92% |

Enum reads improve consistently in all three processes. Lexical-list reads have
a smaller gain; content-model reads remain approximately unchanged. Pattern
speedups are specific to warm reuse: the control compiles a regex for every
scalar, while the candidate reuses it. Both validate every value.

## Pattern working-set pressure

[Process estimates](data/2026-10-03/runtime-optimization/selected-cache-boundaries/summary.json) and
[raw samples](data/2026-10-03/runtime-optimization/selected-cache-boundaries/samples.tar.gz) support this table.
There are eight values per pattern. Reads cycle through keys; writes visit each
field's eight occurrences consecutively. Compare revisions within each row,
because document sizes vary with the number of patterns.

| Operation | Active patterns | 0.34.6 µs | Candidate µs | Change |
| --- | --- | ---: | ---: | ---: |
| read | 1 | 78.357 | 2.320 | -97.04% |
| read | 16 | 1,276.524 | 34.537 | -97.29% |
| read | 17 | 1,358.826 | 206.634 | -84.79% |
| read | 64 | 5,238.168 | 5,254.703 | +0.32% |
| write | 1 | 72.397 | 0.796 | -98.90% |
| write | 16 | 1,178.806 | 12.870 | -98.91% |
| write | 17 | 1,249.130 | 34.669 | -97.22% |
| write | 64 | 4,926.045 | 689.333 | -86.01% |

At 17 patterns, the FIFO prototype was about 4% slower than the control on reads
in both short screening passes. Salted victim selection retains reuse under
that cycle without raising capacity. Against FIFO, the policy screening showed
about 7× faster reads and 5× faster writes for 17 patterns. The retained policy's
longer comparison above confirms gains against the unoptimized control.
At 64 patterns, reads remain approximately unchanged; field-local reuse still
helps writes. This is a bounded reuse cache, rather than a prepared-schema index.

## Generated Rust XML and JSON/Serde control

[Per-process values](data/2026-10-03/runtime-optimization/selected-generated/summary.json),
[metadata](data/2026-10-03/runtime-optimization/selected-generated/metadata.json), and the retained CSVs/model
sources support this four-round control. Each operation has 100 warmup calls,
then seven samples of at least 250ms; the table uses the median of each process's
sample medians, followed by the median across processes. All fields and round
trips are checked before timing, and XML writes specify the declared Batch root.

| Operation | Mode / sensors | 0.34.6 µs | Candidate µs | Change |
| --- | --- | ---: | ---: | ---: |
| json read | borrowed / 1 | 0.0821 | 0.0803 | -2.18% |
| json read | borrowed / 1000 | 60.3231 | 58.9033 | -2.35% |
| json write | borrowed / 1 | 0.0383 | 0.0403 | +5.44% |
| json write | borrowed / 1000 | 25.6639 | 25.9780 | +1.22% |
| xml read | borrowed / 1 | 0.3000 | 0.2873 | -4.26% |
| xml read | borrowed / 1000 | 194.6537 | 185.7288 | -4.58% |
| xml write | borrowed / 1 | 0.1708 | 0.1712 | +0.24% |
| xml write | borrowed / 1000 | 54.2813 | 51.6765 | -4.80% |
| json read | owned / 1 | 0.0808 | 0.0809 | +0.13% |
| json read | owned / 1000 | 59.2453 | 59.6227 | +0.64% |
| json write | owned / 1 | 0.0366 | 0.0367 | +0.40% |
| json write | owned / 1000 | 24.2438 | 24.1003 | -0.59% |
| xml read | owned / 1 | 0.3133 | 0.3145 | +0.38% |
| xml read | owned / 1000 | 234.4708 | 233.4626 | -0.43% |
| xml write | owned / 1 | 0.1750 | 0.1787 | +2.15% |
| xml write | owned / 1000 | 80.0840 | 80.6449 | +0.70% |

There is no material batch slowdown in this control. Some tiny writer cases cost
approximately 2–4 ns more, and borrowed XML batches improve in these builds.
Model sources are identical; these observations do not establish a general
Serde speedup. An apparent borrowed JSON batch-write slowdown in the initial
two-round screen disappeared in the four-round repeat; both attempts are retained.

## Profiling and allocations

The [0.27.0 allocation log](data/2026-10-03/runtime-optimization/diagnostics/old/old-allocations.txt),
[0.34.6 log](data/2026-10-03/runtime-optimization/diagnostics/control/control-allocations.txt), and
[retained-policy log](data/2026-10-03/runtime-optimization/diagnostics/selected/selected-allocations.txt)
show identical allocation counts on the plain fixtures. At 1,000 catalog records,
reads allocate 5,010 times and reallocate 1,014 times; writes allocate 1,001 times
and reallocate 2,007 times. Requested read bytes grew by 768 per document between
0.27.0 and 0.34.6, consistent with 48 bytes of growth across 16 reserved stack
frames. This is cumulative requested memory, **not peak RSS or retained memory**.

The logs record `FieldSchema` growing from 96 to 160 bytes, `ScalarType` from 1
to 32 bytes, and `ValueType` from 16 to 32 bytes. `PolyValue` stays 56 bytes.
Rich scalar cloning costs much more on enum/pattern fixtures even when the
original integer/string fixture does not add allocations.

Callgrind collects only `measured_region`, 25 operations on the 1,000-item catalog.
The [old](data/2026-10-03/runtime-optimization/diagnostics/old/read-1000-annotated.txt),
[control](data/2026-10-03/runtime-optimization/diagnostics/control/read-1000-annotated.txt), and
[retained](data/2026-10-03/runtime-optimization/diagnostics/selected/read-1000-annotated.txt) read profiles record
283,054,621, 294,910,404 and 296,585,254 instructions respectively.
The [old](data/2026-10-03/runtime-optimization/diagnostics/old/write-1000-annotated.txt),
[control](data/2026-10-03/runtime-optimization/diagnostics/control/write-1000-annotated.txt), and
[retained](data/2026-10-03/runtime-optimization/diagnostics/selected/write-1000-annotated.txt) write profiles
record 80,501,030, 88,873,605 and 73,268,130 instructions.
Cursor write-through, formatting and unnecessary list traversal suggested the
write experiments. Read instruction counts have not recovered; timing and
instruction count are separate observations. These simulated counts measure
neither hardware cycles nor latency. Instrumented allocator consumers are never
used for the Criterion tables.

## Experiments and verification

The [screening archive](data/2026-10-03/runtime-optimization/screening.tar.gz) retains eight cumulative
experiments, with two processes per revision, 50 samples, one-second warmup and
two-second measurement targets, plus the working-set screens. The guard-only
serializer experiment was slightly slower; field selection was approximately
neutral on plain records. The first screen retains writer data only, and no read
conclusion is drawn from it. Borrowed metadata, regex reuse, formatter changes,
append-only output and unified scalar state were then tested separately.
The [prototype confirmation archive](data/2026-10-03/runtime-optimization/prototype-confirmation.tar.gz)
preserves the earlier confirmations and contaminated attempt. Archive checksums
and file counts are in [archives.json](data/2026-10-03/runtime-optimization/archives.json).

The [quality gate](data/2026-10-03/runtime-optimization/verification/quality-gate.txt) passes strict Clippy,
321 Rust tests and 112 Python tests with 100% statement and branch coverage.
Additional tests cover repeated/nested lexical lists, whitespace rejection,
escaping, empty lists, integer extremes, schema edits, invalid patterns, raw
restriction text versus trimmed pattern text, split Text/CData/GeneralRef,
mixed branch order, nested frames, nil reads, cache churn and concurrency.

The refreshed [CType sample](data/2026-10-03/runtime-optimization/verification/corpus.txt) matches the
[earlier summary/group table](data/2026-10-03/runtime-optimization/verification/corpus-reference.txt) exactly:
31/31 schema expectations and 23/28 instance round trips, with five existing
failures. The earlier log lacks a source SHA; this is a recorded regression
reference, not a full W3C conformance claim.

An extra fixture exposed escaped XSD enumeration values and nil mixed-branch
serialization limitations. A [minimal reproducer](data/2026-10-03/runtime-optimization/verification/known-limits.rs)
produces the same errors with the [control](data/2026-10-03/runtime-optimization/verification/known-limits-control.txt)
and [retained engine](data/2026-10-03/runtime-optimization/verification/known-limits-selected.txt). These remain
separate correctness work; the optimization retains their behavior.
