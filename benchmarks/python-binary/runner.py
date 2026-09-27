#!/usr/bin/env python3
"""Self-contained benchmark suite comparing Python binary serializers.

Serializers:
  1. PolyXML Binary (polyxml.dumps_binary / polyxml.loads_binary via MessagePack)
  2. Python pickle (protocol 5 stdlib C accelerator)
  3. cloudpickle (standard for distributed PySpark/Ray execution)

Lanes:
  - Direct (uncompressed)
  - Compressed (LZ4 frame compression)

Workloads:
  - Small: Single SensorReading
  - Nested: Order with 10 line items and optional fields
  - Moderate: Batch of 100 SensorReadings

Metrics:
  - Encoded byte size & compression ratio
  - Encode latency (median, min, p95) and throughput (ops/sec)
  - Decode latency (median, min, p95) and throughput (ops/sec)
  - Peak memory allocations (tracemalloc)
"""

from __future__ import annotations

import argparse
import json
import pickle
import statistics
import sys
import time
import tracemalloc
from collections.abc import Callable
from pathlib import Path
from typing import Any

# Add models to path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import cloudpickle
import lz4.frame
import polyxml
from models import (
    Order,
    SensorBatch,
    SensorReading,
    make_sample_batch,
    make_sample_order,
    make_sample_sensor,
)


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

    return {
        "min_us": min_sec * 1_000_000,
        "median_us": med_sec * 1_000_000,
        "p95_us": p95_sec * 1_000_000,
        "ops_per_sec": ops_per_sec,
        "throughput_mb_s": throughput_mb_s,
        "peak_ram_kib": peak_bytes / 1024,
    }


def run_benchmark_suite(iterations: int, warmup: int) -> dict[str, Any]:
    workloads = [
        ("Sensor (Small)", make_sample_sensor(), SensorReading),
        ("Order (Nested)", make_sample_order(10), Order),
        ("Batch (100 Items)", make_sample_batch(100), SensorBatch),
    ]

    results: list[dict[str, Any]] = []

    for name, obj, model_cls in workloads:
        workload_res: dict[str, Any] = {
            "workload": name,
            "serializers": {},
        }

        # ----------------- Uncompressed Payloads -----------------
        poly_bin = polyxml.dumps_binary(obj)
        pickle_bin = pickle.dumps(obj, protocol=5)
        cp_bin = cloudpickle.dumps(obj)

        # ----------------- Compressed Payloads (LZ4) -----------------
        poly_lz4 = lz4.frame.compress(poly_bin)
        pickle_lz4 = lz4.frame.compress(pickle_bin)
        cp_lz4 = lz4.frame.compress(cp_bin)

        # ----------------- Correctness Assertions -----------------
        assert polyxml.loads_binary(poly_bin, model_cls) == obj
        assert pickle.loads(pickle_bin) == obj
        assert cloudpickle.loads(cp_bin) == obj

        assert polyxml.loads_binary(lz4.frame.decompress(poly_lz4), model_cls) == obj
        assert pickle.loads(lz4.frame.decompress(pickle_lz4)) == obj
        assert cloudpickle.loads(lz4.frame.decompress(cp_lz4)) == obj

        serializers_data = {
            "PolyXML Binary": {
                "direct": {
                    "bytes": len(poly_bin),
                    "encode_fn": lambda o=obj: polyxml.dumps_binary(o),
                    "decode_fn": lambda b=poly_bin, cls=model_cls: polyxml.loads_binary(
                        b, cls
                    ),
                },
                "lz4": {
                    "bytes": len(poly_lz4),
                    "encode_fn": lambda o=obj: lz4.frame.compress(
                        polyxml.dumps_binary(o)
                    ),
                    "decode_fn": lambda b=poly_lz4, cls=model_cls: polyxml.loads_binary(
                        lz4.frame.decompress(b), cls
                    ),
                },
            },
            "Pickle 5 (Stdlib C)": {
                "direct": {
                    "bytes": len(pickle_bin),
                    "encode_fn": lambda o=obj: pickle.dumps(o, protocol=5),
                    "decode_fn": lambda b=pickle_bin: pickle.loads(b),
                },
                "lz4": {
                    "bytes": len(pickle_lz4),
                    "encode_fn": lambda o=obj: lz4.frame.compress(
                        pickle.dumps(o, protocol=5)
                    ),
                    "decode_fn": lambda b=pickle_lz4: pickle.loads(
                        lz4.frame.decompress(b)
                    ),
                },
            },
            "CloudPickle": {
                "direct": {
                    "bytes": len(cp_bin),
                    "encode_fn": lambda o=obj: cloudpickle.dumps(o),
                    "decode_fn": lambda b=cp_bin: cloudpickle.loads(b),
                },
                "lz4": {
                    "bytes": len(cp_lz4),
                    "encode_fn": lambda o=obj: lz4.frame.compress(cloudpickle.dumps(o)),
                    "decode_fn": lambda b=cp_lz4: cloudpickle.loads(
                        lz4.frame.decompress(b)
                    ),
                },
            },
        }

        for ser_name, modes in serializers_data.items():
            ser_res: dict[str, Any] = {}
            for mode_name, mode_cfg in modes.items():
                p_bytes = mode_cfg["bytes"]
                enc_times, enc_peak = time_callable(
                    mode_cfg["encode_fn"], warmup, iterations
                )
                dec_times, dec_peak = time_callable(
                    mode_cfg["decode_fn"], warmup, iterations
                )
                ser_res[mode_name] = {
                    "bytes": p_bytes,
                    "encode": compute_metrics(enc_times, p_bytes, enc_peak),
                    "decode": compute_metrics(dec_times, p_bytes, dec_peak),
                }
            workload_res["serializers"][ser_name] = ser_res

        results.append(workload_res)

    return {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "python_version": sys.version,
        "iterations": iterations,
        "warmup": warmup,
        "results": results,
    }


