#!/usr/bin/env python3
"""Prepare matched fixtures and standalone Rust consumers; no timing here."""

import argparse
import json
import shutil
import subprocess
from pathlib import Path
from xml.sax.saxutils import escape

import tomllib

ROOT = Path(__file__).resolve().parents[2]
SUITE = ROOT / "benchmarks/serializer-comparison"


def prepare(output: Path, cli: Path) -> None:
    expected_version = tomllib.loads(
        (ROOT / "crates/polyxml-python/pyproject.toml").read_text()
    )["project"]["version"]
    version = subprocess.run(
        [str(cli), "--version"], check=True, capture_output=True, text=True
    ).stdout.strip()
    if version != f"polyxml {expected_version}":
        raise RuntimeError(
            f"Rebuild the local CLI: expected {expected_version}, got {version}"
        )
    output.mkdir(parents=True, exist_ok=True)
    fixtures = output / "fixtures"
    fixtures.mkdir(exist_ok=True)
    for count in [1, 1000]:
        shutil.copyfile(
            ROOT / f"benchmarks/workloads/sensor-batch/sensor-{count}.xml",
            fixtures / f"sensor_{count}.xml",
        )
    xml = (
        "<Batch>"
        + "".join(
            f"<Sensor><Id>{escape(f'sensor-{i} & <é> {chr(34)}quoted{chr(34)}')}</Id><Value>{i}</Value></Sensor>"
            for i in range(1000)
        )
        + "</Batch>"
    )
    (fixtures / "escaped_1000.xml").write_text(xml, encoding="utf-8")
    for mode, flags in [("owned", ["--zero-copy", "false"]), ("borrowed", [])]:
        directory = output / "generated" / mode
        subprocess.run(
            [
                str(cli),
                "generate",
                str(ROOT / "benchmarks/workloads/sensor-batch/batch.xsd"),
                "--lang",
                "rust",
                "--out",
                str(directory),
                *flags,
            ],
            check=True,
        )
    consumer = output / "rust-consumer"
    source = consumer / "src"
    source.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(SUITE / "rust.rs", source / "main.rs")
    for mode in ["owned", "borrowed"]:
        shutil.copyfile(output / f"generated/{mode}/batch.rs", source / f"{mode}.rs")
    shutil.copyfile(
        ROOT / "benchmarks/workloads/sensor-batch/batch.xsd", source / "schema.xsd"
    )
    core = json.dumps(str(ROOT / "crates/polyxml-core"))
    (consumer / "Cargo.toml").write_text(f"""[package]
name="polyxml-serializer-comparison"
version="0.0.0"
edition="2021"
[workspace]
[features]
allocations=[]
[dependencies]
polyxml={{path={core}}}
quick-xml={{version="=0.42.0",features=["serialize"]}}
serde={{version="1",features=["derive"]}}
serde_json="1"
serde-xml-rs="=0.8.2"
""")
    subprocess.run(
        [
            str(cli),
            "generate",
            str(ROOT / "benchmarks/workloads/sensor-batch/batch.xsd"),
            "--lang",
            "python",
            "--out",
            str(output / "python-models"),
        ],
        check=True,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=SUITE / "target")
    parser.add_argument("--cli", type=Path, default=ROOT / "target/debug/polyxml")
    args = parser.parse_args()
    prepare(args.output.resolve(), args.cli.resolve())
