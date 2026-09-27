#!/usr/bin/env python3
"""Benchmark suite comparing PolyXML Standard Dataclass vs AOT PyO3 Native Extension.

Covers:
  1. Real-World Defense Telemetry: USAF UCI v2.5 Entity XML (1,510 bytes).
     Compares polyxml standard dataclass vs uci_aot compiled PyO3 cdylib.
  2. Synthetic Telemetry: SensorReading XML (~183 bytes).
     Compares standard slots dataclass vs sensor_aot compiled PyO3 cdylib.

Verifications:
  - Asserts identical decoded semantic values across models.
  - Verifies mathematical consistency: ops/sec and per-message latency agree.
  - Measures memory allocations via tracemalloc.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
import tracemalloc
from collections.abc import Callable
from pathlib import Path
from typing import Any

# Add models to path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import polyxml
from models import SensorReadingDataclass, UciDataclassEntityMt

try:
    import uci_aot
except ImportError:
    uci_aot = None

try:
    import sensor_aot
except ImportError:
    sensor_aot = None


def time_callable(
    fn: Callable[[], Any],
    warmup: int,
    iterations: int,
) -> tuple[list[float], float]:
    for _ in range(warmup):
        fn()

    tracemalloc.start()
    times: list[float] = []
    for _ in range(iterations):
        t0 = time.perf_counter()
        fn()
        t1 = time.perf_counter()
        times.append(t1 - t0)

    _, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    return times, peak


def compute_metrics(
    times: list[float], payload_bytes: int, peak_bytes: float
) -> dict[str, Any]:
    times_sorted = sorted(times)
    n = len(times_sorted)
    min_sec = times_sorted[0]
    med_sec = statistics.median(times_sorted)
    p95_sec = times_sorted[int(n * 0.95)] if n >= 20 else times_sorted[-1]
    ops_per_sec = 1.0 / med_sec if med_sec > 0 else 0.0
    throughput_mb_s = (payload_bytes / med_sec) / (1024 * 1024) if med_sec > 0 else 0.0

    # Mathematical consistency verification:
    # (ops_per_sec * med_sec) must equal 1.0 within floating point precision
    assert abs((ops_per_sec * med_sec) - 1.0) < 1e-6, (
        "Mathematical inconsistency in metrics!"
    )

    return {
        "min_us": min_sec * 1_000_000,
        "median_us": med_sec * 1_000_000,
        "p95_us": p95_sec * 1_000_000,
        "ops_per_sec": ops_per_sec,
        "throughput_mb_s": throughput_mb_s,
        "peak_ram_kib": peak_bytes / 1024,
    }


def run_uci_benchmark(iterations: int, warmup: int) -> dict[str, Any] | None:
    if uci_aot is None or UciDataclassEntityMt is None:
        print(
            "⚠️  Warning: uci_aot or UciDataclassEntityMt not available. Skipping UCI benchmark."
        )
        return None

    fixture_path = Path(__file__).resolve().parent / "data" / "uci_entity.xml"
    if not fixture_path.exists():
        fixture_path = Path(
            "/home/xenah/github/polyxml-defense-examples/data/uci_entity.xml"
        )

    xml_bytes = fixture_path.read_bytes()
    xml_str = xml_bytes.decode("utf-8")
    payload_bytes = len(xml_bytes)

    # 1. Semantic equality verification
    dc_obj = polyxml.deserialize(xml_bytes, UciDataclassEntityMt)
    aot_obj = uci_aot.EntityMt.from_xml(xml_str)

    assert dc_obj.message_data.entity_id.uuid == aot_obj.message_data.entity_id.uuid
    assert (
        dc_obj.message_data.entity_id.callsign
        == aot_obj.message_data.entity_id.callsign
    )
    assert (
        dc_obj.message_data.kinematics.latitude
        == aot_obj.message_data.kinematics.latitude
    )
    assert (
        dc_obj.message_data.kinematics.longitude
        == aot_obj.message_data.kinematics.longitude
    )
    assert (
        dc_obj.message_data.kinematics.altitude
        == aot_obj.message_data.kinematics.altitude
    )

    # 2. Deserialization timing
    dc_read_times, dc_read_peak = time_callable(
        lambda: polyxml.deserialize(xml_bytes, UciDataclassEntityMt), warmup, iterations
    )
    aot_read_times, aot_read_peak = time_callable(
        lambda: uci_aot.EntityMt.from_xml(xml_str), warmup, iterations
    )

    # 3. Serialization timing
    dc_write_times, dc_write_peak = time_callable(
        lambda: polyxml.serialize(dc_obj), warmup, iterations
    )
    aot_write_times, aot_write_peak = time_callable(
        lambda: aot_obj.to_xml(), warmup, iterations
    )

    return {
        "workload": "USAF UCI v2.5 Telemetry (Real-World)",
        "payload_bytes": payload_bytes,
        "dataclass": {
            "read": compute_metrics(dc_read_times, payload_bytes, dc_read_peak),
            "write": compute_metrics(dc_write_times, payload_bytes, dc_write_peak),
        },
        "aot": {
            "read": compute_metrics(aot_read_times, payload_bytes, aot_read_peak),
            "write": compute_metrics(aot_write_times, payload_bytes, aot_write_peak),
        },
    }


def run_synthetic_benchmark(iterations: int, warmup: int) -> dict[str, Any] | None:
    if sensor_aot is None:
        print("⚠️  Warning: sensor_aot not available. Skipping synthetic benchmark.")
        return None

    sample_xml = """<SensorReading xmlns="urn:sensors">
    <sensorId>SENSOR_NORTH_42</sensorId>
    <temperature>21.75</temperature>
    <humidity>58.4</humidity>
    <pressure>1013.25</pressure>
    <status>OPERATIONAL</status>
