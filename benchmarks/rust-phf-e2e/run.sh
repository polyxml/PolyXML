#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root/benchmarks/rust-phf-e2e"
python3 generate.py
for size in 16 120; do
    for strategy in match phf; do
        mkdir -p "target/$size/$strategy"
        flags=()
        if [ "$strategy" = phf ]; then flags=(--feature phf); fi
        "${POLYXML_BIN:-$root/target/debug/polyxml}" generate "target/$size/record.xsd" --lang rust "${flags[@]}" --out "target/$size/$strategy"
    done
done
cargo run --release --quiet
