# NeTEx C# and unused-import fixes — 2026-10-03

The complete NeTEx module dependency closure now compiles as both C# records and
mutable classes with zero warnings and errors. Both checks target net8.0, use
MemoryMax=3500M and MemorySwapMax=0, disable compiler servers, and run serially.
The development record check peaked at 2,774,216 KiB process RSS (37.254 s);
the class check peaked at 2,660,420 KiB (24.775 s). Durable final verification
is retained in the companion corpus repository's results/2026-10-03 directory.
These are compile checks, not a claim of full NeTEx XML conformance.

## Generator changes

- Emit ordered-content declarations only when a real item stream is present;
  flatten inherited streams/attributes before removing inherited fields. Missing
  declarations previously left orphan XmlRoot attributes attached to later types.
- Allocate unique choice branch identifiers, reserve their referenced type names
  against nested-name shadowing, and use the same names in metadata and codecs.
  Canonicalize identical QName/type branches in ordered streams so repeated
  occurrences still preserve their list positions without duplicate switch labels.
- Generate virtual/override validation for records and mutable classes and call
  base.Validate. Include validation on custom ordered-content models. Derived
  records previously skipped inherited constraints; the runtime regression checks
  direct calls, base references and IValidatableObject dispatch.
- Add LINQ imports when emitted list/predicate code needs them; do not rely on
  consumer implicit imports. Nested lexical unions call Parse and ToXmlString.
  An unrestricted string fallback ends parse dispatch instead of emitting
  unreachable alternatives. Enum restriction length/pattern checks use XML text.
- Required scalar proxies avoid null checks on value types. Required integer
  proxies reject a missing value; optional proxies retain nullable behavior.
  Fixed-value setters preserve nonnullable typing. Mixed attributes use XML
  lexical serialization, including enum/boolean/simple types and namespaces.
- Rust also allocates unique Serde field names when distinct XML fields share
  a name (including wildcards). The first JSON name is preserved; later collisions
  use unique Rust field names without taking another real XML field's JSON name.
  XML metadata and codecs retain their original names. A borrowed/owned JSON
  round-trip regression denies unreachable patterns and verifies all three values.
- Rust omits empty module glob reexports; Go bases the bytes import on emitted
  code instead of imported ordered types remaining in the schema IR.

## Regression coverage

Generated C# record/class consumers cover inherited mixed text/children/attributes,
boolean/enum attributes, case-normalized branch collisions, referenced-type
shadowing, repeated identical XML branches, nested lexical unions, lists without
implicit imports, enum lexical facets, required default/fixed scalars and missing
required integers. Tests promote relevant warnings to errors. Four particle
fixtures independently pass lxml XMLSchema compilation.

A generated Rust empty-module consumer denies unused imports in monolithic and
split modes. A generated Go module with an external ordered type compiles without
an unused bytes import. All destination-language builds run under memory caps.

The full workspace gate passes format, strict Clippy, workspace tests, Python
lint and 112 tests with 100% statement/branch coverage. The seven-language smoke,
507 UPA decisions matching Xerces, strict docs and companion W3C CType sample
also pass their established baselines (31/31 schemas, 23/28 round trips; five
existing instance failures).

## Remaining scope

The broader CDA/UBL/FpML and non-C# NeTEx matrix remains tracked in #135.
Successful compilation does not establish all schema constraints or codec paths.
In particular, mixed-content extension that adds new elements can still omit
inherited child branches; preserving its complete particle derivation is a
separate parser/codegen follow-up. This change verifies inherited streams with
added attributes and does not claim that untested extension case is fixed.
