# AGENTS.md

Instructions and guidelines for AI coding assistants working in the `PolyXML`
repository.

---

## 1. Project Overview

`PolyXML` is the high-performance, polyglot native XML data-binding engine and schema compiler, providing
ultra-fast bidirectional XML serialization, deserialization, and multi-language code generation across
7 modern ecosystems: **Rust**, **Python** (dataclasses & Pydantic v2), **C++20**, **Java 22+**, **TypeScript 5+**, **Go 1.22+**, and **C# 12 / .NET 8+**.

- **Technology**: Rust 2021, PyO3 (`abi3-py312`), `quick-xml`, `lexical-core`, `smallvec`, `minijinja`.
- **Repository**: `polyxml/PolyXML`
- **Supported Python**: `Python >= 3.12` exclusively.
- **Maintainer**: Bailey Nguyen (`bailey.tan.nguyen@gmail.com`).

---

## 2. Core Architectural Decisions & Invariants

When contributing or refactoring, strictly maintain the following invariants:

1. **Pure Rust Core Engine (`crates/polyxml-core`)**:
   - The core engine MUST have **zero Python, PyO3, or runtime-specific dependencies**.
   - All parsing, serialization, schema parser, and schema IR logic must remain 100% pure, idiomatic Rust.

2. **Python ABI3 Portability (`abi3-py312`)**:
   - The Python extension (`crates/polyxml-python`) is compiled against the stable Python 3.12+ ABI (`abi3`).
   - Do not introduce non-limited C-API calls that violate `abi3-py312` compatibility.

3. **Zero Unnecessary Allocations**:
   - Stream XML tokens using `quick-xml` reader/writer events.
   - Use `lexical-core` for high-throughput integer and float conversions from byte slices.
   - Avoid intermediate DOM allocations during parsing.

4. **Bidirectional Serialization & Deserialization**:
   - Features must support both reading (XML -> Object) and writing (Object -> XML).

5. **100% Code Coverage**:
   - All Python wrapper code in `crates/polyxml-python/python/` must maintain **100%
     statement and branch test coverage** (`fail_under = 100` in `pyproject.toml`).
   - All new features or bug fixes must include corresponding tests in `tests/`.

6. **Language-Agnostic Schema IR & Tarjan SCC Cycle-Cutting**:
   - The schema compiler in `polyxml-core` normalizes XSD into `SchemaIR`.
   - All cyclic and self-referential types must be detected and broken via Tarjan's Strongly Connected Components algorithm (`is_cycle_cut = true`) with minimal cut points (`Box<T>`, pointers, `std::unique_ptr`, `z.lazy`).

