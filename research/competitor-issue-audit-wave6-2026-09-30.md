# Competitor issue audit, sixth pass — 2026-09-30

Five more public issue reports were investigated against PolyXML checkout
`ab637e88495969988a5ac98ce2d05d01f1d47b66` (CLI 0.33.0). The saved
minimal schemas are in [`fixtures/wave6/`](fixtures/wave6/). An independent
`lxml.etree.XMLSchema` validator accepted all five schemas and each instance
called valid below; it rejected the instance called invalid. The public
reports supplied test ideas. These results describe PolyXML, not the current
status of every competitor release.

| Public report | PolyXML result | Existing overlap and limit |
| --- | --- | --- |
| [XSCG #378: duplicate element name in exclusive choice branches](https://github.com/mganss/XmlSchemaClassGenerator/issues/378), `duplicate_choice_branch_name.xsd` | **Affected.** Both `<Person><MaxAge>65</MaxAge></Person>` and `<Person><MinAge>18</MinAge><MaxAge>65</MaxAge></Person>` are valid. Generated Go has two fields with the same XML tag, so `encoding/xml` rejects either with a field-tag conflict. Generated Python's required `min_age` and duplicate `max_age_2` fields prevent the MaxAge-only branch from parsing. Filed [PolyXML #122](https://github.com/polyxml/PolyXML/issues/122). | Related #101 covers a sequence nested inside a choice; this fixture isolates the shared XML name across exclusive branches. |
| [lumeo #155: two choice groups clash](https://github.com/lumeohq/xsd-parser-rs/issues/155), `two_choice_groups.xsd` | **Affected in generated Python semantics.** PolyXML emitted four independent optional branch fields; it accepted and wrote `BStar, BTerm, MeanMotionDot, Agom`, while independent XSD validation rejected that first choice selecting both branches. A legal `BStar, MeanMotionDot, Agom` instance parsed; its serializer used the type-name root, already #104. Filed [PolyXML #123](https://github.com/polyxml/PolyXML/issues/123). | The upstream Rust report is a generated-name collision, not proof its current release accepts invalid XML. #123 is a distinct two-choice-group regression for PolyXML's compositor flattening. |
| [Bergmann #275: abstract base and derived `xsi:type`](https://github.com/Bergmann89/xsd-parser/issues/275), `abstract_derived_xsi.xsd` | **Go affected; Python dispatch covered in parsing.** Independent XSD validation accepted `<document xsi:type="ConcreteDocument"><info>hello</info></document>` with the namespace declaration. Generated Python selected `ConcreteDocument(info='hello')`; its serializer wrote `<ConcreteDocument>`, a root-name gap tracked in #104. Generated Go decoded the same input without error into empty `AbstractDocument` and marshaled `<document></document>`, silently losing `info` and `xsi:type`. Filed [PolyXML #124](https://github.com/polyxml/PolyXML/issues/124). | Closed #53 explicitly tracked polymorphic dispatch but did not establish generated Go behavior. This is a regression audit of the closed feature. |
| [lumeo #141: Rust keyword in namespace prefix](https://github.com/lumeohq/xsd-parser-rs/issues/141), `rust_keyword_namespace/{root,types}.xsd` | **Covered in tested Rust path.** Imported `mod:MedicationOrderDetails` became a valid `MedicationOrderDetails` type reference. A temporary Rust consumer crate built and parsed `<Order><Details><Code xmlns="urn:audit:mod">A</Code></Details></Order>`, yielding `A`. | Only one imported namespace/type and one XML parse were exercised. The test does not establish all namespace collision behavior or a valid serialized global root. No new issue. |
| [XSCG #563: XLink attribute reflection on industry schema](https://github.com/mganss/XmlSchemaClassGenerator/issues/563), `xlink_shared_attribute/{root,xlink}.xsd` | **C# and Go affected; Python covered in parsing.** Two different types refer to the same global `xl:type` attribute. C# `XmlSerializer(typeof(Links))` throws `Invalid name character in 'xl:type'`; Go `xml.Unmarshal` returns success but leaves both attributes nil; Python parsed resource/arc values. Filed [PolyXML #125](https://github.com/polyxml/PolyXML/issues/125). | This fixture does not reproduce the upstream MISMO schema's distinct-attribute-type collision. It isolates a more basic QName mapping failure. #113 is for Go *elements*, not attributes. |

## Commands and scope

```sh
./target/debug/polyxml generate \
  research/fixtures/wave6/duplicate_choice_branch_name.xsd \
  --lang python --lang go --out /tmp/polyxml-wave6-duplicate_choice_branch_name
./target/debug/polyxml generate \
  research/fixtures/wave6/two_choice_groups.xsd \
  --lang python --out /tmp/polyxml-wave6-two_choice_groups
./target/debug/polyxml generate \
  research/fixtures/wave6/abstract_derived_xsi.xsd \
  --lang python --lang go --out /tmp/polyxml-wave6-abstract_derived_xsi
./target/debug/polyxml generate \
  research/fixtures/wave6/rust_keyword_namespace/root.xsd \
  --lang rust --out /tmp/polyxml-wave6-rust_keyword_namespace
./target/debug/polyxml generate \
  research/fixtures/wave6/xlink_shared_attribute/root.xsd \
  --lang python --lang go --lang csharp --out /tmp/polyxml-wave6-xlink_shared_attribute
```

Go findings came from generated-code `go test` programs using `encoding/xml`;
the C# finding came from a .NET 8 executable constructing `XmlSerializer`;
the Rust result came from a consumer crate using the generated source. Those
temporary harnesses are not checked in, but issue #122, #124, and #125 record
the inputs and observed calls. No other generated target was runtime-tested
for these five fixtures.
