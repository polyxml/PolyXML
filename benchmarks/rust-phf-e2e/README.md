# End-to-end Rust `phf` decoder comparison

Run `./benchmarks/rust-phf-e2e/run.sh` from the repository root after
`cargo build -p polyxml-cli`. The script generates identical 16- and 120-field
schemas with the default `match` and opt-in `--feature phf`, then decodes the
same XML document through the generated `from_xml` methods. It checks the last
field before measuring and reports five repetitions in ns/document and MB/s.
Set `POLYXML_PHF_FIRST=1` for a second run with the strategy order reversed;
compare both runs before interpreting a small difference.

These are small and medium schemas only. The 600/1500-element compile-time and
binary-size study remains deferred in issue #55, and no automatic `phf`
threshold should be selected from this limited range. The runner does not
measure allocation or hardware counters. Run on an idle host, retain raw output
with the Git revision and toolchain, and inspect run-to-run spread before
claiming a speedup.
