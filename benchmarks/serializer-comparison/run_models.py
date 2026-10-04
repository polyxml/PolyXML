#!/usr/bin/env python3
"""Refresh native-model Go/C# and matched Java JMH comparisons, serially."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def run(args, env, directory, filename, cwd=ROOT):
    with (directory / filename).open("w") as stream:
        subprocess.run(
            [str(a) for a in args],
            env=env,
            cwd=cwd,
            check=True,
            stdout=stream,
            stderr=subprocess.STDOUT,
        )


def capture(args, env):
    return subprocess.run(
        [str(a) for a in args],
        env=env,
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    ).stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--maven", type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    env = dict(
        os.environ,
        CARGO_BUILD_JOBS="1",
        POLYXML_MEMCAP_BACKEND="systemd",
        DOTNET_ROLL_FORWARD="Major",
        POLYXML_BIN=str(ROOT / "target/debug/polyxml"),
    )
    meta = {
        "revision": capture(["git", "rev-parse", "HEAD"], env),
        "status": capture(["git", "status", "--short"], env),
        "go": capture(["go", "version"], env),
        "dotnet": capture(["dotnet", "--info"], env),
        "java": capture(["java", "-version"], env),
        "maven": capture([args.maven, "-version"], env),
        "cli": capture([env["POLYXML_BIN"], "--version"], env),
        "settings": {
            "go_processes": 5,
            "go_benchtime": "2s",
            "csharp_processes": 5,
            "csharp_iterations": 1000,
            "csharp_min_warmup_pairs": 500,
            "csharp_min_warmup_ms": 2000,
            "java_batch_size": 1000,
            "java_forks": 2,
            "java_warmup": "3 x 2s",
            "java_measurement": "5 x 2s",
        },
    }
    (out / "metadata.json").write_text(json.dumps(meta, indent=2) + "\n")
    source = out / "source"
    source.mkdir()
    for runtime in ("go", "csharp", "java"):
        shutil.copytree(
            ROOT / f"benchmarks/{runtime}",
            source / runtime,
            ignore=shutil.ignore_patterns("target", "bin", "obj", "__pycache__"),
        )
    for repeat in range(5):
        print(f"Go process {repeat + 1}/5", flush=True)
        current = dict(
            env, BENCH_COUNT="1", BENCH_TIME="2s", BENCH_BASELINE_FIRST=str(repeat % 2)
        )
        run(
            [ROOT / "scripts/memcap.sh", "bash", ROOT / "benchmarks/go/run.sh"],
            current,
            out,
            f"go-{repeat}.txt",
        )
    run(
        [
            env["POLYXML_BIN"],
            "generate",
            "benchmarks/csharp/sensor.xsd",
            "--lang",
            "csharp",
            "--style",
            "class",
            "--out",
            "benchmarks/csharp/target/generated",
        ],
        env,
        out,
        "csharp-codegen.log",
    )
    run(
        [
            ROOT / "scripts/memcap.sh",
            "dotnet",
            "build",
            "-c",
            "Release",
            "benchmarks/csharp/Benchmark.csproj",
        ],
        env,
        out,
        "csharp-build.log",
    )
    for repeat in range(5):
        print(f"C# process {repeat + 1}/5", flush=True)
        current = dict(
            env,
            BENCH_ITERATIONS="1000",
            BENCH_WARMUP_MS="2000",
            BENCH_BASELINE_FIRST=str(repeat % 2),
        )
        run(
            [
                ROOT / "scripts/memcap.sh",
                "dotnet",
                ROOT / "benchmarks/csharp/bin/Release/net8.0/Benchmark.dll",
            ],
            current,
            out,
            f"csharp-{repeat}.txt",
            cwd=ROOT / "benchmarks/csharp",
        )
    print("Java clean build and interoperability tests", flush=True)
    run(
        [
            ROOT / "scripts/memcap.sh",
            args.maven,
            "-f",
            "benchmarks/java/pom.xml",
            "clean",
            "package",
        ],
        env,
        out,
        "java-build.log",
    )
    run(
        [args.maven, "-f", "benchmarks/java/pom.xml", "dependency:tree"],
        env,
        out,
        "java-dependencies.txt",
    )
    print("Java JMH: two forks per case", flush=True)
    run(
        [
            ROOT / "scripts/memcap.sh",
            "java",
            "-jar",
            "benchmarks/java/target/benchmarks.jar",
            r"BindingBenchmark\.(direct|jackson|jaxb)(Read|Write)$",
            "-p",
            "workload=settlement,telemetry",
            "-p",
            "batchSize=1000",
            "-wi",
            "3",
            "-i",
            "5",
            "-w",
            "2s",
            "-r",
            "2s",
            "-f",
            "2",
            "-foe",
            "true",
            "-jvmArgsAppend",
            "-Xms256m -Xmx1g",
            "-prof",
            "gc",
            "-rf",
            "json",
            "-rff",
            out / "java-jmh.json",
        ],
        env,
        out,
        "java-jmh.txt",
    )
    for runtime in ("go", "csharp"):
        shutil.copytree(
            ROOT / f"benchmarks/{runtime}/target/generated",
            source / f"{runtime}-generated",
        )
    shutil.copytree(
        ROOT / "benchmarks/java/target/generated-sources", source / "java-generated"
    )
    print(f"Retained model comparisons: {out}")


if __name__ == "__main__":
    main()
