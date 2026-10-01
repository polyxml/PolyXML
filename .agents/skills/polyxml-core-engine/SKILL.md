---
name: polyxml-core-engine
description: >-
  Use this skill when developing, refactoring, or optimizing the pure Rust core XML engine in crates/polyxml-core,
  streaming quick-xml reader/writer events, using lexical-core for conversions, or designing schema IRs.
---

# PolyXML Pure Rust Core Engine Architecture & Patterns

This skill documents the high-performance design patterns and strict constraints for
`crates/polyxml-core`.

## 1. Core Engine Invariants

1. **Zero Python Dependencies**:
   - `crates/polyxml-core` must remain 100% pure, idiomatic Rust.
   - Never import `pyo3`, Python runtime types, or Python C-API dependencies in this crate.
2. **Streaming Event Model**:
   - Avoid building DOM trees in memory during parsing.
   - Stream tokens using `quick-xml::events::Event` (`Event::Start`, `Event::End`, `Event::Text`, `Event::Empty`, `Event::CData`, `Event::GeneralRef`).
3. **Zero Intermediate Allocations**:
   - Parse integers, floats, and booleans directly from raw byte slices `&[u8]` using `lexical_core::parse` and custom byte parsers.
   - Use `smallvec::SmallVec` for small, bounded collections of attributes or namespace declarations on the stack.
