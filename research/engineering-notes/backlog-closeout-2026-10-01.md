# GitHub backlog closeout — 2026-10-01

The user expanded the original category 1/2 implementation scope to finish
the recommended priorities before **Later / Defer**. This report supersedes
the scope exclusions in `category-1-2-fixes-2026-10-01.md`, which records the
first implementation batch. Each subsequent issue was committed and pushed
before moving on; already-fixed issues were closed after acceptance review.

## Completed scope

All 28 audit issues in the selected scope are closed: #101–#116, #118–#126,
and #128–#130. Earlier implementation and acceptance evidence remain in the
original follow-up and issue comments. The later fixes/checks are:

| Issue | Pushed change / acceptance evidence |
| --- | --- |
| #125 | `62bd41b`: expanded-QName global attributes in Go and both C# styles; original XLink fixture and alternate prefixes. |
| #126 | `f5c00f3`: reject unsupported general entities, including skipped children; built-in/numeric references remain usable and external entities remain disabled. |
| #103 | `577e620`: present empty elements receive defaults, absent optional elements stay absent; native/Python and C# read/write tests. |
| #102 | `ad96fe3`: fixed values enforced in typed value space on read/write, including empty fixed elements and defaulted fixed attributes. |
| #123 | `9212262`, `d90340e`: preserve separate choice groups and Go container QNames and validate supported particle streams on input/output in native/Python and Go. |
| #106 | `b22b5a1`: repeated sequence order and occurrence bounds, incomplete groups, unexpected children, and output mutations. |
| #108 | `ffd5b92`: typed whitespace-separated lists, restricted/enum items, XML whitespace and empty lists in native/Python, Go and C#. |
| #116 | `f08818b`: unresolved/unknown types, undeclared prefixes and illegal simpleType children rejected before generation; source/line diagnostics and forward references. |
| #119 | `b3744c4`: explicitly reject unsupported XSD 1.1 conditional alternatives in validate/generate/dry-run rather than weakening the model. |
| #115 | `44834f7`: repeated choice branches retain cardinality; reject empty required choice, repeated scalar alternatives and mixed branches. |
| #122 | `e7c7c9d`: duplicate-name choice boundaries; MinAge alone is legal because its branch's MaxAge is optional. |
| #105 | `d372812`, `f5fc88d`: reject abstract substitution heads, preserve concrete payloads/namespaces, and retain substitution metadata during root filtering. |
| #107 | `947fa83`: empty, ordinary, single-nil and mixed nullable enum collections in records/classes; non-nillable items reject nil. |
| #118 | `00302d8`: Go/C# date/dateTime union lexicals, offsets, timezone-free values and fractional seconds; C# snapshot avoids stale text after mutation. |
| #112 | Full/root-scoped UBL C# records and classes compile and round-trip a representative Invoice. The corpus gate pins all 16 imports, verifies closure, runs in CI, and independently validates input/output. Reduced namespace, root-name, inherited-text/attribute, restriction and temporal-text regressions accompany it. |

## Verification

- Workspace Rust tests, formatting and strict Clippy.
- Python: 110 tests, 100% statement and branch coverage; Ruff passes.
- Seven-target codegen smoke verification.
- `python3 scripts/verify_ubl_csharp.py`: four fresh nullable-enabled .NET 8
  compile/runtime gates; supplier/customer names, dates, quantities, monetary
  values, currency attributes and root QName retained. .NET's XSD engine
  validates input/output independently; lxml also validates the input.
- W3C companion MGroup/MGroupDef selection: 59/59 schema compilations and
  29/32 instance round trips, unchanged from the starting compiler. The
  three existing failures are outside this completed scope.

## Stop boundary and limits

Left open as requested: **#131** (UPA), **#117** (arbitrary-size integers),
**#127** (incremental generated writer), **#55** and **#57** (benchmarks).

Acceptance establishes the saved fixtures and tested runtimes, not complete
W3C conformance or universal target support. Supported particle patterns do
not cover every mixed/model-group/all-group/derivation combination. Go's
constrained-element validation buffers that element's XML. Temporal values
remain bounded by target platform date ranges. Restrictions exercised by list
items and value constraints do not establish every XSD facet. Derived C#
restrictions reuse inherited members; independent XSD validation remains the
authority for stricter restriction constraints. The UBL gate establishes its
representative Invoice path, not every optional UBL branch or a competitor
migration/performance comparison.
