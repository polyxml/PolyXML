<p align="center">
  <a href="https://github.com/polyxml/PolyXML">
    <img src="https://raw.githubusercontent.com/polyxml/PolyXML/main/docs/assets/brand/logo_polyxml_banner.png" alt="PolyXML" width="800">
  </a>
</p>

# PolyXML CLI (`polyxml`)

<p align="center">
  <a href="https://crates.io/crates/polyxml-cli"><img src="https://img.shields.io/crates/v/polyxml-cli.svg?logo=rust&label=crates.io" alt="crates.io"></a>
  <a href="https://github.com/polyxml/homebrew-polyxml"><img src="https://img.shields.io/badge/Homebrew-polyxml-FBB040.svg?logo=homebrew&logoColor=black" alt="Homebrew"></a>
  <a href="https://polyxml.github.io/PolyXML/guides/compiler/"><img src="https://img.shields.io/badge/docs-zensical-blue.svg" alt="Documentation"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</p>

Unified developer command-line interface and polyglot schema compiler toolchain for PolyXML.

`polyxml` parses W3C XSD 1.0 and 1.1 schemas into a unified, language-agnostic Intermediate Representation (IR), resolves cyclic/recursive types via Tarjan's Strongly Connected Components (SCC) algorithm, and compiles production-ready, idiomatic data models and codecs for 7 target languages.


---

## Installation

### 1. Universal One-Line Installer (Linux & macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/polyxml/PolyXML/main/scripts/install.sh | bash
```

### 2. Direct Packages (Debian / Ubuntu & Fedora / RHEL)

Download pre-built packages from [GitHub Releases](https://github.com/polyxml/PolyXML/releases):

```bash
# Debian / Ubuntu / Mint / Pop!_OS (.deb)
sudo dpkg -i polyxml_amd64.deb

# Fedora / RHEL / Rocky Linux / openSUSE (.rpm)
sudo dnf install ./polyxml.x86_64.rpm
```

### 3. Homebrew (macOS & Linux)

```bash
brew install polyxml/polyxml/polyxml
```

### 4. Cargo / Cargo-Binstall

```bash
# Fast pre-compiled binary via binstall
cargo binstall polyxml-cli

# Or compile from source
cargo install polyxml-cli
```

---

## Supported Target Languages

| Target | Flag (`--lang`) | Generated Artifacts & Features |
| :--- | :--- | :--- |
| **Python** | `python` | Modern Python 3.12+ `@dataclass` or Pydantic v2 models, field constraints, zero-copy streaming codecs |
| **Rust** | `rust` | Zero-copy `Cow<'a, str>` & owned structs, automatic recursive boxing (`Box<T>`), streaming serializers/deserializers, rkyv wire format (`--feature rkyv`) |
| **C++** | `cpp` | Modern C++20/C++23 value types, C++20 Modules (`--mode modules`), Glaze reflection (`--backend glaze`), CMake/Meson export |
| **Java** | `java` | Java 22+ records or mutable JavaBeans (`--style pojo`), fluent builders (`--feature builder`), direct StAX codecs (`--feature direct-codec`), Jackson 2/3 XML/JSON annotations (`--backend jackson` / `--backend jackson3`) |
| **TypeScript** | `typescript` | TypeScript 5+ interfaces, discriminated unions, runtime validation schemas via Zod, Valibot, or TypeBox (`--backend`) |
| **Go** | `go` | Idiomatic Go 1.22+ structs with `encoding/xml` tags, reflectionless EasyJSON (`--backend easyjson`) & ByteDance Sonic JIT (`--backend sonic`) |
| **C#** | `csharp` | Modern C# 12 / .NET 8+ mutable classes (`--style class`), records and record structs (`--style record-struct`), compile-time Native AOT source generation (`--backend source-gen`) |

---

## Commands

### 1. `polyxml generate`

Compile schemas directly into code for one or more target languages:

