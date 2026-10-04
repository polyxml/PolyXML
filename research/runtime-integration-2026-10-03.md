# XML runtime integration — October 3, 2026

The combined correctness/writer branch passes the full quality gate: **334 Rust
tests, 115 Python tests, 100% statement/branch wrapper coverage**, strict workspace
formatting and Clippy, and Ruff. It preserves the large mixed-table speedup while
ordinary catalog timings remain within roughly 1% of the selected runtime baseline.
Main remains unchanged.

Measured/tested integration source: `71aff6ca6cbe7106dcc2389a2428571a16d69430`.
Baseline: `1819c91` from [draft PR #137](https://github.com/polyxml/PolyXML/pull/137).
This branch combines [#138](https://github.com/polyxml/PolyXML/pull/138) (XSD
attribute entities/enum literals), [#140](https://github.com/polyxml/PolyXML/pull/140)
(default/fixed generated literals), [#141](https://github.com/polyxml/PolyXML/pull/141)
(namespace URI entities), and [#142](https://github.com/polyxml/PolyXML/pull/142)
(mixed nil round trips and sampled branch lookup). The registry prototype in
[#139](https://github.com/polyxml/PolyXML/pull/139) and the attribute-guard prototype
remain separate experiments because their workload tradeoffs need review.

## Long plain-record comparison

Three alternating process pairs, 100 samples, 3-second warmup, 5-second measurement.
Matched Criterion locks/harness/toolchain; all fixtures validate complete round trips
before timing. Negative time changes mean faster.

| Consumer | Baseline | Integration | Time change | Process-pair changes |
| --- | ---: | ---: | ---: | --- |
| deserialization/catalog_items/1000 | 793.776 us | 791.797 us | -0.25% | -0.11%, -0.31%, -0.78% |
| deserialization/catalog_items/10000 | 8000.110 us | 7909.081 us | -1.14% | -1.14%, +0.11%, -2.00% |
| deserialization/sensor_micro | 1.012 us | 1.015 us | +0.30% | +0.59%, +0.69%, +0.30% |
| serialization/catalog_items/1000 | 177.532 us | 178.628 us | +0.62% | +0.06%, +1.59%, +0.81% |
| serialization/catalog_items/10000 | 1796.153 us | 1808.383 us | +0.68% | +0.63%, +0.64%, +1.56% |
| serialization/sensor_micro | 0.227 us | 0.230 us | +1.60% | +1.60%, +1.05%, +1.96% |

These compare with the already-optimized investigation branch, not a release tag.
The small writer slowdowns are retained; unchanged source paths do not guarantee
identical whole-binary timing. The sensor writer difference is roughly 4 ns.

## Mixed table integration screen

Two process pairs, 50 samples, 1-second warmup, 2-second measurement, 1,000 items.
This is an integration check; the individual writer report retains its longer
three-pair comparison and concentrated, text-heavy, cutoff and power-of-two controls.

| Possible tags | Time change | Process-pair changes |
| --- | ---: | --- |
| branches_1 | -5.13% | -4.00%, -6.27% |
| branches_16 | -5.74% | -5.66%, -5.83% |
| branches_256 | -54.39% | -53.95%, -54.84% |
| branches_64 | -29.17% | -28.26%, -30.06% |

The combined branch retains the large-table improvement in both pairs. Sampling
and temporary indexing are performance heuristics, with no persistent metadata
cache. Scalar/nested mixed nils preserve order and correct namespace bindings.
Existing mixed metadata omits nillability constraints; this does not add full
schema validation or claim full namespace/W3C conformance.

## Installed artifacts and bounded corpus

Fresh CLI/native probes pass for generated default/fixed values, schema attribute
entities, escaped namespace URIs, and a mixed nil XML → JSON → XML round trip.
[The probe](data/runtime-followups-2026-10-03/integration/probe.json) records both
artifact hashes and the imported module path. Worktree-owned build targets and
matching companion bindings avoid stale CLI/native aliases.

Bounded W3C results have byte-identical summary/failure tables to the default-literal
candidate: CType 31/31 schemas and 23/28 instances; AttrDecl 50/50 schemas and
50/50 instances; NIST 100/100 schemas and 447/452 instances. The six existing
failing groups remain. These selected suites do not establish full conformance.

## Evidence and limits

[Evidence](data/runtime-followups-2026-10-03/integration/) includes matched locks,
harnesses, all Criterion samples, confidence intervals, the full gate, corpus logs,
artifact probes and commands. Criterion JSON directories are retained in verified
archives with original member checksums; extract into scratch beside metadata.json
to rerun summarize.py. Top-level SHA256SUMS verifies the delivered files.

Heavy work ran serially with one Cargo worker and the systemd memory cap. No OOM
occurred. Results describe this WSL2 host; requested allocator bytes and latency
measurements are not peak RAM or concurrent throughput. Later commits package
evidence and documentation without changing the measured core sources.
