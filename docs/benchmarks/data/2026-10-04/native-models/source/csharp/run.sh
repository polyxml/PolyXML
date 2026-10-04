#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root/benchmarks/csharp"
mkdir -p target/generated
"${POLYXML_BIN:-$root/target/debug/polyxml}" generate sensor.xsd --lang csharp --style class --out target/generated
dotnet run -c Release --project Benchmark.csproj
