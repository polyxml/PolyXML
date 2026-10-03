#!/usr/bin/env python3
"""Summarize per-process medians without treating samples as independent runs."""

import argparse
import csv
import json
import statistics
from collections import defaultdict
from pathlib import Path


def comparison(baseline, current):
    old = statistics.median(baseline)
    new = statistics.median(current)
    return {
        "baseline_ns": old,
        "current_ns": new,
        "delta_percent": (new / old - 1) * 100,
        "baseline_process_ns": baseline,
        "current_process_ns": current,
        "process_deltas_percent": [
            (b / a - 1) * 100 for a, b in zip(baseline, current, strict=True)
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("results", type=Path)
    args = parser.parse_args()
    records = defaultdict(lambda: defaultdict(list))
    for path in sorted((args.results / "generated").glob("*.csv")):
        label, mode, _ = path.stem.split("-")
        samples = defaultdict(list)
        with path.open() as source:
            for row in csv.DictReader(source):
                samples[(row["operation"], row["count"])].append(
                    float(row["ns_per_op"])
                )
        for (operation, count), values in samples.items():
            records[(mode, operation, count)][label].append(statistics.median(values))
    generated = {
        "/".join(key): comparison(values["baseline"], values["current"])
        for key, values in sorted(records.items())
    }
    records = defaultdict(lambda: defaultdict(list))
    intervals = defaultdict(lambda: defaultdict(list))
    for folder in sorted((args.results / "core").glob("*-*")):
        if not folder.is_dir():
            continue
        label = folder.name.split("-")[0]
        for path in sorted(folder.rglob("new/estimates.json")):
            key = str(path.parent.parent.relative_to(folder))
            estimate = json.loads(path.read_text())["mean"]
            records[key][label].append(estimate["point_estimate"])
            intervals[key][label].append(estimate["confidence_interval"])
    core = {
        key: comparison(values["baseline"], values["current"])
        | {"process_confidence_intervals": intervals[key]}
        for key, values in sorted(records.items())
    }
    (args.results / "summary.json").write_text(
        json.dumps({"generated": generated, "core": core}, indent=2) + "\n"
    )
    for suite, values in [("generated", generated), ("core", core)]:
        for key, row in values.items():
            print(
                f"{suite:9} {key:45} {row['baseline_ns'] / 1000:10.4f} -> {row['current_ns'] / 1000:10.4f} us ({row['delta_percent']:+.2f}%)"
            )


if __name__ == "__main__":
    main()
