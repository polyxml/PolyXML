# Competitor issue audit, fifth pass — 2026-09-30

This pass investigated **21 additional public issue reports** against PolyXML
checkout `ab637e88495969988a5ac98ce2d05d01f1d47b66` (CLI 0.33.0).
Reports are leads, not evidence that a current competitor build still fails.
The statuses below concern the exact saved fixture and target named. A
compile-only result does not establish a working XML codec.

## Method and versions

- Saved minimal XSDs are in [`fixtures/wave5/`](fixtures/wave5/). `lxml.etree.XMLSchema`
  accepted the XSD 1.0 schemas except the two intentionally invalid negative
  tests. `xmlschema.XMLSchema11` accepted the XSD 1.1 alternative schema and
  independently checked its legal and illegal XML instances.
- PolyXML used `./target/debug/polyxml validate` and `generate`; generated
  target code was compiled and exercised where the result below says so.
- Same-schema competitor probes used xgen revision
  `v0.0.0-20260902022537-fb39bae424fe` (its CLI prints 0.1.0),
  XmlSchemaClassGenerator 3.0.1405.0, and xsdata 26.2 where stated. Tools
  were installed under `/tmp/polyxml-audit-tools` and generated output under
  `/tmp/polyxml-wave5-*`; those temporary outputs are not retained in Git.
- A report about an optional setting, runtime client, or separate plugin is
  marked outside PolyXML's feature surface. It does not imply either product
  is correct for the broader schema problem.

## Individual investigations

