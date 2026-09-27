#!/usr/bin/env python3
"""Self-contained benchmark suite comparing PolyXML JSON vs xsdata JSON.

Benchmarks:
  1. Reading (deserialization) from JSON into typed dataclasses.
  2. Writing (serialization) from typed dataclasses to JSON.

Workloads:
  - Small: Single SensorReading (~170 B JSON)
  - Nested: Order with 10 line items and optional fields (~1.2 KB JSON)
  - Moderate: Batch of 100 SensorReadings (~17.5 KB JSON)

Checks:
  - Verifies exact field equality: assert parsed == expected.
  - Checks round-trip fidelity.
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
from models import (
    Order,
    SensorBatch,
    SensorReading,
    make_sample_batch,
    make_sample_order,
    make_sample_sensor,
)
from xsdata.formats.dataclass.parsers.json import JsonParser as XsJsonParser
from xsdata.formats.dataclass.serializers.json import JsonSerializer as XsJsonSerializer


def time_callable(
    fn: Callable[[], Any],
    warmup: int,
    iterations: int,
) -> tuple[list[float], float]:
    """Execute warmup and timed iterations, returning per-iteration seconds and peak memory."""
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

    return {
        "min_us": min_sec * 1_000_000,
        "median_us": med_sec * 1_000_000,
        "p95_us": p95_sec * 1_000_000,
        "ops_per_sec": ops_per_sec,
        "throughput_mb_s": throughput_mb_s,
        "peak_ram_kib": peak_bytes / 1024,
    }


def run_benchmark_suite(iterations: int, warmup: int) -> dict[str, Any]:
    # Workload definitions
    workloads = [
        ("Sensor (Small)", make_sample_sensor(), SensorReading),
        ("Order (Nested)", make_sample_order(10), Order),
        ("Batch (100 Items)", make_sample_batch(100), SensorBatch),
    ]

    results: list[dict[str, Any]] = []

    # Parsers and serializers
    poly_ser = polyxml.JsonSerializer()
    poly_par = polyxml.JsonParser()
    xs_ser = XsJsonSerializer()
    xs_par = XsJsonParser()

    for name, obj, model_cls in workloads:
        # Prepare JSON bytes
        canonical_json = json.dumps(
            json.loads(poly_ser.render(obj)), separators=(",", ":")
        ).encode("utf-8")
        payload_bytes = len(canonical_json)

        # ---------------- Correctness Validation ----------------
        poly_obj = poly_par.from_bytes(canonical_json, model_cls)
        xs_obj = xs_par.from_bytes(canonical_json, model_cls)
        poly_native_obj = polyxml.loads_json(canonical_json, model_cls)

        assert poly_obj == obj, f"PolyXML JsonParser output mismatch on {name}"
        assert xs_obj == obj, f"xsdata JsonParser output mismatch on {name}"
        assert poly_native_obj == obj, f"polyxml.loads_json output mismatch on {name}"

        # Check serialization output roundtrips
        poly_rendered = poly_ser.render(obj)
        xs_rendered = xs_ser.render(obj)
        poly_native_rendered = polyxml.dumps_json(obj)

        assert poly_par.from_string(poly_rendered, model_cls) == obj
        assert xs_par.from_string(xs_rendered, model_cls) == obj
        assert polyxml.loads_json(poly_native_rendered, model_cls) == obj

        # ---------------- Deserialization Timing ----------------
        # 1. PolyXML native loads_json
        times_p_native, peak_p_native = time_callable(
            lambda c_json=canonical_json, m_cls=model_cls: polyxml.loads_json(
                c_json, m_cls
            ),
            warmup,
            iterations,
        )
        # 2. PolyXML JsonParser
        times_p_cls, peak_p_cls = time_callable(
            lambda c_json=canonical_json, m_cls=model_cls: poly_par.from_bytes(
                c_json, m_cls
            ),
            warmup,
            iterations,
        )
        # 3. xsdata JsonParser
        times_xs_read, peak_xs_read = time_callable(
            lambda c_json=canonical_json, m_cls=model_cls: xs_par.from_bytes(
                c_json, m_cls
            ),
            warmup,
            iterations,
        )

        # ---------------- Serialization Timing ----------------
        # 1. PolyXML native dumps_json
        times_p_native_w, peak_p_native_w = time_callable(
            lambda o=obj: polyxml.dumps_json(o), warmup, iterations
        )
        # 2. PolyXML JsonSerializer
        times_p_cls_w, peak_p_cls_w = time_callable(
            lambda o=obj: poly_ser.render(o), warmup, iterations
        )
        # 3. xsdata JsonSerializer
        times_xs_write, peak_xs_write = time_callable(
            lambda o=obj: xs_ser.render(o), warmup, iterations
        )

        results.append(
            {
                "workload": name,
                "payload_bytes": payload_bytes,
                "read": {
                    "polyxml_native": compute_metrics(
                        times_p_native, payload_bytes, peak_p_native
                    ),
                    "polyxml_compat": compute_metrics(
                        times_p_cls, payload_bytes, peak_p_cls
                    ),
                    "xsdata": compute_metrics(
                        times_xs_read, payload_bytes, peak_xs_read
                    ),
                },
                "write": {
                    "polyxml_native": compute_metrics(
                        times_p_native_w, payload_bytes, peak_p_native_w
                    ),
                    "polyxml_compat": compute_metrics(
                        times_p_cls_w, payload_bytes, peak_p_cls_w
                    ),
                    "xsdata": compute_metrics(
                        times_xs_write, payload_bytes, peak_xs_write
                    ),
                },
            }
        )

    return {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "python_version": sys.version,
        "iterations": iterations,
        "warmup": warmup,
        "results": results,
    }


def format_markdown(data: dict[str, Any]) -> str:
    lines = [
        "# Python JSON Benchmarks: PolyXML vs xsdata",
        "",
        f"- **Date**: {data['timestamp']}",
        f"- **Python**: {data['python_version'].split()[0]}",
        f"- **Iterations**: {data['iterations']} (after {data['warmup']} warmup runs)",
        "- **Semantic Check**: Verified `assert parsed == expected` for every runner and model.",
        "",
        "## 1. JSON Deserialization (Read into Typed Dataclass)",
        "",
        "| Workload | Payload | xsdata Median | PolyXML Native Median | Speedup vs xsdata | PolyXML Throughput |",
        "| :--- | :--- | :---: | :---: | :---: | :---: |",
    ]

    for item in data["results"]:
        w = item["workload"]
        b = f"{item['payload_bytes']:,} B"
        xs_med = item["read"]["xsdata"]["median_us"]
        poly_med = item["read"]["polyxml_native"]["median_us"]
        speedup = xs_med / poly_med if poly_med > 0 else 1.0
        mb_s = item["read"]["polyxml_native"]["throughput_mb_s"]
        lines.append(
            f"| **{w}** | {b} | {xs_med:,.1f} μs | **{poly_med:,.1f} μs** | **{speedup:.1f}x faster** | {mb_s:,.1f} MB/s |"
        )

    lines.extend(
        [
            "",
            "## 2. JSON Serialization (Write from Typed Dataclass)",
            "",
            "| Workload | Payload | xsdata Median | PolyXML Native Median | Speedup vs xsdata | PolyXML Throughput |",
            "| :--- | :--- | :---: | :---: | :---: | :---: |",
        ]
    )

    for item in data["results"]:
        w = item["workload"]
        b = f"{item['payload_bytes']:,} B"
        xs_med = item["write"]["xsdata"]["median_us"]
        poly_med = item["write"]["polyxml_native"]["median_us"]
        speedup = xs_med / poly_med if poly_med > 0 else 1.0
        mb_s = item["write"]["polyxml_native"]["throughput_mb_s"]
        lines.append(
            f"| **{w}** | {b} | {xs_med:,.1f} μs | **{poly_med:,.1f} μs** | **{speedup:.1f}x faster** | {mb_s:,.1f} MB/s |"
        )

    lines.append("")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Python JSON Benchmark: PolyXML vs xsdata"
    )
    parser.add_argument(
        "--smoke", action="store_true", help="Run quick smoke test with 5 iterations"
    )
    parser.add_argument(
        "--iterations", type=int, default=50, help="Number of benchmark iterations"
    )
    parser.add_argument(
        "--warmup", type=int, default=10, help="Number of warmup iterations"
    )
    parser.add_argument(
        "--output-json", type=Path, default=None, help="Path to save raw JSON results"
    )
    parser.add_argument(
        "--output-md", type=Path, default=None, help="Path to save Markdown report"
    )
    args = parser.parse_args()

    iterations = 5 if args.smoke else args.iterations
    warmup = 2 if args.smoke else args.warmup

    print(
        f"Running Python JSON benchmarks (iterations={iterations}, warmup={warmup})..."
    )
    data = run_benchmark_suite(iterations=iterations, warmup=warmup)
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
