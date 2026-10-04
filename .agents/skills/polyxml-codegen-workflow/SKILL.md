---
name: polyxml-codegen-workflow
description: >-
  Use this skill when developing, refactoring, or testing code generation across PolyXML's 7 target languages
  (Rust, Python, C++, Java, TypeScript, Go, C#), adding codegen backends, plumbing CLI flags and polyxml.toml manifest options,
  or writing compiler integration tests.
---

# PolyXML Polyglot Codegen Development & Verification Playbook

## Inherited simpleContent

Normalize synthetic text fields only after root-level import/include merging and
reference validation. Follow complex bases iteratively with a cycle guard and
memoized terminal types; keep the complex `base_type` for inherited attributes.
Preserve named simple aliases, enums, unions and lists rather than flattening away
their identities or constraints. Imported frames can contain boxed references
from cycle cutting; unwrap those only while resolving text inheritance.

Runtime schema flattening must retain one most-derived text slot, otherwise
record lookup finds an inherited empty slot and writing can duplicate text.
TypeScript object inheritance applies only to complex bases, never builtin or
named scalar types. Run `test_inherited_simple_content` and the seven-language
`test_inherited_text_codegen` consumers, including Rust owned/borrowed XML and
Serde, Python dataclass/Pydantic, and strict C# record/class execution. Named
simple-type facets are preserved; this does not implement additional facets
written directly inside a simpleContent restriction or empty/list text codecs.


## XSD attribute values

Use separate `CARGO_TARGET_DIR` paths for revision-specific CLI/native-binding
verification. A shared debug target can leave another checkout's same-named
top-level binary or cdylib in place even when Cargo reports a fresh build. Probe
a discriminating fixture before claiming a baseline comparison, record the
executable/binding hashes, and retain rejected preflights separately. The runtime
comparison runners already isolate baseline and candidate consumer targets.

Normalize schema attributes with quick-xml's `normalized_value` before placing
them in the IR. This resolves built-in/numeric references once and normalizes
literal XML attribute whitespace while preserving referenced whitespace. Use the
same decoded values for namespace declarations, QName validation, UPA, facets,
enumerations and default/fixed constraints. Propagate malformed attributes and
reference errors; do not turn them into absent values. Validate attributes even
inside otherwise skipped annotations. Regression fixtures are in
`research/fixtures/schema_attribute_entities.xsd` and
`tests/test_schema_attribute_entities.rs` in the core crate.

Decoded enum values need target-language string literal escaping at every output
site: metadata attributes, constants, conversion tables and parse matches. Rust
debug string formatting is suitable for Rust, but its `\u{...}` escapes are not
portable to other targets. The shared JSON-compatible literal helper also
escapes U+0085/U+2028/U+2029 because C# treats them as source line terminators;
C++ uses fixed-width octal escapes for ASCII controls. Execute
`test_enum_literal_codegen` with all seven toolchains to verify exact runtime
values and Rust Serde round trips, not just whether generated files exist.

Schema defaults and fixed values need the same literal escaping as enums.
Python metadata must not use Rust Debug escapes, and C++ defaults need C++
control-character escapes. C# defaults parsed through an internal XML element
must encode carriage returns as `&#13;` after escaping XML markup; otherwise XML
line-ending normalization changes the lexical value. Execute generated consumers
against independent UTF-8 expected data, including backslash-u text, referenced
whitespace, Unicode line separators, empty-element defaults and fixed rejection.

## C# runtime availability for smoke checks

The C# execution fixtures target `net8.0`. An installed .NET 10 SDK/runtime
can compile those fixtures but does not run them with default roll-forward
settings if the .NET 8 runtime is absent. Install the .NET 8 runtime to verify
the minimum target. For a compatibility smoke on a machine with only .NET 10,
set `DOTNET_ROLL_FORWARD=Major` for the test command and report that runtime
version; this does not verify execution on .NET 8. A missing-runtime panic
poisons the shared test mutex and causes additional C# failures, so rerun the
test binary after correcting the runtime environment.

## XML Schema semantics to regression-lock

When triaging competitor reports, compile a minimal XSD, inspect the IR and
generated code, then parse and serialize valid and invalid XML with the target
runtime. Compare the result against an independent XSD validator. The
2026-09-30 audit in `research/competitor-issue-audit-2026-09-30.md` found these
baseline gaps. Category 1/2 follow-up evidence and remaining limits are in
`research/category-1-2-fixes-2026-10-01.md`; do not treat this historical list
as the current fix status:

- A sequence nested inside a bounded choice is flattened into independent
  fields; a valid alternative-only instance fails in generated Python, while
  generated C# can serialize both branches together. Preserve the compositor
  tree before applying target-specific union representations.
- `fixed_value` reaches `FieldDef` but generated Python and C# accept and
  serialize different values. Test both input validation and output validation.
- Optional element defaults apply to a **present empty** element, not to an
  absent one (W3C XSD primer, section 2.2). Generated Python currently fills
  the absent field with the default and writes it back; Python and C# also
  mishandle the present-empty boolean case.
- Generated Python parsing accepts an unrelated root tag, and the root alias
  is a `TypeAliasType` rather than a usable class. Test root name checks and
  the documented import/constructor path together.
- Generated Go aliases for global root elements have the same wire-name
  problem: `type Schedule = ScheduleType` marshals as `<ScheduleType>` and
  accepts an unrelated root. A root alias needs a wire-name check, not just
  a unique type identifier.
- Generated Go `time.Time` fields compile for optional `xs:time`, but
  `encoding/xml` expects RFC 3339 date-time text; valid XSD `xs:time`,
  `xs:date`, and timezone-free `xs:dateTime` lexical values fail. Exercise
  actual XML parsing after checking generated imports.
- Lexical `xs:union` element tests do not cover union-typed XML attributes.
  The Go union lacks attribute text unmarshalling; C# emits an `[XmlElement]`
  proxy for an attribute and silently drops attribute input. Test the XML
  field kind as well as the union's value parser.
- Official UBL Invoice 2.4 parses and emits C#, but `dotnet build` fails:
  full output has duplicate names and sealed-base inheritance errors;
  root-scoped record output has positional `Value` collisions (`CS8866`);
  root-scoped mutable class output has invalid `Validate` overrides
  (`CS0115`). The audit records the source snapshot and mode differences.
  Reduce these failures to small fixtures before changing the C# generator.
- `substitutionGroup` membership survives schema parsing but generated Python
  and Go fields referencing an abstract head can silently discard concrete
  substitute elements. Test `<Bond>` and `<Equity>` under a required
  `<Instrument ref>` with actual generated XML parsers, not only IR assertions.
- `xs:sequence minOccurs="0" maxOccurs="2"` is flattened into separate
  lists in generated Python. A valid interleaved pair parses but serializes
  with grouped fields and fails independent XSD validation. Preserve group
  occurrences and order; this is distinct from unbounded `xs:choice`.
- A nillable enum item in a repeated C# element is emitted as `List<Enum>`;
  `XmlSerializer` rejects a schema-valid `xsi:nil="true"` item. Model nullable
  *items* as well as collection presence. This differs from abstract-complex
  `xsi:nil` coverage.
- `xs:list` of integers currently becomes a string in Python and Go. A valid
  whitespace-separated list parses as one string, and invalid lexical items
  can pass generated Python parsing. Distinguish list-valued simple types
  from ordinary string restrictions in the IR and codecs.

Detailed follow-up issues: substitution groups #105, repeated sequences #106,
nillable enum collections #107, typed `xs:list` #108, Go temporal lexical
values #109, union-typed attributes #110, and Go root binding #111. Reuse their
embedded XSDs and runtime acceptance checks when implementing fixes.
Official UBL 2.4 Invoice C# compilation is tracked in #112, with the source
snapshot, three generation modes, and compiler error classes recorded there.
The next audit found three more generated-runtime gaps: ordinary namespaced
`xs:element ref` children are silently lost by Go's lexical-prefix XML tag
(#113); Python and Go discard declared `xs:any` foreign elements even though
C# preserves the tested payload (#114); and a bounded choice with one
`maxOccurs="unbounded"` element branch loses that branch's list cardinality
(#115). Treat these separately from substitution groups, unknown-element
tolerance, and unbounded *choice* order, respectively. The small imported enum
attribute and `xs:all` fixtures were covered in their tested paths.

The fifth competitor pass is in
`research/competitor-issue-audit-wave5-2026-09-30.md`. Keep its negative
schemas as negative checks: an unresolved type and an illegal child inside
`xs:simpleType` currently pass CLI validation (#116). XSD 1.1
`xs:alternative` also passes but degrades to untyped roots (#119); use an
XSD 1.1 validator for that fixture. Generated Go/C# cannot handle a valid
40-digit `xs:integer` (#117), and a date/dateTime lexical union has a Go
compile error plus invalid C# output text (#118). Enum-only Go output imports
unused `encoding/xml` (#121), so compile the smallest schema, not only a
complex-type smoke case. Derived Python models with nested `Meta` trigger a
Pyright override error (#120); run a static checker as well as Python import
and XML round-trip tests when changing inheritance metadata.

The sixth pass (`research/competitor-issue-audit-wave6-2026-09-30.md`) adds
four generated-runtime regressions. Two exclusive choice branches may share
one XML element name; flattening them into sibling fields makes Go's XML tags
conflict and Python require an absent branch (#122). Two separate choices in
one complex type must each enforce one selected branch (#123). An abstract
root with a derived `xsi:type` parses in generated Python but generated Go
silently drops the derived content (#124), despite closed #53. An imported
global attribute such as `xl:type` needs its expanded QName: C# cannot
reflect `[XmlAttribute("xl:type")]` and Go misses the value (#125). Use
`lxml.etree.XMLSchema` to check fixture and instance validity, then execute
the generated target runtime; compile success alone misses the Go data loss.

The seventh pass (`research/competitor-issue-audit-wave7-2026-09-30.md`)
checks cross-namespace roots, imported array items, shared module builds,
same-local-name types, and entity behavior. When comparing shared-schema
reports, use `polyxml.toml` modules with `depends_on`, not independent
single-schema invocations: the manifest emits common types once for tested
Python/Go/C# and the C# sources compiled and parsed both roots. Validate
serialized namespace output independently. A controlled XXE probe did not
read an external file, but generated Python silently changed declared and
undeclared `&name;` references to `name` (#126); preserve a no-disclosure
check while fixing that data corruption. Generated Rust's `encode_xml` writes
an already-materialized model to `Write`; incremental production from an
item iterator is a distinct enhancement (#127).

Current xsdata code generation invokes `ruff` as a subprocess. If running an
isolated `xsdata[cli]` virtual environment for same-schema comparisons, put
its `bin/` directory on `PATH` as well as calling its `xsdata` executable;
otherwise generation can fail with `FileNotFoundError: ruff` after writing
files. Record the installed xsdata version and test its generated parser and
serializer before describing a competitive result.

When comparing an upstream issue, run a current competitor release on the
same saved XSD where practical: open issues can be historical. The 2026-09-30
audit's xgen #48 case no longer emitted the reported self-reference, while
current XmlSchemaClassGenerator generated substitution alternatives but failed
`XmlSerializer` construction for identical string-typed alternatives. Record
tool versions and use runtime XML plus an independent XSD validator before a
migration claim.

## Large multi-schema regression caution

### Category 1/2 codec regression checks

- Compile and execute the saved repeated-sequence, substitution-group,
  repeated-branch, and shared-element-name fixtures. Keep document order in
  `SchemaIR.ordered_types` and the existing tagged item stream; do not change
  the source schema's `StructDef.is_mixed` flag to request ordered codecs.
  Preserve this set through imported-IR merges and chameleon namespace rekeys.
- Python global elements need callable root models and expanded-QName checks,
  rather than PEP 695 aliases. Scalar root wrappers also need imports even
  when the original IR contains no structs. Roots colliding with a type or
  another namespace's local name use `*Element` names with numeric suffixes;
  match their `Meta.name` and `Meta.namespace` when selecting a root.
- Go root wrappers delegate to the named type's codecs (including abstract
  `xsi:type` dispatch), but must retain the declared root name on writes.
  Preserve the abstract wrapper's `Selected()` helper.
- Go lexical unions require `UnmarshalText`/`MarshalText` for attributes as
  well as XML element methods. C# lexical proxies must use `XmlAttribute`
  for attribute fields. Test empty union members and invalid lexical input.
- Go temporal wrappers preserve absent timezones and fractional seconds.
  Check updates to the embedded `time.Time` so cached input text cannot
  override an edited value. Calendar ranges remain bounded by Go's parser.
- C# nillable collections need nullable items and `IsNullable=true`, while
  absent non-nillable optional elements need omission methods. Scalar root
  records wrap their value instead of inheriting from primitive/enum types;
  use `default(TargetType)!` to avoid ambiguous record copy constructors.
- Compare W3C groups against the previous compiler with fresh Python
  processes per group when investigating a change in results. The runner
  imports many schemas with repeated class names in one interpreter, so a
  whole-suite summary can mask individual failures. Keep existing schema
  validator/constraint gaps outside a category 1/2 fix claim.

Root-scoped generation (`--root-element`, or `root_elements` in a workspace or
module manifest) parses the complete XSD before filtering `SchemaIR`. Preserve
all named dependencies, including boxed/list refs, while following derived
types only from element/field/union value positions. Inherited base types are
dependencies, not polymorphic entry points: expanding their descendants can
pull an entire large schema back in. On full UCI 2.5, `Entity` should retain
hundreds rather than all 5,558 types. Validate the filtered IR before emission
and measure real selected output with a fresh output directory. Referenced
global element fields may keep a prefixed `xml_name` (for example `t:Head`)
while their `namespace` already contains the resolved URI; strip the lexical
prefix before comparing against substitution-group head QNames.

The corpus's UCI 2.5 `defense_uci` module is a useful real-world regression,
but it contains thousands of types. Run generation and destination-language
compilers one at a time under `scripts/memcap.sh` and a timeout. Do not launch
parallel full-module compiles on a small workstation. For a quick parser sanity
check, `polyxml validate UCI_MessageDefinitions_v2_5_0.xsd` should report
thousands of types, not just a handful. If it does not, inspect the schema
parser's depth accounting after consuming nested element or group subtrees:
the consumed child's closing event is not returned to the parent reader loop.

Multiline `<xs:documentation>` must prefix every physical line when emitted as
Python or Rust comments. Rust raw field identifiers such as `r#type` must use
ordinary identifiers for prefixed local codec variables (`var_type`).

C++ simple type aliases need dependency order (`using AreaType =
DoubleNonNegativeType` requires the base alias first). In C#, unrestricted
named simple types still need declarations because other generated types refer
to them. Empty records that implement `IValidatableObject` need a `Validate`
method, and validation of a derived simple type may need to reach through
multiple `.Value` wrappers. C# choice variants called `Value` or `Equals`
collide with generated record members; rename the generated variant class
while preserving its XML name.

This skill outlines the architectural patterns, options plumbing, and test workflows for the schema compiler and code generators in `crates/polyxml-core/src/codegen/` and `crates/polyxml-cli`.

---

## 1. Codegen Architecture & Layout

All code generators receive a normalized `SchemaIR` from `crates/polyxml-core/src/schema/mod.rs` and generate self-contained source files.

```
crates/polyxml-core/src/codegen/
├── mod.rs                  # Module exports and shared codegen traits
├── rust/mod.rs             # Rust (Zero-copy Cow<'a, str>, Owned, rkyv, streaming codecs)
├── python/mod.rs           # Python 3.12+ (Dataclasses, Pydantic v2, codecs)
├── cpp/mod.rs              # C++20 (Header-only / C++20 Modules, Glaze reflection)
├── java/mod.rs             # Java 22+ (Records, Jackson XML/JSON, sealed interfaces)
├── typescript/mod.rs       # TypeScript 5+ (Interfaces, Zod, Valibot, TypeBox)
├── go/mod.rs               # Go 1.22+ (Structs, xml/json tags, EasyJSON, Sonic)
└── csharp/mod.rs           # C# 12 / .NET 8+ (Record classes/structs, source gen context)
```

Lexical `xs:union` and element `xs:choice` both use `UnionDef`. Check
`UnionDef::is_lexical()` before emitting XML codecs: lexical members parse the
same element text in declaration order, while choice members dispatch on child
element names. The fixture `crates/polyxml-core/tests/fixtures/lexical_union.xsd`
and `test_lexical_union_codegen.rs` cover typed output and runtime parsing.
For C# `XmlSerializer`, a lexical union field needs a typed `[XmlIgnore]`
property plus a string `[XmlElement]` proxy; repeated empty branch tags leave
the typed field unset.

Mixed `xs:complexType` content uses an ordered item union with a `#text`
branch. The text branch is character data, never an element named `#text`.
When changing its codecs, compile and roundtrip a generated fixture with text
before, between, and after both scalar and nested children. Exercise Rust with
both zero-copy settings: owned output still uses `Cow` in attribute parsing,
and generated code must import it. Java and C# nested choice variants must
avoid a class name collision when a branch and its value type share a name.

---

## 2. Standard Pattern for Codegen Options & Backends

When adding or extending features for a target language:

### 1. Define Backend / Mode Enums with `from_str_loose`
Ensure loose string parsing supports lowercase, hyphens, underscores, and aliases:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TargetBackend {
    #[default]
    Standard,
    Alternative,
}

impl TargetBackend {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().replace('-', "_").as_str() {
            "alt" | "alternative" => Self::Alternative,
            _ => Self::Standard,
        }
    }
}
```

### 2. Add Fields to `Options` Struct
```rust
#[derive(Debug, Clone)]
pub struct TargetOptions {
    pub backend: TargetBackend,
    pub custom_flag: bool,
}
```

### 3. Re-export in `crates/polyxml-core/src/codegen/mod.rs`
Always re-export the new enum and options from `polyxml_core::codegen` so `polyxml-cli` and consumers can access them cleanly.

---

## 3. Plumbing into CLI and Workspace Manifest (`polyxml-cli`)

New options must be available in **both** single-schema CLI invocations and declarative `polyxml.toml` builds:

1. **Workspace Manifest (`crates/polyxml-cli/src/config.rs`)**:
   - Add fields to `TargetConfig` and `CodegenTargetConfig`.
   - Update `to_emit_options(...)` to forward the values.
2. **CLI Args (`crates/polyxml-cli/src/main.rs`)**:
   - Add flags to `GenerateArgs` (e.g. `--my-flag`, `--backend <NAME>`).
   - Plumb into `TargetEmitOptions` and `handle_generate(...)`.
   - Plumb into `emit_target_code(...)` for multi-target builds in `run_build(...)`.

---

## 4. Testing & Verification Workflows

### Fast Codegen Unit Tests (Sub-second execution)
Instead of running all 23 workspace test suites, run targeted codegen tests while iterating:

```bash
# Run all codegen tests across all languages in ~1 second
./scripts/test_codegen.sh

# Or run language-specific codegen test suites:
cargo test -p polyxml --test test_rust_codegen
cargo test -p polyxml --test test_python_codegen
cargo test -p polyxml --test test_cpp_codegen
cargo test -p polyxml --test test_java_codegen
cargo test -p polyxml --test test_ts_codegen
cargo test -p polyxml --test test_go_codegen
cargo test -p polyxml --test test_csharp_codegen
```

### CLI Integration Tests
Verify CLI flags and `polyxml.toml` manifest execution:

```bash
cargo test -p polyxml-cli --test test_cli
```

### Real Toolchain E2E Validation
If external compilers are installed on the development machine, run the E2E verification tests:

- **.NET 8**: `dotnet build` executes inside `test_cli_csharp_generation`.
- **Go 1.22+**: `go test` executes inside `test_cli_go_generation`.
- **TypeScript**: `tsc --noEmit` runs inside `test_cli_typescript_generation`.
- **C++20**: `g++ -std=c++20` runs inside `test_cli_cpp_generation`.

---

## 5. Invariants & Common Gotchas

1. **Tarjan SCC Cycle-Cutting**: Recursive and self-referencing types must always be wrapped or boxed (`Box<T>`, `std::unique_ptr`, `v.lazy(...)`, `Type.Recursive(...)`, `z.lazy(...)`).
2. **Preserve Defaults**: When `--backend` or new flags are omitted, code generation MUST remain 100% backward compatible with zero behavioral changes.
3. **Closing Braces**: When dynamically emitting interfaces/structs with varying field annotations, ensure trailing braces and blank lines (`out.push_str("}\n\n")`) are consistently emitted.
4. **Git Pre-Push Hook**: Pre-push verifies all package versions match across 5 files. If remote `main` has advanced due to automated CI releases (`chore(release): X.Y.Z`), run `git pull --rebase origin main` before pushing.
5. **Never Derive Type Identifiers from `qname.local` Directly**: all named-type
   identifiers must flow through the language's `type_ident` helper (see
   section 7) so cross-namespace collisions stay disambiguated.
6. **`xsd:extension` Base-Field Flattening (Rust + Java records)**: targets
   without inheritance must inline the base chain via the shared
   `codegen/mod.rs::flatten_fields(s, ir)` (root first, cycle-cut on revisit,
   most-derived declaration of a schema name shadows inherited duplicates so
   the simpleContent `value` field stays singular). For Rust it is required at
   EVERY field-iteration site: `compute_types_with_lifetime`, `emit_struct`,
   the `has_patterns` probe, and the decode `field_metas` build —
   `encode_xml`/`validate_patterns` follow `field_metas` automatically.
   Missing the lifetime site emits `Cow<'a, str>` in a struct without `<'a>`
   (generated code fails to compile); missing codec sites silently drops
   inherited data. Java's `model_fields` returns the flattened list for every
   mode — records declare it as components, classes re-slice the own-fields
   tail because they `extends` instead. Backends with real inheritance (TS
   `extends`, Go embed, C++/C# `: Base`, Python `class Derived(Base)`) must
   NOT flatten — Python leans on `__dataclass_fields__` (see §11).
7. **simpleContent `FieldKind::Text` codecs**: generated
   Rust codecs now consume `FieldKind::Text` — `emit_text_content_parse`
   reads the element content via `read_element_text` and runs it through the
   same scalar/facet path as elements (`var_value = Some(...)`; patterns from
   the extension base are enforced via `validate_patterns`), and
   `emit_text_content_serialize` writes the value between Start/End. Only
   scalar-backed Text is emitted. Struct-typed text values are unsupported;
   do not describe them as working without a generated round-trip test.
   The module allow-header includes `unused_assignments` because the text path
   assigns its slot unconditionally. Java records cannot `extends`, so
   `model_fields` inlines `flatten_fields`.
   Regression-locked by `test_rust_simple_content_text_codec` plus runtime
   round-trips (numeric, patterned, string, bad-number rejection).
8. **Text events arrive split — readers must accumulate**: quick-xml emits
   `a &amp; b` as `Text("a ")` + `GeneralRef("amp")` + `Text(" b")` and CDATA
   as its own event, so `read_element_text` (both `zero_copy` variants)
   appends every segment and resolves refs exactly like
   `parser.rs::append_general_ref` (char refs → `resolve_char_ref()`,
   predefined → `escape::resolve_xml_entity()`, otherwise the raw name —
   lenient, never an error). Never overwrite the accumulator per event (the
   pre-fix last-wins helper silently dropped data) and never pair split-text
   assembly with `trim_text(true)` — per-segment trimming eats spaces adjacent
   to refs (`x &amp; y` → `x&y`); trim once on the assembled buffer (see
   `transcoder.rs`, which now also unescapes attribute values and resolves
   general refs instead of dropping them via `_ => {}`). Regression-locked by
   `test_rust_read_element_text_accumulates_refs_and_cdata` and
   `test_schemaless_preserves_refs_cdata_and_attr_entities`.

## 6. Java/C# Model Styles and Direct Java Codecs

- `--style record|pojo|class` is shared by Java and C#. Records remain the default;
  `pojo`/`class` select mutable models. Java's `--feature builder,direct-codec`
  must also be forwarded as `features = [...]` in both manifest forms.
- Java `--feature validation` emits Jakarta Bean Validation annotations on
  records and POJO fields. Keep the option disabled by default and add it to
  the CLI feature allowlist and manifest path together. The generated Java
  still imports `java.util.regex.Pattern` for constructor checks, so emit the
  Bean Validation `@Pattern` with its fully qualified name to avoid the import
  collision. Compile generated output against `jakarta.validation-api` when
  changing this path.
- Gregorian partial date builtins are already `PrimitiveType` variants. The
  schema parser's `add_gregorian_types` pass wraps uses in named simple types
  with patterns so existing pattern validation paths run across generators.
  When extending the patterns, keep the core streaming converter and the
  generated validators aligned, including the `14:00` timezone boundary and
  month-specific day limits. Java's `build_facet_checks` and TypeScript's
  Zod/Valibot/TypeBox facet helpers must classify these primitives as strings.
- Java mutable models/builders live in `java/models.rs`; StAX companions live in
  `java/codec.rs`. Inheritance must share field-name allocation between accessors,
  builders, and codecs. A derived builder extends its base builder and overrides
  inherited fluent methods with a covariant return type.
- Call `validate_direct_codecs` before CLI emission. Wildcards/dynamic `anyType`
  cannot be silently dropped. The direct reader must consume exactly one element
  and leave the cursor on END_ELEMENT; nested codecs rely on this contract.
- C# mutable class style must apply to schema complex types, simple wrappers,
  union branches, and root wrappers. Preserve XML/JSON attributes and invoke
  inherited validators.
- Real Java compilation/round trips run in `test_java_codegen`; C# mutable XML and
  source-generated JSON round trips run in `test_csharp_codegen`. Jackson/JAXB
  interoperability is tested by `mvn -f benchmarks/java/pom.xml clean test` after
  building the CLI. This requires Maven network access for dependencies initially.
- `benchmarks/java` contains JMH read/write and mutation workloads. Use JDK 22+
  and `-Ppanama` for native comparisons. Newer JDKs require the explicit JMH
  annotation-processor path or the jar lacks `META-INF/BenchmarkList`. Clean the
  Maven target when switching profiles. See its README for workload limits;
  synthetic scalar projections are not full ISO 20022/UCI schema benchmarks.
- If present, the sibling checkout `../polyxml-finance-examples` carries the ISO 20022
  `schemas/finance/pacs_008_core.xsd`. Its `scripts/generate_all.sh` regenerates
  all 7 targets from whichever local `target/{debug,release}/polyxml` is newer;
  a clean `git status` there proves byte-for-byte output parity after codegen
  changes. The same schema is the best large-schema smoke for
  `--style pojo --feature builder,direct-codec`: `javac` the output and round-trip
  `data/pacs_008_customer_credit_transfer.xml` through the generated root codec.
- `benchmarks/java -Ppanama` needs JDK 22+.
  Set `JAVA_HOME`/`PATH` to a suitable JDK and run
  `cargo build --release -p polyxml-c` first so `-Djava.library.path=target/release`
  resolves `libpolyxml.so`. `mvn clean` deletes `target/*.json`, so always rerun
  the JMH smoke (`-p batchSize=10 -wi 0 -i 1 -r 100ms -f 1 -foe true`) after a
  clean build; its JSON lands in the gitignored `benchmarks/java/target/`.

## 7. Cross-Namespace Type-Name Disambiguation

Flat (single-file/module) output must survive two types that share a local name
across namespaces (e.g. `urn:a|Address` + `urn:b|Address`). The mechanism lives
in `crates/polyxml-core/src/codegen/mod.rs`:

- `build_type_name_map(ir, name_of)` assigns a unique identifier to every type
  in `ir.types`: the first name in `BTreeMap` iteration order keeps the bare
  name; collisions get numeric suffixes (`Address`, `Address2`, `Address3`, …).
- The map is installed per thread via `set_type_name_map(...)` into a
  `thread_local!` `RefCell<HashMap<QName, String>>` and read with
  `lookup_type_name(qname, fallback)`. Names absent from the map (references to
  unloaded external types) fall back to the language's default derivation —
  single-namespace schemas are therefore byte-for-byte unchanged.

Rules when touching any codegen:

1. **Install the map at every public entry that emits types.** For most
   languages that is `generate_module`; Java also needs it in `generate_files`
   (it calls `generate_single_type` directly) and C++ needs it in both
   `generate_header` and `generate_module_unit` (both are real emitters).
2. **Each language defines one `type_ident(q: &QName)` helper** whose fallback
   equals the language's historical derivation exactly (`AsPascalCase` for
   Rust/Python, `to_java_type_name`, `to_cpp_type_name`, `to_csharp_type_name`,
   `to_ts_type_name`, `to_go_type_name`). Route every *identifier* site through
   it: declarations, `TypeRef::Named` mappings, base/inheritance clauses,
   `typeof`/`extends`/`Schema` references, codec class names, and the
   `declared_names` sets used to guard root-element aliases.
3. **Never disambiguate wire names.** XML tags, `xml_name`, `[XmlRoot(...)]`,
   Jackson's `localName`, Pydantic `Meta.name`, `variant_name`s, and root
   aliases derived from *element* names keep using raw local names.
4. Special-shaped sites must preserve historical output when nothing
   collides: C++ `{enum}_from_string` uses the snake-of-local stem unless the
   disambiguated name differs; TypeScript `{name}Schema` refs swap the inner
   name only (declarations derive from the same `type_ident`).
5. Regression lock: `crates/polyxml-core/tests/test_schema_audit.rs` asserts
   both colliding types surface as `Address` + `Address2` in all 7 languages.

## 8. Python Multi-Pattern Validation

Pydantic's `Field(pattern=...)` accepts a single regex, and multiple
`StringConstraints(pattern=...)` annotations do NOT AND (the last one wins).
When a simple type carries **more than one** pattern facet (common since
derived types inherit their base's patterns at parse time):

- `emit_simple_type` appends `AfterValidator(_polyxml_patterns(r"p1", r"p2"))`
  to the `Annotated[...]` alias (single-pattern types keep the plain
  `Field(pattern=...)` kwarg, anchored for full-value matching).
- `needs_pattern_validator(ir)` gates emission of the module-level
  `_polyxml_patterns` factory helper, the `import re` line, and the
  `AfterValidator,` prefix on the pydantic import — keep these three in sync.
- Semantics: each pattern must match the entire lexical value (`fullmatch`);
  patterns inherited across derivation steps all apply (AND).
- Field-level facets (`FieldDef.facets`) are never populated by the parser
  today, so `emit_struct`'s field path needs no `AfterValidator` wiring; if
  that ever changes, mirror the type-level handling there.

## 9. Unified CLI Options

- `polyxml-cli/src/options.rs` normalizes and validates backend/style/features
  for direct generation and both manifest forms. Validate all targets before
  schema parsing, dry-run success, or output creation. Core `from_str_loose`
  parsers can still return `None`; the CLI must reject invalid values before
  emitter defaults are applied.
- New opt-ins belong in repeatable `--feature` and manifest `features` arrays.
  Keep CLI and manifest parity; unknown manifest keys and unsupported target
  options must fail clearly.
- `--zero-copy` (and manifest `zero_copy`) stays a first-class bool because
  `--feature` cannot express `false`; it alone selects owned Rust output, and
  `--zero-copy=false --feature zero-copy` is still rejected as a contradiction.
- Defaults remain unchanged, including C# record classes, Rust zero-copy, and
  Python slots/kw-only. Expose only implemented styles/features. Python plain
  classes and C# mutable structs are not yet generator capabilities.
- Rust features today: `zero-copy`, `rkyv`, `phf`. `--feature phf`
  emits a per-struct `__{Struct}ElementId` enum plus a
  `__{STRUCT}_ELEMENT_DISPATCH: ::phf::Map<&'static str, ...>` static built with
  `phf_codegen`, and routes both `Event::Start` and `Event::Empty` child arms
  through `.get(e.local_name().as_ref())`; union branch tags all map to one
  variant. Default (feature off) output must stay byte-identical — lock both
  directions in the codegen test (`test_rust_phf_dispatch_emission`). Attr,
  union, and enum-value `match` sites are intentionally left as `match`.
  Consuming crates need `phf = "0.14"`. Benchmarks/docs live in
  `benches/tag_dispatch.rs` (regenerate fixtures via
  `scripts/gen_tag_dispatch_fixtures.py`) and
  `docs/benchmarks/rust-phf-dispatch.md`; hardware counters on hosts without
  the `perf` binary go through `scripts/perf_stat.sh` (`perf_event_open`).
- **Cap heavy builds/benches with `scripts/memcap.sh`.** Benchmark entry
  points (`benchmarks/run_all.sh`, `benchmarks/cli/benchmark.sh`,
  `scripts/perf_stat.sh`) already re-exec through it; ad-hoc
  `cargo bench`/`--release` builds of large generated crates should be
  wrapped too. It caps the tree at `POLYXML_MEMCAP_PCT` (default 60%) of
  available RAM in a systemd scope (`MemorySwapMax=0`, `ulimit -v`
  fallback): an over-limit build gets OOM-killed inside its cgroup
  (exit 137) instead of freezing the host — uncapped 1500-element builds
  froze a 7.7-GiB WSL box repeatedly before this guard existed. Wrap
  measure/compile-size scripts per build step (own scope each) so one
  OOM kill doesn't abort the rest; `POLYXML_MEMCAP_LEVEL` (nesting depth)
  is set automatically so wrapped scripts can't re-exec in a loop, and
  `POLYXML_MEMCAP_DISABLE=1` opts out. Known historical datum: generated consumer
  crates at **600 and 1500 elements need >=3.2 GiB for a single `rustc`**
  (both the `match` and `phf` variants — it scales with field count, not
  match-arm count), so the original compile-time/size measurements were
  deferred to #55 and must not be retried uncapped on
  8-GiB-class hosts. The numeric-field follow-up completed both tiers at a
  deliberate 70% cap on the 16-GiB WSL instance, with about 6.7 GiB peak RSS.
  The October 2, 2026 `benchmarks/rust-phf-compile/run.py` retry also
  exceeded a 5,597-MiB cap at 600 fields (match, rustc 1.99.0), so even a
  16-GiB host does not guarantee success. The runner requires systemd
  cgroups and stops on failure; retain scope/kernel evidence.
  Increase the cap only deliberately after checking Linux/host headroom.
  WSL's visible RAM may be lower than Windows physical RAM; retain a
  substantial reserve outside the scope. See the dispatch study for raw logs.
- Shared CLI options apply to every `--lang`, not just the preceding one. Use
  per-target manifest entries for heterogeneous configurations. Schema-less
  `generate` must not silently ignore generation overrides.

- Shell completion adapters live under `polyxml-cli/src/completions/`. Their
  hidden `__complete` helper filters candidates through the same option resolver
  used for generation; preserve multi-target intersection and language aliases.
  Bash is exercised in CLI integration tests. Zsh/Fish tests run when those
  shells are available on PATH, and skip otherwise.
- `benchmarks/cli/benchmark.sh` uses hyperfine with `--shell=none` to avoid shell
  calibration error for sub-5ms startup measurements. Results are fresh processes
  with warm OS caches, not machine-reboot or cold-disk startup measurements.
- `benchmarks/cli/startup.py` alternates warm `posix_spawn` runs
  with runs after `POSIX_FADV_DONTNEED` on the CLI executable. Run a release
  build under `scripts/memcap.sh` first, then check the recorded major-fault
  counts before calling the second group executable-cache cold. The script
  does not evict shared libraries or reproduce a post-reboot host; see
  `benchmarks/cli/README.md` for results and limits.

- Color diagnostics respect `NO_COLOR`. When checking terminal color in a PTY,
  unset `NO_COLOR` in the test subprocess and use a color-capable `TERM`; test
  the opt-out separately.

## 10. Pattern OR/AND Semantics & Enforcement

W3C XSD combines `xs:pattern` two ways: multiple patterns **within one
`<xs:restriction>` are OR'd**; patterns inherited **across derivation steps are
AND'd**. The IR keeps `RestrictionFacets.patterns: Vec<String>` flat — the
parser encodes each restriction's alternatives as ONE regex group:

- `schema_parser` collapses `patterns.len() > 1` per restriction into
  `(alt1)|(alt2)` (a singleton stays verbatim), then inheritance prepends/appends
  each derived step's entry. Downstream code just ANDs the flat list — each
  entry is already an OR-group. **Never** AND the pre-collapse alternatives.
- Round-trip tests must serialize `RestrictionFacets`, not the whole
  `SchemaIR`: `SchemaIR.types` is a `BTreeMap<QName, _>` and serde_json rejects
  struct map keys ("key must be a string"). This is pre-existing and unrelated
  to facets.

Shared helpers in `codegen/mod.rs` (use them; don't re-derive):

- `primitive_base(ty, ir)` — resolves a simple alias chain to its primitive,
  cycle-safe. Patterned simple types must emit their **primitive** base (e.g.
  `std::string`, not the alias) so validators/codecs operate on real scalars.
- `patterned_simple(ty, ir)` — the patterned `SimpleTypeDef` behind a field
  type, unwrapping `Boxed`/`List`.

Per-language enforcement (all gated so unpatterned schemas stay byte-identical):

- **Rust**: free `validate_{Type}_patterns(&str)` fns with `OnceLock<Regex>`
  per pattern; structs gain `validate_patterns()` called from `from_xml`/
  `to_xml`, attributes/text/empty-element paths call the free fn directly.
- **Go**: `Validate() error` on patterned aliases (+ `UnmarshalText`/
  `MarshalText` for string bases so `encoding/xml` enforces on read/write);
  struct `Validate` recurses into fields, lists, and pointers. Imports
  `regexp`/`fmt` only when patterns exist.
- **C++**: free `validate_{Type}_patterns(std::string_view)` with function-local
  `static const std::regex`; struct `validate()` calls it. **Inline field
  expressions into the call** — binding `const auto& value = <field>;`
  self-references when the field is named `value` (range-for over a member named
  `value` is fine). `#include <regex>` is conditional on `ir`.
- **Java**: `Pattern.compile(p).matcher(v).matches()`; direct codecs resolve
  patterned aliases through `primitive_base`.
- **C#**: `Regex.IsMatch` with `\A(?:...)\z` full-value anchors;
  escape `\` before `"` in the pattern string.
- **TypeScript**: Zod chains `.regex(...)` checks; Valibot chains
  `v.regex(...)` checks; TypeBox emits one `Type.String({pattern})` or an
  intersection of several such schemas. Escape `/` inside regex literals.

Execution tests live in `crates/polyxml-core/tests/test_pattern_codegen.rs`
(wired into `scripts/test_codegen.sh`): they **run** generated Go/C++/Rust/
Python/Java/C# validators against values matching only one alternative, values
satisfying two restriction steps, and reject lists/optionals carrying a bad
element. The TS leg needs `POLYXML_TS_TEST_MODULES` pointing at a node_modules
with `zod@3 valibot@1 @sinclair/typebox@0.34 typescript@5` and skips otherwise.

### Composite module compile checks

For composite module builds, compile generated Java and Go source rather than
checking import text alone. The small `shared_modules` CLI fixture covers an
imported type whose local name is disambiguated (`Person2`), which Java must
reference by its fully qualified owning package name. The NeTEx corpus can
also expose parser placeholders left by `<xs:element ref>` in both fields and
choice branches. Resolve those references against global element declarations
after includes/imports and group expansion, before cycle detection. The
CLI's Java import scan must include fields inherited through external base
types because Java records flatten those fields into derived records. The
Gregorian builtin synthesis pass runs once per parser frame, so it must leave
the base type of an already synthesized builtin untouched; otherwise Go emits
self-referential aliases such as `type GDay GDay`.
When the schema parser consumes a nested element or group subtree, decrement
the enclosing type/group depth exactly once; otherwise later top-level
declarations disappear from generated modules. Global `<xs:attribute ref>`
fields need their declared attribute type resolved after imports and includes.
Go lexical unions may contain other lexical unions: parse the nested branch
through its XML decoder and serialize its selected lexical value, rather than
casting text to the nested union struct.
Go root element aliases must reserve normalized names already used by types
and earlier aliases: distinct XML names such as `item` and `item_` both become
the Go identifier `Item`.

TypeScript mixed-content derived types may redeclare `items` with a wider
union. An `interface extends` rejects that override (TS2430), and a plain
intersection keeps the narrower inherited type. Use `Omit<Base, "items"> &`
for inherited field names redeclared by the derived type, including in the
`use_interface = false` mode; the HL7 CDA corpus compile check exercises it.
The RailML TypeScript corpus check exercises implicit `xml:lang` attributes
from Dublin Core and a choice-bearing derived type (`eTrackNode`) that must
remain a struct so its children can extend it.

C++ dependency order must include mixed-content unions and inheritance.
`std::vector<Struct>` can hold an incomplete struct at declaration, while
`std::variant` aliases and `std::optional<Struct>` require complete types.
For a cyclic dependency, box a struct field or a union branch with
`std::unique_ptr` and recompute the order; only cut an edge when a path leads
back to its owner. A cyclic edge may point backward in a provisional order,
so use graph reachability rather than position to choose cuts. The HL7 CDA
`datatypes-base.xsd` intentionally comments out its local `cs` declaration
and includes `datatypes-rX-cs.xsd` instead. The corpus must retain that
included file; uncommenting the local declaration diverges from HL7 and
would duplicate `cs` when the include is present. Verify with the bounded
`hl7_cda` C++ module check.

## 11. Python Abstract Meta & Runtime Type Discovery

Core streaming nil handling must check `xsi:nil` before calling
`resolve_record_schema` for nested complex fields, including list items and
both `Event::Start` and `Event::Empty`. Otherwise a nillable abstract type
raises `requires xsi:type` before it can become `PolyValue::Null`.
`test_xsi_type::nil_abstract_complex_type_skips_concrete_dispatch` covers
both element forms and list fields.

`xsi:type` dispatch is a **dynamic-runtime feature** (see
`polyxml-core-engine` skill §5 and `docs/guides/polymorphism.md`); the
codegen only has to expose two facts to the PyO3 layer:

- **`Meta.abstract = True`** is emitted for `is_abstract` structs in
  `codegen/python/mod.rs` (inside the `emit_meta` block, after
  `namespace`). PyO3 reads it in `extract_schema_from_class` →
  `builder.is_abstract(true)`, which turns unknown `xsi:type` values into
  loud errors instead of silent truncation. Only abstract types may emit
  it — locked by `test_python_abstract_meta_emission`.
- **Class inheritance must stay intact**: `emit_struct` already emits
  `class Derived(Base):`, and PyO3 builds a derived class's schema from
  `__dataclass_fields__`/`model_fields`, which include inherited fields —
  do not flatten inheritance away or variant schemas will miss base fields.

Key gotchas when touching this area:

- Variant discovery (`discover_variants`) must run **after** both global
  cache inserts in `get_or_create_schema_meta`, otherwise a subclass field
  typed as the base class re-enters extraction and recurses infinitely.
- Variant keys come from `Meta.name` (the XML type name), not the class
  name — codegen may mangle class identifiers (`type_ident`) while
  `xsi:type` always carries the wire QName local part.
- `polyxml.serialize` takes an optional **`target_type`** (pyo3 + wrapper):
  given the declared base, a concrete instance is converted against the
  base schema and the variant's record triggers `xsi:type` re-emission at
  the root; omitted, the instance's concrete schema is used (no selector).
  Both branches are covered in `tests/test_xsi_type.py` (11 tests, kept at
  100% statement/branch coverage).

Global attribute references must retain their local name and namespace URI.
C# `XmlAttribute` (including lexical proxies) needs `Namespace`; Go attribute
XML tags must include the namespace even when it matches the owning type's
namespace. Regression: `test_qualified_attributes` executes both runtimes
with alternative prefixes and namespace declarations on child elements.

Optional element defaults apply only to present empty elements; optional
attribute defaults also apply when absent. Python emits XML default metadata
separately from constructor defaults, and runtime FieldSchema carries the
lexical default. C# defaulted scalar elements use a string XML proxy so an
empty lexical value can be substituted before XmlSerializer's typed parser.
Exercise both self-closing and explicit empty tags with boolean/int/string.

`SchemaIR.content_models` retains source choice/sequence particle boundaries
independently of flattened fields. Python Meta.content_pattern and Go XML
codecs enforce supported element-only choices on reads and writes. Mixed
content, model groups, all-groups, and derivations need separate handling;
do not infer complete content-model validation from these patterns. Chameleon
namespace adoption must rekey both model owners and particle element QNames.

Named xs:list types retain TypeRef::List as their simple base; they are one
whitespace-separated lexical value, independent of maxOccurs collections.
Python `tokens` metadata maps annotated lists to native ScalarType::List;
Go named slices implement text codecs, and C# wrappers expose typed Value
lists with a text proxy. Validate each item (including named restrictions
and enum lexical values), and reject items containing XML whitespace on
output. Empty lists remain distinct from omitted optional elements.

Schema validity gates run before generation/dry-run: illegal direct children
of simpleType, undeclared QName prefixes, unknown XSD built-ins, and unresolved
type references are errors. Resolve references only after root-frame imports
and forward declarations are merged. File parsing wraps diagnostics with the
source path; grammar checks include line numbers. Annotation payloads must
remain opaque to type-reference checking.

XSD 1.1 xs:alternative conditional types are explicitly unsupported: schema
preflight returns a source/line diagnostic and CLI validation/generation/
dry-run fail before emitting a weaker model. Do not silently accept it, and
ignore similarly named content inside annotation payloads.

Temporal lexical unions must accept both timezone-free and zoned dates and
date-times. Go date union parsing needs both date layouts. C# DateOnly and
DateTimeOffset erase lexical distinctions, so retain parsed spelling alongside
a canonical value snapshot and discard it when the typed value changes. Use
invariant XML conversion, reject culture-specific date strings, and validate
generated round trips against an independent XSD engine. Do not claim support
for the entire XSD temporal range from platform date types.

UBL C# acceptance requires `python3 scripts/verify_ubl_csharp.py`: it hashes
all 16 official XSDs, checks import closure, compiles full/root-scoped records
and classes under .NET 8, and independently validates an Invoice round trip.
Root wrapper names need a separate reserved-name map covering emitted types
and enum helpers. Preserve XML QNames while disambiguating C# identifiers.
Derived simple-content models reuse inherited text and repeated attribute
members; duplicate properties can compile yet fail XmlSerializer reflection.
Ordinary element metadata must include referenced namespaces. C# DateOnly,
TimeOnly, DateTimeOffset and TimeSpan text needs XML string proxies, retaining
parsed lexicals only while the typed value stays unchanged.

Go buffered content-model codecs must preserve a parsed XMLName when marshaling
an instance directly. Validate the buffered alias output, then encode the alias
with its resolved start element; forwarding decoded tokens can redeclare XML
namespaces and change output. Keep the namespaced unbounded-choice runtime
regression in the full workspace gate.

Integer mapping changes require `scripts/verify_unbounded_integer.py` (run with
`uv run --with lxml python`) after building the CLI and reinstalling Python.
It compiles/runs all seven targets and independently validates XML from available
codecs. C++/TypeScript checks cover storage and schema validation, not XML codecs.
Unbounded integers use lexical strings in Rust/C++/C#/TypeScript, Go text-codec
string wrappers, Java BigInteger, and Python int. Update Rust lexical dispatch
and lifetime inference together; changing only its primitive mapping can emit
`Cow::decode_xml`. Go defined aliases need their own text methods. C# strings
need XML proxies to enforce integer lexical grammar on both read and write.

### Spring Boot 4 / Jackson 3 verification

Java `jackson3` (aliases `jackson-3`, `jackson_3`, `spring-boot-4`) shares
model generation with Jackson 2 but imports XML annotations from
`tools.jackson.dataformat.xml.annotation`. Core annotations stay in
`com.fasterxml.jackson.annotation`. Preserve `jackson`/`spring`/`spring-boot`
legacy selection. CLI and manifest equivalence is covered by a CLI regression.

Run `scripts/verify_spring_boot.sh` with Maven and Java 25 on JAVA_HOME/PATH.
The temporary Maven fixture pins Boot 4.1.1 with managed Jackson 3.1.5 and
executes actual JSON/XML HTTP converters, Jakarta validation, mapper round
trips, and direct codecs for generated records and POJOs. It is deliberately
separate from the fast suite because uncached Maven dependencies need network.
Records with simpleContent and attributes need `@JacksonXmlText` and an
`XmlMapperBuilderCustomizer` setting `nameForTextElement("value")`; Jackson's
default empty text name cannot bind a record's value creator property.
Do not replace that XML text annotation with a child element annotation.

Jackson 3 sorts properties alphabetically by default. Emit `@JsonPropertyOrder`
from flattened schema fields for its struct models, including inherited fields,
or valid reads can produce XSD-invalid XML on writes. The Spring fixture checks
returned XML against the source XSD independently using the JDK validator.

C# positional record properties named Equals/GetHashCode/ToString collide with
synthesized methods (UCI QueryPET's Equals field exposed CS8866). Suffix their
identifiers with Value before applying per-struct uniqueness; preserve original
XML names. The particle wire round-trip test covers these names and an explicit
EqualsValue collision in both record and mutable-class modes.

Rust binary primitives preserve hex/base64 XML lexical strings, matching the
core runtime ScalarType::String mapping. Mapping them to Cow<[u8]>/Vec<u8>
without binary codecs causes type errors in UCI fields and choice branches.
Verify required/optional/repeated elements, attributes, simple content and
choice round trips in both owned and borrowed generator modes.

Composite module builds keep the closure of owned types and global element
references. Imported parser frames may synthesize unused ordered-content helpers
after downstream substitutions; those must not trigger missing-owner errors when
the canonical owner's definition won the merge. Traverse struct bases/fields,
union branches, and simple bases; reachable imported types still need owners.
The CLI substitution-helper fixture covers both outcomes.

Python triple-quoted docstrings must escape every double quote, not only triple
quote sequences. A trailing quoted word (NeTEx's Default is "Outbound") otherwise
creates four adjacent quotes and invalid syntax. The AST regression verifies
exact enum/class documentation text, quotes and backslashes, for both backends.

## C# optional enum attributes

XmlSerializer rejects nullable enum properties marked XmlAttribute. Keep the
nullable typed property XmlIgnore and expose a string XML lexical proxy. Null
omits the attribute; parse via the enum's XmlEnum lexical names, preserving the
namespace. Exclude the proxy from JSON and retain the typed property's JSON
wire name. Test present and absent imported inline enums with Go and both C#
record/class modes; compilation alone misses this reflection-time error.

Module builds retain ordered-content markers from directly owned definitions.
An imported context can synthesize a reachable mixed-content union in an already
owned namespace after substitutions become visible. Such helpers (identified
by their #text branch) may use a unique namespace owner; ambiguous namespaces
and ordinary imported declarations still need explicit owners. Unused imported
helpers are pruned before this step. Test multi-schema modules and ambiguity.

## Generated warning triage: choices and inherited names

Rust tagged-union fields with an empty XML name represent inline choices;
fields with a nonempty XML name represent an element wrapping a choice.
Dispatch named wrappers by the field name and preserve their outer element on
writes. Reading a wrapper must consume its end and accept exactly one branch.
Cover required, optional, repeated, empty-string branches, invalid alternatives,
owned/borrowed strings and PHF dispatch, with unreachable-pattern warnings
promoted to errors in a generated consumer. UCI's repeated Percentage and
NumberOfSteps warnings exposed incorrect flattening of named wrappers.

C# reserves generated property names from every ancestor before naming new
members. NeTEx's inherited nameOfClass attribute and NameOfClass element are
distinct XML members despite their identical normalized C# identifier. Suffix
the derived identifier while retaining XML metadata; adding new or suppressing
CS0108 does not establish correct serialization. Check record/class builds,
base-reference access, XML and JSON round trips, and multi-level collisions.

## Full NeTEx C# and empty module imports

Ordered-content C# emission must find a real mixed item union before returning
through the custom codec path. Flatten inherited item streams and attributes
before removing inherited fields; otherwise an XmlRoot attribute is emitted
without a declaration and leaks onto the next type. Mixed IXmlSerializable
models also need validation methods. Record/class validation uses virtual and
override methods, calling base.Validate, so base constraints remain effective.
Readonly record structs cannot have virtual methods.

Allocate unique nested choice identifiers in every declaration, annotation,
parser and formatter. Reserve referenced top-level type names against shadowing;
retain original XML names. Deduplicate identical QName/type branches only in
ordered item streams, where repeated occurrences retain their list positions.
Keep different XML names/types distinct. Compute the names once per branch list
rather than rescanning all prior branches for every lookup on large schemas.

C# generated list codecs need their own System.Linq import; consumer implicit
usings are not guaranteed. Nested lexical unions call Parse/ToXmlString, and an
unrestricted string fallback ends parse dispatch to avoid unreachable code.
Apply enum restriction string facets to ToXmlValue, preserving the outer member
name in validation errors. Required default/fixed value types cannot be checked
against null; required unbounded-integer proxies must reject missing values,
while optional proxies retain nullable types. Mixed attribute codecs parse and
format XML lexicals via XmlSerializer, including booleans, enums and simple
wrappers, and preserve attribute namespaces.

Rust modules with no public declarations omit their empty glob reexport. Go
imports bytes only when emitted code uses bytes, since external ordered types
can remain in the IR without being emitted locally. Compile reduced generated
modules to catch unused imports rather than just inspecting source text.

Rust Serde field names also need uniqueness within each generated struct. XML
attributes/elements or multiple wildcards can share an XML name while retaining
distinct model fields. Preserve the first JSON wire name and disambiguate later
collisions with the generated Rust field name, reserving all real XML field
names before allocating suffixes. Do not alter XML metadata or codecs. Cover
borrowed and owned JSON round trips and a collision with a real named element;
deny unreachable patterns in the generated consumer. UBL exposed duplicate
Serde wildcard names even after its unused module reexport was removed.

The inherited mixed-stream regression covers an extension adding attributes.
An extension adding new child elements still has a separate gap: the derived
item union can contain only its own branches, so flattening selects that stream
and omits inherited children. The independently valid saved case is
research/fixtures/mixed_content_extension_children.{xsd,xml}. Preserve the
complete extension particle when fixing it and distinguish restrictions before
merging ancestor branches; do not claim full mixed-inheritance support from
compile success or the added-attribute regression alone.

## Generated model documentation and Serde

Keep an explicit generation-to-XML/JSON section in every language guide. Rust
XML methods use quick-xml directly; Serde attributes describe JSON, not a
third-party Serde XML mapping. Borrowed XML models do not make JSON parsing
zero-copy: the current Cow string derives lack serde(borrow). Check real
owned/borrowed XML and JSON round trips, use the declared root name explicitly
when it differs from its type name, and distinguish C++/TypeScript model-only
generation from their runtime binding schemas. Strict docs builds catch broken
anchors and misplaced code fences, including guides appended after old fences.

When Python generated-model tests use an isolated absolute CARGO_TARGET_DIR,
resolve the CLI under that target, not the checkout's default target directory
or an unrelated global executable. Relative target values resolve from the
checkout because the helper builds with that cwd. The generated-model test
helper now honors this override and builds the matching CLI if needed. Keep
workspace targets owned by one worktree; switching checkouts in a shared target
can leave stale unchanged executables even after a core compile message.
