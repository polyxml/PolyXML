# Namespace URI decoding — October 3, 2026

Source: `ad9282e`, compared with runtime investigation baseline `1819c91`.

Namespace declarations previously retained escaped XML source text. The URI
`urn:A&amp;B` therefore failed to match the schema URI `urn:A&B`, affecting
strict root matching and xsi:type dispatch. Decode declaration values once,
propagate errors, and preserve inherited bindings that are already decoded.

Four regressions cover default and prefixed roots, full round trips, local and
inherited xsi:type bindings, streamed Start/Empty records, double decoding and
undefined entities. The original three positive regressions fail on the baseline.
This is a namespace URI decoding fix, not a claim of full namespace conformance.

Builds and tests use a dedicated workspace target with one Cargo worker and the
repository systemd memory cap. See the retained evidence for exact commands and
results. No compiler or generated-model behavior changes are included.

The complete quality gate passes: 325 Rust tests, strict workspace formatting
and Clippy, Ruff, and 112 Python tests with 100% statement and branch coverage.
Raw baseline failures, focused checks and the full gate are retained in
[the evidence directory](data/runtime-followups-2026-10-03/namespace-entities/).
