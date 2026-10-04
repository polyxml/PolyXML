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
- Run heavy work serially, with one Cargo worker and the repository memory cap.
  Short comparisons screen hypotheses. Repeat retained improvements with longer
  measurements, alternating revision order and preserving every raw sample.
- Anchor Criterion filters: `^serialization/` means writes only; `serialization`
  also matches deserialization. Remove stale results only in the runner's owned
  scratch directory before measurement, not from retained raw evidence.
- Check rich enums, patterns, lists and mixed branches alongside plain records.
  The `mixed.rs` consumer checks text/scalar/enum/pattern/nested ordered items.
  Text/GeneralRef/CData boundaries may coalesce when written: compare adjacent
  text's concatenated value, rather than treating token segmentation as XML
  semantics. Keep nil correctness tests separate from baseline timing consumers
  when the baseline cannot serialize nil.
  Warm pattern-cache gains do not establish cold-start, churn or concurrent
  throughput. Entry/key bounds do not constitute a compiled-regex byte budget.
- Preserve validation, errors, split Text/CData/GeneralRef handling, nil reads,
  nesting and metadata mutation semantics. Borrow scalar metadata from the schema
  owned by a frame instead of cloning rich scalar definitions per element.
- Include the 17-pattern cycle when changing eviction: a 16-entry FIFO has
  systematic misses, while salted victim selection preserves reuse at the same
  capacity. Confirm hot cases too; pressure improvements alone do not establish
  universal throughput gains.
- Publish positive and negative experiments with exact source revisions, raw
  evidence and remaining regressions. Run the full quality gate before pushing
  the experiment branch; a branch request does not authorize a merge to main.

For mixed schemas with many possible child tags, measure the linear branch scan
with `mixed_branches.rs`, including sparse one-item controls. A temporary index
can borrow kind keys and branch references for large repeated payloads without
persisting stale mutable metadata. Preserve the original first-match behavior
for duplicate kind names; test metadata edits, tagged records, nil and unknown
kinds. Keep small tables on a linear path and measure the chosen crossover.

Check the indexing cutoff with short documents too. A 256-branch table built for
64 mixed items was 30% slower, despite a 54% gain at 1,000 items. Requiring at
least 64 items and half as many items as branches avoids that observed crossover
regression. Retain 32/64/128-item controls, and keep the cutoff heuristic separate
from semantic behavior: duplicate first-match ordering and metadata edits still
need independent correctness tests.

Also measure concentrated tag reuse and text-heavy content. Even above the
size cutoff, repeated first-branch hits were 11–20% slower with a hash table.
A bounded, evenly spaced sample can select linear lookup when the observed
branch-search work is low. Treat sampling as a performance heuristic only;
normal writer validation must still examine every item and preserve errors.
