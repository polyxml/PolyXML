# End-to-end Rust `phf` decoder comparison

Run `./benchmarks/rust-phf-e2e/run.sh` from the repository root after
`cargo build -p polyxml-cli`. The script generates identical 16- and 120-field
schemas with the default `match` and opt-in `--feature phf`, then decodes the
same XML document through the generated `from_xml` methods. It checks the last
field before measuring and reports five repetitions in ns/document and MB/s.
Set `POLYXML_PHF_FIRST=1` for a second run with the strategy order reversed;
compare both runs before interpreting a small difference.

These are small and medium schemas only. The separate [600/1500-element numeric-consumer compile-time and
binary-size study](../rust-phf-compile/README.md) addresses issue #55. It does
not measure decoder throughput; no automatic `phf` threshold should be selected
from this limited end-to-end range. The runner does not
measure allocation or hardware counters. Run on an idle host, retain raw output
with the Git revision and toolchain, and inspect run-to-run spread before
claiming a speedup.
