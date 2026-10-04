#!/usr/bin/env python3
"""Repeat an already-built C# comparison with tiered compilation disabled."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
from pathlib import Path

from run_models import ROOT, capture, run


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    directory = args.directory.resolve()
    assert (directory / "metadata.json").is_file(), "Run run_models.py first"
    assert not list(directory.glob("csharp-steady-*.txt")), "Preserve existing runs"
    dll = ROOT / "benchmarks/csharp/bin/Release/net8.0/Benchmark.dll"
    env = dict(
        os.environ,
        DOTNET_ROLL_FORWARD="Major",
        DOTNET_TieredCompilation="0",
        POLYXML_MEMCAP_BACKEND="systemd",
        BENCH_ITERATIONS="1000",
        BENCH_WARMUP_MS="2000",
    )
    metadata = {
        "revision": capture(["git", "rev-parse", "HEAD"], env),
        "tiered_compilation": False,
        "processes": 6,
        "iterations": 1000,
        "minimum_warmup_pairs": 500,
        "minimum_warmup_ms": 2000,
        "dll_sha256": hashlib.sha256(dll.read_bytes()).hexdigest(),
        "note": "Default-tiering runs retained separately as diagnostics; first-model effect persisted after warmup.",
    }
    (directory / "csharp-control.json").write_text(
        json.dumps(metadata, indent=2) + "\n"
    )
    shutil.copyfile(Path(__file__), directory / "source/control_csharp.py")
    for repeat in range(6):
        print(f"C# full-JIT control {repeat + 1}/6", flush=True)
        current = dict(env, BENCH_BASELINE_FIRST=str(repeat % 2))
        run(
            [ROOT / "scripts/memcap.sh", "dotnet", dll],
            current,
            directory,
            f"csharp-steady-{repeat}.txt",
            cwd=ROOT / "benchmarks/csharp",
        )


if __name__ == "__main__":
    main()
