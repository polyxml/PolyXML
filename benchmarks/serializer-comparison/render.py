#!/usr/bin/env python3
"""Render the October comparison page directly from the retained summaries."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "docs/benchmarks/data/2026-10-04"


def render(output):
    values = json.loads((DATA / "serializer-comparison/summary.json").read_text())
    native = json.loads((DATA / "native-models/summary.json").read_text())
    lookup = {(v["runtime"], v["case"], v["op"], v["lane"]): v for v in values}
    models = {
        (v["runtime"], v.get("count", v.get("workload")), v["op"], v["lane"]): v
        for v in native
    }
    lines = []

    def text(value):
        lines.extend(value.strip("\n").splitlines())
        lines.append("")

    def table(headers, rows):
        lines.append("| " + " | ".join(headers) + " |")
        lines.append("| " + " | ".join([":---"] + ["---:"] * (len(headers) - 1)) + " |")
        for row in rows:
            lines.append("| " + " | ".join(row) + " |")
        lines.append("")

    def timing(runtime, case, fmt, lanes, baseline):
        rows = []
        for lane, label in lanes:
            read = lookup[runtime, case, f"{fmt}_read", lane]["median_us"]
            write = lookup[runtime, case, f"{fmt}_write", lane]["median_us"]
            base = baseline(lane) if callable(baseline) else baseline
            bread = lookup[runtime, case, f"{fmt}_read", base]["median_us"]
            bwrite = lookup[runtime, case, f"{fmt}_write", base]["median_us"]
            rows.append(
                [
                    label,
                    f"{read:.2f}",
                    f"{write:.2f}",
                    f"{bread / read:.2f}×",
                    f"{bwrite / write:.2f}×",
                ]
            )
        table(
            [
                "Serializer / model",
                "Read µs ↓",
                "Write µs ↓",
                "Read relative ↑",
                "Write relative ↑",
            ],
            rows,
        )

    rust_lanes = [
        ("polyxml_generated_owned", "**PolyXML generated · String**"),
        ("quick_xml_serde_owned", "quick-xml Serde · String"),
        ("serde_xml_rs_owned", "serde-xml-rs · String"),
        ("polyxml_generated_borrowed", "**PolyXML generated · Cow**"),
        ("quick_xml_serde_borrowed", "quick-xml Serde · Cow"),
    ]
    python_lanes = [
        ("polyxml_dataclass", "**PolyXML · generated dataclasses**"),
        ("xsdata_dataclass", "xsdata · same dataclasses"),
        ("stdlib_elementtree_typed_adapter", "stdlib ElementTree · typed adapter¹"),
        ("lxml_typed_adapter", "lxml · typed adapter¹"),
    ]
    rust_baseline = lambda lane: (
        "quick_xml_serde_borrowed"
        if lane.endswith("borrowed")
        else "quick_xml_serde_owned"
    )
    text("""---
title: Serializer comparisons (October 2026)
description: Matched XML and JSON serializer tables for PolyXML 0.34.8, with native baselines, competitor bindings, allocations, repeated samples and coverage limits.
---

# XML and JSON serializer comparisons

These tables compare **the same inputs and decoded values within each runtime**.
They show native options, competitor libraries and PolyXML's generated bindings.
They do not rank languages against one another. Read and write costs are separate;
**lower latency is better**, and a relative score above 1 means faster than the
stated baseline. All results are from **PolyXML 0.34.8, October 4, 2026**.

The main sensor tables use **1,000 records / 53,795 XML bytes**. Every `Id` and
integer `Value` is checked before timing. Setup and file I/O are excluded;
reads construct the complete model, and writes allocate fresh output. The
[retained samples and all-case summary](data/2026-10-04/serializer-comparison/summary.md)
include single-record and escaped-text cases plus process variation.

## What is actually being compared?

| Runtime | Native option in this comparison | PolyXML path | Competitor / baseline | Meaning |
| :--- | :--- | :--- | :--- | :--- |
| Rust | No stdlib XML binding used; `serde_json` for JSON | Generated XML codec; Serde-derived JSON models | quick-xml Serde, serde-xml-rs; handwritten Serde JSON | XML binding engines; JSON model overhead |
| Python | ElementTree / `json` plus handwritten typed mapping | Rust-backed binding into generated dataclasses | xsdata on the same dataclasses; lxml typed adapter; Pydantic XML reference | End-to-end typed binding; specialized adapters labeled separately |
| Java | StAX API; Woodstox provider on this classpath | Generated POJOs with direct StAX codec | Jackson XML on the same POJOs; XJC-generated JAXB | Typed XML binding, on a separate 80-field workload |
| Go | `encoding/xml` | Generated root model and native serializer | Flat handwritten struct and same native serializer | Generated model / root-method overhead |
| C# | `XmlSerializer` | Generated inherited classes and native serializer | Flat handwritten class and same native serializer | Generated model overhead under an explicit JIT configuration |
| C++ | Existing C ABI binding | Native value graph mapped into models | Equivalent competitor not measured here | Coverage gap; narrow string-search adapter excluded |
| TypeScript / Wasm | Existing JS object parsers | Wasm binding | Equivalent typed return graph not measured here | Coverage gap; existing parser studies remain separate |

