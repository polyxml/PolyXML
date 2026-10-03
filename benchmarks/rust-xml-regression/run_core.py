#!/usr/bin/env python3
"""Run an identical Criterion XML harness against two core revisions."""

import argparse
import json
import os
import platform
import shutil
import subprocess
from pathlib import Path


def run(args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--rounds", type=int, default=2)
    parser.add_argument("--samples", type=int, default=100)
    parser.add_argument("--warmup", type=float, default=3)
    parser.add_argument("--measurement", type=float, default=5)
    parser.add_argument("--filter", default="")
    parser.add_argument("--harness", type=Path)
    args = parser.parse_args()
    if args.rounds < 1:
        parser.error("--rounds must be positive")
    root = Path(__file__).resolve().parents[2]
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    build = root / "benchmarks/rust-xml-regression/target"
    env = os.environ | {"CARGO_BUILD_JOBS": "1", "POLYXML_MEMCAP_BACKEND": "systemd"}
    # Use one harness for both revisions, with round-trip checks outside timing.
    source = (
        args.harness or root / "crates/polyxml-core/benches/core_benchmarks.rs"
    ).read_text()
    source = source.replace(
        "    (schema, xml)\n",
        '    let value = deserialize(&xml, Arc::clone(&schema)).unwrap();\n    let output = serialize("Sensor", &value, &schema, None).unwrap();\n    assert_eq!(value, deserialize(&output, Arc::clone(&schema)).unwrap());\n    (schema, xml)\n',
    )
    source = source.replace(
        "    (catalog_schema, xml.into_bytes())",
        '    let xml = xml.into_bytes();\n    let value = deserialize(&xml, Arc::clone(&catalog_schema)).unwrap();\n    assert_eq!(value.get("items").unwrap().as_list().unwrap().len(), count);\n    let output = serialize("Catalog", &value, &catalog_schema, None).unwrap();\n    assert_eq!(value, deserialize(&output, Arc::clone(&catalog_schema)).unwrap());\n    (catalog_schema, xml)',
    )
    (output / "harness.rs").write_text(source)
    executables = {}
    revisions = {}
    for label, repo in [("baseline", args.baseline.resolve()), ("current", root)]:
        revisions[label] = run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True
        ).stdout.strip()
        crate = build / f"core-{label}"
        (crate / "benches").mkdir(parents=True, exist_ok=True)
        (crate / "benches/xml.rs").write_text(source)
        (crate / "Cargo.toml").write_text(
            '[package]\nname="core-regression"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\npolyxml={path='
            + json.dumps(str(repo / "crates/polyxml-core"))
            + '}\ncriterion={version="=0.8.2",default-features=false,features=["cargo_bench_support"]}\n[[bench]]\nname="xml"\nharness=false\n'
        )
        seed = build / "core-baseline/Cargo.lock"
        if seed.exists() and seed != crate / "Cargo.lock":
            (crate / "Cargo.lock").write_bytes(seed.read_bytes())
        with (output / f"{label}-build.txt").open("w") as log:
            result = run(
                [
                    str(root / "scripts/memcap.sh"),
                    "cargo",
                    "bench",
                    "--no-run",
                    "--message-format=json",
                    "--manifest-path",
                    str(crate / "Cargo.toml"),
                ],
                env=env,
                stdout=subprocess.PIPE,
                stderr=log,
            )
        for line in result.stdout.splitlines():
            artifact = json.loads(line)
            if (
                artifact.get("reason") == "compiler-artifact"
                and artifact.get("executable")
                and artifact["target"]["name"] == "xml"
            ):
                executables[label] = (crate, artifact["executable"])
        (output / f"{label}.lock").write_bytes((crate / "Cargo.lock").read_bytes())
    (output / "metadata.json").write_text(
        json.dumps(
            {
                "revisions": revisions,
                "sample_size": args.samples,
                "warmup_seconds": args.warmup,
                "measurement_seconds": args.measurement,
                "rounds": args.rounds,
                "filter": args.filter,
                "order": "alternate baseline/current per round",
                "os": platform.platform(),
                "cpu": Path("/proc/cpuinfo")
                .read_text()
                .split("model name\t: ")[1]
                .splitlines()[0],
                "rustc": run(["rustc", "-Vv"], capture_output=True).stdout,
            },
            indent=2,
        )
        + "\n"
    )
    for round_index in range(args.rounds):
        for label in (
            ["baseline", "current"] if round_index % 2 == 0 else ["current", "baseline"]
        ):
            crate, executable = executables[label]
            # A filter must not retain results from earlier unfiltered runs.
            shutil.rmtree(crate / "target/criterion", ignore_errors=True)
            with (output / f"{label}-{round_index}.txt").open("w") as log:
                run(
                    [
                        str(root / "scripts/memcap.sh"),
                        executable,
                        "--bench",
                        "--noplot",
                        "--sample-size",
                        str(args.samples),
                        "--warm-up-time",
                        str(args.warmup),
                        "--measurement-time",
                        str(args.measurement),
                        *([args.filter] if args.filter else []),
                    ],
                    cwd=crate,
                    env=env,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                )
            shutil.copytree(
                crate / "target/criterion",
                output / f"{label}-{round_index}",
                ignore=shutil.ignore_patterns("report", "*.svg", "*.html"),
                dirs_exist_ok=True,
            )


if __name__ == "__main__":
    main()
