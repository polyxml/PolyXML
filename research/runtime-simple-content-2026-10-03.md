# Inherited simpleContent — October 3, 2026

The shared schema IR now resolves a derived complex type's synthetic text field to
its inherited scalar type. Runtime schema construction retains one most-derived
text slot and all inherited attributes. TypeScript emits object inheritance only
for complex bases, avoiding `extends Int` and scalar aliases.

Tested source: `4421e3ce9f76024121919cc3d2f7fa5a1ecced24`.
Parent: `ea19f7f` from [draft PR #143](https://github.com/polyxml/PolyXML/pull/143).
The parent integrates the selected runtime improvements and correctness fixes;
experimental registry/attribute optimizations remain separate. Main is unchanged.

## Reproduction and implementation

Three bounded CType failures (`basetd00101m1`, `basetd00101m2`,
`derivationmethod00101m1`) contained numeric or string text. Generated Python
incorrectly declared `value: BaseModel` instead of the inherited scalar. This was
not an empty-text failure. The initial four reduced regression tests all failed
on the integration baseline; the retained log distinguishes that baseline from
later environment/setup failures.

The root-level post-pass runs after include/import merging and type-reference
validation. An iterative walk with a visited set rejects cyclic simpleContent
inheritance. Memoized terminal references avoid repeated base-chain traversal.
Named simple aliases, enums, unions and lists retain their identities and facets;
complex `base_type` remains available for attribute inheritance. Imported-frame
boxed text references are unwrapped during this walk; element recursion is
unchanged.

Runtime flattening previously retained several fields named `value`. Parsing filled
the last slot, but record lookup returned the first empty slot; writing could emit
several text slots. Base text slots are now skipped when the derived type declares
its own text. The regression verifies one text field, numerical values, inherited
attributes, constraints and XML round trips.

## Verification

Five core regressions cover multi-level forward declarations, terminal alias
constraints, import/cache behavior, textual cycles and enum/union/list identity.
A generated-consumer test compiles and executes all seven targets:

- Rust owned and borrowed streaming XML plus Serde JSON round trips.
- Python dataclass and Pydantic XML/JSON round trips.
- Go `encoding/xml` and JSON round trips.
- C# record and class XML/JSON round trips with warnings as errors.
- Strict C++20 compilation/execution and Java/TypeScript model execution verify
  the inherited scalar and attributes. These checks do not claim XML codecs for
  those default generation modes.

Independent lxml/libxml2 XSD validation accepts three valid fixture instances and
rejects three invalid scalar/facet inputs. CLI smoke emits all seven ecosystems.
C# consumers target net8.0 and run on the installed .NET 10 runtime with
`DOTNET_ROLL_FORWARD=Major`; minimum .NET 8 execution is not verified.

The first direct consumer command used system Python, which lacked `polyxml`;
rerunning with the project venv passed that stage. An optional integer attribute
also exposed an existing C# XmlSerializer limitation; the reduced cross-language
fixture uses a required integer attribute to isolate text inheritance. Neither
setup failure is a candidate speed/correctness result.

This fix does not implement empty text/list codecs, additional facets declared
inside simpleContent restrictions, optional C# scalar attribute proxies, or full
W3C conformance. No additional runtime speedup is claimed: the preceding reports
retain the benchmark measurements and controls.

## Full gate and bounded W3C corpus

The final gate passes **340 Rust tests, 115 Python tests, 100% statement and branch
wrapper coverage**, workspace fmt/strict Clippy and Ruff. An older Java consumer
constructed text as a nested `Measurement`; its expected value now uses
`BigDecimal` and checks the numeric value. The failed pre-update gate is retained.

| Bounded suite | Schemas passed | Baseline round trips | Candidate round trips |
| --- | ---: | ---: | ---: |
| CType (limit 50; 31 groups available) | 31/31 | 23/28 | **26/28** |
| AttrDecl (limit 50) | 50/50 | 50/50 | 50/50 |
| NIST (limit 100) | 100/100 | 447/452 | 447/452 |

The three inherited-text failures disappear. The two CType substitution-root
failures and the one NIST scalar group remain. AttrDecl and NIST summary/failure
tables are unchanged. These bounded runs are not a full conformance result.

Fresh candidate CLI/native artifacts pass discriminating probes for this fix and
all integrated entity/default/namespace/mixed-nil fixes. Artifact hashes and the
actual imported native module path are in
[probe.json](data/runtime-followups-2026-10-03/simple-content/probe.json).
All logs, toolchain versions and replay scripts are retained in
[the evidence directory](data/runtime-followups-2026-10-03/simple-content/), with
[SHA256SUMS](data/runtime-followups-2026-10-03/simple-content/SHA256SUMS).
