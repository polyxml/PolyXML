# Generated default and fixed strings — October 3, 2026

This branch extends the entity and enum fix in draft PR #138. The implementation
is `cb041c0`; stricter consumer tests and isolated-target test tooling are
`a7ed359`. Main is unchanged.

## Reproduced failure and fix

The pre-fix CLI successfully generates a model for
[the fixture](fixtures/default_literal_escaping.xsd), but Python rejects its
source as an unterminated string literal. The default formatter escaped quotes
without correctly escaping backslashes, referenced whitespace or source line
separators. Python default/fixed metadata additionally used Rust Debug Unicode
escapes. Similar gaps existed in C++ string initializers and C# default/fixed
proxy literals.

Use the existing JSON-compatible literal encoder for Python and C# and the C++
encoder for its string initializers. C# reparses these lexical values through an
internal XML element; encode carriage returns as `&#13;` after escaping markup
so XML line-ending normalization does not change referenced carriage returns.

This fixes literal encoding at these sites. It does not implement missing
default semantics for other field kinds or backends, and it does not claim that
all generated pattern/default literal sites have been audited.

## Executed consumers

An independent UTF-8 expected file checks quotes, literal backslash-u text,
tabs, LF, CR, U+0085, U+2028, accented text and an emoji. Execute generated Python
dataclasses and Pydantic models, C++ with `-Wall -Wextra -Werror`, and C# with
`TreatWarningsAsErrors`. Check Python defaults and metadata, the C++ string
initializer, and C# attribute initialization, empty-element defaults, proxy
assignment and fixed-value rejection. The C# consumer targets .NET 8 but executes
on installed .NET 10 with roll-forward; this does not test the minimum runtime.
The actual CLI-generated Python model also matches the independent value.

## Quality and bounded corpus results

The complete gate passes 327 Rust tests, strict workspace formatting and Clippy,
Ruff, and 115 Python tests with 100% statement and branch coverage. All seven CLI
backends pass their emission smoke check; the enum test inherited from #138
also compiles and executes all seven language consumers.

The first full gate reached Python testing but could not find the CLI it built:
the helper ignored `CARGO_TARGET_DIR`. It now searches that target and avoids an
unrelated global CLI when the override is explicit. Three regressions exercise
absolute/relative targets and building the matching CLI. The smoke script also
honors the target override. The complete retry passes; both logs are retained.

The companion runner uses the isolated CLI and freshly installed native binding,
whose paths and SHA-256 hashes are recorded. Its summary and failure tables are
byte-identical to the entity-fix comparison:

| Sample | Schema expectations passed | Instance round trips passed |
| --- | ---: | ---: |
| CType (limit 50; 31 groups available) | 31/31 | 23/28 |
| AttrDecl (first 50 groups) | 50/50 | 50/50 |
| NISTXMLSchemaDatatypes (first 100 groups) | 100/100 | 447/452 |

Existing failures remain; these bounded runs do not establish full conformance.
[Raw verification logs, source probes and hashes](data/runtime-followups-2026-10-03/default-literals/)
retain the failure preflight and successful retry. Heavy work was serial with
one Cargo worker under the repository memory cap.
