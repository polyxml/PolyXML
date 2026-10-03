# Generated Rust dispatch compile cost

This standalone consumer study addresses issue #55. It uses the deterministic
enterprise tags from `scripts/gen_tag_dispatch_fixtures.py` (seed + tier),
one required `xs:unsignedInt` field per tag, and identical XML for match and phf.
Each executable reads XML at runtime, decodes every field, serializes it back,
and asserts byte equality. Both codecs remain reachable in the binary.

Build the CLI first, then run from the repository root:

```bash
POLYXML_MEMCAP_BACKEND=systemd POLYXML_MEMCAP_PCT=40 CARGO_BUILD_JOBS=1 \
  scripts/memcap.sh cargo build -p polyxml-cli
POLYXML_MEMCAP_PCT=70 python3 benchmarks/rust-phf-compile/run.py \
  --output /tmp/phf-compile-results
```

The runner **requires systemd user scopes** and forces MemoryMax with swap
disabled. It ignores inherited cap bypass flags, uses one Cargo worker, and
caps each build separately. The default is 40% of currently available RAM;
`POLYXML_MEMCAP_PCT` can deliberately adjust that after checking host headroom.
An over-limit build stops the study and
retains logs; never retry uncapped. Ensure sufficient headroom for other host
work before running. The initial 600-field string attempt exceeded the default
cap; the full numeric study uses 70% on a host with about 13.7 GiB available,
leaving roughly 4 GiB of available memory outside the build cap.
Use `POLYXML_MEMCAP_PCT=70` only with comparable headroom.
The 1500-field numeric cold builds peaked around 6.6 GiB RSS, above the default
40% cap on this host. Choose a fresh output directory, then retain published
results under a dated `docs/benchmarks/data/` directory.

The numeric workload compares dispatch strategies with scalar fields and avoids the
very slow release compilation encountered with 600 owned/borrowed string fields.
Use `--field-type string` to reproduce that separate workload. String attempts
are retained separately; a cancelled build is not a compile-time sample.
Compile cost and binary sizes depend on field types, codecs reached by the
consumer, and compiler version; do not generalize the numeric measurements to
all large schemas.

Generated crates and their separate target directories live under this
suite's ignored `target/`. A fresh target is required for each cold build;
remove only the desired generated crate's target before repeating. Use
`--tiers 16 --repeats 1 --output benchmarks/rust-phf-compile/target/smoke`
to check the harness before the full study.
Targets are separated by field type. `--strategies match` or `--strategies phf`
can repeat one variant without removing another variant's build output.

Each variant gets one cold release build (dependencies compiled into a fresh
target, with downloads and resolution excluded), followed by three rebuilds
after touching only `src/record.rs`. Wall time includes memory-scope setup;
GNU time logs also retain compiler-process-tree maximum RSS and build time.
Cold means a fresh Cargo target, **not** flushed OS page caches. Variants run
in match/phf order, so small cold-time differences can reflect cache/order.
Default Cargo release settings apply; no LTO or custom optimization flags.
Exact dependency locks, host/toolchain/revision metadata, raw logs, `size`
and `size -A` output plus on-disk ELF byte counts are retained beside
`results.json`.

This measures generated decoder/encoder consumer cost, rather than the
lookup-only benchmark. Historical small-schema results used a different
consumer and toolchain; compare strategies within this run.
