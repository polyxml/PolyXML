# UBL 2.4 Invoice C# gate

Run `python3 scripts/verify_ubl_csharp.py` from the repository root. Requires
Python 3.12+, Rust, and the .NET 8 SDK/runtime. The CI Rust job runs this gate.

The script fetches the official OASIS Invoice entry schema and its 15 imports
into `target/ubl-2.4/xsd/`, preserving relative paths. `sha256.json` pins every
file and import closure; a changed or unlisted dependency fails the gate.
Use `--schema-dir PATH` to supply an existing pinned tree and `--binary PATH`
to use an already-built CLI. The official schema sources are fetched from
https://docs.oasis-open.org/ubl/os-UBL-2.4/xsd/ rather than vendored here.

Each run creates fresh nullable-enabled .NET 8 projects for the full schema
graph and Invoice root, both as records and mutable classes. It compiles each
model and round-trips `invoice.xml`, an authored test invoice containing
supplier/customer names, currency attributes, quantities, and line amounts.
The .NET XSD engine validates input and output independently of PolyXML; the
gate also checks leaf values, attributes, and the document QName. The input
was additionally validated with lxml against the same pinned schema tree.

Reduced regressions are in `crates/polyxml-core/tests/test_ubl_csharp.rs` and
`test_csharp_codegen.rs`: type/root name collisions across namespaces,
inherited simple-content values and attributes, self-closing restrictions,
and temporal XML text. C# identifier suffixes do not rename XML QNames.

This establishes the tested Invoice path and compilation of the imported
corpus. It does not claim every UBL document, optional branch, restriction
facet, or arbitrary XSD temporal year works in every target. Derived C#
properties reuse base declarations; independent schema validation remains
the authority for stricter restriction constraints.