def format_markdown(data: dict[str, Any]) -> str:
    lines = [
        "# Python Binary Serialization Benchmarks",
        "",
        f"- **Date**: {data['timestamp']}",
        f"- **Python**: {data['python_version'].split()[0]}",
        f"- **Iterations**: {data['iterations']} (after {data['warmup']} warmup runs)",
        "- **Semantic Check**: Verified `assert loads(dumps(obj)) == obj` for all serializers.",
        "",
        "## 1. Uncompressed Serialization & Deserialization",
        "",
        "| Workload | Serializer | Payload Size | Encode Median | Decode Median | Encode Ops/s | Decode Ops/s |",
        "| :--- | :--- | :---: | :---: | :---: | :---: | :---: |",
    ]

    for item in data["results"]:
        w = item["workload"]
        for ser, modes in item["serializers"].items():
            direct = modes["direct"]
            b = f"{direct['bytes']:,} B"
            enc_us = f"{direct['encode']['median_us']:.1f} μs"
            dec_us = f"{direct['decode']['median_us']:.1f} μs"
            enc_ops = f"{direct['encode']['ops_per_sec']:,.0f}"
            dec_ops = f"{direct['decode']['ops_per_sec']:,.0f}"
            is_poly = "PolyXML" in ser
            ser_label = f"**{ser}**" if is_poly else ser
            lines.append(
                f"| {w} | {ser_label} | {b} | {enc_us} | {dec_us} | {enc_ops} | {dec_ops} |"
            )

    lines.extend(
        [
            "",
            "## 2. Compressed (LZ4) Serialization & Deserialization",
            "",
            "| Workload | Serializer | LZ4 Size | Encode+LZ4 Median | Decode+LZ4 Median | Encode+LZ4 Ops/s | Decode+LZ4 Ops/s |",
            "| :--- | :--- | :---: | :---: | :---: | :---: | :---: |",
        ]
    )

    for item in data["results"]:
        w = item["workload"]
        for ser, modes in item["serializers"].items():
            lz4_mode = modes["lz4"]
            b = f"{lz4_mode['bytes']:,} B"
            enc_us = f"{lz4_mode['encode']['median_us']:.1f} μs"
            dec_us = f"{lz4_mode['decode']['median_us']:.1f} μs"
            enc_ops = f"{lz4_mode['encode']['ops_per_sec']:,.0f}"
            dec_ops = f"{lz4_mode['decode']['ops_per_sec']:,.0f}"
            is_poly = "PolyXML" in ser
            ser_label = f"**{ser}**" if is_poly else ser
            lines.append(
                f"| {w} | {ser_label} | {b} | {enc_us} | {dec_us} | {enc_ops} | {dec_ops} |"
            )

    lines.append("")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Python Binary Serialization Benchmark"
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
        f"Running Python binary benchmarks (iterations={iterations}, warmup={warmup})..."
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