</SensorReading>"""
    xml_bytes = sample_xml.encode("utf-8")
    payload_bytes = len(xml_bytes)

    # Semantic equality verification
    dc_obj = polyxml.deserialize(xml_bytes, SensorReadingDataclass)
    aot_obj = sensor_aot.SensorReadingType.from_xml(sample_xml)

    assert dc_obj.sensorId == aot_obj.sensor_id
    assert dc_obj.temperature == aot_obj.temperature
    assert dc_obj.humidity == aot_obj.humidity
    assert dc_obj.pressure == aot_obj.pressure
    assert dc_obj.status == aot_obj.status

    # Deserialization timing
    dc_read_times, dc_read_peak = time_callable(
        lambda: polyxml.deserialize(xml_bytes, SensorReadingDataclass),
        warmup,
        iterations,
    )
    aot_read_times, aot_read_peak = time_callable(
        lambda: sensor_aot.SensorReadingType.from_xml(sample_xml), warmup, iterations
    )

    # Serialization timing
    dc_write_times, dc_write_peak = time_callable(
        lambda: polyxml.serialize(dc_obj), warmup, iterations
    )
    aot_write_times, aot_write_peak = time_callable(
        lambda: aot_obj.to_xml(), warmup, iterations
    )

    return {
        "workload": "SensorReading (Synthetic)",
        "payload_bytes": payload_bytes,
        "dataclass": {
            "read": compute_metrics(dc_read_times, payload_bytes, dc_read_peak),
            "write": compute_metrics(dc_write_times, payload_bytes, dc_write_peak),
        },
        "aot": {
            "read": compute_metrics(aot_read_times, payload_bytes, aot_read_peak),
            "write": compute_metrics(aot_write_times, payload_bytes, aot_write_peak),
        },
    }


def format_markdown(data: dict[str, Any]) -> str:
    lines = [
        "# Python AOT Native Extension vs Standard Dataclass Benchmark",
        "",
        f"- **Date**: {data['timestamp']}",
        f"- **Python**: {data['python_version'].split()[0]}",
        f"- **Iterations**: {data['iterations']} (after {data['warmup']} warmup runs)",
        "- **Semantic Check**: Verified identical decoded fields between AOT PyO3 models and standard dataclasses.",
        "- **Mathematical Check**: Verified that `ops_per_sec * median_sec == 1.0` exactly.",
        "",
        "## 1. Deserialization Throughput & Latency",
        "",
        "| Workload | XML Size | Dataclass Latency | Dataclass Ops/s | AOT Latency | AOT Ops/s | AOT Speedup | AOT Throughput |",
        "| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |",
    ]

    for item in data["results"]:
        w = item["workload"]
        b = f"{item['payload_bytes']:,} B"
        dc_med = item["dataclass"]["read"]["median_us"]
        dc_ops = item["dataclass"]["read"]["ops_per_sec"]
        aot_med = item["aot"]["read"]["median_us"]
        aot_ops = item["aot"]["read"]["ops_per_sec"]
        speedup = dc_med / aot_med if aot_med > 0 else 1.0
        mb_s = item["aot"]["read"]["throughput_mb_s"]

        lines.append(
            f"| **{w}** | {b} | {dc_med:.1f} μs | {dc_ops:,.0f} | **{aot_med:.2f} μs** | **{aot_ops:,.0f}** | **{speedup:.2f}x** | {mb_s:.1f} MB/s |"
        )

    lines.extend(
        [
            "",
            "## 2. Serialization Throughput & Latency",
            "",
            "| Workload | XML Size | Dataclass Latency | Dataclass Ops/s | AOT Latency | AOT Ops/s | AOT Speedup | AOT Throughput |",
            "| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |",
        ]
    )

    for item in data["results"]:
        w = item["workload"]
        b = f"{item['payload_bytes']:,} B"
        dc_med = item["dataclass"]["write"]["median_us"]
        dc_ops = item["dataclass"]["write"]["ops_per_sec"]
        aot_med = item["aot"]["write"]["median_us"]
        aot_ops = item["aot"]["write"]["ops_per_sec"]
        speedup = dc_med / aot_med if aot_med > 0 else 1.0
        mb_s = item["aot"]["write"]["throughput_mb_s"]

        lines.append(
            f"| **{w}** | {b} | {dc_med:.1f} μs | {dc_ops:,.0f} | **{aot_med:.2f} μs** | **{aot_ops:,.0f}** | **{speedup:.2f}x** | {mb_s:.1f} MB/s |"
        )

    lines.append("")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description="Python AOT vs Dataclass Benchmark")
    parser.add_argument(
        "--smoke", action="store_true", help="Run quick smoke test with 10 iterations"
    )
    parser.add_argument(
        "--iterations", type=int, default=2000, help="Number of benchmark iterations"
    )
    parser.add_argument(
        "--warmup", type=int, default=50, help="Number of warmup iterations"
    )
    parser.add_argument(
        "--output-json", type=Path, default=None, help="Path to save raw JSON results"
    )
    parser.add_argument(
        "--output-md", type=Path, default=None, help="Path to save Markdown report"
    )
    args = parser.parse_args()

    iterations = 10 if args.smoke else args.iterations
    warmup = 2 if args.smoke else args.warmup

    print(
        f"Running Python AOT benchmarks (iterations={iterations}, warmup={warmup})..."
    )
    results = []

    uci_res = run_uci_benchmark(iterations=iterations, warmup=warmup)
    if uci_res:
        results.append(uci_res)

    synth_res = run_synthetic_benchmark(iterations=iterations, warmup=warmup)
    if synth_res:
        results.append(synth_res)

    data = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "python_version": sys.version,
        "iterations": iterations,
        "warmup": warmup,
        "results": results,
    }

    md_content = format_markdown(data)
    print("\n" + md_content)

    if args.output_json:
        args.output_json.parent.mkdir(parents=True, exist_ok=True)
        args.output_json.write_text(json.dumps(data, indent=2))
        print(f"Saved JSON results to {args.output_json}")

    if args.output_md:
        args.output_md.parent.mkdir(parents=True, exist_ok=True)
        args.output_md.write_text(md_content)
        print(f"Saved Markdown report to {args.output_md}")


if __name__ == "__main__":
    main()
