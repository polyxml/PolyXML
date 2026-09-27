---
title: XSD-to-Code Generator & CLI Toolchain
description: Generate type-safe data models and codecs across 7 programming languages from W3C XML Schemas using polyxml.
---

# XSD-to-Code Generator & CLI Toolchain (`polyxml`)

PolyXML includes a high-performance, polyglot XSD-to-code generator and CLI toolchain (`polyxml`) that parses W3C XSD 1.0 and 1.1 schemas, builds a language-agnostic Intermediate Representation (PolyXML-IR), resolves complex type cycles via Tarjan's Strongly Connected Components (SCC) algorithm, and emits idiomatic, production-ready data contracts and codecs across **7 modern programming languages**.

---

## 🚀 Key Features

- **Pure-Rust XSD 1.0 & 1.1 Parser**: Zero dependencies on legacy libraries like `libxml2` or Apache Xerces.
- **PolyXML Intermediate Representation (IR)**: Normalized schema representation preserving namespaces, facets, substitution groups, documentation, and cardinality.
- **Tarjan SCC Cycle-Cutting**: Automatically identifies self-referential and mutually recursive types, calculating the minimal set of cycle-cut points to prevent infinite size allocations (`Box<T>`, pointers, `std::unique_ptr`, `z.lazy`).
- **Simultaneous Multi-Target Compilation**: Emit models for Python, Rust, C++, Java, TypeScript, Go, and C# in a single compiler invocation.
- **Ordered Mixed Content**: Preserve text and child elements from `xs:complexType mixed="true"` in one typed item sequence.
- **W3C Conformance Tested**: Validated against the official W3C XML Schema Test Suite (XSTS) via the [polyxml-w3c-tests](https://github.com/polyxml/polyxml-w3c-tests) harness.

---

## Mixed content

For a mixed complex type, PolyXML generates an `items` collection containing
text and child-element variants in document order. For example,
`<description>open <b>bold</b> close</description>` becomes three items:
text `"open "`, element `b` with value `"bold"`, and text `" close"`.
Attributes remain regular fields on the parent model. Rust, Python, Go,
Java direct codecs, and C# can read and write this ordered sequence. C++ and
TypeScript expose it as typed variants for application code.

---

## 📦 Installation

Compile and install the `polyxml` CLI directly from source using Cargo:

```bash
# From the PolyXML workspace root
cargo install --path crates/polyxml-cli
```

Verify installation:

```bash
polyxml --help
```

---

## 🛠️ CLI Commands

### 1. `polyxml generate`

Generate code directly from one or more `.xsd` schema files:

```bash
# Generate Python dataclasses
polyxml generate --lang python --out ./generated/python schema.xsd

# Generate Pydantic v2 models with runtime facet validation
polyxml generate --lang python --backend pydantic --out ./generated/python schema.xsd

# Generate high-performance Ahead-of-Time (AOT) PyO3 native extension crate
polyxml generate --lang python --backend aot --package my_models --out ./generated/native_models schema.xsd

# Generate zero-copy Rust models with inherent streaming codecs
polyxml generate --lang rust --feature zero-copy --codecs --out ./generated/rust schema.xsd

# Generate all 7 languages simultaneously
polyxml generate \
  --lang python \
  --lang rust \
  --lang cpp \
  --lang java \
  --lang typescript \
  --lang go \
  --lang csharp \
  --out ./generated \
  schema.xsd
```

#### Flags and Options

| Option | Flag | Description | Default |
|---|---|---|---|
| **Target Language** | `-l`, `--lang` | Target language (`python`, `rust`, `cpp`, `java`, `typescript`, `go`, `csharp`). Can be specified multiple times. | `python` |
| **Output Directory** | `-o`, `--out` | Target directory for generated source files. | `generated` |
| **Model Style** | `--style` | Target-specific type representation; see the table below. | Existing target default |
| **Enhancements** | `--feature NAME` | Repeatable; also accepts comma-separated names. | None added |
| **Target Backend** | `-b`, `--backend` | Target backend (`dataclass`/`pydantic`/`aot` for Python; `standard`/`jackson` for Java; `standard`/`glaze` for C++; `interfaces`/`zod`/`valibot`/`typebox` for TypeScript; `standard`/`easyjson`/`sonic` for Go). | Target default |
| **Compilation Mode** | `-m`, `--mode` | Target packaging mode (`header` or `modules` for C++). | Target default |
| **Streaming Codecs**| `--codecs` | Emit inherent zero-copy streaming XML serializers and deserializers. | `true` |
| **Package / Namespace** | `-p`, `--package` | Namespace or package name for Java, Go, C#, or C++. | Target default |
| **Custom Header** | `--custom-header` | Custom comment, license, or linter directive text to prepend to generated files. | `None` |
| **Dry Run** | `--dry-run` | Validate options and parse schemas without writing to disk. | `false` |
| **Format** | `--format` | Automatically format generated code using host toolchains (`ruff`, `cargo fmt`, `clang-format`, `gofmt`). | `false` |

---

### Backend, style, and feature compatibility

Options apply to every language selected in one invocation. Use `polyxml.toml`
when targets need different options. Invalid values, unsupported combinations,
and conflicting options fail before schema parsing or output creation,
including with `--dry-run`.

| Target | Backends (first is default) | Styles | Features |
| --- | --- | --- | --- |
| Python | `dataclass`, `pydantic` | `dataclass` (dataclass backend only) | `slots`, `kw-only` (dataclass backend only) |
| Rust | `standard` | — | `zero-copy`, `phf`; `rkyv` is accepted but [currently incompatible with rkyv 0.8](../languages/rust.md) |
| TypeScript | `interfaces`, `zod`, `valibot`, `typebox` | — | — |
| Java | `standard`, `jackson` | `record` (default), `pojo` (alias `class`) | `builder`, `direct-codec`, `validation` |

| C# | `standard`, `source-gen` | `record-class` (default), `record-struct`, `class` (mutable) | — |
| C++ | `standard`, `glaze` | — | — |
| Go | `standard`, `easyjson`, `sonic` ([Sonic tag limitation](../languages/go.md)) | — | — |

Java's `--feature validation` (or `features = ["validation"]` in a manifest)
adds Jakarta Bean Validation annotations to generated records and mutable
classes. Add `jakarta.validation:jakarta.validation-api:3.1.0` to the consuming
Java project. Without the feature, generated models need no Jakarta API.

Schemas using `xs:gDay`, `xs:gMonth`, `xs:gYear`, `xs:gYearMonth`, or
`xs:gMonthDay` generate named partial date types with lexical pattern checks.
The string value is retained, including any timezone suffix. Generated
validators reject invalid ranges such as `---32`, `--13`, `--02-30`, and
offsets beyond `14:00`; the schema-backed Rust parser checks the same values
when reading XML.

Defaults are unchanged: Rust zero-copy and Python slots/keyword-only fields are
already enabled. Future features such as `aot`, Python plain `class`,
and C# mutable `struct` are rejected until their generators support them.

```bash
polyxml generate schema.xsd --lang rust --feature phf
polyxml generate schema.xsd --lang java --style pojo --feature builder --feature direct-codec
polyxml generate schema.xsd --lang csharp --backend source-gen --style record-struct
```

The same options work in both `[[generate]]` and `[codegen.<target>]`:

```toml
[codegen.java]
output = "generated/java"
backend = "jackson"
style = "pojo"
features = ["builder", "direct-codec"]
```

The CLI rejects unsupported target options and manifest fields. For owned Rust
strings, use `--zero-copy=false` (or `zero_copy = false` in the manifest);
combining it with `--feature zero-copy` is an error.

`polyxml generate` without schema paths delegates to the manifest. Put target
options in that manifest; command-line generation overrides are rejected rather
than silently ignored.

### 2. `polyxml build`

Declaratively compile complex, multi-schema enterprise projects using a workspace manifest (`polyxml.toml`):

```bash
polyxml build --config polyxml.toml
```

#### Workspace Manifest Example (`polyxml.toml`)

```toml
[workspace]
name = "enterprise-iso20022"
schemas = ["schemas/iso20022/*.xsd"]
include_dirs = ["schemas/common/"]
output_base_dir = "./generated"
custom_header = "// Copyright (c) 2026 Enterprise Corp. All rights reserved."

[[generate]]
target = "python"
output = "src/generated/python"
backend = "pydantic"
codecs = true

[[generate]]
target = "rust"
output = "src/generated/rust"
features = ["phf"]
codecs = true

[[generate]]
target = "java"
output = "src/generated/java"
package = "com.enterprise.banking.iso20022"
backend = "jackson"
# style = "pojo"        # record (default) | pojo (JavaBeans, alias class)
# features = ["builder", "direct-codec"]

[[generate]]
target = "typescript"
output = "src/generated/ts"
backend = "valibot"

[[generate]]
target = "cpp"
output = "src/generated/cpp"
mode = "modules"
backend = "glaze"

[[generate]]
target = "go"
output = "src/generated/go"
package = "payments"
backend = "sonic"

[[generate]]
target = "csharp"
output = "src/generated/csharp"
namespace = "Enterprise.Banking.Iso20022"
backend = "source-gen"
style = "record-struct"
```

---

### 3. `polyxml validate`

Statically check W3C XML schemas for structural validity, element types, and cycle topology without emitting code:

```bash
polyxml validate schemas/*.xsd
```

Validates:
- XML syntax and W3C XSD element structure.
- Type reference integrity and namespace imports.
- Strongly Connected Components and recursion depth.

---

### 4. `polyxml transcode`

Bidirectionally convert complete XML and JSON documents, with optional schema guidance. The command accepts stdin and stdout pipes, but reads the full input and buffers the full output before writing it:

```bash
# 1. Transcode XML to JSON with W3C XSD schema typing
polyxml transcode --schema order.xsd --pretty order.xml --out order.json

# 2. Connect stdin / stdout pipes
cat order.xml | polyxml transcode --schema order.xsd > order.json

# 3. Transcode JSON back to XML with specified root element
cat order.json | polyxml transcode --schema order.xsd --root order --pretty > order.xml

# 4. Dynamic schema-less transcoding with attribute (@attr) and text (#text) preservation
polyxml transcode legacy.xml --out modern.json
```

#### Flags and Options

| Option | Flag | Description | Default |
|---|---|---|---|
| **Input** | `[INPUT]` | Input file path, or `-` / omitted for standard input. | `stdin` |
| **Output** | `-o`, `--out` | Output file path, or `-` / omitted for standard output. | `stdout` |
| **From Format** | `--from` | Input format (`xml` or `json`). Auto-detected if omitted. | Auto-detect |
| **To Format** | `--to` | Output format (`xml` or `json`). Auto-detected if omitted. | Auto-detect |
| **Schema** | `-s`, `--schema` | Optional W3C XSD schema file for typed schema-directed transcoding. | None |
| **Root Element**| `-r`, `--root` | Root XML element tag name (used when transcoding JSON to XML). | None |
| **Pretty** | `--pretty` | Format output with indentation and newlines. | `false` |

---

## 🎯 Target Language Matrix

| Target | Language Version | Paradigm | Key Highlights |
|---|---|---|---|
| **Python** | Python 3.12+ | `@dataclass`, Pydantic v2 & PyO3 AOT | PEP 695 type aliases, PEP 604 unions, Ahead-of-Time (AOT) PyO3 C-extensions (`--backend aot`), zero-copy streaming codecs, native `.to_json()` / `.from_json()` methods |
| **Rust** | Rust 2021 / 2024 | Zero-Copy & Owned Structs | Lifetime inference `<'a>`, automatic `Box<T>` cycle breaks, inherent streaming XML codecs, inherent `.to_json_string()` codecs |
| **C++** | C++20 / C++23 | Header-Only Value Types | `std::variant` choices, `std::unique_ptr` cycle breaks, C++20 concepts, CMake/Meson export |
| **Java** | Java 22+ | Modern Records & Sealed Interfaces | Exhaustive switch pattern matching, compact constructor facet validation |
| **TypeScript** | TypeScript 5+ | Interfaces & Discriminated Unions | Runtime Zod schemas, circular references handled via `z.lazy()`, `as const` enums |
| **Go** | Go 1.22+ | Structs with `encoding/xml` & `encoding/json` | Dual `xml:"..."` and `json:"..."` struct tags, `json:"-"` on `XMLName`, canonical initialisms (`ID`, `URL`), choice exclusivity |
| **C#** | C# 12 / .NET 8+ | Records with Primary Constructors | Parameterless constructors, dual `XmlSerializer` and `System.Text.Json` attributes (`[JsonPropertyName]`, `[JsonConverter]`), `IValidatableObject` |

---

## 📊 W3C XML Schema Conformance

PolyXML's schema compiler and runtime codecs are continuously tested against the official **W3C XML Schema 1.0 / 1.1 Test Suite (XSTS)** using our dedicated test harness repository, **[polyxml-w3c-tests](https://github.com/polyxml/polyxml-w3c-tests)**.

- **Schema Compilation Pass Rate**: **99.8% (635 / 636 groups)**
- **Instance Validation & Round-Trip Pass Rate**: **96.4% (489 / 507 instances)**

For full conformance benchmark metrics across Sun Microsystems, Microsoft, and NIST test sets, visit the [polyxml-w3c-tests repository](https://github.com/polyxml/polyxml-w3c-tests).

---

## 🌐 Real-World Enterprise Repositories

Explore full-scale enterprise examples demonstrating `polyxml.toml` manifests and multi-target compilation in real-world production settings:

- **[🛸 Defense & Aerospace Showcase (polyxml-defense-examples)](https://github.com/polyxml/polyxml-defense-examples)**:
  Compiles the **USAF UCI v2.5** schema standard and bridges edge sensor telemetry from **Anduril Lattice** across all 7 languages. Includes an [Ahead-of-Time PyO3 example](https://github.com/polyxml/polyxml-defense-examples/blob/main/examples/python/bridge_aot.py) (`backend = "aot"`).
- **[💳 FinTech & Banking Showcase (polyxml-finance-examples)](https://github.com/polyxml/polyxml-finance-examples)**:
  Compiles the **ISO 20022 `pacs.008`** schema standard and bridges instant payment webhooks (FedNow, Stripe, Plaid) across all 7 languages.
- **[🚍 Smart Cities & Transit Showcase (polyxml-transit-examples)](https://github.com/polyxml/polyxml-transit-examples)**:
  Compiles the European **CEN SIRI v2.0** and **NeTEx** schemas and bridges live Google GTFS-Realtime feeds across all 7 languages.



### Shell completion

Generate and install a completion script for your shell:

```bash
# Bash: add this to ~/.bashrc
source <(polyxml completions bash)

# Zsh: save _polyxml into a directory already on $fpath, then run compinit
polyxml completions zsh > /your/completion/directory/_polyxml

# Fish
mkdir -p ~/.config/fish/completions
polyxml completions fish > ~/.config/fish/completions/polyxml.fish
```

Backend, style, and feature suggestions use the CLI's validation rules and the
selected `--lang` (including aliases). With multiple languages, completion only
suggests values accepted by every selected target. Without `--lang`, it uses the
Python default. Bash also completes
comma-separated features; repeat `--feature` for portable completion across shells.

### CLI startup benchmark

Run `./benchmarks/cli/benchmark.sh` with `hyperfine` installed. It builds the
release CLI and measures help rendering plus argument validation and parsing of
an empty schema. See [the benchmark methodology](https://github.com/polyxml/PolyXML/blob/main/benchmarks/cli/README.md)
for measurement limits and output location.