| # | Public issue and saved fixture | PolyXML result | Current competitor or scope limit |
| --- | --- | --- | --- |
| 1 | [xgen #59: enum and choice interaction](https://github.com/xuri/xgen/issues/59), `enum_choice_interaction.xsd` | **Covered in tested TypeScript output.** The named size enum has only Small/Medium/Large; the inline style enum has Value1/Value2; the text branch remains a string. `tsc --noEmit` passed. | Current xgen's size enum was contaminated with Value1/Value2 and modeled the no-elements branch as string. This is a narrow same-snapshot generation advantage. |
| 2 | [xgen #27: punctuation and leading-digit enum values](https://github.com/xuri/xgen/issues/27), `enum_punctuation.xsd` | **TypeScript covered; Go compile gap.** TS symbols FooBar, V2g, and UrnIsoStd retain their exact lexical values and `tsc` passed. Go's enum-only file has an unused `encoding/xml` import: [PolyXML #121](https://github.com/polyxml/PolyXML/issues/121). | Current xgen emitted invalid TS member names `FOO.BAR`, `2G`, and `urn:iso:std`; `tsc` reported parse errors. |
| 3 | [xgen #42: unbounded choice with six refs](https://github.com/xuri/xgen/issues/42), `choice_six_refs.xsd` | **Covered for tested Go order.** Independent XSD validation accepted `<e d="2"><a1 n="x"/><a6 n="z"/><a2 n="y"/></e>`. PolyXML parsed three ordered items and serialized A1, A6, A2. | Current xgen now emits A6 and parses it, so the historic missing-branch report is resolved in this shape. Its serializer grouped A1, A2, A6, changing wire order. |
| 4 | [xgen #19: simple-content extension](https://github.com/xuri/xgen/issues/19), `simple_content_extension.xsd` | **Covered in tested Go path.** Generated `Amount` has character-data `Value` and `Currency` attribute. `<Root currency="USD">12.50</Root>` round-tripped. | Only extension of built-in `xs:string` was tested; the report's cross-namespace variant remains unverified. |
| 5 | [xgen #44: forward-declared type](https://github.com/xuri/xgen/issues/44), `forward_type.xsd` | **Covered in tested Go compile.** A field referring to a later `CodeType` retained that named type and compiled. | Current xgen also generated compiling Go, but flattened the field to `string`. Neither probe checked runtime facet enforcement. |
| 6 | [xgen #10: unbounded `xs:integer`](https://github.com/xuri/xgen/issues/10), `unbounded_integer.xsd` | **Affected.** A schema-valid 40-digit integer is generated as Go `int64`, C# `long`, and Rust `i64`. Go and C# deserialization rejected it as out of range. [PolyXML #117](https://github.com/polyxml/PolyXML/issues/117). | Current xgen Go uses `int`, also insufficient. No claim about all seven PolyXML runtimes. |
| 7 | [xgen #85: unknown type fallback](https://github.com/xuri/xgen/issues/85), `unknown_type.xsd` | **Affected, stronger CLI failure.** The unresolved `MissingType` XSD is invalid, yet `polyxml validate` prints valid and generation succeeds; Go emits `type Root = MissingType` and fails compile. [PolyXML #116](https://github.com/polyxml/PolyXML/issues/116). | The upstream report motivates resolution checks; its exact fallback mechanism differs. |
| 8 | [XSCG #552: hyphenated fixed attribute](https://github.com/mganss/XmlSchemaClassGenerator/issues/552), `hyphen_fixed_attribute.xsd` | **Name covered; fixed-value enforcement already tracked.** C# emits `[XmlAttribute("ID-CLASS")]` and serializes NAME when explicitly set. It also accepts an invalid WRONG, covered by [PolyXML #102](https://github.com/polyxml/PolyXML/issues/102). | Current XSCG omits the attribute set to its fixed default via `[DefaultValue("NAME")]`. Omission of an optional fixed attribute is schema-valid; this is not a validation failure on its own. |
| 9 | [XSCG #273: only first collection item](https://github.com/mganss/XmlSchemaClassGenerator/issues/273), `repeat_two.xsd` | **Covered in tested small Python and Go paths.** Both parsed Item A and Item B. | The reported WFS/Filter nesting was not recreated; this probe checks only an ordinary repeated child. |
| 10 | [XSCG #399: date/dateTime union serialization](https://github.com/mganss/XmlSchemaClassGenerator/issues/399), `date_union.xsd` | **Affected in Go and C#.** Go generation succeeds, but generated code fails to compile at `time.Time(value)` from a string. C# parses a zoned dateTime, then serializes `03/15/2023 12:30:00 +00:00`, which is not the XSD lexical form; date-only round-tripped. [PolyXML #118](https://github.com/polyxml/PolyXML/issues/118). | Current XSCG generated a string field and round-tripped this zoned input unchanged on the exact fixture. |
| 11 | [XSCG #550: `UseShouldSerializePattern` and `Specified`](https://github.com/mganss/XmlSchemaClassGenerator/issues/550) | **Setting-specific; no matching PolyXML switch.** The report is about interaction between two XSCG C# emission options, so it cannot be reproduced by toggling an equivalent PolyXML option. | This is a feature-surface boundary, not evidence of C# optional-field correctness; [#103](https://github.com/polyxml/PolyXML/issues/103) covers a concrete default-value issue. |
| 12 | [XSCG #511: XSD 1.1 `xs:alternative`](https://github.com/mganss/XmlSchemaClassGenerator/issues/511), `alternative_xsd11.xsd` | **Affected.** `XMLSchema11` accepts two conditional branch instances and rejects a mismatched branch. PolyXML reports success but emits Go `Limit = any`, Python `Limit = object`, and empty C# `Limit`, losing conditional types. [PolyXML #119](https://github.com/polyxml/PolyXML/issues/119). | XSD 1.1 support must be implemented or explicitly rejected; no claim about XSCG's current output. |
| 13 | [XSCG #387: invalid-schema error location](https://github.com/mganss/XmlSchemaClassGenerator/issues/387), `invalid_location.xsd` | **Affected.** An `xs:element` directly inside `xs:simpleType` is illegal per independent validator; PolyXML `validate` and `generate` both report success. [PolyXML #116](https://github.com/polyxml/PolyXML/issues/116) combines this with unresolved QName validation and requests useful locations. | Upstream issue focuses on diagnostics; PolyXML currently misses the rejection itself. |
| 14 | [xsdata #1131: custom `XmlDateTime` subclass in a choice](https://github.com/tefra/xsdata/issues/1131) | **Runtime customization is outside tested PolyXML API.** The report requires a user-defined xsdata scalar subclass and `XmlSerializer` choice dispatch; PolyXML has no matching generated Python subclass hook in this audit. | General temporal lexical support is still open in [#109](https://github.com/polyxml/PolyXML/issues/109) and union handling in [#118](https://github.com/polyxml/PolyXML/issues/118); those are separate from this extension hook. |
| 15 | [xsdata #1118: nested `Meta` inheritance warning](https://github.com/tefra/xsdata/issues/1118), `meta_inheritance.xsd` | **Affected in generated Python.** `DerivedType(BaseType)` declares an independent nested `Meta`; Pyright reports `reportIncompatibleVariableOverride` at generated line 50. [PolyXML #120](https://github.com/polyxml/PolyXML/issues/120). | The checker also reported missing PolyXML package imports in the temporary environment, but the override diagnostic is independent. Runtime parse/serialize was not checked. |
| 16 | [xsdata #1135: SOAP 1.2 client transport](https://github.com/tefra/xsdata/issues/1135) | **Not applicable to schema codegen.** PolyXML's compiler emits data models and XML codecs, not a SOAP transport client matching xsdata's client API. | A SOAP message schema could be assessed separately for binding, without implying SOAP transport support. |
| 17 | [Bergmann xsd-parser #256: unnecessary `xmlns:xsi`](https://github.com/Bergmann89/xsd-parser/issues/256), `repeat_two.xsd` | **Covered in tested Rust serialization.** A temporary generated-code consumer compiled and serialized one item as `<RootType><Item>A</Item></RootType>` with no unused `xmlns:xsi`. | Root tag is the type name; the separate root-binding defect remains [#104](https://github.com/polyxml/PolyXML/issues/104). Only a simple non-nil case was tested. |
| 18 | [lumeo xsd-parser-rs #154: case-colliding enum variants](https://github.com/lumeohq/xsd-parser-rs/issues/154), `enum_case_collision.xsd` | **Python/TS naming covered; Go compile gap.** YES/yes and NO/no receive distinct generated symbols, and TypeScript compiles. Go's enum-only unused import is [#121](https://github.com/polyxml/PolyXML/issues/121). | This is a naming probe for PolyXML, not a same-version run of lumeo's Rust generator. |
| 19 | [lumeo xsd-parser-rs #132: nested repeated sequence](https://github.com/lumeohq/xsd-parser-rs/issues/132), `nested_sequence_repeated.xsd` | **Covered in the tested one-child group.** Generated Go/Python model OrderHandle as a list; Go parsed two values and wrote them back in order. | A repeated group with multiple sibling children remains [PolyXML #106](https://github.com/polyxml/PolyXML/issues/106); group minOccurs enforcement was not checked here. |
| 20 | [JAXB tools #601: HyperJAXB timezone mapping](https://github.com/highsource/jaxb-tools/issues/601) | **Plugin-specific; no direct PolyXML counterpart.** This concerns HyperJAXB persistence mapping, not ordinary XML dateTime model generation. | PolyXML XML date/time handling has separately confirmed issues [#109](https://github.com/polyxml/PolyXML/issues/109) and [#118](https://github.com/polyxml/PolyXML/issues/118). |
| 21 | [JAXB tools #189: nested same-name element/class](https://github.com/highsource/jaxb-tools/issues/189), `nested_same_name.xsd` | **Covered in tested Python path.** Generated distinct `ResponseType` and `ResponseTypeResponseType`; nested `<Response><Response><Code>X</Code></Response></Response>` parsed correctly. Go output compiled. | Closed [PolyXML #51](https://github.com/polyxml/PolyXML/issues/51) addressed anonymous name collisions; this is regression evidence, not a new issue. |

## New PolyXML issues filed during this pass

The six confirmed, distinct failures were filed while triaging, with legal
and illegal fixtures, observed target behavior, upstream lead, and acceptance
checks in each issue:

| PolyXML issue | Scope |
| --- | --- |
| [#116](https://github.com/polyxml/PolyXML/issues/116) | Invalid XSDs falsely pass validation and can emit uncompilable code. |
| [#117](https://github.com/polyxml/PolyXML/issues/117) | `xs:integer` values outside 64-bit range fail in generated Go/C#. |
| [#118](https://github.com/polyxml/PolyXML/issues/118) | `xs:date`/`xs:dateTime` union output fails Go compile and C# lexical round trip. |
| [#119](https://github.com/polyxml/PolyXML/issues/119) | Valid XSD 1.1 conditional type alternatives silently degrade to untyped models. |
| [#120](https://github.com/polyxml/PolyXML/issues/120) | Python derived `Meta` class triggers a Pyright override error. |
| [#121](https://github.com/polyxml/PolyXML/issues/121) | Enum-only Go output imports `encoding/xml` without use and fails compilation. |

The earlier PolyXML issues linked in the rows remain the appropriate homes
for overlapping failures. No new issue was filed for a covered small case,
an untested broader variant, or a competitor-specific option/plugin/client.

## Reproduction commands

```sh
for xsd in research/fixtures/wave5/*.xsd; do
  ./target/debug/polyxml validate "$xsd"
  ./target/debug/polyxml generate "$xsd" \
    --lang python --lang go --lang csharp --out "/tmp/$(basename "$xsd" .xsd)"
done
```

`unknown_type.xsd` and `invalid_location.xsd` are intentionally invalid;
their current zero exit status is the failure under test. `alternative_xsd11.xsd`
requires an XSD 1.1 validator; an XSD 1.0-only validator rejecting it says
nothing about the fixture's validity. To make a migration claim beyond these
rows, retain a representative XML instance and run both current generators on
the same pinned schema and target runtime.