```bash
# Generate Python dataclasses
polyxml generate --lang python --out ./generated/python schemas/order.xsd

# Generate Pydantic v2 models with runtime validation
polyxml generate --lang python --backend pydantic --out ./generated/python schemas/order.xsd

# Generate Java models for Java 25 / Spring Boot 4 with Jackson 3
polyxml generate --lang java --backend jackson3 --package com.enterprise.banking --out ./generated/java schemas/order.xsd

# Generate C++20 Modules with Glaze reflectionless serde
polyxml generate --lang cpp --mode modules --backend glaze --package enterprise::crm --out ./generated/cpp schemas/order.xsd

# Generate zero-copy Rust models with codecs and rkyv wire format
polyxml generate --lang rust --feature zero-copy --codecs --feature rkyv --out ./generated/rust schemas/order.xsd

# Generate TypeScript with tree-shakeable Valibot schemas
polyxml generate --lang ts --backend valibot --out ./generated/ts schemas/order.xsd

# Generate Go models with ByteDance Sonic JIT tags
polyxml generate --lang go --backend sonic --package crm --out ./generated/go schemas/order.xsd

# Generate C# record structs with Native AOT source generation
polyxml generate --lang csharp --style record-struct --backend source-gen --namespace Enterprise.Crm --out ./generated/csharp schemas/order.xsd

# Multi-target compilation in a single invocation
polyxml generate \
  --lang python \
  --lang rust \
  --lang cpp \
  --lang java \
  --lang typescript \
  --lang go \
  --lang csharp \
  --out ./generated \
  schemas/pain.001.001.09.xsd

# Generate selected root elements and their reachable types from a large schema
polyxml generate --lang rust --root-element Entity --root-element PositionReport --out ./generated/uci schemas/uci.xsd

# Dry-run inspection without writing files to disk
polyxml generate --lang rust --dry-run schemas/order.xsd
```

### 2. `polyxml build`

Build an entire multi-target, multi-schema project declaratively from a `polyxml.toml` workspace manifest:

```bash
polyxml build --config polyxml.toml
```

### 3. `polyxml validate`

Statically check W3C XML schemas for structural validity, element types, and cycle topology:

```bash
polyxml validate schemas/*.xsd
```

---

## Workspace Manifest (`polyxml.toml`)

```toml
[workspace]
name = "enterprise-data-pipeline"
schemas = ["schemas/iso20022/*.xsd"]
root_elements = ["CustomerCreditTransferInitiation", "PaymentReturn"] # Optional: restrict to specific roots
include_dirs = ["schemas/common/"]
output_base_dir = "./generated"

[[generate]]
target = "python"
output = "src/generated/python"
backend = "pydantic"
codecs = true

[[generate]]
target = "rust"
output = "src/generated/rust"
features = ["zero-copy", "rkyv"]
codecs = true

[[generate]]
target = "java"
output = "src/generated/java"
package = "com.enterprise.banking.iso20022"
backend = "jackson3" # Spring Boot 4; use "jackson" for Jackson 2

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

## 🌐 Production Reference Repositories

Explore full-scale repositories using PolyXML CLI manifests across all 7 languages:
- **[🛸 Defense & Aerospace](https://github.com/polyxml/polyxml-defense-examples)**: Anduril Lattice SDK ↔ USAF UCI v2.5 XML (includes PyO3 AOT native extension)
- **[💳 FinTech & Banking](https://github.com/polyxml/polyxml-finance-examples)**: Instant Payments ↔ ISO 20022 pacs.008 XML
- **[🚍 Smart Cities & Transit](https://github.com/polyxml/polyxml-transit-examples)**: Google GTFS-Realtime ↔ CEN SIRI v2.0 & NeTEx XML


## Shell completion

Run `polyxml completions bash`, `polyxml completions zsh`, or
`polyxml completions fish` to generate a shell script. Backend, style, and feature
suggestions are filtered by the selected language(s). See the
[compiler guide](../../docs/guides/compiler.md#shell-completion) for installation.

For Spring Boot dependency setup and the XML-text record customizer, see the
[Java guide](../../docs/languages/java.md#spring-boot-4-and-jackson-3). Generated
records and POJOs are tested on Java 25 with Spring Boot 4.1.1; run
`./scripts/verify_spring_boot.sh` from the repository root to reproduce the checks.
