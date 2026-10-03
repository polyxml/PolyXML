---
name: polyxml-phf-compile-benchmark
description: Reproduce and report capped generated Rust match versus phf consumer build time and section sizes for issue 55.
---

# Generated Rust dispatch compile study

1. Read `benchmarks/rust-phf-compile/README.md` and the compile-cost section
   of `docs/benchmarks/rust-phf-dispatch.md`. Build the current CLI through
   `scripts/memcap.sh` with `CARGO_BUILD_JOBS=1`.
2. Check Linux MemAvailable and, on WSL, Windows physical/free RAM. WSL may
   expose only 16 GiB on a 32-GiB machine. Select a deliberate cap that leaves
   substantial available memory outside the build. The runner requires
   systemd user scopes, forces swap off and ignores inherited bypass flags.
   Never disable the cap to finish a benchmark.
3. Run the default numeric-field 16-element smoke before the 600/1500 tiers.
   Use the fixture generator's `make_tags(count, SEED + count)` without changing
   its vocabulary or seed. Numeric and string workloads are separate: required
   string fields caused expensive release compilation even at 600 elements. Keep
   cancelled/failed string attempts separate from completed numeric results.
4. Run `POLYXML_MEMCAP_PCT=70 python3 benchmarks/rust-phf-compile/run.py
   --output <fresh-results-directory>` only with comparable headroom to the
   October 2026 host (about 13.7 GiB available). Otherwise choose a lower cap.
   The default is 40%. Do not run other heavy builds alongside this study.
5. Each variant must use a fresh Cargo target for its cold build. Generated
   targets are under `benchmarks/rust-phf-compile/target/<field-type>/<tier>/<strategy>/target`; remove only a generated target when repeating. Use
   `--tiers` and `--strategies` to select a subset. Preserve prior logs and
   choose a fresh output directory rather than overwriting samples.
6. Retain exact dependency locks, all build logs and `results.json`. Fetching
   and resolution are excluded from timing; cold means fresh Cargo artifacts,
   not flushed filesystem caches. Default release settings apply. Three
   generated-file-only rebuilds expose variation. Builds can take many minutes
   at 1500 fields; record completed times rather than extrapolating them.
7. Every consumer must read XML at runtime and assert an exact round trip of
   all fields. Pass `Some("Record")` to `encode_xml` because a generated root
   type alias otherwise serializes under the underlying type name.
8. On a capped failure, retain the scope's journal and matching kernel OOM
   evidence. GNU time may be killed before writing its peak RSS. Distinguish
   a deliberate cancellation from a cgroup OOM, and neither from a successful
   compile-time sample. Keep the host reserve when adjusting a cap deliberately.
9. Publish build times and byte counts for `.text`, `.rodata`, `.data`, `.bss`
   and `.data.rel.ro`. Pointer-bearing phf entries can live in relocation-backed
   read-only storage, so `.rodata` alone undercounts table storage. Link each
   table to raw evidence, explain field types and reachable codecs, and avoid
   comparing directly with historical small-schema consumers/toolchains.
10. Revisit the lookup-only recommendation using the measured compile cost;
    build time and size cannot establish a decoder-throughput crossover or an
    automatic threshold. Run Ruff, formatting/diff checks and the strict docs
    build before completing the report.
