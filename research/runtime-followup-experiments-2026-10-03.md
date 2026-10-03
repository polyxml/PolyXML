# Runtime follow-up experiments — October 3, 2026

These isolated experiments compare against `1819c91` from the runtime investigation
in draft PR #137. They do not change main. All timing consumers use matched source,
Criterion locks and toolchain, and alternate baseline/candidate process order.
Heavy runs are serial under the repository memory cap with one Cargo worker.
Raw samples, per-process confidence intervals, source patches, revision metadata
and focused-test logs are retained in [the evidence directory](data/runtime-followups-2026-10-03/performance-experiments/).

## Empty variant registry (`42efd1f`)

A shared atomic populated flag avoids the registry lock when no variants exist.
The setter updates the flag while holding the write lock; schemas and their
clones share it. Tests cover set/clear and concurrent registry access.

Three process pairs, 100 samples, 3-second warmup and 5-second measurement:

| Consumer | Candidate time change |
| --- | ---: |
| Catalog read, 1,000 items | +0.25% |
| Catalog read, 10,000 items | +0.00% |
| Sensor read | +0.49% |
| Catalog write, 1,000 items | -0.90% |
| Catalog write, 10,000 items | -1.02% |
| Sensor write | -4.98% (about 11 ns) |
| 1,000-item empty-registry read | -1.90% |
| 1,000-item registered-variant read | +1.56% |
| 1,000-item empty-registry write | -6.81% |
| 1,000-item registered-variant write | +0.77% |

Negative means faster. Empty-registry writes improved in every process pair;
registered reads slowed in all three. Registered writes were noisy (-1.69% to
+6.93% per pair). Ordinary catalog reads did not improve. This is a workload
tradeoff, not a broad recovery of the historical parsing regression. Keep it
experimental pending review of the additional synchronization and populated path.

The registry branch passes the complete quality gate: 323 Rust tests, strict
workspace formatting and Clippy, Ruff, and 112 Python tests with 100% statement
and branch coverage. An initial gate stopped because the local tsc directory
was absent from PATH; the complete rerun with the prepared toolchain passed.
Both logs are retained.

Allocation counts and requested bytes match the baseline for the three plain
payload sizes. Simulated Callgrind instruction counts change by about -0.12%
on reads and -0.55% on writes; they are not hardware cycle measurements.

## Borrow parent schema (`3017a88`)

Replace the per-child Arc clone with a borrow from the parent frame; end the
borrow before mutating the stack. Mixed kinds retain one string allocation.
Wildcard, mixed, nil, depth-limit, fixed-value and xsi:type focused tests pass.
Two screening process pairs, 50 samples, 1-second warmup, 2-second measurement:
1,000-item reads +1.65%; 10,000-item reads -0.74%; sensor reads +1.53%.
There is no consistent measured improvement. Do not promote this as a speedup.

## Compact mixed parent state (`149140b`)

Store a nonzero parent branch index instead of Option<String> and resolve the
kind from the parent schema when its child ends. The frame is 16 bytes smaller
on this host. A nested mixed regression verifies differing parent branch tables,
text order, complete round trips and truncated XML errors. Focused tests pass.
Screen settings match the borrowing experiment: 1,000-item reads +0.29%;
10,000-item reads +2.63%; sensor reads +1.78%. Smaller state did not establish
faster parsing. Retain the patch as a rejected performance prototype.

## Fixed-value guard (`6cbd21f`)

Checking whether a fixed value exists before looking up a parsed value produced
-0.90%, +0.55% and +0.19% on the three read payloads in a short screen.
This was neutral and was not selected.

## Limits

These are single-host WSL2 results, not concurrent throughput or peak-memory
measurements. Short screens reject weak hypotheses; they are not publishable
claims of small improvements. Full quality gates are required before promoting
an implementation; focused prototype tests alone are insufficient. Main remains
unchanged. Existing PR #137 still contains the selected serializer, scalar-state
and bounded pattern-cache improvements.
