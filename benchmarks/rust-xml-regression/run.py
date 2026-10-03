#!/usr/bin/env python3
"""Same-host, alternating-order generated Rust XML/Serde regression check."""

import argparse
import json
import os
import platform
import subprocess
from pathlib import Path


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--rounds", type=int, default=2)
    args = parser.parse_args()
    if args.rounds < 1:
        parser.error("--rounds must be positive")
    root = Path(__file__).resolve().parents[2]
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    build = root / "benchmarks/rust-xml-regression/target"
    env = os.environ | {"CARGO_BUILD_JOBS": "1", "POLYXML_MEMCAP_BACKEND": "systemd"}
    metadata = {
        "os": platform.platform(),
        "cpu": Path("/proc/cpuinfo")
        .read_text()
        .split("model name\t: ")[1]
        .splitlines()[0],
        "rounds": args.rounds,
        "warmup": "100 operations per operation/size",
        "samples": "7 x >=250ms per operation/size/process",
        "rustc": run(["rustc", "-Vv"], capture_output=True).stdout,
        "revisions": {},
    }
    (output / "harness.rs").write_bytes(
        (root / "benchmarks/rust-xml-regression/main.rs").read_bytes()
    )
    executables = {}
    for label, repo in [("baseline", args.baseline.resolve()), ("current", root)]:
        metadata["revisions"][label] = run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True
        ).stdout.strip()
        with (output / f"{label}-build.txt").open("w") as log:
            run(
                [
                    str(root / "scripts/memcap.sh"),
                    "cargo",
                    "build",
                    "-p",
                    "polyxml-cli",
                ],
                cwd=repo,
                env=env,
                stdout=log,
                stderr=subprocess.STDOUT,
            )
            for mode in ["borrowed", "owned"]:
                crate = build / f"{label}-{mode}"
                (crate / "src").mkdir(parents=True, exist_ok=True)
                source = (root / "benchmarks/rust-xml-regression/main.rs").read_text()
                if mode == "owned":
                    source = source.replace("BatchType<'_>", "BatchType")
                (crate / "src/main.rs").write_text(source)
                run(
                    [
                        str(repo / "target/debug/polyxml"),
                        "generate",
                        str(root / "benchmarks/workloads/sensor-batch/batch.xsd"),
                        "--lang",
                        "rust",
                        f"--zero-copy={'true' if mode == 'borrowed' else 'false'}",
                        "--out",
                        str(crate / "src"),
                    ],
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
                (crate / "Cargo.toml").write_text(
                    '[package]\nname="xml-regression"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\npolyxml={path='
                    + json.dumps(str(repo / "crates/polyxml-core"))
                    + '}\nquick-xml="=0.42.0"\nserde={version="1",features=["derive"]}\nserde_json="1"\n'
                )
                # Keep dependency resolutions identical across consumers. The only
                # local package version or newly required dependencies may differ.
                seed = build / "baseline-borrowed/Cargo.lock"
                if seed.exists() and seed != crate / "Cargo.lock":
                    (crate / "Cargo.lock").write_bytes(seed.read_bytes())
                run(
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
                )
                executables[f"{label}-{mode}"] = crate / "target/release/xml-regression"
                (output / f"{label}-{mode}.lock").write_bytes(
                    (crate / "Cargo.lock").read_bytes()
                )
                (output / f"{label}-{mode}-batch.rs").write_bytes(
                    (crate / "src/batch.rs").read_bytes()
                )
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    for round_index in range(args.rounds):
        labels = (
            ["baseline", "current"] if round_index % 2 == 0 else ["current", "baseline"]
        )
        for mode in ["borrowed", "owned"]:
            for label in labels:
                with (output / f"{label}-{mode}-{round_index}.csv").open("w") as log:
                    run(
                        [
                            str(root / "scripts/memcap.sh"),
                            str(executables[f"{label}-{mode}"]),
                            str(root / "benchmarks/workloads/sensor-batch"),
                        ],
                        env=env,
                        stdout=log,
                    )


if __name__ == "__main__":
    main()
