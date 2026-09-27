#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root/benchmarks/workloads/sensor-batch"
mkdir -p target/python
"${POLYXML_BIN:-$root/target/debug/polyxml}" generate batch.xsd --lang python --out target/python
"${POLYXML_PYTHON:-$root/.venv/bin/python}" python_bench.py
