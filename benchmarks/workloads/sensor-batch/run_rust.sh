#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
if [ -z "${POLYXML_MEMCAP_LEVEL:-}" ]; then
    exec "$root/scripts/memcap.sh" "$0" "$@"
fi
cd "$root/benchmarks/workloads/sensor-batch"
mkdir -p target/rust
"${POLYXML_BIN:-$root/target/debug/polyxml}" generate batch.xsd --lang rust --out target/rust
CARGO_TARGET_DIR="$root/benchmarks/rust-phf-e2e/target" cargo run --release --quiet --manifest-path rust/Cargo.toml
