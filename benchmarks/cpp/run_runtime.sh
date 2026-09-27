#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root/benchmarks/cpp"
mkdir -p target/generated
"$root/scripts/memcap.sh" cargo build --release -p polyxml-c -p polyxml-cli
"${POLYXML_BIN:-$root/target/release/polyxml}" generate "$root/benchmarks/workloads/sensor-batch/batch.xsd" --lang cpp --out target/generated
g++ -std=c++20 -O3 -Wall -Wextra -I target/generated -I "$root/bindings/cpp/include" -I "$root/crates/polyxml-c/include" runtime.cpp -L "$root/target/release" -lpolyxml -Wl,-rpath,"$root/target/release" -o target/runtime
./target/runtime
