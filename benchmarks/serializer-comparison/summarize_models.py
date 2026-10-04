#!/usr/bin/env python3
"""Retain process-level native-model statistics and full JMH errors/forks."""

from __future__ import annotations

import argparse
import json
import re
import statistics
from collections import defaultdict
from pathlib import Path


def summarize(directory):
    samples = defaultdict(list)
    for repeat in range(5):
        text = (directory / f"go-{repeat}.txt").read_text()
        matches = re.findall(
            r"BenchmarkXML/size=(\d+)/(generated|baseline)/(read|write)-\d+\s+\d+\s+([\d.]+) ns/op\s+[\d.]+ MB/s\s+(\d+) B/op\s+(\d+) allocs/op",
            text,
        )
        assert len(matches) == 8, (repeat, matches)
        for count, lane, op, ns, size, calls in matches:
            samples[("go", int(count), lane, op)].append(
                (float(ns) / 1000, int(size), int(calls))
            )
    control = json.loads((directory / "csharp-control.json").read_text())
    assert control["tiered_compilation"] is False and control["processes"] == 6
    for repeat in range(6):
        text = (directory / f"csharp-steady-{repeat}.txt").read_text()
        matches = re.findall(
            r"(generated|baseline),size=(\d+),(read|write),ns/op=([\d.]+),B/op=(\d+)",
            text,
        )
        assert len(matches) == 8, (repeat, matches)
        for lane, count, op, ns, size in matches:
            samples[("csharp", int(count), lane, op)].append(
                (float(ns) / 1000, int(size), None)
            )
    results = []
    for (runtime, count, lane, op), values in sorted(samples.items()):
        times = [v[0] for v in values]
        row = {
            "runtime": runtime,
            "count": count,
            "lane": lane,
            "op": op,
            "median_us": statistics.median(times),
            "process_us": times,
            "process_range_us": [min(times), max(times)],
            "allocated_bytes": statistics.median(v[1] for v in values),
        }
        if runtime == "go":
            row["allocation_calls"] = statistics.median(v[2] for v in values)
        results.append(row)
    jmh = json.loads((directory / "java-jmh.json").read_text())
    assert len(jmh) == 12
    seen = set()
    for item in jmh:
        lane_op = item["benchmark"].split(".")[-1]
        lane, op = re.fullmatch(r"(direct|jackson|jaxb)(Read|Write)", lane_op).groups()
        key = (item["params"]["workload"], lane, op)
        assert key not in seen
        seen.add(key)
        metric = item["primaryMetric"]
        assert metric["scoreUnit"] == "ops/s" and len(metric["rawData"]) == 2
        assert all(len(fork) == 5 for fork in metric["rawData"])
        results.append(
            {
                "runtime": "java",
                "workload": key[0],
                "lane": lane,
                "op": op.lower(),
                "batches_per_second": metric["score"],
                "jmh_score_error": metric["scoreError"],
                "jmh_score_confidence": metric["scoreConfidence"],
                "fork_means_batches_per_second": [
                    statistics.mean(fork) for fork in metric["rawData"]
                ],
                "allocated_bytes_per_message": item["secondaryMetrics"][
                    "gc.alloc.rate.norm"
                ]["score"]
                / 1000,
            }
        )
    (directory / "summary.json").write_text(json.dumps(results, indent=2) + "\n")
    return results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    summarize(args.directory)
