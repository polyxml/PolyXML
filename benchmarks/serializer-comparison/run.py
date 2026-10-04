#!/usr/bin/env python3
"""Build separately instrumented consumers, retain all samples and verify outputs."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import platform
import shutil
import subprocess
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SUITE = Path(__file__).resolve().parent
TARGET = SUITE / "target"


def command(args, **kwargs):
    return subprocess.run([str(a) for a in args], cwd=ROOT, check=True, **kwargs)


def capture(args):
    return command(args, stdout=subprocess.PIPE, text=True).stdout.strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_exports(directory):
    checked = []
    for path in sorted(directory.glob("*.xml")):
        count = 1 if path.stem.endswith("sensor_1") else 1000
        escaped = path.stem.endswith("escaped_1000")
        root = ET.fromstring(path.read_bytes())
        assert root.tag == "Batch", path
        children = list(root)
        assert len(children) == count, path
        for i, node in enumerate(children):
            assert node.tag == "Sensor" and [c.tag for c in node] == ["Id", "Value"], (
                path
            )
            expected = f'sensor-{i} & <é> "quoted"' if escaped else f"sensor-{i}"
            assert node[0].text == expected and int(node[1].text) == i, path
        checked.append(
            {"file": path.name, "sha256": sha(path), "bytes": path.stat().st_size}
        )
    assert len(checked) == 33, (
        f"Expected 18 Rust + 15 Python XML outputs, got {len(checked)}"
    )
    return checked


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--python", type=Path, default=TARGET / "venv/bin/python")
    parser.add_argument("--rounds", type=int, default=6)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--millis", type=int, default=150)
    args = parser.parse_args()
    if min(args.rounds, args.samples, args.millis) < 1:
        parser.error("rounds, samples and millis must be positive")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)  # preserve previous measurements
    source = output / "source"
    source.mkdir()
    for path in SUITE.glob("*.py"):
        shutil.copyfile(path, source / path.name)
    shutil.copyfile(SUITE / "rust.rs", source / "rust.rs")
    env = dict(os.environ, CARGO_BUILD_JOBS="1", POLYXML_MEMCAP_BACKEND="systemd")
    manifest = TARGET / "rust-consumer/Cargo.toml"
    binary = TARGET / "rust-consumer/target/release/polyxml-serializer-comparison"
    binaries = TARGET / "bin"
    binaries.mkdir(exist_ok=True)
    metadata = {
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "revision": capture(["git", "rev-parse", "HEAD"]),
        "git_status": capture(["git", "status", "--short"]),
        "polyxml_cli": capture([ROOT / "target/debug/polyxml", "--version"]),
        "platform": platform.platform(),
        "cpu": capture(["lscpu"]),
        "memory": Path("/proc/meminfo").read_text(),
        "rustc": capture(["rustc", "-Vv"]),
        "cargo": capture(["cargo", "-V"]),
        "python": capture([args.python, "--version"]),
        "python_packages": capture(["uv", "pip", "freeze", "--python", args.python]),
        "native": json.loads(
            capture(
                [
                    args.python,
                    "-c",
                    "import hashlib,json,polyxml,polyxml._polyxml as n; from pathlib import Path; print(json.dumps(dict(version=polyxml.__version__,path=n.__file__,sha256=hashlib.sha256(Path(n.__file__).read_bytes()).hexdigest())))",
                ]
            )
        ),
        "rounds": args.rounds,
        "samples": args.samples,
        "millis": args.millis,
        "source_sha256": {p.name: sha(p) for p in source.iterdir()},
        "host_note": "WSL host with IDE services; no concurrent local build or benchmark during measured rounds. No CPU affinity or frequency controls.",
    }
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    for feature, name in [(False, "timing"), (True, "allocations")]:
        build = [
            ROOT / "scripts/memcap.sh",
            "cargo",
            "build",
            "--release",
            "--manifest-path",
            manifest,
        ]
        if feature:
            build += ["--features", "allocations"]
        with (output / f"build-{name}.log").open("w") as log:
            command(build, env=env, stdout=log, stderr=subprocess.STDOUT)
        shutil.copyfile(binary, binaries / name)
        (binaries / name).chmod(0o755)
    for path in (TARGET / "rust-consumer/src").iterdir():
        shutil.copyfile(
            path,
            source / path.name
            if path.name != "main.rs"
            else source / "consumer-main.rs",
        )
    shutil.copyfile(manifest, source / "Cargo.toml")
    shutil.copyfile(manifest.parent / "Cargo.lock", source / "rust-consumer.lock")
    shutil.copyfile(TARGET / "python-models/batch.py", source / "python-models.py")
    shutil.copytree(TARGET / "fixtures", output / "fixtures")
    measured_env = dict(
        env,
        BENCH_SAMPLES=str(args.samples),
        BENCH_MILLIS=str(args.millis),
        BENCH_EXPORT=str(output / "verified-outputs"),
    )
    # Alternate runtime order. Rust and Python are never measured concurrently.
    for repeat in range(args.rounds):
        measured_env["BENCH_ORDER"] = str(repeat)
        lanes = ["rust", "python"] if repeat % 2 == 0 else ["python", "rust"]
        for runtime in lanes:
            print(f"round {repeat + 1}/{args.rounds}: {runtime}", flush=True)
            run = (
                [binaries / "timing", TARGET / "fixtures"]
                if runtime == "rust"
                else [args.python, SUITE / "python_bench.py", TARGET]
            )
            with (
                (output / f"{runtime}-{repeat}.csv").open("w") as raw,
                (output / f"{runtime}-{repeat}.log").open("w") as log,
            ):
                command(
                    [ROOT / "scripts/memcap.sh", *run],
                    env=measured_env,
                    stdout=raw,
                    stderr=log,
                )
    with (output / "rust-allocations.csv").open("w") as raw:
        command(
            [binaries / "allocations", TARGET / "fixtures"],
            env=measured_env,
            stdout=raw,
        )
    metadata["independent_xml_checks"] = validate_exports(output / "verified-outputs")
    metadata["source_sha256"] = {p.name: sha(p) for p in source.iterdir()}
    metadata["binary_sha256"] = {p.name: sha(p) for p in binaries.iterdir()}
    metadata["fixture_sha256"] = {
        p.name: sha(p) for p in (output / "fixtures").iterdir()
    }
    for runtime, expected in [("rust", 54), ("python", 48)]:
        for repeat in range(args.rounds):
            with (output / f"{runtime}-{repeat}.csv").open() as stream:
                rows = list(csv.DictReader(stream))
            assert len(rows) == expected * args.samples
    metadata["finished_utc"] = datetime.now(timezone.utc).isoformat()
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(f"Retained samples and verified 33 XML outputs: {output}")


if __name__ == "__main__":
    main()
