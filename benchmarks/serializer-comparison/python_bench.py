#!/usr/bin/env python3
"""Matched typed consumers; no tracemalloc or allocation counter in timings."""

from __future__ import annotations

import csv
import importlib
import json
import os
import sys
import time
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path

import polyxml
from lxml import etree
from pydantic_xml import BaseXmlModel, element
from xsdata.formats.dataclass.parsers import XmlParser
from xsdata.formats.dataclass.parsers.json import JsonParser
from xsdata.formats.dataclass.serializers import XmlSerializer
from xsdata.formats.dataclass.serializers.config import SerializerConfig
from xsdata.formats.dataclass.serializers.json import JsonSerializer

TARGET = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TARGET / "python-models"))
models = importlib.import_module("batch")


@dataclass(slots=True, kw_only=True)
class Batch(models.BatchType):
    """Give the generated type its declared XSD element name for every lane."""

    class Meta:
        name = "Batch"
        strict_root = True


class PSensor(BaseXmlModel, tag="Sensor"):
    id: str = element(tag="Id")
    value: int = element(tag="Value")


class PBatch(BaseXmlModel, tag="Batch"):
    sensor: list[PSensor] = element(tag="Sensor")


def expected(count, escaped):
    return [
        (f'sensor-{i} & <é> "quoted"' if escaped else f"sensor-{i}", i)
        for i in range(count)
    ]


def check(value, pairs):
    assert [(sensor.id, sensor.value) for sensor in value.sensor] == pairs


def tree_read(xml, api):
    tree = api.fromstring(xml)
    return Batch(
        sensor=[
            models.SensorType(id=node.findtext("Id"), value=int(node.findtext("Value")))
            for node in tree.findall("Sensor")
        ]
    )


def tree_write(value, api):
    root = api.Element("Batch")
    for sensor in value.sensor:
        node = api.SubElement(root, "Sensor")
        api.SubElement(node, "Id").text = sensor.id
        api.SubElement(node, "Value").text = str(sensor.value)
    return api.tostring(root, encoding="utf-8")


def standard_json_read(raw):
    value = json.loads(raw)
    return Batch(
        sensor=[
            models.SensorType(id=s["Id"], value=s["Value"]) for s in value["Sensor"]
        ]
    )


def standard_json_write(value):
    return json.dumps(
        {"Sensor": [{"Id": s.id, "Value": s.value} for s in value.sensor]},
        ensure_ascii=False,
        separators=(",", ":"),
    ).encode("utf-8")


def measure(writer, lane, case, op, input_size, output_size, fn):
    for _ in range(64):
        fn()
    start = time.perf_counter_ns()
    for _ in range(10):
        fn()
    cost = (time.perf_counter_ns() - start) / 10
    chunk = max(1, min(100, int(10_000_000 / max(1, cost))))
    samples = int(os.environ.get("BENCH_SAMPLES", "9"))
    duration = int(os.environ.get("BENCH_MILLIS", "250")) * 1_000_000
    for sample in range(samples):
        iterations = 0
        start = time.perf_counter_ns()
        while time.perf_counter_ns() - start < duration:
            for _ in range(chunk):
                fn()  # result destruction included; normal cyclic GC stays enabled
            iterations += chunk
        elapsed = time.perf_counter_ns() - start
        writer.writerow(
            [
                "sample",
                lane,
                case,
                op,
                sample,
                iterations,
                f"{elapsed / iterations:.3f}",
                "",
                "",
                input_size,
                output_size,
            ]
        )


def main():
    assert polyxml.__version__ == "0.34.8", polyxml.__version__
    xp = XmlParser()
    xs = XmlSerializer(config=SerializerConfig(xml_declaration=False))
    jp = JsonParser()
    js = JsonSerializer(config=SerializerConfig(indent=None))
    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(
        [
            "kind",
            "lane",
            "case",
            "op",
            "sample",
            "iterations",
            "ns_per_op",
            "allocations",
            "requested_bytes",
            "input_bytes",
            "output_bytes",
        ]
    )
    order = int(os.environ.get("BENCH_ORDER", "0"))
    cases = [
        ("sensor_1", 1, False),
        ("sensor_1000", 1000, False),
        ("escaped_1000", 1000, True),
    ]
    cases = cases[order % 3 :] + cases[: order % 3]
    for case, count, escaped in cases:
        xml = (TARGET / f"fixtures/{case}.xml").read_bytes()
        pairs = expected(count, escaped)
        value = Batch(sensor=[models.SensorType(id=s, value=i) for s, i in pairs])
        pvalue = PBatch(sensor=[PSensor(id=s, value=i) for s, i in pairs])
        raw_json = standard_json_write(value)
        lanes = [
            (
                "polyxml_dataclass",
                "xml",
                lambda xml=xml: polyxml.deserialize(xml, Batch),
                lambda value=value: polyxml.serialize(value),
            ),
            (
                "xsdata_dataclass",
                "xml",
                lambda xml=xml: xp.from_bytes(xml, Batch),
                lambda value=value: xs.render(value).encode("utf-8"),
            ),
            (
                "stdlib_elementtree_typed_adapter",
                "xml",
                lambda xml=xml: tree_read(xml, ET),
                lambda value=value: tree_write(value, ET),
            ),
            (
                "lxml_typed_adapter",
                "xml",
                lambda xml=xml: tree_read(xml, etree),
                lambda value=value: tree_write(value, etree),
            ),
            (
                "pydantic_xml",
                "xml",
                lambda xml=xml: PBatch.from_xml(xml),
                lambda pvalue=pvalue: pvalue.to_xml(encoding="utf-8"),
            ),
            (
                "polyxml_dataclass",
                "json",
                lambda raw_json=raw_json: polyxml.deserialize_json(raw_json, Batch),
                lambda value=value: polyxml.serialize_json(value),
            ),
            (
                "xsdata_dataclass",
                "json",
                lambda raw_json=raw_json: jp.from_bytes(raw_json, Batch),
                lambda value=value: js.render(value).encode("utf-8"),
            ),
            (
                "stdlib_json_typed_adapter",
                "json",
                lambda raw_json=raw_json: standard_json_read(raw_json),
                lambda value=value: standard_json_write(value),
            ),
        ]
        # Fully validate every lane before any lane is timed.
        sizes = {}
        schema = etree.XMLSchema(
            etree.parse(str(TARGET / "rust-consumer/src/schema.xsd"))
        )
        for lane, fmt, read, write in lanes:
            check(read(), pairs)
            out = write()
            assert isinstance(out, bytes)
            if fmt == "xml":
                schema.assertValid(etree.fromstring(out))
                check(tree_read(out, ET), pairs)
            else:
                assert json.loads(out) == json.loads(raw_json)
            sizes[(lane, fmt)] = len(out)
            export = os.environ.get("BENCH_EXPORT")
            if export:
                path = Path(export)
                path.mkdir(parents=True, exist_ok=True)
                (path / f"python-{lane}-{case}.{fmt}").write_bytes(out)
        shift = order % len(lanes)
        for lane, fmt, read, write in lanes[shift:] + lanes[:shift]:
            size = len(xml) if fmt == "xml" else len(raw_json)
            measure(writer, lane, case, f"{fmt}_read", size, 0, read)
            measure(writer, lane, case, f"{fmt}_write", size, sizes[(lane, fmt)], write)


if __name__ == "__main__":
    main()