7. **Standalone W3C Conformance Benchmarks**:
   - Deep W3C XSTS conformance testing is maintained in the dedicated companion repository [`polyxml-w3c-tests`](https://github.com/polyxml/polyxml-w3c-tests) to keep the main repository CI fast. Any compiler changes should be verified against `polyxml-w3c-tests`.


---

## 3. Tooling & Development Workflow

### Rust Toolchain & Build

- Build workspace:
  ```bash
  cargo check --workspace
  ```
- Run Rust tests:
  ```bash
  cargo test --workspace
  ```

### Rust Linting & Formatting

- Formatting:
  ```bash
  cargo fmt --check
  ```
- Clippy:
  ```bash
  cargo clippy --workspace --all-targets -- -D warnings
  ```

### Python Linting & 100% Coverage Testing

- Build Python extension:
  ```bash
  cd crates/polyxml-python
  maturin develop
  ```
- Run Python tests with coverage:
  ```bash
  pytest --cov=polyxml --cov-branch --cov-fail-under=100
  ```
- Python Linting:
  ```bash
  ruff check python/ tests/
  ```

### Fast Codegen Testing & Multi-Target Verification

PolyXML provides dedicated convenience scripts to dramatically accelerate compiler development:

- **Fast Codegen Test Runner (`scripts/test_codegen.sh`)**:
  Runs all 7 language codegen test suites, schema IR tests, and CLI integration tests in ~2 seconds (bypassing unrelated benchmarks/audio tests):
  ```bash
  ./scripts/test_codegen.sh
  ```
- **End-to-End Multi-Target Smoke Test (`scripts/verify_codegen.sh`)**:
  Builds `polyxml` CLI and compiles a representative schema containing recursive types, enums, choices, and facets into all 7 target ecosystems and their backends:
  ```bash
  ./scripts/verify_codegen.sh
  ```
- **Version Parity Check & Bump (`scripts/sync_version.py`)**:
  Checks or sets identical versions across all 5 manifests (`Cargo.toml`, `pyproject.toml`, `package.json`, `pom.xml`, `CMakeLists.txt`):
  ```bash
  ./scripts/sync_version.py --check
  ./scripts/sync_version.py --set 0.17.0
  ```
- **Unified Dual-Language Quality Gate (`scripts/gate.sh`)**:
  Runs fmt, clippy, workspace tests, maturin develop, ruff, and 100% pytest coverage:
  ```bash
  ./scripts/gate.sh
  ```

---

## 4. Git Hooks & Automated Release Invariants

1. **Pre-commit & Pre-push Hooks**:
   - The repository installs git hooks that enforce version parity, `cargo fmt`, `ruff`, and `cargo check`.
   - Never bypass hooks with `--no-verify` unless strictly instructed.
2. **Automated CI Releases & Rebasing**:
   - When a commit lands on `origin/main`, CI creates an automated release commit `chore(release): X.Y.Z [skip ci]`.
   - Before pushing local changes, always run:
     ```bash
     git pull --rebase origin main
     git push origin main
     ```

---

## 5. Verification Checklist

Before completing any task:

1. **Rust Format**: Ensure `cargo fmt --check` passes with zero differences.
2. **Rust Clippy**: Ensure `cargo clippy --workspace --all-targets -- -D warnings` produces 0 warnings.
3. **Rust Tests**: Ensure `cargo test --workspace` passes cleanly (or `./scripts/test_codegen.sh` for codegen-only changes).
4. **Python Tests & Coverage**: Ensure `pytest --cov=polyxml --cov-branch --cov-fail-under=100` passes with **100% coverage**.
5. **Python Lint**: Ensure `ruff check python/ tests/` passes with 0 errors.
6. **Multi-Target Smoke Verification**: Run `./scripts/verify_codegen.sh` when modifying schema compilation or CLI flags.

---

## 6. Workspace Skills Maintenance

Custom agent runbooks and procedures are stored as skills in `.agents/skills/<skill_name>/SKILL.md`:

- **`polyxml-codegen-workflow`**: Playbook for developing, refactoring, and verifying code generators across all 7 target languages (Rust, Python, C++, Java, TypeScript, Go, C#), plumbing options from core to CLI/manifest, and validating output.
- **`polyxml-core-engine`**: High-performance streaming XML parser (`quick-xml`), zero-allocation conversions (`lexical-core`), and Tarjan SCC cycle-cutting architecture in `crates/polyxml-core`.
- **`polyxml-abi3-workflow`**: Maturin develop, `abi3-py312` conformance audits, dual-language testing, and 100% statement/branch coverage.
- **`polyxml-benchmark-workflow`**: Shared fixtures, smoke versus publishable runs, raw-result retention, and strict documentation checks.

When working in this repository:
1. **Consult & Use Skills**: When working on specific subsystems, refer to the corresponding skill in `.agents/skills/`.
2. **Keep Skills Up to Date**: If you discover a bug, an undocumented toolchain requirement, or an improved workflow while working on a task, **you MUST update the relevant `SKILL.md`** so subsequent agents benefit from the fix.
3. **Capture New Workflows**: When introducing a new complex, multi-step, or repeatable workflow, create a new skill directory in `.agents/skills/<skill_name>/SKILL.md` following standard frontmatter conventions.
