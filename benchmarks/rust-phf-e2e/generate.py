#!/usr/bin/env python3
"""Generate identical small/medium XML schemas and documents for both strategies."""

from pathlib import Path

root = Path(__file__).resolve().parent / "target"
for count in (16, 120):
    folder = root / str(count)
    folder.mkdir(parents=True, exist_ok=True)
    fields = "".join(
        f'<xs:element name="Field{i:03}" type="xs:string"/>' for i in range(count)
    )
    schema = (
        '<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">'
        '<xs:element name="Record" type="RecordType"/>'
        '<xs:complexType name="RecordType"><xs:sequence>'
        f"{fields}</xs:sequence></xs:complexType></xs:schema>"
    )
    document = (
        "<Record>"
        + "".join(f"<Field{i:03}>value-{i:03}</Field{i:03}>" for i in range(count))
        + "</Record>"
    )
    (folder / "record.xsd").write_text(schema)
    (folder / "record.xml").write_text(document)
