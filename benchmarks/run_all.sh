#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# PolyXML Rust and Python Benchmarking Suite
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Cap the whole run at POLYXML_MEMCAP_PCT (default 60%) of available RAM via
# scripts/memcap.sh: release builds + Criterion can exhaust small hosts and
# freeze them. The LEVEL guard makes the re-exec idempotent (no loop) on
# hosts without a systemd user manager; POLYXML_MEMCAP_DISABLE=1 opts out.
if [ -z "${POLYXML_MEMCAP_LEVEL:-}" ]; then
    exec "${ROOT_DIR}/scripts/memcap.sh" "${BASH_SOURCE[0]}" "$@"
fi

echo "======================================================================"
echo "⚡ PolyXML Rust & Python Benchmarking Suite"
echo "======================================================================"

# 1. Ensure Rust toolchain
if command -v cargo >/dev/null 2>&1; then
    CARGO_BIN="cargo"
elif [ -f "$HOME/.cargo/bin/cargo" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
    CARGO_BIN="cargo"
else
    echo "❌ Error: cargo not found."
    exit 1
fi

# 2. Ensure Python environment
PYTHON_BIN="${ROOT_DIR}/.venv/bin/python"
if [ ! -f "${PYTHON_BIN}" ]; then
    echo "Creating virtual environment at .venv..."
    uv venv "${ROOT_DIR}/.venv" --python 3.12
fi

echo "Installing benchmark requirements..."
uv pip install -q -r "${SCRIPT_DIR}/python/requirements.txt" maturin

# 3. Build Rust Python extension in Release mode
echo ""
echo "📦 Building polyxml-python in release mode..."
cd "${ROOT_DIR}/crates/polyxml-python"
"${ROOT_DIR}/.venv/bin/maturin" develop --release

# 4. Run Rust Criterion Benchmarks (Pure Core Engine)
echo ""
echo "🦀 Running Rust polyxml-core Criterion benchmarks..."
cd "${ROOT_DIR}"
"${CARGO_BIN}" bench --bench core_benchmarks

# 5. Run Python Comparative Multi-Parser Benchmarks
echo ""
echo "🐍 Running Python comparative benchmarks..."
cd "${ROOT_DIR}"
"${PYTHON_BIN}" -m benchmarks.python \
    --workload all \
    --catalog-sizes 1000 10000 \
    --iterations 25 \
    --output-md "${SCRIPT_DIR}/python/results.md" \
    --output-json "${SCRIPT_DIR}/python/results.json"

echo ""
echo "======================================================================"
echo "✅ Benchmarking Complete!"
echo "   - Rust reports: target/criterion/"
echo "   - Python Markdown: benchmarks/python/results.md"
echo "   - Python JSON: benchmarks/python/results.json"
echo "   - Other language suites: see benchmarks/README.md"
echo "======================================================================"
