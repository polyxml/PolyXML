---
title: Transcoding & Streaming Guide
description: High-throughput bidirectional XML and JSON document transcoding using PolyXML's native Rust streaming core.
---

# XML ↔ JSON Transcoding & Streaming (`polyxml transcode`)

PolyXML includes a high-throughput, bidirectional document transcoding utility (`polyxml transcode`) that converts complete XML documents to JSON and JSON documents back to XML. Transcoding can run **schema-directed** with W3C XSD schemas for strict type mapping, or **schema-free** for rapid document conversion with full attribute preservation.

---

## ⚡ Quick Start

```bash
# 1. Transcode XML to JSON with strict W3C XSD schema typing
polyxml transcode --schema order.xsd --pretty order.xml --out order.json

# 2. Pipe directly through standard input and standard output
cat order.xml | polyxml transcode --schema order.xsd > order.json

# 3. Transcode JSON back to XML with specified root element
cat order.json | polyxml transcode --schema order.xsd --root order --pretty > order.xml

# 4. Dynamic schema-less transcoding with attribute (@attr) and text (#text) preservation
polyxml transcode legacy.xml --out modern.json
```

---

## 🛠️ CLI Options

| Option | Flag | Description | Default |
|---|---|---|---|
| **Input File** | `[INPUT]` | Path to source document, or `-` / omitted for standard input (`stdin`). | Standard input |
| **Output File** | `-o`, `--out` | Path to destination file, or `-` / omitted for standard output (`stdout`). | Standard output |
| **From Format** | `--from` | Input format (`xml` or `json`). Automatically detected if omitted. | Auto-detected |
| **To Format** | `--to` | Output format (`xml` or `json`). Automatically detected from target or opposite of input. | Auto-detected |
| **Schema** | `-s`, `--schema` | Path to W3C XSD schema for strongly-typed schema-directed conversion. | None (schema-free) |
| **Root Element** | `-r`, `--root` | Root XML element tag name (required when converting JSON to XML without a schema). | None |
| **Pretty Print** | `--pretty` | Format output with readable indentation and line breaks. | `false` (compact) |

---

## 1. Schema-Directed Transcoding (Recommended)

When supplied with a W3C XSD schema (`-s`, `--schema`), PolyXML applies exact XSD primitive and complex types to the conversion:

- **Integers and Floats**: Numeric fields (`xs:int`, `xs:decimal`, `xs:double`) are serialized as native JSON numbers rather than strings.
- **Booleans**: `xs:boolean` values (`true`, `false`, `1`, `0`) are normalized to native JSON `true` / `false`.
- **Arrays & Collections**: Repeated elements (`maxOccurs="unbounded"`) always produce JSON arrays, even when a single item appears in a document.
- **Nil Values**: Elements with `xsi:nil="true"` convert to JSON `null`.

### Example

Given `telemetry.xsd`:
```xml
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="Telemetry">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="id" type="xs:int"/>
        <xs:element name="battery" type="xs:double"/>
        <xs:element name="online" type="xs:boolean"/>
        <xs:element name="sensor" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
      </xs:sequence>
    </xs:complexType>
  </xs:element>
</xs:schema>
```

And input XML `telemetry.xml`:
```xml
<Telemetry>
  <id>42</id>
  <battery>98.5</battery>
  <online>true</online>
  <sensor>IMU</sensor>
  <sensor>GPS</sensor>
</Telemetry>
```

Running:
```bash
polyxml transcode --schema telemetry.xsd --pretty telemetry.xml --out telemetry.json
```

Produces typed JSON:
```json
{
  "id": 42,
  "battery": 98.5,
  "online": true,
  "sensor": [
    "IMU",
    "GPS"
  ]
}
```

---

## 2. Schema-Free Transcoding

When no schema is provided, PolyXML applies standard attribute and mixed-content mappings:
- XML attributes are prefixed with `@` (e.g. `@id`, `@version`).
- Text content within elements containing attributes or child elements is assigned to `#text`.
- Simple elements convert directly to string key-value pairs.

### Example

```bash
polyxml transcode --pretty legacy_config.xml --out config.json
```

Input XML:
```xml
<Server id="prod-01" region="us-east-1">
  <Name>Gateway</Name>
  <Port>8080</Port>
</Server>
```

Output JSON:
```json
{
  "@id": "prod-01",
  "@region": "us-east-1",
  "Name": "Gateway",
  "Port": "8080"
}
```

---

## 3. JSON to XML Transcoding

To convert JSON payloads back into XML documents:

```bash
cat order.json | polyxml transcode --schema order.xsd --root order --pretty > order.xml
```

> [!NOTE]
> When transcoding JSON back to XML without an XSD schema, `--root <ELEMENT_NAME>` is required so PolyXML knows what root element tag to wrap the document in.

---

## 4. Pipeline & Unix Pipe Integration

Because `polyxml transcode` reads from `stdin` and writes to `stdout` by default, it integrates cleanly into Unix pipelines, curl commands, and message queue workers:

```bash
# Fetch XML from an API, transcode to JSON, and inspect with jq
curl -s https://api.example.com/feed.xml | polyxml transcode --schema feed.xsd | jq '.entries[] | .title'

# Transcode JSON from a Kafka topic or webhook into XML
cat webhook.json | polyxml transcode --schema payment.xsd --root PaymentMessage > payload.xml
```
