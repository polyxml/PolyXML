#!/usr/bin/env python3
"""Summarize an explicit experiment's independent Criterion process rounds."""

import argparse
import csv
from collections import defaultdict
import json
import statistics
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("experiment", type=Path)
    parser.add_argument("--generated", action="store_true")
    args = parser.parse_args()
    if args.generated:
        summarize_generated(args.experiment)
        return
    metadata = json.loads((args.experiment / "metadata.json").read_text())
    records = {}
    for file in sorted((args.experiment / "baseline-0").rglob("new/estimates.json")):
        key = file.relative_to(args.experiment / "baseline-0")
        estimates = {
            label: [
                json.loads((args.experiment / f"{label}-{i}" / key).read_text())["mean"]
                for i in range(metadata["rounds"])
            ]
            for label in ["baseline", "current"]
        }
        values = {
            label: [row["point_estimate"] for row in rows]
            for label, rows in estimates.items()
        }
        old, new = (
            statistics.median(values[label]) for label in ["baseline", "current"]
        )
        name = str(key.parent.parent)
        records[name] = {
            "baseline_ns": old,
            "candidate_ns": new,
            "delta_percent": (new / old - 1) * 100,
            "speedup": old / new,
            "process_deltas_percent": [
                (b / a - 1) * 100
                for a, b in zip(values["baseline"], values["current"], strict=True)
            ],
            "process_estimates": estimates,
        }
        print(
            f"{name:45} {old / 1000:10.3f} -> {new / 1000:10.3f} us; {(new / old - 1) * 100:+.2f}%; {old / new:.2f}x"
        )
    (args.experiment / "summary.json").write_text(json.dumps(records, indent=2) + "\n")


def summarize_generated(experiment):
    records = defaultdict(lambda: defaultdict(list))
    for path in sorted(experiment.glob("*.csv")):
        label, mode, _ = path.stem.split("-")
        samples = defaultdict(list)
        with path.open() as source:
            for row in csv.DictReader(source):
                samples[(row["operation"], row["count"])].append(
                    float(row["ns_per_op"])
                )
        for (operation, count), values in samples.items():
            records[(mode, operation, count)][label].append(statistics.median(values))
    summary = {}
    for key, values in sorted(records.items()):
        old, new = (
            statistics.median(values[label]) for label in ["baseline", "current"]
        )
        name = "/".join(key)
        summary[name] = {
            "baseline_ns": old,
            "candidate_ns": new,
            "delta_percent": (new / old - 1) * 100,
            "speedup": old / new,
            "process_values_ns": values,
            "process_deltas_percent": [
                (b / a - 1) * 100
                for a, b in zip(values["baseline"], values["current"], strict=True)
            ],
            "statistic": "median across process sample medians",
        }
        print(
            f"{name:45} {old / 1000:10.3f} -> {new / 1000:10.3f} us; {(new / old - 1) * 100:+.2f}%"
        )
    (experiment / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")


if __name__ == "__main__":
    main()
