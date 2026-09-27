#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root/benchmarks/cpp"
mkdir -p target/generated
"${POLYXML_BIN:-$root/target/debug/polyxml}" generate sensor.xsd --lang cpp --out target/generated
g++ -std=c++20 -O3 -Wall -Wextra -I target/generated bench.cpp -o target/bench
./target/bench
