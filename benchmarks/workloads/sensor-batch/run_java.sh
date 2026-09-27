#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$root/benchmarks/workloads/sensor-batch"
mkdir -p target/java/generated target/java/classes
"${POLYXML_BIN:-$root/target/debug/polyxml}" generate batch.xsd --lang java --style pojo --feature direct-codec --out target/java/generated
javac -d target/java/classes target/java/generated/*.java java/SharedSensorBench.java
java -cp target/java/classes SharedSensorBench
