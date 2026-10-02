# Contributing to PolyXML

Thank you for your interest in contributing to **PolyXML**! As a polyglot, high-performance native XML engine, contributions across any of our supported language ecosystems (Rust, Python, C++, Go, Java, TypeScript) are welcome.

---

## 1. Code of Conduct

All contributors and participants agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## 2. Development Prerequisites

- **Rust**: Latest stable toolchain via `rustup` (with `rustfmt` and `clippy`).
- **Python**: Python >= 3.12 with `uv` or `pip` and `maturin`.
- **Go**: Go >= 1.22 (for Go bindings).
- **C++**: CMake >= 3.20 and a C++20-compliant compiler (GCC 11+, Clang 13+, or MSVC 2022).
- **Node.js**: Node.js >= 20 with npm (for TypeScript bindings).
- **Java**: OpenJDK >= 22 (with Project Panama support) and Maven >= 3.8.

---

## 3. Workflow & Verification

### Git Hooks (Recommended)

To run automated formatting, linting, and version parity checks locally before committing:
```bash
# Enable repository git hooks (zero external dependencies):
git config core.hooksPath .githooks

# Or, if using the Python pre-commit framework:
pre-commit install
```

Before submitting any Pull Request, ensure the respective checks pass:

### Rust Core & C-ABI
```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Python Bindings (100% Coverage Enforced)
```bash
cd crates/polyxml-python
maturin develop
ruff check python/ tests/
pytest --cov=polyxml --cov-branch --cov-fail-under=100
```

### Go Bindings
```bash
cargo build --release -p polyxml-c
cd bindings/go
go test -v ./...
```

### C++20 Bindings
```bash
cargo build --release -p polyxml-c
cd bindings/cpp
cmake -B build
cmake --build build
ctest --test-dir build --output-on-failure
```

### TypeScript / Node.js Bindings
```bash
cd crates/polyxml-js
napi build --platform --release
npm test
```

---

## 4. Submitting a Pull Request

1. Fork the repository and create a feature branch (`git checkout -b feat/my-feature`).
2. Implement your changes, including corresponding unit/integration tests.
3. Commit with clear, conventional messages (e.g. `feat(core): add support for mixed content nodes`).
4. Push to your fork and open a Pull Request against `main`. Ensure all CI checks pass.

### Java 25 and Spring Boot integration

Set `JAVA_HOME` and `PATH` to a Java 25 JDK, install Maven, and run:

```bash
./scripts/verify_spring_boot.sh
```

This generates records and POJOs, runs Spring Boot 4.1.1 JSON/XML HTTP tests,
checks validation, and validates returned XML against the fixture XSD. Maven
downloads uncached dependencies. The dedicated Java 25 CI job runs the same
command. See [the fixture](tests/java-spring/README.md) and
[Java setup](docs/languages/java.md#spring-boot-4-and-jackson-3).
