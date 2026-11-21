# PolyXML Test Suite

Workspace-level fixtures and cross-target integration harnesses live here, alongside crate-level unit, parser, CLI, and codegen tests under `crates/*/tests/`.

## Directory Structure

```text
tests/
├── README.md                  # Test layout, conventions, and runner commands
├── fixtures/
│   ├── samples/               # Real-world XML documents (Atom, SOAP, UBL, streaming catalog)
│   ├── incremental/           # Streaming / incremental writer schemas
│   └── xsd/                   # Shared XSD conformance schemas organized by feature domain
│       ├── attributes/        # Attribute entities, global refs, shared xlink attributes
│       ├── compositors/       # Sequence, choice, cardinality, and UPA schemas
│       ├── inheritance/       # Derived types, simpleContent inheritance, substitution groups
│       ├── security/          # XXE and general entity rejection schemas
│       ├── standards/         # Real-world standard corpus harnesses (e.g. UBL 2.4)
│       ├── types/             # Built-ins, defaults/fixed values, escaping, lists, unions
│       └── validation/        # Rejected unsupported constructs (e.g. XSD 1.1 xs:alternative)
└── java-spring/               # End-to-end Java 25 / Spring Boot 4 / Jackson 3 integration harness
```

Crate-specific suites live next to the code they exercise:

- `crates/polyxml-core/tests/` — Schema IR, parser, UPA validator, runtime codec, and 7-language codegen compile/round-trip suites (Go, Rust, Java, C#, Python, C++, TypeScript).
- `crates/polyxml-cli/tests/` — CLI option parsing, `polyxml.toml` manifest builds, shell completions, and workspace emission tests.
- `crates/polyxml-python/tests/` — Python runtime conformance and generated dataclass/Pydantic model round-trip suites.
- `crates/polyxml-js/test/` — Node.js N-API conformance and serializer/deserializer suite.

## Naming & Fixture Conventions

1. **Describe the behavior, not the origin.** Name test files, test functions, and fixtures after the XSD construct or runtime invariant they verify (never issue numbers or audit labels).
2. **Inline single-use schemas.** If an XSD is only used by one test function or one test file, keep it inline with the test so the schema and assertions stay together.
3. **Group shared schemas by domain.** Only place `.xsd`/`.xml` files under `tests/fixtures/` when they are reused across crates/languages or represent a multi-file schema graph (`xs:import` / `xs:include`).
4. **No orphaned fixtures.** Every file in `tests/fixtures/` must be exercised by an active test or verification script.

## Running Tests

- **Core Rust & multi-language codegen** (runs Go, Java, TypeScript, C++, Python, C# checks depending on installed toolchains):
  ```sh
  cargo test -p polyxml
  ```
- **CLI integration suite**:
  ```sh
  cargo test -p polyxml-cli
  ```
- **Python bindings & generated model suite**:
  ```sh
  pytest crates/polyxml-python/tests
  ```
- **Node.js N-API bindings**:
  ```sh
  npm --prefix crates/polyxml-js test
  ```
- **Java 25 / Spring Boot 4 harness**:
  ```sh
  ./scripts/verify_spring_boot.sh
  ```
- **UBL 2.4 Invoice C# gate** (requires .NET 8 SDK):
  ```sh
  python3 scripts/verify_ubl_csharp.py
  ```
