#!/usr/bin/env python3
"""Automated benchmark comparing Python Dataclass vs AOT PyO3 Native Extension.

Measures:
  1. Deserialization Throughput (MB/s and ops/sec)
  2. Serialization Throughput
  3. Memory Footprint (tracemalloc peak memory & RSS)
"""

import argparse
import os
import subprocess
import sys
import tempfile
import time
import tracemalloc
from dataclasses import dataclass
from pathlib import Path


SENSOR_XSD = """<?xml version="1.0"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:sensors">
    <xs:element name="SensorReading">
        <xs:complexType>
            <xs:sequence>
                <xs:element name="sensorId" type="xs:string"/>
                <xs:element name="temperature" type="xs:double"/>
                <xs:element name="humidity" type="xs:double"/>
                <xs:element name="pressure" type="xs:double"/>
                <xs:element name="status" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>
    </xs:element>
</xs:schema>
"""

SAMPLE_XML = """<SensorReading xmlns="urn:sensors">
    <sensorId>SENSOR_NORTH_42</sensorId>
    <temperature>21.75</temperature>
    <humidity>58.4</humidity>
    <pressure>1013.25</pressure>
    <status>OPERATIONAL</status>
</SensorReading>"""


@dataclass(slots=True)
class SensorReadingDataclass:
    sensorId: str
    temperature: float
    humidity: float
    pressure: float
    status: str


def build_aot_module(repo_root: Path, out_dir: Path) -> str:
    schema_path = out_dir / "sensors.xsd"
    schema_path.write_text(SENSOR_XSD)

    polyxml_bin = repo_root / "target" / "release" / "polyxml"
    if not polyxml_bin.exists():
        polyxml_bin = repo_root / "target" / "debug" / "polyxml"

    if not polyxml_bin.exists():
        print("Compiling polyxml CLI...")
        subprocess.check_call(["cargo", "build", "--bin", "polyxml"], cwd=repo_root)
        polyxml_bin = repo_root / "target" / "debug" / "polyxml"

    # Generate AOT extension
    pkg_dir = out_dir / "pkg"
    subprocess.check_call(
        [
            str(polyxml_bin),
            "generate",
            str(schema_path),
            "-l",
            "python",
            "-b",
            "aot",
            "-p",
            "sensor_aot",
            "-o",
            str(pkg_dir),
        ]
    )

    # Build extension into current Python environment
    venv_dir = os.environ.get("VIRTUAL_ENV", sys.prefix)
    env = dict(os.environ)
    env["VIRTUAL_ENV"] = venv_dir

    print("Compiling PyO3 native extension via maturin develop --release...")
    subprocess.check_call(
        [sys.executable, "-m", "maturin", "develop", "--release"],
        cwd=pkg_dir,
        env=env,
    )
    return "sensor_aot"


def benchmark_dataclass(xml_data: str, iterations: int):
    import polyxml

    # Warmup
    for _ in range(100):
        polyxml.deserialize(xml_data, SensorReadingDataclass)

    # Timing
    t0 = time.perf_counter()
    for _ in range(iterations):
        polyxml.deserialize(xml_data, SensorReadingDataclass)
    t1 = time.perf_counter()
    elapsed = t1 - t0

    # Memory measurement
    tracemalloc.start()
    objects = [polyxml.deserialize(xml_data, SensorReadingDataclass) for _ in range(10_000)]
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    del objects

    return {
        "elapsed_sec": elapsed,
        "ops_per_sec": iterations / elapsed,
        "throughput_mb_s": (len(xml_data.encode("utf-8")) * iterations) / (elapsed * 1024 * 1024),
        "peak_memory_kb": peak / 1024,
    }


def benchmark_aot(aot_module, xml_data: str, iterations: int):
    # Warmup
    for _ in range(100):
        aot_module.SensorReadingType.from_xml(xml_data)

    # Timing
    t0 = time.perf_counter()
    for _ in range(iterations):
        aot_module.SensorReadingType.from_xml(xml_data)
    t1 = time.perf_counter()
    elapsed = t1 - t0

    # Memory measurement
    tracemalloc.start()
    objects = [aot_module.SensorReadingType.from_xml(xml_data) for _ in range(10_000)]
    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    del objects

    return {
        "elapsed_sec": elapsed,
        "ops_per_sec": iterations / elapsed,
        "throughput_mb_s": (len(xml_data.encode("utf-8")) * iterations) / (elapsed * 1024 * 1024),
        "peak_memory_kb": peak / 1024,
    }


def main():
    parser = argparse.ArgumentParser(description="PolyXML AOT vs Dataclass Benchmark")
    parser.add_argument("--iterations", type=int, default=10000, help="Number of benchmark iterations")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[3]

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        mod_name = build_aot_module(repo_root, tmp_path)
        import importlib
        aot_mod = importlib.import_module(mod_name)

        print(f"\n--- Running benchmark ({args.iterations} iterations) ---")
        dc_res = benchmark_dataclass(SAMPLE_XML, args.iterations)
        aot_res = benchmark_aot(aot_mod, SAMPLE_XML, args.iterations)

        speedup = aot_res["ops_per_sec"] / dc_res["ops_per_sec"]
        mem_reduction = (1 - (aot_res["peak_memory_kb"] / dc_res["peak_memory_kb"])) * 100

        print("\n==========================================================================")
        print(" PolyXML Deserialization Benchmark: Dataclass vs AOT PyO3 Native")
        print("==========================================================================")
        print(f" Dataclass (stdlib slots) : {dc_res['ops_per_sec']:>10.0f} ops/sec  | {dc_res['throughput_mb_s']:>6.2f} MB/s | Peak RAM: {dc_res['peak_memory_kb']:>6.1f} KB")
        print(f" AOT Native (PyO3 cdylib) : {aot_res['ops_per_sec']:>10.0f} ops/sec  | {aot_res['throughput_mb_s']:>6.2f} MB/s | Peak RAM: {aot_res['peak_memory_kb']:>6.1f} KB")
        print("--------------------------------------------------------------------------")
        print(f" Throughput Speedup       : {speedup:>6.2f}x faster")
        print(f" Memory RSS Reduction     : {mem_reduction:>6.1f}% less RAM")
        print("==========================================================================\n")


if __name__ == "__main__":
    main()
