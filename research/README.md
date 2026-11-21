# PolyXML Research & Ecosystem Compatibility Audits

This directory holds PolyXML's public **W3C XSD / XML ecosystem compatibility audits**, **engineering investigations**, and **runtime studies**.

## Purpose & Transparency

Because PolyXML compiles W3C XML Schema 1.0/1.1 into seven distinct target languages (Rust, Python, Go, C#, Java 22+, C++20, and TypeScript), subtle real-world XSD edge cases matter far more than toy examples.

We publish these studies in full for three reasons:
1. **Cross-ecosystem accountability**: Every compatibility audit reproduces real public issues reported across established XML/XSD projects (`tefra/xsdata`, `Bergmann89/xsd-parser`, `mganss/XmlSchemaClassGenerator`, `sissaschool/xmlschema`, `dapper91/pydantic-xml`, `tafia/quick-xml`, `NaturalIntelligence/fast-xml-parser`, `eclipse-ee4j/jaxb-ri`, `FasterXML/jackson-dataformat-xml`, `highsource/jaxb-tools`, `xuri/xgen`, `lumeohq/xsd-parser-rs`, `media-io/yaserde`, and `hooklift/gowsdl`).
2. **Equal reporting of passes and gaps**: Reports document cases where PolyXML passes out of the box **and** cases where reproduction exposes a real bug or missing feature in PolyXML—filing actionable GitHub issues with minimal schemas rather than sweeping failures under the rug.
3. **Clean separation from production tests**: Minimal conformance schemas live in [`tests/fixtures/conformance/`](../tests/fixtures/conformance/) where automated Rust, Python, Go, C#, Java, C++, and TypeScript test suites run them directly; this `research/` directory contains documentation and historical verification artifacts only.

## Methodology & Modern Design Stance

- **Programmatic deduplication**: Each new wave extracts all previously audited upstream issue/PR URLs from `research/ecosystem-audits/*.md` and `polyxml/PolyXML` issues so every pass covers fresh reports.
- **Minimal reproducers first**: Every candidate bug is reduced to a standalone XSD under [`tests/fixtures/conformance/`](../tests/fixtures/conformance/) and exercised via `polyxml validate`, `polyxml generate`, and target compilers.
- **Modern-by-design rejections**: PolyXML targets modern language idioms (Python 3.12+ PEP 695 aliases & `StrEnum`, Rust zero-copy `quick-xml`, Java 22+ records & sealed interfaces, C++20 concepts & `std::variant`, TypeScript 5+ `as const` unions) and secure defaults (DTD & XXE disabled). Legacy compatibility shims (e.g., `xsd.exe` `Item` + `ItemElementName` arrays) are intentionally declined in favor of type-safe sum types.

---

## Directory Layout

```text
research/
├── README.md                  # This index & methodology guide
├── ecosystem-audits/          # Public XSD/XML ecosystem compatibility passes (Waves 1–9)
├── engineering-notes/         # Internal codebase audits, warning/corpus fixes, & closeouts
└── runtime-studies/           # Runtime deep dives + reproducible probe logs in data/
```

---

## Ecosystem Compatibility Audits (`ecosystem-audits/`)

| Wave | Date | CLI Baseline | Scope | Issues Filed | Report |
|---|---|---|---|---|---|
| Waves 1–4 | 2026-09-30 | `0.33.0` | Initial multi-pass audit + UBL 2.4 & eBay probes | `#101`–`#115` | [wave-01-04-2026-09-30.md](ecosystem-audits/wave-01-04-2026-09-30.md) |
| Wave 5 | 2026-09-30 | `0.33.0` | 21 additional public reports | `#116`–`#121` | [wave-05-2026-09-30.md](ecosystem-audits/wave-05-2026-09-30.md) |
| Wave 6 | 2026-09-30 | `0.33.0` | 5 targeted multi-choice, xsi, & XLink regressions | `#122`–`#125` | [wave-06-2026-09-30.md](ecosystem-audits/wave-06-2026-09-30.md) |
| Wave 7 | 2026-09-30 | `0.33.0` | 10 namespace, XXE, & incremental writer checks | `#126`–`#127` | [wave-07-2026-09-30.md](ecosystem-audits/wave-07-2026-09-30.md) |
| Wave 8 | 2026-09-30 | `0.33.0` | 50-item deep dive across 5 XSD codegen tools | `#128`–`#131` | [wave-08-2026-09-30.md](ecosystem-audits/wave-08-2026-09-30.md) |
| Wave 9 | 2026-11-20 | `0.40.0` | 50-item modern ecosystem deep dive across 13 repos | `#163`–`#169` | [wave-09-2026-11-20.md](ecosystem-audits/wave-09-2026-11-20.md) |

---

## Engineering Notes (`engineering-notes/`)

- [internal-codebase-audit-100-2026-09-30.md](engineering-notes/internal-codebase-audit-100-2026-09-30.md) — 100-item code-level audit across parser, IR, CLI, runtime, and codegen targets.
- [category-1-2-fixes-2026-10-01.md](engineering-notes/category-1-2-fixes-2026-10-01.md) — First implementation batch for compiler/type-checker and codec findings.
- [backlog-closeout-2026-10-01.md](engineering-notes/backlog-closeout-2026-10-01.md) — Full closeout of Wave 1–8 priority issues.
- [integer-and-upa-closeout-2026-10-01.md](engineering-notes/integer-and-upa-closeout-2026-10-01.md) — Follow-up verification for `#117` (unbounded integers) and `#131` (W3C UPA validation).
- [generated-warning-audit-2026-10-03.md](engineering-notes/generated-warning-audit-2026-10-03.md) — Audit of Rust, TypeScript, and C# compiler warnings in generated code.
- [corpus-csharp-import-fixes-2026-10-03.md](engineering-notes/corpus-csharp-import-fixes-2026-10-03.md) — NeTEx corpus C# record/class compilation and unused-import fixes.

---

## Runtime Studies (`runtime-studies/`)

Deep-dive investigations into the Rust/Python native XML/JSON engine and generated runtime semantics (with raw logs, toolchain snapshots, and SHA-256 manifests retained under [`runtime-studies/data/`](runtime-studies/data/)):

- [entities-2026-10-03.md](runtime-studies/entities-2026-10-03.md) — XML attribute normalization & escaped enum/facet literals.
- [default-literals-2026-10-03.md](runtime-studies/default-literals-2026-10-03.md) — Escaped `default` and `fixed` literal propagation across 7 targets.
- [namespace-entities-2026-10-03.md](runtime-studies/namespace-entities-2026-10-03.md) — Namespace URI entity decoding consistency.
- [simple-content-2026-10-03.md](runtime-studies/simple-content-2026-10-03.md) — Inherited `xs:simpleContent` resolution across IR & codegen.
- [mixed-writer-2026-10-03.md](runtime-studies/mixed-writer-2026-10-03.md) — Mixed-content XML writer semantics & performance.
- [followup-experiments-2026-10-03.md](runtime-studies/followup-experiments-2026-10-03.md) — Isolated Criterion & throughput benchmarks.
- [integration-2026-10-03.md](runtime-studies/integration-2026-10-03.md) — Integrated runtime quality gate & parity validation.
