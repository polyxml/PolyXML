# XSD attribute decoding and generated enum literals

The runtime investigation exposed an XSD enum declared as `R&amp;D` that rejected an XML value of `R&D`. The schema parser copied raw attribute text into the IR. This also affected facets, default/fixed values, namespace declarations, and QName validation. The UPA pass already normalized attributes, so schema passes could disagree.

Branch `fix/xsd-attribute-values` normalizes attributes consistently with quick-xml’s XML 1.0 normalization API and propagates errors. Built-in and numeric references are decoded once, literal attribute whitespace is normalized, and referenced tabs/newlines/carriage returns survive. Duplicate enum values are removed after decoding. Malformed attributes and undefined references are rejected even inside otherwise skipped annotations. This does not add a custom DTD entity resolver.

Decoding quotes exposed another problem: several generators emitted the resulting value directly inside source string literals. The [saved intermediate Rust output and syntax failure](data/runtime-followups-2026-10-03/entities/decoded-but-unescaped-rust/rust-syntax.log) show that failure. Generated enum constants, metadata attributes, conversion tables and parse matches now use target-appropriate escaping. Rust uses its own string syntax; C++ uses fixed-width octal for ASCII controls; the other targets use JSON-compatible escapes with explicit Unicode line-terminator escaping for C#.

The fixtures are `research/fixtures/schema_attribute_entities.xsd` and `research/fixtures/enum_literal_escaping.xsd`. [An independent lxml validator](data/runtime-followups-2026-10-03/entities/independent-validator.txt) accepted the schema and all four valid entity fixture instances. Core tests check enum/facet/default/fixed values, namespace/QName consistency, normalization, negative constraints and errors. The seven-language test compiles and executes generated consumers, compares values with independent data files, and verifies Rust Serde serialization/deserialization. It includes quotes, backslashes that resemble Unicode escapes, Unicode, tabs, newlines, carriage returns, and Unicode line separators.

[The final quality gate](data/runtime-followups-2026-10-03/entities/quality-gate.txt) passed 326 Rust tests, strict workspace Clippy, Rust formatting, and 112 Python tests with 100% statement and branch coverage. [All seven enum consumers passed](data/runtime-followups-2026-10-03/entities/seven-language-enums.txt). [Toolchain versions](data/runtime-followups-2026-10-03/entities/toolchains.json) include Rust 1.99.0, Python 3.14.4, GCC 15.2, Go 1.26, Java 25, TypeScript 5.9 and .NET 10. The C# consumer targets .NET 8 and uses .NET 10 roll-forward; this does not verify execution on the minimum .NET 8 runtime.

Compiler and generated enum implementation revision: `dc38fde`. Heavy checks used one Cargo worker and the systemd memory cap with swap disabled. Main remains untouched. 

The [isolated companion comparison](data/runtime-followups-2026-10-03/entities/corpus-comparison.json) has identical summaries and failure tables for baseline and candidate:

| Suite | Groups / schema expectations passed | Instance round trips passed |
| --- | --- | --- |
| CType, limit 50 | 31 / 31 | 23 / 28 |
| AttrDecl, limit 50 | 50 / 50 | 50 / 50 |
| NISTXMLSchemaDatatypes, limit 100 | 100 / 100 | 447 / 452 |

These are selected suites, not full W3C conformance. The six failing groups predate this fix. [Baseline logs](data/runtime-followups-2026-10-03/entities/baseline/CType.txt), [candidate logs](data/runtime-followups-2026-10-03/entities/candidate/CType.txt), and the adjacent AttrDecl/NIST files retain every reported result. [Artifact hashes and discriminating CLI/native probes](data/runtime-followups-2026-10-03/entities/source-artifacts.json) establish the tested sources. The baseline native probe rejects `R&D`; the candidate accepts it and produces correctly decoded defaults/fixed values. [The seven-target CLI smoke passed](data/runtime-followups-2026-10-03/entities/seven-target-smoke.txt) using the isolated candidate executable.

An earlier corpus preflight incorrectly reused another worktree’s top-level CLI in a shared debug target directory. Its source probe caught the mismatch. [That rejected attempt](data/runtime-followups-2026-10-03/entities/rejected-shared-target-preflight/README.txt) is retained and excluded from the comparison above. Rebuilding each revision in its own target directory resolved the problem; the workflow skill now records this requirement.
