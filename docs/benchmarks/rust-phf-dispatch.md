# Rust Tag Dispatch: `match` vs `HashMap` vs `phf`

> Status: lookup benchmark results below are produced on a Linux x86-64 host with a
> functional PMU (`perf_event_paranoid = 2`). Commands are reproducible with
> stock tooling; see [Reproducing](#reproducing).

## What changed

Rust codegen gained an opt-in **`--feature phf`** flag that swaps the generated
child-element dispatch from a linear string `match` to a compile-time perfect
hash table built with `phf_codegen`:

```bash
polyxml generate schema.xsd --lang rust --feature phf
# or in polyxml.toml:
#   [[generate]]
#   target = "rust"
#   features = ["phf"]
```

For every struct with element children the generator emits:

```rust
/// Perfect-hash element dispatch id for `Address` (`--feature phf`).
#[derive(Clone, Copy)]
enum __AddressElementId { Street, City, ... }

static __ADDRESS_ELEMENT_DISPATCH: ::phf::Map<&'static str, __AddressElementId> = ::phf::Map {
    key: ..., disps: &[...], entries: &[("city", __AddressElementId::City), ...],
};
```

and both `Event::Start` and `Event::Empty` arm dispatches become
`__ADDRESS_ELEMENT_DISPATCH.get(e.local_name().as_ref())` lookups. Union-typed
fields map every branch tag onto one variant. Default output (no feature) is
byte-identical to before — locked by `test_rust_phf_dispatch_emission`.

Consuming crates must add `phf = "0.14"` to their `Cargo.toml`.

## Methodology

* **Framework**: Criterion.rs (`benches/tag_dispatch.rs`).
* **Scale tiers**: small **16**, medium **120**, large **600**, and
  extra-large **1500** elements, so the `match`→`phf` crossover is
  *measured* directly rather than extrapolated. Tags are seeded
  two-word pseudo-ISO-20022 vocabulary
  (`scripts/gen_tag_dispatch_fixtures.py`, 10-45 byte names) so LLVM's
  length-bucketed memcmp chains are neither favored nor gamed.
* **Baselines**:
  1. `match tag { "..." => ... }` — LLVM lowers these to length-bucketed
     vectorized/memcmp chains, not jump tables (byte-array keys can't jump-table).
  2. Runtime `HashMap<&str, u32>` (SipHash-1-3, std default).
  3. Compile-time `phf::Map<&str, u32>`.
* **Hit and miss paths** are measured separately per strategy
  (`*_hit` = full-tier sweep, throughput reported in tags/s;
  `*_miss` = lookup of an unknown sentinel tag).
* **Tokenization is intentionally excluded** to isolate lookup cost. A
  small/medium generated-decoder check appears below; larger decoder tiers
  remain open in #57. The lookup-only crossover is not a proven
  decoder-throughput crossover. Throughput below reports tags/s from the
  measured lookup sweeps.

## Generated decoder check (September 2026)

The [end-to-end runner](https://github.com/polyxml/PolyXML/tree/main/benchmarks/rust-phf-e2e)
generated `match` and `phf` decoders from identical **16- and 120-field**
schemas. Each decoder parsed the same complete XML document through its
generated `from_xml` method, including tokenization, dispatch, and field
decoding. It verified the last field before timing. After 1,000 warmup calls,
each strategy ran five repetitions of 100,000 documents (16 fields) or 10,000
documents (120 fields). We also reversed strategy order in a second process.

Host: AMD Ryzen 5 4500, Ubuntu 26.04.1 on WSL2, rustc 1.98.1. Times below
are median microseconds per document; lower is faster.

| Fields | XML bytes | Match first: `match` / `phf` | Phf first: `match` / `phf` |
| :--- | ---: | ---: | ---: |
| 16 | 497 | 2.639 / 3.095 µs | 2.474 / 2.962 µs |
| 120 | 3,617 | 21.095 / 22.689 µs | 20.241 / 22.317 µs |

The default `match` decoder remained faster at both sizes and in both orders.
These dense all-fields documents do not establish a crossover for larger,
sparser, or unknown-tag workloads. No automatic threshold is justified by
this experiment; `phf` remains explicit opt-in. The 600/1500-element
[numeric-consumer compile-time and binary-size results](#large-tier-numeric-consumers-october-2026)
are now available; they do not measure decoder throughput. Allocation and
hardware counters were not captured for this end-to-end run; see the
[match-first raw output](data/2026-09-26/rust-phf-match-first.txt) and
[phf-first raw output](data/2026-09-26/rust-phf-phf-first.txt).

## Results — throughput (Criterion)

Host: AMD Ryzen 5 4500, Linux x86-64 (WSL2), rustc 1.98.1, Criterion 0.8.
All times are per full-tier sweep unless noted; throughput is tags/second as
reported by `Throughput::Elements`.

### Hit path (lookup of a tag present in the tier)

| Tier | `match` hit | hit thrpt | `HashMap` hit | hit thrpt | `phf` hit | hit thrpt |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| small (16) | **52.1 ns** | 307.1 M tags/s | 275.0 ns | 58.2 M tags/s | 440.3 ns | 36.3 M tags/s |
| medium (120) | **658.3 ns** | 182.3 M tags/s | 2.063 µs | 58.2 M tags/s | 3.232 µs | 37.1 M tags/s |
| large (600) | 11.141 µs | 53.9 M tags/s | **10.613 µs** | 56.5 M tags/s | 15.843 µs | 37.9 M tags/s |
| large (1500) | 75.590 µs | 19.8 M tags/s | **27.044 µs** | 55.5 M tags/s | 40.352 µs | 37.2 M tags/s |

### Miss path (lookup of an unknown sentinel tag)

| Tier | `match` miss | `HashMap` miss | `phf` miss |
| :--- | ---: | ---: | ---: |
| small (16) | **5.36 ns** | 18.67 ns | 21.40 ns |
| medium (120) | **7.07 ns** | 17.69 ns | 22.02 ns |
| large (600) | 26.77 ns | **17.14 ns** | 21.06 ns |
| large (1500) | 46.19 ns | **17.64 ns** | 21.39 ns |

Interpretation:

* **Small (16) & medium (120)**: LLVM's length-bucketed `memcmp` chain wins
  decisively — `match` is 5.3× faster than `HashMap` and 8.4× faster than
  `phf` at 16 tags (52 ns vs 440 ns per sweep). On this toolchain, `phf`
  trails at the measured 16- and 120-tag tiers because it pays a fixed
  ~27 ns/tag (SipHash-1-3
  + displacement probe) while small `memcmp` chains are a handful of
  predictable vectorized compares.
* **Large (600)**: the crossover happens between tiers. `HashMap` edges
  ahead of `match` (10.6 µs vs 11.1 µs) while `phf` (15.8 µs) is still
  behind both — the linear `memcmp` chain cost has grown super-linearly
  with the number of distinct string lengths, but not yet past `phf`'s
  constant per-tag cost.
* **Large (1500)**: `match` collapses to 19.8 M tags/s (75.6 µs — 3.8×
  slower than at 600 tags scaled, the classic i-cache/branch-predictor
  blow-up of hundreds of `memcmp` arms). `phf` (40.4 µs) finally beats it
  by 1.87× while staying perfectly flat at ~37 M tags/s across **all**
  tiers. `HashMap` stays fastest on hits (27.0 µs) — it hashes once and
  probes a SIMD-friendly control-byte table — but it is a *runtime* build,
  i.e. only a baseline, not a shippable static-dispatch option.
* **Misses**: `match` degrades linearly with tier size (5.4 ns → 46.2 ns,
  since a miss walks the whole chain) while both hash structures are flat
  (`HashMap` ~17–19 ns, `phf` ~21–22 ns constant). XML deserialization
  only dispatches tags that exist in the schema, so the miss path matters
  mainly for schema-validation workloads.

## Results — hardware counters (`perf stat`)

The canonical command (requires the `perf` binary):

```bash
perf stat -e cycles,instructions,branches,branch-misses,L1-icache-load-misses \
  ./target/release/deps/tag_dispatch-<hash> --bench '<filter>'
```

This host lacks the `perf` binary, so counters are read through the same PMU
via `perf_event_open` using the committed harness (same events, same
semantics, userspace-only as required by `perf_event_paranoid = 2`):

```bash
scripts/perf_stat.sh -- ./target/release/deps/tag_dispatch-<hash> --bench '<filter>'
```

Counters are for a **full sweep of the tier** (warm process, summed over
Criterion's iterations by the harness — compare rows within a tier, not
across tiers). IPC = instructions / cycles; miss% = branch-misses / branches.

| Tier | Strategy | cycles | instructions | branches | branch-misses | L1I misses | IPC | miss% |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 16 | match | 36.0 B | 93.2 B | 20.9 B | 13.0 M | 74.9 K | 2.587 | 0.062% |
| 16 | hashmap | 41.8 B | 126.4 B | 11.5 B | 12.9 M | 109 K | 3.024 | 0.112% |
| 16 | phf | 37.6 B | 94.7 B | 7.8 B | 13.0 M | 103 K | 2.519 | 0.168% |
| 120 | match | 44.8 B | 124.1 B | 21.2 B | 17.1 M | 332 K | 2.770 | 0.081% |
| 120 | hashmap | 40.5 B | 122.6 B | 11.1 B | 13.1 M | 124 K | 3.030 | 0.118% |
| 120 | phf | 36.4 B | 92.3 B | 7.5 B | 14.8 M | 106 K | 2.539 | 0.196% |
| 600 | match | 46.3 B | 121.7 B | 16.6 B | 87.1 M | 12.5 M | 2.630 | 0.526% |
| 600 | hashmap | 45.4 B | 135.0 B | 12.2 B | 13.8 M | 123 K | 2.974 | 0.113% |
| 600 | phf | 39.7 B | 102.5 B | 8.3 B | 13.3 M | 106 K | 2.582 | 0.160% |
| 1500 | match | 44.1 B | 97.1 B | 12.0 B | 205.1 M | 92.1 M | 2.203 | 1.704% |
| 1500 | hashmap | 37.4 B | 107.0 B | 9.7 B | 24.9 M | 134 K | 2.860 | 0.257% |
| 1500 | phf | 44.3 B | 111.2 B | 9.0 B | 35.0 M | 130 K | 2.508 | 0.388% |

Interpretation:

* **Branch misses and instruction-cache misses grow for `match` at scale.**
  From 120 → 1500 tags, `match`'s branch-miss rate grows
  21× (0.081% → 1.704%) and its L1 instruction-cache misses explode
  **277×** (332 K → 92.1 M) as hundreds of string-literal `memcmp` arms
  stop fitting in the 32 KiB I-cache and the predictor saturates. Both hash
  structures stay ~flat by comparison
  (`phf` L1I: 103 K → 130 K over the same range).
* **`phf` is the lowest-work dispatch of the three**: fewest branches at
  every tier (7.5–9.0 B vs 16–21 B for `match` at small/medium — one hash
  instead of a compare chain), and instruction counts *fall* as tiers grow
  because loop overhead amortizes. Its disadvantage is raw latency per
  lookup (SipHash + probe ≈ 27 ns/tag), not scalability — visible as the
  lowest-but-stable IPC (≈2.5, load-dependent probe stalls).
* **`HashMap` runs at the highest IPC (≈2.9–3.0)** thanks to hashbrown's
  branch-poor SIMD control-byte scan, but pays the most instructions at
  small tiers (126.4 B vs 93.2 B for `match` at 16 tags).
* At 1500 tags `match` executes **1.7 branch-misses per 100 branches and
  ~710× more I-cache misses than `phf`** (92.1 M vs 130 K) — the wall-clock
  collapse in the throughput table (19.8 M vs 37.2 M tags/s) is these
  counters, not extra work.

## Compilation time & binary size

**Small schema (completed).** Same standalone consumer crate (detached from
the workspace, `--release`), rebuilt after `touch`-ing only the generated
file; section sizes via `size`/`size -A` on the example binary:

| Variant | gen-rebuild time | first build incl. `phf` dep | `.text` | `.rodata` | `.data` | `.bss` |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: |
| `match` (default) | 2.51–2.54 s | — | 2 590 640 | 367 768 | 271 608 | 3 048 |
| `phf` | 2.53–2.55 s | 2.75 s | 2 591 728 | 367 912 | 271 928 | 1 640 |
| **Δ** | +0.02 s (noise) | +0.2 s one-time | **+1 088 (+0.04 %)** | **+144 (+0.04 %)** | +320 | −1 408 |

Interpretation (small schema):

* **`phf` dispatch is effectively free at build scale**: +0.04 % `.text`,
  +144 bytes `.rodata` (the perfect-hash tables live in read-only data),
  and a rebuild delta within run-to-run noise. The only real build cost is
  a one-time ~0.2 s to compile the `phf` dependency.
* This matches the runtime story: the table itself is tiny — what differs
  between the strategies is *how each lookup walks it*, not how much code
  or data gets emitted for that small historical consumer.

### Large-tier numeric consumers (October 2026)

The [standalone consumer runner](https://github.com/polyxml/PolyXML/tree/main/benchmarks/rust-phf-compile)
uses the same deterministic enterprise tags as the lookup fixture generator,
with one required `xs:unsignedInt` field per tag. Each executable reads its
XML input at runtime, decodes every field, writes XML with the explicit root
name, and asserts byte equality. Both codecs remain reachable. These numbers
apply to this numeric-field workload; historical small-schema results used a
different consumer and toolchain and are not directly comparable.

Host: Intel Core i5-11600K, Ubuntu 26.04.1 / WSL2, rustc 1.99.0 (LLVM 23.1.1).
[Windows has 32 GiB physical RAM](data/2026-10-02/rust-phf-compile-numeric/windows-memory.json); WSL sees 15.5 GiB, with about 13.7 GiB
initially available. Builds run sequentially with one Cargo worker under
`scripts/memcap.sh`, deliberately set to 70% available RAM (about 9.5 GiB),
with swap disabled. This leaves roughly 4 GiB available outside the cap.

Cold builds use a fresh Cargo target for each variant; downloads and
resolution happen before timing, and OS page caches are not flushed. The
three rebuilds touch only generated `src/record.rs`. Default Cargo release
settings apply. Wall times include scope setup; match runs before phf, so
small cold-time differences can reflect order and caches. Exact dependency
locks and every raw sample are retained with the
[run metadata](data/2026-10-02/rust-phf-compile-numeric/results.json),
[600 match log](data/2026-10-02/rust-phf-compile-numeric/600-match.log),
[600 phf log](data/2026-10-02/rust-phf-compile-numeric/600-phf.log),
[1500 match log](data/2026-10-02/rust-phf-compile-numeric/1500-match.log), and
[1500 phf log](data/2026-10-02/rust-phf-compile-numeric/1500-phf.log).

| Fields | Variant | Cold build | Generated-file rebuilds (3) | Max RSS across builds |
| :--- | :--- | ---: | ---: | ---: |
| 600 | match | 134.53 s | 69.79–70.50 s | 1,647 MiB |
| 600 | phf | 121.85 s | 57.12–57.66 s | 1,555 MiB |
| 1500 | match | 933.35 s | 850.93–873.39 s | 6,720 MiB |
| 1500 | phf | 814.00 s | 736.81–761.13 s | 6,815 MiB |

Section sizes below are bytes from `size -A`; ELF bytes is the on-disk file
size. Both are retained in the same
[raw results](data/2026-10-02/rust-phf-compile-numeric/results.json).
`.data.rel.ro` is included because pointer-bearing phf entries occupy
relocation-backed read-only storage in these position-independent binaries;
`.rodata` alone does not capture all table storage.

| Fields | Variant | ELF bytes | `.text` | `.rodata` | `.data.rel.ro` | `.rela.dyn` | `.data` | `.bss` |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 600 | match | 1,809,096 | 902,115 | 92,972 | 9,688 | 20,040 | 2,656 | 224 |
| 600 | phf | 1,788,208 | 861,123 | 84,916 | 24,256 | 34,608 | 2,656 | 224 |
| 1500 | match | 3,199,568 | 1,794,611 | 185,908 | 9,688 | 20,040 | 2,656 | 224 |
| 1500 | phf | 3,179,200 | 1,717,379 | 173,188 | 45,856 | 56,208 | 2,656 | 224 |

At 600 numeric fields, phf's median rebuild is about 18% shorter (57.49 s
versus 70.16 s) and its `.text` is 40,992 bytes smaller (4.5%). Combined
`.rodata` + `.data.rel.ro` grows by 6,512 bytes. At 1500 fields, the median
rebuild is about 12% shorter (748.77 s versus 851.51 s), `.text` shrinks by
77,232 bytes (4.3%), and combined read-only storage grows by 23,448 bytes
(12% of those sections, 1.3% of match's `.text`). `.data` and `.bss` are
unchanged at both tiers. Pointer-bearing entries also increase `.rela.dyn`
relocation metadata by 14,568 / 36,168 bytes. Including that metadata and ELF
headers/symbols, the complete files are 20,888 bytes smaller at 600 fields
(1.2%) and 20,368 bytes smaller at 1500 (0.6%). All four consumers passed the
complete XML round trip.

The additional read-only storage remains small in absolute terms relative to
the generated code, while compile time improves in this workload. The
historical small-schema size delta does not describe these larger consumers.
These results provide no build-cost reason to raise the lookup-only
recommendation, but they cannot establish an end-to-end decoding crossover or
justify automatic phf selection. Keep the recommendation conditional and
opt-in; field types and toolchain affect compile costs. Peak RSS across all
builds was about 6.7 GiB and WSL swap use remained zero.

### String-field attempts and limits

The initial required-string-field 600 match consumer exceeded a 5,597-MiB
cap (40% available RAM). The kernel recorded rustc anon-RSS of 5,704,504 KiB
at the **cgroup** OOM kill. No successful compile time or binary size came
from that attempt; see the
[metadata](data/2026-10-02/rust-phf-compile/results.json),
[build log](data/2026-10-02/rust-phf-compile/600-match.log),
[cgroup outcome](data/2026-10-02/rust-phf-compile/600-match-cgroup.txt), and
[kernel evidence](data/2026-10-02/rust-phf-compile/600-match-oom.txt).

A deliberate 70% retry passed that memory barrier with a cgroup peak of about
6.1 GiB. The generated string consumer spent over 20 minutes compiling, so
that attempt was deliberately cancelled before switching the study to numeric
fields. It was **not** OOM-killed. See the
[cancelled attempt](data/2026-10-02/rust-phf-compile-70pct/results.json),
[cgroup measurements](data/2026-10-02/rust-phf-compile-70pct/cancelled-string-cgroup.json),
and [scope log](data/2026-10-02/rust-phf-compile-70pct/cancelled-string-cgroup.txt).
WSL swap use remained zero. The runner can reproduce the string workload
with `--field-type string`; numeric results should not be generalized to
string-heavy schemas. Large string-field compile cost remains unmeasured.

## Lookup-only threshold recommendation

**Consider `--feature phf` for lookup-heavy schemas with ≥ ~900 element tags
on a struct; keep the default `match` below that.** This is based on the
dispatch microbenchmark, not an end-to-end XML decoding result.

Derivation (per-tag hit cost, linear interpolation between the measured
tiers): `match` costs 18.6 ns/tag at 600 tags and 50.4 ns/tag at 1500
(tags/s × tier size), while `phf` is flat at ≈26.9 ns/tag at every tier.
Solving `18.6 + (50.4−18.6)·(N−600)/900 = 26.9` gives the `match`→`phf`
hit crossover at **N ≈ 835**; we round up to **900** for measurement noise
and schema drift. Notes:

* This is a *recommendation*, not an auto-enable: the feature stays
  opt-in (`--feature phf` / `features = ["phf"]`).
* On this toolchain (rustc 1.98.1 / LLVM), the measured small and medium
  tiers favor `match` by a 5–8× margin. Re-run
  `cargo bench --bench tag_dispatch` on your toolchain before adopting a
  threshold.
* `phf`'s real win at ≥900 tags is **predictability**: flat ~37 M tags/s
  and flat counters from 16 → 1500 tags, whereas `match` degrades 3.8×
  (and its counters, catastrophically) with no upper bound in sight.
* Miss-heavy workloads (schema validation of unknown tags) cross over much
  earlier: `match` misses degrade linearly (5.4 → 46.2 ns) while `phf`
  misses are flat (~21 ns), intersecting around **≈380 tags**. If your
  workload dispatches unknown tags on a hot path, prefer `phf` from ~400.
* The runtime `HashMap` baseline beats `phf` on hits at every tier, but it
  cannot ship as static dispatch (it requires a runtime build of the table
  per deserializer instance); it is reported only as a reference point.

## Reproducing

Every command below can be wrapped in `scripts/memcap.sh` (default: cap the
run at 60% of available RAM in an isolated cgroup, swap off) so a large-tier
build or bench cannot freeze a small host — the benchmark entry points
(`benchmarks/run_all.sh`, `benchmarks/cli/benchmark.sh`,
`scripts/perf_stat.sh`) already re-exec through it. Set
`POLYXML_MEMCAP_DISABLE=1` to opt out.

```bash
# 1. Throughput (all tiers/strategies, ~3 minutes)
scripts/memcap.sh cargo bench -p polyxml --bench tag_dispatch

# 2. Hardware counters for one strategy/tier (perf_stat.sh caps itself;
#    pass --bench or Criterion runs test mode and you measure startup)
BIN=$(ls -t target/release/deps/tag_dispatch-* | grep -v '\.d$' | head -1)
scripts/perf_stat.sh -- "$BIN" --bench 'tag_dispatch/large_600/phf_hit'

# 3. Regenerate tag fixtures (deterministic seed)
python3 scripts/gen_tag_dispatch_fixtures.py

# 4. Sequential generated-consumer compile time / size comparison.
#    Requires a systemd user manager; forces cgroup caps, swap off, one worker.
#    Stops on failure and retains raw output. Use a fresh output directory.
#    Numeric tiers completed (#55). This 70% cap assumes about 13.7 GiB
#    available RAM; retain a substantial reserve and never retry uncapped.
POLYXML_MEMCAP_PCT=70 python3 benchmarks/rust-phf-compile/run.py --output /tmp/phf-compile-results
```
