#!/usr/bin/env python3
"""Build identical allocation diagnostics against an explicit source checkout."""

import argparse
import json
import os
import platform
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    repo = args.repo.resolve()
    revision = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True
    ).strip()
    core_status = subprocess.check_output(
        ["git", "-C", str(repo), "status", "--porcelain", "--", "crates/polyxml-core"],
        text=True,
    )
    if core_status:
        parser.error("commit core changes before collecting diagnostics")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    crate = root / "benchmarks/rust-runtime-investigation/target" / args.label
    (crate / "src").mkdir(parents=True, exist_ok=True)
    fixtures = (
        (root / "crates/polyxml-core/benches/core_benchmarks.rs")
        .read_text()
        .split("fn bench_deserialization")[0]
    )
    fixtures = "\n".join(
        line for line in fixtures.splitlines() if not line.startswith("use criterion::")
    )
    (crate / "src/fixtures.rs").write_text(fixtures + "\n")
    (crate / "src/main.rs").write_bytes(
        (Path(__file__).parent / "diagnostics.rs").read_bytes()
    )
    (crate / "Cargo.toml").write_text(
        '[package]\nname="xml-diagnostics"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\npolyxml={path='
        + json.dumps(str(repo / "crates/polyxml-core"))
        + "}\n[profile.release]\ndebug=1\n"
    )
    for name, source in [
        ("harness.rs", crate / "src/main.rs"),
        ("fixtures.rs", crate / "src/fixtures.rs"),
        ("Cargo.toml", crate / "Cargo.toml"),
    ]:
        (output / f"{args.label}-{name}").write_bytes(source.read_bytes())
    seed = root / "benchmarks/rust-xml-regression/target/core-current/Cargo.lock"
    if seed.exists():
        (crate / "Cargo.lock").write_bytes(seed.read_bytes())
    env = os.environ | {"CARGO_BUILD_JOBS": "1", "POLYXML_MEMCAP_BACKEND": "systemd"}
    with (output / f"{args.label}-build.txt").open("w") as log:
        subprocess.run(
            [
                str(root / "scripts/memcap.sh"),
                "cargo",
                "build",
                "--release",
                "--manifest-path",
                str(crate / "Cargo.toml"),
            ],
            env=env,
            stdout=log,
            stderr=subprocess.STDOUT,
            check=True,
        )
    executable = crate / "target/release/xml-diagnostics"
    with (output / f"{args.label}-allocations.txt").open("w") as log:
        for count in [0, 1000, 10000]:
            for operation in ["read", "write"]:
                subprocess.run(
                    [
                        str(root / "scripts/memcap.sh"),
                        str(executable),
                        operation,
                        str(count),
                        "1",
                    ],
                    env=env,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    check=True,
                )
    (output / f"{args.label}.lock").write_bytes((crate / "Cargo.lock").read_bytes())
    (output / f"{args.label}-metadata.json").write_text(
        json.dumps(
            {
                "revision": revision,
                "core_status": core_status,
                "os": platform.platform(),
                "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
                "executable": str(executable),
                "profile": "release debug=1; allocation instrumentation; not used for latency numbers",
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
