#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root/benchmarks/workloads/sensor-batch"
if [ ! -f "$root/crates/polyxml-wasm/pkg/index.node.js" ]; then
    echo "Build the Wasm package first: ./crates/polyxml-wasm/build.sh" >&2
    exit 1
fi
node wasm.mjs