## Rust XML: generated codecs versus Serde adapters

[Source, generated models and dependency lock](https://github.com/polyxml/PolyXML/tree/main/docs/benchmarks/data/2026-10-04/serializer-comparison/source),
[raw summary](data/2026-10-04/serializer-comparison/summary.json).
Both the generated codecs and quick-xml's Serde adapter use **quick-xml 0.42.0**;
this compares their binding code, not different tokenizers. Ratios use quick-xml
Serde **with the same string ownership** as the row.
""")
    timing("rust", "sensor_1000", "xml", rust_lanes, rust_baseline)
    text("""The generated owned codec takes about half the read time of quick-xml Serde,
and its writer takes roughly a quarter of the time on this fixture. Borrowed
XML avoids string copies for plain text; escaping can require owned strings.
Both Cow readers are checked to actually borrow on the plain fixture. See
[quick-xml's borrowing rules](https://docs.rs/quick-xml/0.42.0/quick_xml/de/index.html).

The dynamic Rust engine returns a `PolyValue` record/list graph rather than the
typed structs above. Its separate reference result is:""")
    row = ["PolyXML dynamic · PolyValue"]
    for op in ("xml_read", "xml_write"):
        row.append(
            f"{lookup['rust', 'sensor_1000', op, 'polyxml_dynamic']['median_us']:.2f}"
        )
    table(["Value representation", "Read µs ↓", "Write µs ↓"], [row])
    text("""### Rust allocation diagnostics

[Separate instrumented output](data/2026-10-04/serializer-comparison/rust-allocations.csv).
These are **total requested allocation bytes and allocation/reallocation calls**
per operation, not peak RAM or retained memory. Timings above come from a
separate binary without the allocation counter.""")
    rows = []
    for lane, label in rust_lanes + [
        ("polyxml_dynamic", "PolyXML dynamic · different value graph")
    ]:
        r = lookup["rust", "sensor_1000", "xml_read", lane]
        w = lookup["rust", "sensor_1000", "xml_write", lane]
        rows.append(
            [
                label,
                f"{r['requested_bytes']:,}",
                f"{w['requested_bytes']:,}",
                f"{r['allocation_calls']:,}",
                f"{w['allocation_calls']:,}",
            ]
        )
    table(
        [
            "Serializer / model",
            "Read requested B",
            "Write requested B",
            "Read calls",
            "Write calls",
        ],
        rows,
    )
    text("""## Python XML: complete typed results

[Same-run samples and source](data/2026-10-04/serializer-comparison/summary.md).
PolyXML and xsdata construct the **same generated slotted dataclasses**. The
stdlib and lxml adapters construct those same models and convert integers
inside timing; they are not DOM-only parser timings. Ratios use **xsdata**.""")
    timing("python", "sensor_1000", "xml", python_lanes, "xsdata_dataclass")
    text("""¹ These handwritten adapters are specialized to this two-field schema. They
are useful native baselines, but do not implement a general schema-driven binder.
ElementTree's adapter reads this batch faster than PolyXML; PolyXML writes it
faster. That distinction matters alongside the larger gains over xsdata.

Pydantic XML builds a different, validating model graph. Keep it as a reference
rather than treating it as the same dataclass task:""")
    r = lookup["python", "sensor_1000", "xml_read", "pydantic_xml"]["median_us"]
    w = lookup["python", "sensor_1000", "xml_write", "pydantic_xml"]["median_us"]
    table(
        ["Reference representation", "Read µs ↓", "Write µs ↓"],
        [["pydantic-xml · Pydantic v2 models", f"{r:.2f}", f"{w:.2f}"]],
    )
    text("""Python timings use a **release native extension, without tracemalloc**.
The old traced Python measurements are not mixed into this table. We do not
report Python-only traced bytes as total Rust/libxml2 allocation.

## JSON: separate from XML

[All JSON samples, source and allocation diagnostics](data/2026-10-04/serializer-comparison/summary.md).
The plain 1,000-sensor JSON input is identical within each runtime; all writers
are checked against the same JSON values.

### Rust JSON: the same serde_json engine

Generated Rust JSON models and handwritten models all use **serde_json 1.0.151**.
These rows measure model choices, not a separate PolyXML JSON engine. The
generated Cow JSON model does not carry field-level Serde borrowing annotations;
its strings deserialize as owned. The XML borrowing result above does not apply.""")
    rows = []
    for lane, label in [
        ("polyxml_generated_owned", "PolyXML-generated · String"),
        ("polyxml_generated_borrowed", "PolyXML-generated · Cow"),
        ("serde_json_handwritten", "Handwritten Serde · String"),
    ]:
        rows.append(
            [
                label,
                f"{lookup['rust', 'sensor_1000', 'json_read', lane]['median_us']:.2f}",
                f"{lookup['rust', 'sensor_1000', 'json_write', lane]['median_us']:.2f}",
            ]
        )
    table(["Model / serde_json", "Read µs ↓", "Write µs ↓"], rows)
    text("""### Python JSON: typed dataclasses

Ratios use **xsdata JSON**. The stdlib adapter includes JSON parsing and complete
model construction, or model-to-dict conversion and UTF-8 output, inside timing.""")
    timing(
        "python",
        "sensor_1000",
        "json",
        [
            ("polyxml_dataclass", "**PolyXML · generated dataclasses**"),
            ("xsdata_dataclass", "xsdata · same dataclasses"),
            ("stdlib_json_typed_adapter", "stdlib json · specialized typed adapter"),
        ],
        "xsdata_dataclass",
    )
    text("""PolyXML is faster than xsdata here, while the specialized stdlib adapter is
faster than both. This fixture does not justify claiming PolyXML is the fastest
Python JSON serializer.

## Go and C#: generated models using native XML serializers

[All native-model samples, sources and runtime settings](https://github.com/polyxml/PolyXML/tree/main/docs/benchmarks/data/2026-10-04/native-models),
[computed summary](data/2026-10-04/native-models/summary.json).
These are **model comparisons using the same serializer in each language**.
Readers use the shared sensor fixture; writers produce equivalent sensor values.
Go writes UTF-8 bytes, while C# writes a materialized UTF-16 string. Compare
within each runtime, not between these rows. Allocations are Go's `benchmem`
bytes/op or .NET's current-thread managed bytes/op.""")
    rows = []
    for runtime, label in [
        ("go", "Go · encoding/xml"),
        ("csharp", "C# · XmlSerializer"),
    ]:
        for lane, model in [("generated", "generated"), ("baseline", "handwritten")]:
            r = models[runtime, 1000, "read", lane]
            w = models[runtime, 1000, "write", lane]
            rows.append(
                [
                    f"{label} · {model}",
                    f"{r['median_us']:.2f}",
                    f"{w['median_us']:.2f}",
                    f"{r['allocated_bytes']:,.0f}",
                    f"{w['allocated_bytes']:,.0f}",
                ]
            )
    table(
        [
            "Runtime / model",
            "Read µs ↓",
            "Write µs ↓",
            "Read allocated B",
            "Write allocated B",
        ],
        rows,
    )
    text("""Go's generated root type has custom `MarshalXML`/`UnmarshalXML` methods for
root naming and namespace checks. The flat handwritten baseline lacks those
methods. The custom root writer adds allocations and timing overhead in this
comparison.

C# uses **.NET 10.0.12**, with the `net8.0` build rolled forward. The published
control sets **`DOTNET_TieredCompilation=0`**, excludes serializer construction,
and warms each model for at least two seconds / 500 read-write pairs. This is
an explicit warmed JIT configuration, not default startup/tiered-PGO behavior.
The [default-tiering diagnostics and control settings](https://github.com/polyxml/PolyXML/tree/main/docs/benchmarks/data/2026-10-04/native-models)
are retained: their first-model effect persisted despite longer warmup, so they
are not used to rank models. See the
[.NET compilation settings](https://learn.microsoft.com/en-us/dotnet/core/runtime-config/compilation).

## Java XML: direct StAX, Jackson and JAXB

[Raw JMH JSON, including every fork, score error and GC metric](data/2026-10-04/native-models/java-jmh.json),
[fixture/output lengths and execution log](data/2026-10-04/native-models/java-jmh.txt).
Java uses its own **80-field synthetic projections**, with seven populated
string fields and 73 absent optional fields. Each operation processes **1,000
independent messages**. These are not official ISO/UCI documents and are not
the sensor workload. Read IDs vary; writes serialize the preconstructed ID-0
model to fresh bytes 1,000 times. Setup checks every input across typed readers.

The [recorded StAX providers](data/2026-10-04/native-models/java-stax-providers.txt)
are Woodstox 7.1.1 on this shaded classpath. The direct codec uses the JDK StAX
API with that provider; this is not a claim about the JDK's default XML parser.

Numbers are **batches/s**, with JMH's reported score error; higher is better.
Direct StAX and Jackson populate the same generated POJOs. JAXB populates the
equivalent XJC models. Panama's different native return graph is excluded.""")
    rows = []
    for workload in ("settlement", "telemetry"):
        for lane, label in [
            ("direct", "PolyXML direct StAX"),
            ("jackson", "Jackson XML"),
            ("jaxb", "JAXB / XJC"),
        ]:
            r = models["java", workload, "read", lane]
            w = models["java", workload, "write", lane]
            rows.append(
                [
                    f"{workload.title()} · {label}",
                    f"{r['batches_per_second']:.1f} ± {r['jmh_score_error']:.1f}",
                    f"{w['batches_per_second']:.1f} ± {w['jmh_score_error']:.1f}",
                    f"{r['allocated_bytes_per_message']:,.0f}",
                    f"{w['allocated_bytes_per_message']:,.0f}",
                ]
            )
    table(
        [
            "Workload / typed binding",
            "Read batches/s ↑",
            "Write batches/s ↑",
            "Read heap B/message",
            "Write heap B/message",
        ],
        rows,
    )
    text("""Output lengths differ because serializers emit different declarations and
namespace formatting; the log records them. Overlapping JMH errors do not
establish a winner. GC metrics cover the Java heap.

## Small inputs and escaped text

The [complete summary](data/2026-10-04/serializer-comparison/summary.md) retains
all lanes. The compact checks below show that the relative result changes with
input size and escaping. Same timing method and ownership-matched baselines
as the main Rust table.""")
    for case, title in [
        ("sensor_1", "Rust · one sensor (65 B XML)"),
        ("escaped_1000", "Rust · 1,000 sensors with entities, quotes and UTF-8"),
    ]:
        text(f"### {title}")
        timing("rust", case, "xml", rust_lanes, rust_baseline)
    text("""For Python, the [all-case summary](data/2026-10-04/serializer-comparison/summary.md)
includes the same single-record and escaped-text fixtures. Native call overhead,
model construction and escaping matter differently at each size; the main
1,000-record ratio should not be applied to every XML document.

## Environment, variation and reproduction

[Exact metadata, hashes and dependency versions](data/2026-10-04/serializer-comparison/metadata.json),
[native-model toolchains and settings](data/2026-10-04/native-models/metadata.json),
[reproducible suite](https://github.com/polyxml/PolyXML/tree/main/benchmarks/serializer-comparison).
The engine/compiler source is release **0.34.8 / `89786c3`**. Harness revisions
and generated-source hashes are recorded separately in the artifacts.

| Item | Configuration |
| :--- | :--- |
| Host | Intel Core i5-11600K, 6 cores / 12 logical CPUs; WSL2, Ubuntu 26.04.1; 15.5 GiB visible RAM |
| Rust | rustc/cargo 1.99.0; release; quick-xml 0.42.0, serde-xml-rs 0.8.2, Serde 1.0.229, serde_json 1.0.151 |
| Python | CPython 3.12.15; release ABI3 extension; xsdata 26.2, lxml 6.1.3, pydantic-xml 2.21.1, Pydantic 2.13.5 |
| Go / .NET | Go 1.26.0; .NET SDK 10.0.112 / runtime 10.0.12; C# control disables tiered compilation |
| Java | JDK 25.0.4.1; JMH 1.37; Jackson XML 2.21.2; JAXB 4.0.5; two forks, three 2-second warmups, five 2-second measurements; 256 MiB initial / 1 GiB max heap |
| Scheduling | Builds and timings serial; model/case order rotated; memory-capped processes; IDE background services present; no CPU affinity/frequency control |

Rust/Python use **six independent processes**, each with five samples of at
least 150 ms per operation after warmup. Tables report the **median of process
medians**. [All process ranges](data/2026-10-04/serializer-comparison/summary.md)
are descriptive minimum–maximum ranges, not confidence intervals. Samples
within a process are correlated. Go uses five fresh processes / 2-second bench
windows; C# control uses six fresh processes / 1,000 calls per operation,
with first-model order balanced. Their
[process ranges](data/2026-10-04/native-models/summary.json) are also retained.
Java reports JMH throughput and its own score error rather than these medians.

Every reader field and writer value is checked outside timing. Python XML
outputs pass XSD validation through lxml; all 33 exported Rust/Python XML
outputs are checked again using independent stdlib ElementTree parsing.
JSON outputs are compared as complete values. Writer byte lengths are retained
in the raw CSVs. This is a performance comparison of these valid fixtures,
not a conformance score or a full-schema-validation timing.

[September results](language-results-2026-09.md) remain historical; they use a
different host/version and are not blended with these figures. C++ and
TypeScript need equivalent return graphs and repeated matched measurements
before joining this typed comparison. Existing
[C++ native results](language-results-2026-09.md#c-native-c-abi-binding-and-model-adapter)
and [Wasm parser studies](wasm-vs-js.md) retain their own scope.
""")
    output.write_text("\n".join(lines))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "docs/benchmarks/serializer-comparison-2026-10.md",
    )
    args = parser.parse_args()
    render(args.output)
