#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root/benchmarks/cpp"
mkdir -p target
"$root/scripts/memcap.sh" cargo build --release -p polyxml-c
g++ -std=c++20 -O3 -Wall -Wextra -I "$root/bindings/cpp/include" -I "$root/crates/polyxml-c/include" runtime.cpp -L "$root/target/release" -lpolyxml -Wl,-rpath,"$root/target/release" -o target/runtime
./target/runtime
