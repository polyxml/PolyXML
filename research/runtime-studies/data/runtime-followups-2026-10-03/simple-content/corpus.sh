#!/usr/bin/env bash
set -euo pipefail
root=/home/bailey/github/PolyXML
repo=/home/bailey/github/polyxml-simple-content
companion=/home/bailey/github/polyxml-w3c-tests
export POLYXML_MEMCAP_BACKEND=systemd CARGO_BUILD_JOBS=1 DOTNET_ROLL_FORWARD=Major
export CARGO_TARGET_DIR="$root/target/followup-simple-content-verification"
cd "$repo"
scripts/memcap.sh scripts/verify_codegen.sh > /tmp/polyxml-simple-content-smoke.log 2>&1
mkdir -p /tmp/polyxml-followup-simple-content-bin
cp "$CARGO_TARGET_DIR/debug/polyxml" /tmp/polyxml-followup-simple-content-bin/polyxml
export PATH=/tmp/polyxml-followup-simple-content-bin:$PATH
test ! -e "$root/target/release/polyxml"
scripts/memcap.sh uv --directory "$companion" run --no-sync "$root/.venv/bin/maturin" develop --manifest-path "$repo/crates/polyxml-python/Cargo.toml" > /tmp/polyxml-simple-content-companion-binding.log 2>&1
uv --directory "$companion" run --no-sync python /tmp/polyxml-probe-simple-content.py > /tmp/polyxml-simple-content-probe.json 2> /tmp/polyxml-simple-content-probe.log
for suite in CType AttrDecl NIST; do
 limit=50
 if [[ "$suite" == NIST ]]; then limit=100; fi
 scripts/memcap.sh uv --directory "$companion" run --no-sync runner.py --suite "$suite" --limit "$limit" > "/tmp/polyxml-simple-content-$suite.log" 2>&1
done
