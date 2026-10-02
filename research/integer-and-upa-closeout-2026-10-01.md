# Follow-up issues #117 and #131

## #117 — unbounded integers

Unbounded `xs:integer`, `positiveInteger`, `negativeInteger`, `nonPositiveInteger`,
and `nonNegativeInteger` preserve arbitrary-size values. Bounded builtins retain
their existing mappings. Core dynamic values store validated decimal strings;
integer bounds and fixed-value comparisons do not use floating point.

| Target | Unbounded representation | Evidence |
| --- | --- | --- |
| Rust | String / Cow<str> | Both generated codec modes compile and round-trip exact digits |
| Go | Validated string wrappers with text codecs | Generated XML read/write, invalid lexical rejection |
| C# | String with XML validation proxies | Record and mutable class XML read/write, invalid lexical rejection |
| Java | BigInteger | Generated direct XML codec read/write |
| Python | int | Dataclass and Pydantic native XML read/write |
| C++ | std::string | Generated model compiles; lexical validator accepts/rejects values |
| TypeScript | string | Zod, Valibot, and TypeBox compile and validate lexical strings |

`scripts/verify_unbounded_integer.py` checks the 40-digit repro, positive and
negative values, and signed 64-bit boundaries. It independently validates every
XML output with lxml and compares the decimal digits. C++ and TypeScript do not
have XML codecs for this check; this is not a claim of XML runtime parity there.
The representation change affects model APIs and JSON: lexical-storage targets
expose strings instead of fixed-width numbers.

Validation: workspace tests, warning-free workspace/all-target clippy, seven-target
smoke, Python 112 tests with 100% statement/branch coverage, all four pinned UBL
C# modes. Companion W3C MGroup: 59/59 schema checks, 31/32 XML round trips (one
existing required-field constructor failure).
