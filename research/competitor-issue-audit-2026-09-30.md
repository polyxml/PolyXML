# Competitor issue audit — 2026-09-30

Final selected-priority implementation and issue closure are recorded in
[the 2026-10-01 backlog closeout](backlog-closeout-2026-10-01.md).

Implementation follow-up for the scoped category 1/2 bugs is recorded in
[category-1-2-fixes-2026-10-01.md](category-1-2-fixes-2026-10-01.md), including
runtime regression evidence and the limits of the fix claims. Findings below
describe the audit snapshot rather than current implementation status.

The [fifth pass](competitor-issue-audit-wave5-2026-09-30.md) investigates 21
more public reports with saved fixtures and links six new PolyXML issues
[#116–#121](competitor-issue-audit-wave5-2026-09-30.md#new-polyxml-issues-filed-during-this-pass).
The [sixth pass](competitor-issue-audit-wave6-2026-09-30.md) investigates five
more reports, confirms four distinct PolyXML regressions, and files #122–#125.
The [seventh pass](competitor-issue-audit-wave7-2026-09-30.md) investigates ten
more reports, files one XML parser bug (#126) and one incremental-writer
enhancement (#127), and records the scoped cases that passed.

This is a targeted triage of public issue reports, not a full compatibility or
conformance review. Statuses describe the current local PolyXML checkout.
Generated code was inspected after `cargo build -p polyxml-cli`; suspected
failures were exercised with generated Python and C# runtimes and checked with
an independent XSD validator. The repeated choice case also ran its Go XML
round-trip test.

| Public report | PolyXML status | Evidence and next step |
| --- | --- | --- |
| [XmlSchemaClassGenerator #616: repeated choice loses interleaved order](https://github.com/mganss/XmlSchemaClassGenerator/issues/616) | **Covered for the tested case** | `crates/polyxml-core/tests/test_unbounded_choice.rs` asserts one ordered `items` list for all seven generators and executes a Go parse/serialize round trip for A, B, A. `cargo test -p polyxml --test test_unbounded_choice` passed. Other generated runtimes were not exercised by this test. |
| [XmlSchemaClassGenerator #612: named string restriction becomes a list](https://github.com/mganss/XmlSchemaClassGenerator/issues/612) | **Not affected in the tested shape** | A named `xs:string` restriction used once generated `BitmapWeek` as an `Annotated[str, ...]` alias in Python and a scalar value record in C#, rather than a list. C# also emitted length and pattern validation on that record. This does not establish full facet enforcement in every target. |
| [xsdata #1184: optional element default is lost](https://github.com/tefra/xsdata/issues/1184) | **Affected; earlier assessment corrected** | [W3C XSD primer](https://www.w3.org/TR/xmlschema-0/) says an absent optional element stays absent, while a present empty element receives the default. Python generated `bool | None = field(default=False, ...)`: parsing `<Root/>` returned `flag=False` and serialization emitted `<Flag>false</Flag>`; parsing `<Flag/>` returned an empty string. C# kept absence as `null`, but `XmlSerializer` rejected `<Flag/>` as a boolean. |
| [XmlSchemaClassGenerator #568: fixed element value becomes mutable](https://github.com/mganss/XmlSchemaClassGenerator/issues/568) | **Affected** | For `<xs:element name="Code" type="xs:string" fixed="0013"/>`, C# accepted, parsed, and serialized `Code="wrong"`; `IValidatableObject.Validate` returned no errors. Python also parsed and serialized the wrong value. An independent XSD validator rejected that XML. The parser stores `fixed_value` in the IR, but generators do not consume it. |
| [xsdata #1120: sequence nested in choice loses grouping](https://github.com/tefra/xsdata/issues/1120); related [XmlSchemaClassGenerator #285: wrong sequence order](https://github.com/mganss/XmlSchemaClassGenerator/issues/285) | **Affected** | For a choice between `sequence(First, Second)` and `Alternative`, Python generated three independent fields, with `First` and `Second` required and `Alternative` optional. Parsing the valid alternative-only XML raised `TypeError`. C# validation and serialization accepted a model containing both branches, which an independent XSD validator rejected. Preserve particle grouping in the IR and test generated round trips. |
| [xsdata #1141: unrelated root element accepted](https://github.com/tefra/xsdata/issues/1141) | **Affected in generated Python** | A generated `RootType.from_xml` accepted `<Other><Code>0013</Code></Other>` even though the schema declares only `Root`; `RootType.to_xml` emitted `<RootType>`, not `<Root>`. The generated `Root` is a `type` alias that cannot be instantiated or called with `from_xml`. |
| [JAXB tools #577: duplicate namespace prefix mapping in generated `package-info`](https://github.com/highsource/jaxb-tools/issues/577) | **Mechanism not applicable; user need unverified** | PolyXML does not emit JAXB `package-info` or use the `-Xnamespace-prefix` plugin. A Factur-X namespace serialization fixture would be needed before making a broader interoperability claim. |
| [xsdata #1217: eBay schema download/generation fails](https://github.com/tefra/xsdata/issues/1217) | **Promising compile-level result; runtime unverified** | The live eBay XSD version 1475 was fetched on 2026-09-30 (SHA-256 `e2b584a7a78d4929f1a5073cd36072905e45e3f8b6b7df82f85ef593e0a0d60f`). `polyxml validate` reported 645 types and 152 roots; full Python generation, `compileall`, and module import passed. No eBay XML document was round-tripped, and xsdata was not run against this exact snapshot. |
| [XmlSchemaClassGenerator #566: UBL 2.4 Invoice generation produces no output](https://github.com/mganss/XmlSchemaClassGenerator/issues/566) | **PolyXML parses but generated C# does not compile** | Official UBL Invoice 2.4 plus 15 recursively imported XSDs parsed as 1,608 types and 2,101 global elements. Full C# generation completed, but `dotnet build` failed with duplicate declarations and inheritance errors. `--root-element Invoice` retained 787 types, but default record output failed with `CS8866`; mutable class output failed with `CS0115`. This is not yet a migration claim. |

## Filed PolyXML issues

- [#101: preserve nested sequence-in-choice branches](https://github.com/polyxml/PolyXML/issues/101)
- [#102: enforce XSD fixed values](https://github.com/polyxml/PolyXML/issues/102)
- [#103: apply optional element defaults correctly](https://github.com/polyxml/PolyXML/issues/103)
- [#104: bind generated Python root models to declared elements](https://github.com/polyxml/PolyXML/issues/104)

Related closed work was checked against current `main`: [#79](https://github.com/polyxml/PolyXML/issues/79) covers repeated unbounded choices with single-element branches, not the bounded choice with a nested sequence in #101. [#98](https://github.com/polyxml/PolyXML/issues/98) filters generated graphs by global root element, not Python runtime root binding in #104. The earlier [edge-case audit #51](https://github.com/polyxml/PolyXML/issues/51) did not include these four reproductions. Existing generated-model tests use a declared `WarehouseInventory` root but parse `<Inventory>`, so they do not assert global root-name matching.

### eBay schema compile probe

The upstream URL is mutable. Reuse the recorded SHA-256 before treating a
later download as the same schema:

```sh
curl -fsSL https://developer.ebay.com/webservices/latest/ebaysvc.xsd \
  -o /tmp/polyxml-ebaysvc.xsd
sha256sum /tmp/polyxml-ebaysvc.xsd
./target/debug/polyxml validate /tmp/polyxml-ebaysvc.xsd
./target/debug/polyxml generate /tmp/polyxml-ebaysvc.xsd \
  --lang python --out /tmp/polyxml-ebay-python
./.venv/bin/python -m compileall -q /tmp/polyxml-ebay-python
```

The generated `ebaysvc.py` imported under Python 3.12. This checks schema
compilation and Python syntax/import, not field correctness or an eBay XML
round trip. Do not claim an xsdata comparison until both tools run on the same
saved XSD and a representative XML fixture.

### UBL 2.4 Invoice compile probe

Fetched the [official invoice XSD](https://docs.oasis-open.org/ubl/os-UBL-2.4/xsd/maindoc/UBL-Invoice-2.4.xsd)
and its 15 transitive `schemaLocation` dependencies, preserving the relative
`maindoc/` and `common/` paths. The main XSD SHA-256 was
`0d1b79c25ac860e16be5583b63c60d8b97cc3b1a0a9f46c43111c88096bbd2fd`.
PolyXML validation and generation passed in under a second, but `dotnet build`
failed in each tested mode:

| Mode | Result |
| --- | --- |
| Full C# output | Duplicate generated names (`CS0101`) and inheritance from sealed types (`CS0509`). |
| `--root-element Invoice`, default record | 787 retained types; conflicting positional `Value` members (`CS8866`). |
| `--root-element Invoice --style class` | Generated `Validate` overrides without a matching base method (`CS0115`). |

The first errors should be reduced to small multi-namespace/simple-content
schemas before filing a new PolyXML issue. This does not establish that
XmlSchemaClassGenerator still fails on the exact same saved UBL snapshot.

## Reproduction schema for the local generation checks

Save this as `repro.xsd`, then run:

```sh
cargo run -q -p polyxml-cli -- generate repro.xsd \
  --lang python --lang csharp --out /tmp/polyxml-competitor-audit
```

```xml
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:simpleType name="BitmapWeek">
    <xs:restriction base="xs:string">
      <xs:minLength value="7"/>
      <xs:maxLength value="7"/>
      <xs:pattern value="[0-1]{7}"/>
    </xs:restriction>
  </xs:simpleType>
  <xs:element name="Root">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="WeeklyPattern" type="BitmapWeek"/>
        <xs:element name="property1" type="xs:boolean" default="false"/>
        <xs:element name="property2" type="xs:boolean" minOccurs="0" default="false"/>
        <xs:element name="PROC-IDENT" type="xs:string" fixed="0013"/>
        <xs:choice>
          <xs:sequence>
            <xs:element name="First" type="xs:string"/>
            <xs:element name="Second" type="xs:string"/>
          </xs:sequence>
          <xs:element name="Alternative" type="xs:string"/>
        </xs:choice>
      </xs:sequence>
    </xs:complexType>
  </xs:element>
</xs:schema>
```

## Second pass: Go and related generator issues

This pass checked additional public reports against current `main` and against
closed PolyXML work. Minimal schemas are in `research/fixtures/`. The findings
below are scoped to the tested target and payloads. Follow-up issues from this
pass were filed after the third-pass review; see the issue list below.

| Public report | PolyXML status | Evidence |
| --- | --- | --- |
| [xgen #115: optional `xs:time` omits Go import](https://github.com/xuri/xgen/issues/115) | **Compile issue covered; broader runtime gap found** | `optional_time.xsd` generated `import "time"` for `*time.Time`, and `go test` compiled. `xml.Unmarshal` of XSD-valid `<Schedule><StartTime>12:34:56</StartTime></Schedule>` failed because Go's `time.Time` expects an RFC 3339 date-time, not an XSD time-only value. With `temporal_scalars.xsd`, Go also rejected valid `xs:date` and timezone-free `xs:dateTime` strings; RFC 3339 `xs:dateTime` with `Z` parsed. The independent XSD validator accepted all four documents. |
| [xsdata #1213: same child name under two wrappers merges collections](https://github.com/tefra/xsdata/issues/1213) | **Covered in tested Python parsing** | `duplicate_wrappers.xsd` generated distinct `ldeps` and `ddeps` wrapper types. Python parsed one `<dep>` under each into the correct list. Its serializer still used the type-name root, which is already tracked in PolyXML #104. |
| [xgen #74: mixed content loses text](https://github.com/xuri/xgen/issues/74) | **Covered in tested core and Go paths** | `test_mixed_content` passed: core round-tripped interleaved text and child elements. Generated Go parsed five ordered items and serialized `before <b>bold</b> between <i>italic</i> after` in the same order. Other generated runtimes were checked for model shape, not exercised here. Related closed PolyXML #88. |
| [xgen #64: `xs:union` is modeled incorrectly](https://github.com/xuri/xgen/issues/64) | **Partial coverage; Go and C# attribute gaps found** | `test_lexical_union_codegen` passed for element content. For `lexical_union_attribute.xsd`, independent XSD validation accepted both `count="42"` and `count=""`; generated Python parsed both. Generated Go returned `cannot unmarshal into models.IntOrEmpty` for both because the union lacks attribute text unmarshalling. Generated C# silently left `Count` null when reading either attribute and wrote a child `<count>42</count>` instead of an attribute because its proxy uses `[XmlElement]`. Related closed PolyXML #85 did not test union attributes. |
| [XmlSchemaClassGenerator #475: optional scalar becomes a collection](https://github.com/mganss/XmlSchemaClassGenerator/issues/475) | **Covered for tested C# shape** | An optional named enum field generated `YesNoT? IsDeprecated = null`, not a collection. The upstream request also suggests pre-filling an absent optional element from its XSD default; the [W3C XSD primer](https://www.w3.org/TR/xmlschema-0/) says element defaults apply when an element is present and empty, not absent. |
| [JAXB tools #558: choice allows multiple properties](https://github.com/highsource/jaxb-tools/issues/558) | **Covered for tested Java model shape** | A bounded top-level choice between two elements generated a sealed `Data` interface with `Text` and `Number` record variants. No Java XML round trip was run for this case. Nested sequence branches remain open in PolyXML #101. |
| [xgen #33: named `xs:group` reference is not expanded](https://github.com/xuri/xgen/issues/33) | **Covered by existing regression** | Closed PolyXML #51 included named group expansion; `cargo test -p polyxml --test test_schema_audit` passed its group case. |
| [xgen #46: incorrect Go root XML tag](https://github.com/xuri/xgen/issues/46) | **Affected in tested Go output** | `optional_time.xsd` generated `type Schedule = ScheduleType` with an unconstrained `XMLName`. `xml.Marshal(Schedule{})` emitted `<ScheduleType>`, and `xml.Unmarshal("<Other/>", &Schedule{})` succeeded, though the schema declares only `<Schedule>`. This mirrors the Python root problem in PolyXML #104 and should be scoped across targets. |

### Reproduce the three Go gaps

From the repository root, generate each fixture with `--lang go`, then use
`encoding/xml` with the generated `ScheduleType` or `RootType`:

```sh
./target/debug/polyxml generate research/fixtures/optional_time.xsd \
  --lang go --out /tmp/polyxml-optional-time
./target/debug/polyxml generate research/fixtures/lexical_union_attribute.xsd \
  --lang go --out /tmp/polyxml-union-attribute
./target/debug/polyxml generate research/fixtures/temporal_scalars.xsd \
  --lang go --out /tmp/polyxml-temporal
```

- Decode `<Schedule><StartTime>12:34:56</StartTime></Schedule>` into
  `ScheduleType`: `time.Time` returns an RFC 3339 parse error.
- Decode `<Root count="42"/>` into `RootType`: Go returns
  `cannot unmarshal into models.IntOrEmpty`.
- Marshal `Schedule{}`: Go writes `<ScheduleType></ScheduleType>`; decoding
  `<Other/>` into `Schedule` also succeeds.

Union attributes in Rust/Java/C++/TypeScript still need checking. Go root
binding and Go temporal lexical handling now have separate, scoped issues.

## Third pass: current-release comparisons and additional wire probes

This pass used PolyXML checkout `ab637e88495969988a5ac98ce2d05d01f1d47b66`,
xgen `v0.0.0-20260902022537-fb39bae424fe` (`xgen -v` reports 0.1.0),
and XmlSchemaClassGenerator 3.0.1405.0. The external tools were installed in
`/tmp/polyxml-audit-tools`; each ran on the **same saved XSD** as PolyXML.
`lxml.etree.XMLSchema` independently checked the instance validity. These are
small cases, not whole-product scores. xgen and XmlSchemaClassGenerator do not
provide PolyXML's seven-target API, so only their matching targets were
compared.

| Public report and fixture | Current PolyXML result | Same-snapshot competitor result |
| --- | --- | --- |
| [xgen #98: element and type share a name](https://github.com/xuri/xgen/issues/98), `forward_same_name.xsd` | Generated Python, Go and C#; Go compiled, Python parsed a valid `<Item>`. The generated type itself is usable, although it lacks a separate root alias and unconstrained root names remain #104. | Current xgen emitted `type Item *Item` rather than a struct with `Code`. `go test` on the declaration alone passed; no xgen XML runtime test was performed. Closed PolyXML #82 already cited this competitor report, so this is regression evidence, not a new issue. |
| [xgen #48: inline restricted simple type self reference](https://github.com/xuri/xgen/issues/48), `inline_restriction.xsd` | Generated Go declared `HeaderTypeOperationSimpleType OperationCode`; `go test` passed a marshal/unmarshal round trip for `ABC`. | Current xgen emitted a `string` field and no self-referential type. The historical report is **not currently reproduced**; it also loses the named restriction's type. Neither test established length-facet enforcement. |
| [XSCG #36: substitution groups](https://github.com/mganss/XmlSchemaClassGenerator/issues/36), `substitution_group.xsd` | **Affected in Python and Go.** The valid `<Portfolio><Bond>A</Bond><Equity>B</Equity></Portfolio>` parsed as an empty `instrument` list in Python and Go. Go's test failed with the lost values; Python's serializer emitted an empty payload. The schema IR has a substitution-group test, but the generated field still matches the lexical `t:Instrument` head. | Current XSCG emitted `Bond` and `Equity` alternatives, so the old issue is not evidence that it lacks all substitution support. For this fixture, its generated C# failed at `new XmlSerializer(typeof(Portfolio))`: multiple string alternatives need `XmlChoiceIdentifierAttribute`. Current xgen Go also emitted `xml:"t:Instrument"`, with no branch fields; runtime not checked. |
| [xsdata #1121: repeated sequence grouping and cardinality](https://github.com/tefra/xsdata/issues/1121), `repeated_sequence.xsd` | **Affected in Python.** A valid `First, Second, First, Second` document parsed into separate `first` and `second` lists. Serialization grouped them as `First, First, Second, Second`, invalid even after correcting the already-known root-name problem. Invalid incomplete and misordered sequences were also accepted by the Python parser. C# shape is likewise two independent lists; its runtime was not exercised. | xsdata was not run on this fixture; this is a problem-class comparison only. Distinct from closed PolyXML #79 and open #101: repetition belongs to the *sequence group*, not an unbounded single-element choice or a sequence inside a bounded choice. |
| [XSCG #551: nillable enum collection](https://github.com/mganss/XmlSchemaClassGenerator/issues/551), `nillable_enum_collection.xsd` | **Affected in C#.** Generated `List<Code>?` without nullable items. A schema-valid `<Item xsi:nil="true"/><Item>A</Item>` failed deserialization with `'' is not a valid value for Code`. | Current XSCG generated `Collection<Code>` with `IsNullable=true`; `XmlSerializer` construction failed with `IsNullable may not be 'true' for value type Generated.Code`. Both tools fail this fixture, by different routes. Closed PolyXML #86 covers `xsi:nil` on abstract complex types, not nillable enum items. |
| [xgen #30: multiple `xs:list` types](https://github.com/xuri/xgen/issues/30), `list_simple_types.xsd` | **Generation covered; semantics incomplete.** All three targets generated, but Python/Go modeled `xs:list` of integers as a string, and Python accepted `one two` where the independent XSD validator rejected it. Python parsed valid `1 2` as one string rather than a typed integer list. | xgen was not run on this particular fixture. Its report concerns a generation syntax error with multiple lists and imports; the local result only addresses a small same-file shape. |

### Re-run the third-pass fixtures

```sh
for name in forward_same_name inline_restriction substitution_group repeated_sequence nillable_enum_collection list_simple_types; do
  ./target/debug/polyxml validate "research/fixtures/$name.xsd"
  ./target/debug/polyxml generate "research/fixtures/$name.xsd" \
    --lang python --lang go --lang csharp --out "/tmp/polyxml-audit-$name"
done
```

The comparisons above additionally used `go test`, `dotnet run`, generated
Python `from_xml`/`to_xml`, and `lxml.etree.XMLSchema`. The temporary runtime
drivers were not checked into the repository; the XSDs and exact input values
are retained here. A durable CI regression should add those drivers before a
fixed claim is published. In particular, a successful generated-code build
alone never proves a successful XML round trip.

### Detailed PolyXML issues filed after the second and third passes

The small-fixture issues embed their XSD and input; #112 records the official
UBL entry-schema hash and relative import layout. Each records the local checkout and
observed target behavior, links the public competitor report, and states
acceptance checks. These are PolyXML bugs or missing semantics; the
competitor issue is a research lead, not proof that the competitor currently
fails every matching case.

| PolyXML issue | Confirmed scope |
| --- | --- |
| [#105: substitution-group members](https://github.com/polyxml/PolyXML/issues/105) | Python and Go silently drop concrete substitute elements. |
| [#106: repeated sequence groups](https://github.com/polyxml/PolyXML/issues/106) | Python loses group order and emits invalid child order. |
| [#107: nillable enum collection](https://github.com/polyxml/PolyXML/issues/107) | C# rejects a valid nil item at runtime. |
| [#108: typed `xs:list`](https://github.com/polyxml/PolyXML/issues/108) | Python/Go expose string values; Python accepts invalid integer items. |
| [#109: Go temporal lexical values](https://github.com/polyxml/PolyXML/issues/109) | Go compiles but rejects valid date/time lexical forms. |
| [#110: union-typed attributes](https://github.com/polyxml/PolyXML/issues/110) | Go rejects valid attributes; C# ignores them and emits elements. |
| [#111: Go root names](https://github.com/polyxml/PolyXML/issues/111) | Go emits a type-name root and accepts an unrelated root. |
| [#112: UBL Invoice C# compilation](https://github.com/polyxml/PolyXML/issues/112) | Official UBL 2.4 parses and generates, but full and root-scoped C# output fails `dotnet build` in three tested modes. |
| [#113: Go namespaced element reference](https://github.com/polyxml/PolyXML/issues/113) | Go silently drops a valid global referenced child with a namespace. |
| [#114: wildcard element data loss](https://github.com/polyxml/PolyXML/issues/114) | Python drops a declared foreign element; Go stores a nil placeholder and writes no child. |
| [#115: choice branch occurrence](https://github.com/polyxml/PolyXML/issues/115) | Go rejects a valid repeated branch; Python loses branch identity and cardinality. |
| [#128: xs:anyAttribute wildcards](https://github.com/polyxml/PolyXML/issues/128) | Python, Go, and C# drop `<xs:anyAttribute>` and discard all wildcard attributes on deserialize. |
| [#129: Rust digit-prefixed module names](https://github.com/polyxml/PolyXML/issues/129) | Schema filenames starting with numbers generate invalid Rust syntax (`pub mod <digit>...;`) in `mod.rs`. |
| [#130: simpleContent element default typing](https://github.com/polyxml/PolyXML/issues/130) | Python sets string literal default for simpleContent dataclass, emitting empty elements `<flag></flag>`. |
| [#131: Unique Particle Attribution (UPA) validation](https://github.com/polyxml/PolyXML/issues/131) | `polyxml validate` accepts non-deterministic content models violating W3C UPA. |

## Fourth pass: namespace refs, wildcards, and choice cardinality

This pass used the same PolyXML checkout as above, current xgen
`v0.0.0-20260902022537-fb39bae424fe`, and xsdata 26.2 in a temporary Python
3.12 virtual environment. The new saved schemas are
`global_ref_namespace.xsd`, `synthetic_content_name.xsd`,
`choice_branch_cardinality.xsd`, `all_group.xsd`, and the two-file
`imported_attribute_type/` fixture. `lxml.etree.XMLSchema` checked the legal
XML instances. The target runtimes, not generation alone, determined statuses.

| Public report / case | PolyXML result | Same-snapshot or limiting evidence |
| --- | --- | --- |
| [xgen #101: prefixed global element references](https://github.com/xuri/xgen/issues/101) | **Go affected; Python covered in parsing.** For a required `ref="t:Child"`, Go emitted `xml:"t:Child"`. Both `<Container xmlns="urn:audit:ref"><Child>abc</Child></Container>` and equivalent prefixed XML parsed without error but left `Child == ""`. Python parsed the default-namespace instance as `child='abc'`. [PolyXML #113](https://github.com/polyxml/PolyXML/issues/113). | Current xgen emitted the same lexical tag and also dropped the child in Go for both valid instances. This is a shared gap, not a migration advantage. Distinct from #105 substitution membership and #111 root name binding. |
| [Bergmann #265: `content` attribute collides with generated content field](https://github.com/Bergmann89/xsd-parser/issues/265); related [XSCG #482: wildcard model](https://github.com/mganss/XmlSchemaClassGenerator/issues/482) | **Collision avoided in tested Rust shape; wildcard loss found.** Generated Rust source had one `content` attribute field, not a duplicate synthetic field; a Rust compile was not run. For valid `<o:Extra>y</o:Extra>` matched by `xs:any namespace="##other"`, generated Python returned `any=[]`. Go returned one `nil` wildcard item and marshaled no extension child. [PolyXML #114](https://github.com/polyxml/PolyXML/issues/114). | Generated C# `XmlSerializer` parsed the extension into a `System.Xml.XmlElement` and wrote it back with its namespace and text. The two upstream reports have different mechanisms; no claim is made about their current runtime behavior. |
| [xsdata #1215: list cardinality inside a choice](https://github.com/tefra/xsdata/issues/1215) | **Affected.** In a bounded choice with `Timing maxOccurs="unbounded"`, Go generated `Timing *string` and rejected the XSD-valid two-Timing instance as a multiple-choice violation. Python emitted `type RootType = str | str | str`, losing branch tags and repetition. [PolyXML #115](https://github.com/polyxml/PolyXML/issues/115). | Current xsdata 26.2 generated `timing: list[str]` and scalar optional `drive`/`load` fields on this exact XSD; its parser and serializer round-tripped two Timing values. This small case is covered in xsdata despite its broader IP-XACT report remaining open. |
| [xsdata #1218: attribute simple-type dependency](https://github.com/tefra/xsdata/issues/1218) | **Covered in tested two-file shape.** An imported enum `c:Code` used by a required local `code` attribute stayed typed in Python, Go, and C#. Python parsed `Code.A`; Go and C# parsed and serialized `code="A"` correctly. | The report concerns a large CDA schema with missing dependencies. This small import case does not establish CDA coverage; no current xsdata comparison was run. |
| [lumeo #140: support `xs:all`](https://github.com/lumeohq/xsd-parser-rs/issues/140) | **Covered in tested Python path.** PolyXML generated both required fields. Generated Python parsed either legal child order and raised on a missing required child. | Only a two-child `xs:all` was exercised. Other targets and nested groups were not runtime-tested. |

The `xs:any` probe came from the Bergmann report's shape; the upstream issue is
about a duplicate generated field, while PolyXML's confirmed bug is loss of
declared wildcard XML. Likewise, xsdata #1215 is a research lead, not evidence
that xsdata 26.2 fails the saved minimal choice fixture.

### Research queue, not verified behavior

The open trackers contain more plausible tests, but the reports below have
**not** been reproduced against PolyXML. Keep them out of competitive claims
until each has a legal XSD, an instance, an independent validity result, and a
generated-runtime test. Existing PolyXML work is noted so effort is not
duplicated.

| Cluster | Public issue leads | PolyXML overlap / next audit |
| --- | --- | --- |
| Namespace and global-element references | [XSCG #563](https://github.com/mganss/XmlSchemaClassGenerator/issues/563), [lumeo #141](https://github.com/lumeohq/xsd-parser-rs/issues/141) | Go's ordinary global-ref gap is now #113; next test XLink/MISMO attribute QNames and cross-module runtime binding. Closed #94/#97 covered ownership/imports, not every wire mapping. |
| Choice cardinality and alternatives | [XSCG #378](https://github.com/mganss/XmlSchemaClassGenerator/issues/378), [lumeo #155](https://github.com/lumeohq/xsd-parser-rs/issues/155) | Branch occurrence is now #115; test duplicate branch names and multiple choice groups against #79/#101/#106 before filing more. |
| Attribute/simple-type dependency | [xsdata #1218](https://github.com/tefra/xsdata/issues/1218) | The two-file imported enum attribute passed in three targets. Test the large CDA dependency graph separately; the small case cannot settle its reported failure. |
| Polymorphism and substitution | [Bergmann #275](https://github.com/Bergmann89/xsd-parser/issues/275), [JAXB #713](https://github.com/highsource/jaxb-tools/issues/713) | Closed #53 covered `xsi:type` dispatch at its tested level. Run a generated runtime with nested imported derived types and check both parse and write. |
| Multi-schema/industry corpus | [xgen #106](https://github.com/xuri/xgen/issues/106), [XSCG #571](https://github.com/mganss/XmlSchemaClassGenerator/issues/571), [xsdata #1218](https://github.com/tefra/xsdata/issues/1218) | Closed #81/#97 and the UBL result show that parse success and generated compilation differ. Use pinned snapshots and representative XML before a migration claim. |
| Security and streaming | [xsdata #1212](https://github.com/tefra/xsdata/issues/1212), [JAXB #596](https://github.com/highsource/jaxb-tools/issues/596), [xsdata #1031](https://github.com/tefra/xsdata/issues/1031) | Separate schema loading, XML parsing, external entity handling, and memory use; measure with bounded inputs. These are not comparable bug claims yet. |

## Suggested priority

1. Fix nested sequence-in-choice modeling. It rejects a valid branch and can
   produce invalid XML across multiple targets.
2. Enforce `fixed` values on generated models and codecs, starting with C# and
   Python, and checking the other targets before a broad support claim.
3. Apply optional element defaults only to present empty elements; preserve
   absence in every target.
4. Enforce root names and emit usable global element models in generated
   Python and Go.
5. Triage Go temporal values and Go/C# union attributes with small regression
   fixtures.
6. Reduce the UBL C# build failures and verify a compiled root-scoped Invoice
   before using UBL in a migration comparison.
7. Preserve substitution-group members, repeated sequence grouping, nillable
   enum items, and typed `xs:list` values in generated runtimes. The first two
   can silently lose data or emit invalid XML.
8. Run the saved eBay XSD with representative XML in both tools and test a
   Factur-X namespace fixture before making migration claims.

Use only the tested repeated-choice and named-restriction cases in competitive
claims. Link to the public issue and to a PolyXML regression test or runnable
fixture; do not infer that every schema or target is covered.
