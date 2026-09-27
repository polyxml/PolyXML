"""Decode the shared sensor XML with locally generated PolyXML Python models."""

from __future__ import annotations

import importlib
import sys
from pathlib import Path
from time import perf_counter_ns

import polyxml
import tomllib

here = Path(__file__).resolve().parent
repo = here.parents[2]
expected = tomllib.loads((repo / "crates/polyxml-python/pyproject.toml").read_text())[
    "project"
]["version"]
if polyxml.__version__ != expected:
    raise SystemExit(
        f"Installed polyxml {polyxml.__version__} differs from local source {expected}; "
        "run maturin develop --release in the intended Python environment"
    )
sys.path.insert(0, str(here / "target/python"))
BatchType = importlib.import_module("batch").BatchType


def main() -> None:
    for count, iterations in ((1, 10_000), (1000, 100)):
        xml = (here / f"sensor-{count}.xml").read_bytes()
        value = BatchType.from_xml(xml)
        assert len(value.sensor) == count
        assert value.sensor[-1].id == f"sensor-{count - 1}"
        assert value.sensor[-1].value == count - 1
        for _ in range(100):
            BatchType.from_xml(xml)
        for repeat in range(5):
            start = perf_counter_ns()
            for _ in range(iterations):
                value = BatchType.from_xml(xml)
            elapsed = perf_counter_ns() - start
            print(
                f"python,size={count},repeat={repeat},ns/op={elapsed / iterations:.1f},"
                f"xml_bytes={len(xml)},last_id={value.sensor[-1].id}"
            )


if __name__ == "__main__":
    main()
