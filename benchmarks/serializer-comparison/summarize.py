#!/usr/bin/env python3
"""Summarize retained CSV samples without treating samples as independent runs."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path


def summarize(directory):
    metadata = json.loads((directory / "metadata.json").read_text())
    samples = defaultdict(lambda: defaultdict(list))
    seen = set()
    for runtime in ("rust", "python"):
        for repeat in range(metadata["rounds"]):
            with (directory / f"{runtime}-{repeat}.csv").open() as stream:
                for row in csv.DictReader(stream):
                    assert row["kind"] == "sample"
                    key = (runtime, row["case"], row["op"], row["lane"])
                    identity = (*key, repeat, int(row["sample"]))
                    assert identity not in seen, identity
                    seen.add(identity)
                    ns = float(row["ns_per_op"])
                    assert ns > 0 and int(row["iterations"]) > 0
                    samples[key][repeat].append(ns / 1000)
    allocations = {}
    with (directory / "rust-allocations.csv").open() as stream:
        for row in csv.DictReader(stream):
            key = ("rust", row["case"], row["op"], row["lane"])
            assert key not in allocations
            allocations[key] = {
                "allocation_calls": int(row["allocations"]),
                "requested_bytes": int(row["requested_bytes"]),
            }
    results = []
    for key, repeats in sorted(samples.items()):
        assert len(repeats) == metadata["rounds"]
        assert all(len(values) == metadata["samples"] for values in repeats.values())
        medians = [statistics.median(repeats[i]) for i in range(metadata["rounds"])]
        results.append(
            dict(zip(("runtime", "case", "op", "lane"), key, strict=True))
            | {
                "median_us": statistics.median(medians),
                "process_medians_us": medians,
                "process_range_us": [min(medians), max(medians)],
            }
            | allocations.get(key, {})
        )
    (directory / "summary.json").write_text(json.dumps(results, indent=2) + "\n")
    lines = [
        "# All retained serializer comparisons",
        "",
        "Microseconds per operation; median of process medians. Range is the",
        "minimum–maximum process median, not a confidence interval. Requested",
        "bytes and allocation calls come from a separate Rust diagnostic build.",
        "",
        "| Runtime | Case | Operation | Lane | Median µs | Process range µs | Alloc calls | Requested B |",
        "| :--- | :--- | :--- | :--- | ---: | ---: | ---: | ---: |",
    ]
    for row in results:
        low, high = row["process_range_us"]
        lines.append(
            f"| {row['runtime']} | {row['case']} | {row['op']} | {row['lane']} | {row['median_us']:.3f} | {low:.3f}–{high:.3f} | {row.get('allocation_calls', '—')} | {row.get('requested_bytes', '—')} |"
        )
    (directory / "summary.md").write_text("\n".join(lines) + "\n")
    return results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    summarize(args.directory)