4. **Complete Event Coverage in Read Loops**:
   - quick-xml SPLITS character data: `a &amp; b` arrives as `Text` + `GeneralRef` + `Text`, and CDATA is its own event. Any accumulating loop (`parser.rs`, `transcoder.rs`, generated codecs) must handle `Text`/`CData`/`GeneralRef` or content silently vanishes.
   - Resolve refs leniently like `parser.rs::append_general_ref`: `is_char_ref()`/`resolve_char_ref()` for `&#NN;`/`&#xNN;`, `escape::resolve_xml_entity()` for `amp`/`lt`/`gt`/`quot`/`apos`, else push the raw name (never error — a document the core accepts must not fail in generated code).
   - Never enable `trim_text` on a reader that assembles split text: per-segment trimming loses spaces around refs (`x &amp; y` → `x&y`). Trim the assembled buffer once at element end (the transcoder `End` arm).
   - Attribute values arrive raw too — always `escape::unescape` them (`parse_attributes`, the transcoder's Start/Empty attr arms).
   - XSD Gregorian partial dates map to `ScalarType::XmlGregorian` and pass
     through `converters::valid_gregorian` on parse. Serialization checks the
     same scalar before writing attributes, text, and elements. Keep this
     byte-level validator aligned with schema parser synthetic simple-type
     patterns: `+14:00` is valid, `+14:01` is not, and `gMonthDay` needs
     month-specific day limits.
5. **Document Transcoding Is Buffered**:
   - `transcoder::xml_to_json` and `json_to_xml` accept complete byte slices and return complete byte vectors. The schema-free path constructs a `serde_json::Value` tree; the CLI reads all stdin or file input before calling it. Pipes do not make this API incremental or zero-copy. Keep CLI help and docs distinct from `XmlItemStream` and the Wasm record-stream API.

## 2. Testing & Verification

Run tests and clippy specifically against `polyxml-core` (package name `polyxml`):

```bash
# Run core crate unit tests
cargo test -p polyxml

# Check formatting
cargo fmt --check

# Strict warnings-as-errors clippy audit
cargo clippy -p polyxml --all-targets -- -D warnings
```

## 3. Benchmarking Core Engine

To run benchmarks on the pure Rust engine against XML payloads:

```bash
cargo bench -p polyxml
```

## 4. Schema Parser Architecture (`schema_parser/mod.rs`)

The XSD parser runs **frame-based post-passes**: `parse_str_internal` wraps
`parse_str_body` with a frame-depth counter, and every frame end (schema root,
include/import) runs `inherit_pattern_facets` → `expand_group_refs` →
`resolve_cycles` (last, so cuts see the final graph).

Key mechanisms (regression-locked in `crates/polyxml-core/tests/test_schema_audit.rs`):

1. **Named groups**: `<xs:group name>` bodies are captured into
   `GroupDef`/`PendingGroupRef` state; `<xs:group ref>` usages expand via
   `parse_group_body` + `consume_inline_element_type` at frame end.
2. **Enum dedup**: duplicate `<xs:enumeration>` values are dropped; distinct
   values whose identifiers collide get numeric suffixes.
3. **Nested inline types**: an `<xs:element>` carrying an inline
   `complexType`/`simpleType` is extracted as a uniquely named top-level type
   (`unique_type_name`) — its fields must never leak into the parent struct.
4. **simpleContent**: `<xs:extension>` emits a `Text` field named `value`
   carrying the extension base type (decoded/encoded by generated Rust codecs
   via `emit_text_content_parse`/`emit_text_content_serialize`).
5. **Pattern inheritance**: derived simple types append their base type's
   patterns (AND semantics) via `inherit_pattern_facets`; enum facet
   inheritance is intentionally skipped (`EnumDef` has no facets).
6. **File cache & chameleon includes**: the cache stores the raw post-passed
   IR keyed by canonical path (`file_cache`), replays group state
   (`file_groups`) on hit, and re-keys chameleon (namespace-less) includes
   per-includer via `rekey_to_namespace`/`rekey_new_state`. A missing local
   `<xs:include>` or `<xs:redefine>` must return a resolution error; silently
   skipping it leaves unresolved type references that generators may mistake
   for valid model types. Explicit imports retain their existing behavior.
7. Parser reuse across `parse_file` calls is supported; groups partially
   replay from `file_groups` on cache hit.
8. **Mixed content**: compile `mixed="true"` structs after group expansion into
   a repeated item union containing a `#text` branch plus child-element
   branches. The runtime schema keeps branch metadata separately from the
   synthetic list field. Parse `Text`, `CData`, and `GeneralRef` events into
   that list in document order; serialize each item in the same order. Keep
   attributes outside the list and choose a distinct synthetic field name if
   an actual schema field already uses `items`.
9. **Implicit XML namespace and derived choices**: preload the reserved `xml`
   prefix as `http://www.w3.org/XML/1998/namespace` even when `xmlns:xml` is
   absent. Global `xml:lang` attribute references then resolve to the imported
   declaration instead of an undefined `Lang` model type. A bounded top-level
   `xs:choice` inside a complex-type extension must keep the derived struct
   and its base; represent the choice as a synthetic union field. Only
   base-less choice-only types may become standalone unions.

## 5. `xsi:type` Polymorphic Dispatch

`ModelSchema.strict_root` optionally checks the document element's expanded
QName before polymorphic dispatch. It applies at the document entry point,
not at nested frame creation, because a nested field's name can differ from
its type's name. Check both `Event::Start` and `Event::Empty`. Ordered
element-only sequences and substitution fields use `SchemaIR.ordered_types`
to select the tagged item codec without changing `StructDef.is_mixed`.

The runtime dispatches polymorphic elements through a **type registry on
`ModelSchema`** — never through generated code. Documented for users in
`docs/guides/polymorphism.md`; regression-locked in
`crates/polyxml-core/tests/test_xsi_type.rs`.

- **Registry**: `ModelSchema` carries `is_abstract: bool` plus a
  `variants: Arc<RwLock<Vec<Arc<ModelSchema>>>>` (methods: `set_variants`,
  `variants`, `has_variants`, `find_variant_qname(namespace, local_bytes)`,
  `matches_variant(candidate)`). The lock lets Python refresh the registry
  after new subclasses are imported. Variants carry their QName namespace
  and local name (`xml_name`), which is `Meta.name` on the Python side.
- **Population (`schema.rs::from_ir`)**: `build_struct` (a) flattens the
  `xs:extension` content model by collecting the base-chain structs
  (root-base first, cycle-guarded) and prepending their fields — mirroring
  Java codegen's `collect`, so derived runtime schemas carry inherited
  fields — and (b) scans the `BTreeMap` of types for transitive
  derivations (`derives_from`), building each inside a `visited`
  insert/remove guard before `set_variants`. **Python-side population
  (pyo3 `lib.rs`)**: after both global caches are inserted in
  `get_or_create_schema_meta`, `discover_variants` walks
  `__subclasses__()` transitively (ptr-seen set, extracts only
  dataclass/Pydantic classes) — running *after* cache insertion so a
  subclass field typed as the base resolves from cache instead of
  re-entering extraction. Cached classes refresh the registry on lookup;
  refresh walks nested field schemas too, so a subclass loaded after a
  containing model was cached is visible. A thread-local discovery guard
  prevents recursive refresh during extraction.
- **Parse (`parser.rs`)**: `resolve_record_schema(declared, start, scope)` runs at
  *every* frame creation site — root `Start`/`Empty` in
  `deserialize_with_limit`, the root frame in `parse_sub_tree`, both
  nested `Start`/`Empty` arms (single and list), and the `XmlItemStream`
  `Empty` arm. Namespace scopes track declarations through nested elements,
  including streamed records. The attribute must resolve to the XML Schema
  Instance namespace, and its QName value must match both the variant's
  namespace and local name. Policy: registered concrete variant → switch
  schema; absent or unknown value on `is_abstract` →
  `PolyXmlError::SchemaError` listing known variants, or an
  "escape hatch" error when none are registered; everything else falls
  back to the declared schema. Schemas that are neither abstract nor have
  variants return immediately without scanning attributes (zero-overhead
  fast path — this is also why a content attribute named `type` can never
  be misdispatched on plain types).
- **Serialize (`serializer.rs`)**: `write_model` checks
  `schema.matches_variant(rec)` on `Record` values, shadows the effective
  schema to the variant, and pushes `xsi:type` (qualified QName via
  `qualify_element`) after content attributes. Serialization errors when
  namespaces are disabled or XSI has no bound prefix. `NamespaceContext::collect_namespaces`
  recurses `variants` and injects the XSI URI so `xmlns:xsi` is always
  declared at the root whenever dispatch is possible.
- **JSON (`json.rs`)**: `poly_value_to_json_value` prefers the record's
  own schema when its name differs from the declared one, so variant-only
  fields survive `xml_to_json` (JSON carries no selector marker).
- **Escape hatches**: generated static codecs (Rust serde, Java) do *not*
  dispatch — Java's codec errors loudly; deserialize the concrete type or
  use the dynamic runtime. `polyxml-js` bindings are scalar-flat today, so
  dispatch is unreachable from JS until the binding supports nested schemas;
  the core dispatch algorithm itself is shared.
