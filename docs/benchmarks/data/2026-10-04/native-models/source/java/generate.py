"""Generate an identical, reproducible scalar schema for all benchmark backends."""

import os
import subprocess
from pathlib import Path

base = Path(__file__).resolve().parent
repo = base.parent.parent
target = base / "target"
target.mkdir(exist_ok=True)
fields = ["id", "status", "timestamp", "amount", "latitude", "longitude", "payload"]
fields += [f"optional{i}" for i in range(73)]
schema = '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">\n'
schema += '<xs:element name="message" type="Message"/>\n<xs:complexType name="Message"><xs:sequence>\n'
for field in fields:
    optional = ' minOccurs="0"' if field.startswith("optional") else ""
    schema += f'<xs:element name="{field}" type="xs:string"{optional}/>\n'
schema += "</xs:sequence></xs:complexType></xs:schema>\n"
xsd = target / "workload.xsd"
xsd.write_text(schema)
binary = Path(os.environ.get("POLYXML_BIN", repo / "target/debug/polyxml"))
if not binary.exists():
    subprocess.run(["cargo", "build", "-p", "polyxml-cli"], cwd=repo, check=True)
for style, package in [("pojo", "pojo"), ("record", "records")]:
    subprocess.run(
        [
            str(binary),
            "generate",
            str(xsd),
            "--lang",
            "java",
            "--style",
            style,
            "--feature",
            "builder,direct-codec",
            "--backend",
            "jackson",
            "--package",
            f"io.polyxml.bench.{package}",
            "--out",
            str(target / "generated-sources/polyxml" / package),
        ],
        check=True,
    )
helper = target / "generated-sources/polyxml/RecordMutation.java"
args = ", ".join(
    "status" if field == "status" else f"value.{field}()" for field in fields
)
helper.write_text(
    "package io.polyxml.bench;\npublic final class RecordMutation {\n"
    "public static io.polyxml.bench.records.Message withStatus(io.polyxml.bench.records.Message value, String status) {\n"
    f"return new io.polyxml.bench.records.Message({args});\n" + "}\n}\n"
)
subprocess.run(
    [
        str(binary),
        "generate",
        str(base / "cases.xsd"),
        "--lang",
        "java",
        "--style",
        "pojo",
        "--feature",
        "builder,direct-codec",
        "--backend",
        "jackson",
        "--package",
        "io.polyxml.bench.cases",
        "--out",
        str(target / "generated-sources/polyxml/cases"),
    ],
    check=True,
)
