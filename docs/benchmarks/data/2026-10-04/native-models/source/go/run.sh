#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root/benchmarks/go"
mkdir -p target/generated
"${POLYXML_BIN:-$root/target/debug/polyxml}" generate sensor.xsd --lang go --out target/generated
go test -run '^$' -bench BenchmarkXML -benchmem -count "${BENCH_COUNT:-1}" -benchtime "${BENCH_TIME:-1s}" ./...
