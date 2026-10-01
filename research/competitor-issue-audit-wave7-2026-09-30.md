# Competitor issue audit, seventh pass — 2026-09-30

Ten additional public reports were checked against PolyXML checkout
`ab637e88495969988a5ac98ce2d05d01f1d47b66` (CLI 0.33.0).
Minimal XSDs are in [`fixtures/wave7/`](fixtures/wave7/). The XSDs and
specified legal XML instances passed `lxml.etree.XMLSchema`. Conclusions are
limited to the exact tested target, schema, and API. Closed upstream reports
are included as regression leads, not claims about a current competitor bug.

| # | Public report | PolyXML finding and evidence | Limit / follow-up |
| --- | --- | --- | --- |
| 1 | [XSCG #567: root in different namespace from its type](https://github.com/mganss/XmlSchemaClassGenerator/issues/567), `cross_namespace_root/{a,shared}.xsd` | **Covered in tested C# path.** `ElementA` has `[XmlRoot("ElementA", Namespace="urn:audit:A")]` while `BaseType` is in `urn:audit:shared`. `XmlSerializer(typeof(ElementA))` parsed `<a:ElementA><s:Code>A</s:Code></a:ElementA>`; serialized output passed independent XSD validation. | The upstream issue also requests an `XmlInclude` emission switch. PolyXML did not generate `XmlInclude` in this small fixture; no option parity claim is made. |
| 2 | [XSCG #214: imported array item namespace](https://github.com/mganss/XmlSchemaClassGenerator/issues/214), `array_namespace/{root,array}.xsd` | **Covered in tested C# path.** A `Container/Values` child typed with imported `ArrayOfString` parsed two namespace-qualified `a:string` items; the serialized output passed independent XSD validation. | This tests the named imported array type and two items, not the report's exact Microsoft namespace or nillable-item behavior. |
| 3 | [xsdata #1179: preserve named XSD groups as separate classes](https://github.com/tefra/xsdata/issues/1179), `named_group.xsd` | **Wire behavior covered; representation request unmet.** PolyXML expanded `DocumentNameGroup` into `ComponentType` fields and generated no group class. Generated Python parsed Vendor, Name, Version correctly. | Named `xs:group` is a schema composition mechanism; flattening its fields can preserve wire behavior. Separate generated group classes are an optional model API design, not a demonstrated correctness bug. Existing group expansion tests and closed #51 overlap. |
| 4 | [XSCG #571: reuse shared types across profile modules](https://github.com/mganss/XmlSchemaClassGenerator/issues/571), `multi_common/` | **Covered in tested module workflow.** A `polyxml.toml` with common, order, and invoice modules plus `depends_on=["common"]` emitted one `SharedType` declaration in each language's common module. Generated Go packages compiled. Generated C# common/order/invoice sources compiled together, and `XmlSerializer` parsed both profile roots with their correct Id values. | This small fixture does not reproduce the attached V2G corpus or run Microsoft's separate `sgen` build step. |
| 5 | [XSCG #497: avoid regenerating common schemas](https://github.com/mganss/XmlSchemaClassGenerator/issues/497), `multi_common/` | **Covered for tested Python/Go/C# modules.** The manifest built the shared schema once; Python order/invoice import the common class, Go aliases the common package type, and C# uses `Generated.Common.SharedType`. Python imports, Go compilation, and C# runtime checks passed. | Running each input independently still emits its imported type; use the manifest dependency workflow for deduplication. No claim about all targets or arbitrary project graphs. |
| 6 | [xsdata #1212: XXE through lxml event parser](https://github.com/tefra/xsdata/issues/1212), `xxe_text.xsd` | **Controlled external-file probe did not reproduce disclosure; separate data corruption found.** Generated Python parsed `&audit;` declared as a local `file:///tmp/...` sentinel but returned `"audit"`, not the file contents. It also returned `"audit"` for an internal entity declared as `EXPECTED_VALUE` and for an undeclared reference. Predefined `&amp;` worked. Filed [PolyXML #126](https://github.com/polyxml/PolyXML/issues/126). | This is a controlled single-file test, not a comprehensive XXE review of all parser entry points or network schemes. #126 concerns silent entity-name substitution, not a confirmed security disclosure. |
| 7 | [JAXB tools #596: catalog DTD download](https://github.com/highsource/jaxb-tools/issues/596) | **No direct PolyXML catalog-parser counterpart.** A source/API search found no XML Catalog input or resolver option in PolyXML CLI/schema parser; the report concerns JAXB's catalog file, not an ordinary XSD import. | No catalog document was fed to PolyXML, and this row makes no assertion about remote `schemaLocation` handling or all external entity behavior. No issue filed for a feature the product does not expose. |
| 8 | [xsdata #1031: write huge trees without retaining them](https://github.com/tefra/xsdata/issues/1031) | **Partial capability; enhancement filed.** Generated Rust has `encode_xml<W: Write>(&self, writer, ...)`, so an existing model can write to a sink. Repeated data still resides in model collections, and generated `to_xml()` plus core `XmlSerializer::serialize()` return complete byte vectors. There is no just-in-time repeated-item producer API. Filed [PolyXML #127](https://github.com/polyxml/PolyXML/issues/127). | No memory benchmark was run. This is an incremental-production feature request, not a demonstrated regression in ordinary serialization or a claim that event writing is absent. |
| 9 | [xgen #106: same local type names and duplicate multi-file output](https://github.com/xuri/xgen/issues/106), `same_local_type_names/{root,a,b}.xsd` | **Same-name collision covered in tested targets.** Distinct QNames `a:SharedType` and `b:SharedType` became `SharedType` and `SharedType2` in Python/Go/C#. Go output compiled; Python parsed the legal `<Root><First><A>x</A></First><Second><B>y</B></Second></Root>` into the correct two classes/values. The separate module manifest produced no duplicated generated paths in its small graph. | The upstream folder's nested duplicate-file layout was not recreated exactly; no current xgen comparison was run. |
| 10 | [XSCG #554: remove text-only wrapper classes](https://github.com/mganss/XmlSchemaClassGenerator/issues/554), `simple_text.xsd` | **Correctness covered; representation preference differs.** PolyXML generated a `TaxableAmount` value wrapper in C#/Python/Go and Python parsed `<Amount>12.50</Amount>` as `TaxableAmount(value='12.50')`. | The public report asks for a scalar API in place of a wrapper. Removing it could lose named type identity or future facets/attributes; no malformed XML or failed runtime was observed, so no bug issue was filed. |

## New issues filed during this pass

- [#126: general entity references become their names](https://github.com/polyxml/PolyXML/issues/126). The test found no external sentinel disclosure, but declared internal and undeclared entities silently produced incorrect string data.
- [#127: incremental producer-side XML writing](https://github.com/polyxml/PolyXML/issues/127). This is an enhancement with a proposed bounded-memory benchmark, distinct from sink streaming of a complete model.

## Reproduction notes

The legal XML for the namespace and module cases is visible in each XSD's
declared names and in the detailed issue/audit notes. For C# checks, .NET 8
`XmlSerializer` parsed inputs and the two namespace cases had their serialized
outputs revalidated by `lxml`. The module fixture contains the exact
`polyxml.toml`; run `polyxml build --config
research/fixtures/wave7/multi_common/polyxml.toml`. Its output is configured
under `/tmp/polyxml-wave7-modules`. The security probe used only a harmless
file containing `POLYXML_AUDIT_SENTINEL_7F29`; it did not read a system file.

This pass did not rerun current competitor releases. Each linked public report
is a research lead; the local result and its limit appear in the same row.
