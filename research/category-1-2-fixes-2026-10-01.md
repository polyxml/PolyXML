# Category 1 and 2 implementation follow-up — 2026-10-01

Historical first-batch report. The expanded backlog scope and final acceptance
results are in [the backlog closeout](backlog-closeout-2026-10-01.md).

Scope: the six compiler/type-checker issues and fourteen codec/data-loss
issues in the pasted competitor audit. Category 3 validation/specification
work and the category 4 streaming enhancement are excluded.

The starting checkout was `38ca082`. It already contained the category 1
commit `5eb93a4` and category 2 commits for #128, #114, #124, #113, and #101.
This continuation adds the remaining nine category 2 fixes and strengthens
the nested-sequence-in-choice runtime checks. Existing local formatting
changes were retained. The remaining fixes are recorded together because they
share schema compilation changes and cross-issue runtime regression tests.

| Issue | Implementation/evidence |
| --- | --- |
| #129, #121, #112, #118, #120, #130 | Carried forward from `5eb93a4`; existing Rust/Go/C#/Python codegen and CLI regression suites exercised. No new full official UBL corpus build is claimed. |
| #128 | Existing wildcard-attribute model and codec work retained. |
| #114 | Existing Python/Go wildcard-element work retained. |
| #124 | Existing Go abstract-root dispatch retained; root wrappers preserve `Selected()` and delegate codecs. Generated Python root models also retain sibling `xsi:type` derivations and their payload. |
| #113 | Existing Go expanded-QName reference matching retained. |
| #101 | Bounded nested sequence branches bind sibling wire elements instead of adding an artificial sequence wrapper element. Python, Go, and C# execute both the paired sequence and alternative-only document. |
| #106 | `SchemaIR.ordered_types` selects tagged ordered item streams for repeated sequences, preserving A1/B1/A2/B2 wire order without changing the source schema's mixed-content flag. |
| #109 | Go temporal wrappers accept the tested XSD date, time, and timezone-free dateTime forms; retain timezone presence/absence and fractional seconds; serialize edits to the embedded `time.Time` instead of stale input text. |
| #110 | Go lexical unions implement text codecs usable by XML attributes. C# lexical proxy properties emit `XmlAttribute`. Runtime checks cover integer, empty, and invalid attribute values. |
| #111 | Go global-root wrappers check the expanded QName and marshal with the declared root name, delegating underlying codecs. |
| #104 | Python global elements emit callable model classes with strict document QName checks. Colliding root/type names use `Element` suffixes; distinct namespace roots remain separately accessible. Scalar root generation includes required imports. |
| #105 | Referenced substitution heads expand transitively to concrete members in an ordered item stream. Bond/Equity/Bond survives parse and serialization in Python, Go, and C#. |
| #107 | C# nillable repeated elements use nullable item types and `IsNullable=true`; A/null/B survives both record and class round trips. |
| #115 | Bounded choices with repeated element branches retain list cardinality and bind repeated sibling elements. |
| #122 | Identically typed optional occurrences of the same expanded element name share one binding. Both MinAge/MaxAge and MaxAge-only documents round-trip; C# omits absent optional elements. |

Regression entry points:

- `crates/polyxml-core/tests/test_go_codegen.rs`: root/union attributes,
  temporal scalar lexicals and edits, ordered particles and choice branches.
- `crates/polyxml-core/tests/test_csharp_codegen.rs`: nullable enum items,
  lexical attributes, and particle wire structure in records and classes.
- `crates/polyxml-python/tests/test_generated_models.py`: dataclass/Pydantic
  root binding, imported namespace collisions, abstract root payloads, and
  particle structure through two parse/write cycles.

Validation includes workspace Rust tests, codegen/CLI suites, strict Clippy,
formatting, seven-ecosystem smoke builds, Python lint and 100% statement/branch
coverage. Fourteen generated Python round trips were also independently
validated against their XSDs with lxml.

The companion W3C MGroup/MGroupDef selection contains 59 groups. Both the
starting compiler and this continuation compile 59/59 schemas and pass 29/32
instance round trips. A fresh-process comparison was used while isolating
an imported-root name collision; the remaining failures also occur before
these changes. This is a targeted regression check, not full W3C conformance.

Generated API changes are intentional: use global element models for document
root binding; repeated sequences and substitutions expose tagged ordered
items; Go temporal values embed `time.Time` in lexical-preserving wrappers.
When more than one abstract root can bind a derived Python instance, choose
the declared root explicitly with `polyxml.serialize(..., target_type=Root)`.

Limits: the tests establish the saved reproductions, not every possible
particle tree or target backend. Full choice exclusivity, group-bound
validation, arbitrary-size integers, and other category 3 gaps remain outside
scope. Go temporal lexical parsing still uses Go's calendar/parser range.
