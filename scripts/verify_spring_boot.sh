#!/usr/bin/env bash
# Compile generated records/POJOs and execute Boot HTTP and mapper round trips.
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
java -version
if ! javac -version 2>&1 | grep -qE '^javac 25([.]|$)'; then
  echo 'Set JAVA_HOME and PATH to a Java 25 JDK.' >&2
  exit 1
fi
cargo build --manifest-path "$ROOT_DIR/Cargo.toml" -p polyxml-cli
TEST_DIR="$(mktemp -d)"
trap 'rm -rf "$TEST_DIR"' EXIT
cp -R "$ROOT_DIR/tests/java-spring/." "$TEST_DIR/"
for style in pojo record; do
  "$ROOT_DIR/target/debug/polyxml" generate "$TEST_DIR/schema.xsd" --lang java \
    --backend jackson3 --style "$style" --feature builder,validation,direct-codec \
    --package "example.$style" --out "$TEST_DIR/src/main/java/example/$style"
done
mvn -B -f "$TEST_DIR/pom.xml" test
