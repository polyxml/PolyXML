#!/usr/bin/env bash
# ==============================================================================
# scripts/test_codegen.sh - Ultra-Fast Codegen Test Runner for PolyXML
# ==============================================================================
# Runs all 7 language codegen test suites, schema IR tests, and CLI integration
# tests without running unrelated binary/audio/benchmark tests.
# ==============================================================================
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}==> [1/3] Running pure Rust schema IR & Tarjan SCC tests...${NC}"
cargo test -p polyxml --test test_schema_ir --test test_upa

echo -e "${BLUE}==> [2/3] Running all 7 language codegen test suites...${NC}"
cargo test -p polyxml \
  --test test_rust_codegen \
  --test test_python_codegen \
  --test test_cpp_codegen \
  --test test_java_codegen \
  --test test_ts_codegen \
  --test test_go_codegen \
  --test test_csharp_codegen \
  --test test_pattern_codegen

echo -e "${BLUE}==> [3/3] Running PolyXML CLI code generation & manifest integration tests...${NC}"
cargo test -p polyxml-cli --test test_cli

echo -e "${GREEN}✨ All PolyXML polyglot codegen test suites passed!${NC}"
