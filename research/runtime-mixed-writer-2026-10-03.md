# Mixed XML writer investigation — October 3, 2026

Final implementation `58cb346`; initial prototype `6965d33` with harness `d3bf172`,
compared with selected runtime baseline `1819c91` (draft PR #137). Main is unchanged.

The selected writer fixes mixed nil round trips and accelerates distributed
large-table writes by 31% (64 tags) and 55% (256 tags) in the final longer run.
A sampled selector avoids the measured short/concentrated-payload regressions
of the initial index. The final results are below under “Final selector”; earlier
sections preserve the prototypes and their failures. Tiny-document differences
and process variance remain explicit. Main is unchanged.

## Correctness

The reader produced tagged nulls for present mixed elements carrying xsi:nil,
but the writer rejected those values. Emit a present nil element for scalar and
nested branches, preserving order and the distinction from an empty string.
Bind the instance namespace locally, avoiding shadowing the element QName prefix.
Independent namespace-aware output checks cover default/custom prefixes and
namespace-disabled output. Mixed branch metadata currently omits nillability
constraints; this fix preserves the dynamic reader's existing permissive behavior.

## Algorithm

The previous writer searched every mixed branch for each tagged item. For sufficiently large tables/payloads and sampled costly lookups,
build a temporary borrowed-key hash index once per container. Smaller cases retain linear search. Select the lookup once before
the loop. The index borrows metadata, preserves the first match for duplicate
variant names, and is rebuilt each write so public metadata edits remain visible.
A regression exercises indexed nulls, objects, records, duplicate names, metadata
mutation and invalid kinds. This adds no persistent cache or public schema fields.

## Initial long branch-table comparison (`6965d33`)

Three alternating process pairs, 100 samples, 3-second warmup, 5-second measurement.
Fixtures validate every integer/kind and complete ordered round trips before timing.

| Possible child tags | Items | Baseline | Candidate | Time change |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 1,000 | 64.850 us | 59.922 us | -7.60% |
| 16 | 1,000 | 69.575 us | 65.136 us | -6.38% |
| 64 | 1,000 | 99.859 us | 73.502 us | -26.39% |
| 256 | 1,000 | 169.214 us | 78.286 us | -53.74% |

These measure the complete nil/factored-loop/index candidate. Small-table gains
cannot be attributed to the hash index because those cases retain linear search.

## Rejected and diagnostic attempts

The first nil implementation's scalar/pattern timing changed substantially between
short and longer runs. A cold nil helper removed hot-path guards and two nil-only
string allocations, but scalar process pairs still varied. Even pinning to CPU 4
gave +17.34%, -1.56% and +2.18%; a median alone obscures that instability.

A valid Callgrind run over 25 warm writes reported scalar instructions down 2.58%
and pattern instructions up 0.25% for the cold-helper prototype. Simulated
instructions are not cycles or latency. The first profile missed the measured
marker and collected zero; it is retained as a rejected attempt.

A shared workspace debug target also supplied a stale test executable for the
factored-loop prototype. Its missing helper symbols exposed the mismatch. The
same source passed in a worktree-owned target; both preflight logs are retained.
Standalone timing consumers use separate baseline/current targets and explicit
source dependencies. All heavy runs are serial, one Cargo worker, under the
systemd memory cap. Results describe this WSL2 host, not concurrent throughput.

## Initial sparse and read screen (`6965d33`)

Two process pairs, 50 samples, 1-second warmup, 2-second measurement. One-item
writes changed +3.42%, +2.31%, +4.19% and +2.72% for 1/16/64/256 branches,
respectively (roughly 3–6 ns slower). One-item reads range from -1.62% to +0.81%;
1,000-item reads range from -1.00% to +2.77%. This writer optimization does not
establish improved read speed or better latency for tiny documents. Retain these
controls alongside the larger write gains; the short screen is not precise enough
to attribute small read changes to a particular source change.

## Initial common mixed-content controls (`6965d33`)

Same long-run settings as the branch-table comparison.

| 1,000-item writer | Time change | Process-pair changes |
| --- | ---: | --- |
| enum | -3.34% | -2.80%, -3.74%, -1.75% |
| nested | +1.71% | +1.71%, -0.68%, +2.99% |
| pattern | -1.66% | -2.44%, -1.66%, +10.07% |
| scalar | -0.59% | -1.23%, -1.64%, +10.77% |
| text | -2.32% | -2.98%, -1.48%, -1.78% |

These small changes do not establish a universal mixed-content speedup. The large
branch-table result is the principal performance finding. Nested writes retain a
small measured slowdown; scalar writes are roughly flat in this comparison.

## Initial plain-record screen (`6965d33`)

Two process pairs with the short-screen settings.

| Consumer | Time change |
| --- | ---: |
| deserialization/catalog_items/1000 | +0.49% |
| deserialization/catalog_items/10000 | +0.29% |
| deserialization/sensor_micro | +3.29% |
| serialization/catalog_items/1000 | -2.04% |
| serialization/catalog_items/10000 | -0.83% |
| serialization/sensor_micro | -0.79% |

Ordinary catalog parsing is roughly flat. The tiny sensor read is about 33 ns
slower; writer source changes do not guarantee unchanged whole-binary read timing.

## Initial allocation diagnostics (`6965d33`)

A counting allocator surrounds one warmed write, using the exact branch-table
fixture. Sparse writes and the 1/16-branch large cases match baseline allocations.
The 64-branch/1,000-item case adds one allocation and 3,216 requested bytes;
256 branches add one allocation and 12,816 requested bytes. Output bytes and
buffer reallocations match. Keys borrow existing strings. Requested bytes are
cumulative allocator requests, not peak or retained memory. The table is dropped
after each container write.

## Crossover correction

The initial 64-item trigger (`6965d33`) was too eager for large tables. A short
32/64/128-item screen found 256 branches with 64 items 30.45% slower, despite
being 16.56% faster at 128 items and 53.74% faster at 1,000. With 64 branches,
64 and 128 items improved 14.76% and 22.64%. The final trigger additionally
requires at least half as many items as branches. This avoids building a large
index for the observed short-document loss. The heuristic is workload dependent;
text-only content and repeatedly using an early branch can save less lookup work.
The initial source's full gate and all measurements remain archived as provenance.

## Distribution-aware selection

A size-only trigger still slowed 1,000 repeated first-tag writes by 11.20%
(64 branches) and 19.88% (256). Sample up to 16 items across the payload before
indexing: require four tagged samples and average linear search depth of 16.
Text samples and low-cost early-tag reuse retain linear lookup. Normal validation
still processes every item. Offsets inside the sample bins avoid repeatedly
sampling one tag when a choice cycle divides the stride; 1,024-item controls
check that common power-of-two case. Sampling is bounded and deterministic,
with no allocation. It can miss other distributions, so this remains a heuristic.

## Final selector (`58cb346`)

Three process pairs, 100 samples, 3-second warmup, 5-second measurement.

| 1,000-item writer | Baseline | Candidate | Time change | Process-pair changes |
| --- | ---: | ---: | ---: | --- |
| branches_1 | 64.841 us | 61.384 us | -5.33% | -6.57%, -3.85%, -7.98% |
| branches_16 | 70.082 us | 65.587 us | -6.41% | -6.09%, -6.41%, -12.35% |
| branches_256 | 176.588 us | 79.247 us | -55.12% | -54.79%, -54.88%, -57.18% |
| branches_64 | 106.933 us | 73.844 us | -30.94% | -29.58%, -30.94%, -31.45% |

The 64/256-branch cases improve in every pair. The source retains linear search
for small tables, so their improvement belongs to the complete writer change,
not the index alone. Final concentrated first-tag screens improve 3.67% and
2.44% with 64/256 branches, respectively. Power-of-two distributed screens
improve 26.64% and 54.44%. Text-heavy content with 261 possible branches is
roughly flat (-1.73%). These controls use the shorter two-pair settings.

### Final common-content comparison

Same long-run settings as the final branch-table comparison.

| 1,000-item writer | Time change | Process-pair changes |
| --- | ---: | --- |
| enum | -4.47% | -3.42%, -6.94%, -3.58% |
| nested | -2.01% | -2.71%, -1.41%, -1.69% |
| pattern | -5.40% | -5.86%, -5.40%, -5.54% |
| scalar | -1.85% | -12.49%, -1.85%, +0.89% |
| text | -5.11% | -5.31%, -3.96%, -5.04% |

These are complete writer-binary results, with process variation retained above.
No hash table is used for these five-branch fixtures. Earlier prototypes did
show small regressions; keep their evidence instead of replacing the history
with only this favorable final run. Tiny sparse writes still differ by a few
nanoseconds. Read controls remain roughly flat, not a claimed read optimization.

### Final plain-record screen

Two pairs, 50 samples, 1-second warmup, 2-second measurement.

| Consumer | Time change |
| --- | ---: |
| deserialization/catalog_items/1000 | -0.53% |
| deserialization/catalog_items/10000 | +1.05% |
| deserialization/sensor_micro | +0.58% |
| serialization/catalog_items/1000 | -1.01% |
| serialization/catalog_items/10000 | -1.76% |
| serialization/sensor_micro | -3.75% |

The final allocation rerun matches the initial counts and table-size costs;
sampling adds no allocation. These are single-host latency measurements.
They do not establish concurrent throughput or a compiled-schema memory budget.

## Verification and evidence

The complete final quality gate passes: 324 Rust tests, strict workspace formatting
and Clippy, Ruff, and 112 Python tests with 100% statement and branch coverage.
The worktree-owned target is `target/followup-mixed-index-verification` in the
primary checkout. Compiler/toolchain consumer tests run as part of the full gate.

[Raw evidence](data/runtime-followups-2026-10-03/mixed-writer/) includes every
Criterion sample and per-process confidence interval, matching locks/harnesses,
allocation diagnostics, source patches for each prototype, instruction profiles,
both final/initial gates and the rejected preflights. SHA256SUMS covers all files.
The final selector is `58cb346`; historical sections identify their older source.
All heavy work ran serially under the memory cap with one Cargo worker.
