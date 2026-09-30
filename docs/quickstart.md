---
title: Multi-Language Quickstart
description: Get started with PolyXML in Rust, Python, C++, Go, Java, or TypeScript in under 5 minutes.
---

# Multi-Language Quickstart

Choose your preferred language to see how PolyXML deserializes XML payloads into strongly-typed models:

=== "Python"

    ### Installation
    ```bash
    pip install polyxml
    ```

    ### Example
    ```python
    from dataclasses import dataclass, field
    import polyxml

    @dataclass
    class Sensor:
        id: int = field(metadata={"type": "Attribute"})
        name: str = field(metadata={"type": "Element"})
        reading: float = field(metadata={"type": "Element"})
        calibrated: bool = field(metadata={"type": "Element"})

    xml = b"""
    <Sensor id="101">
        <name>Barometric Altimeter</name>
        <reading>1013.25</reading>
        <calibrated>true</calibrated>
    </Sensor>
    """

    # 1. Deserialize XML directly into dataclass
    sensor = polyxml.deserialize(xml, Sensor)
    print(f"Sensor: {sensor.name}, Reading: {sensor.reading}")

    # 2. Serialize model back to formatted XML (supports namespaces & ns_map)
    xml_output = polyxml.serialize(sensor, indent=2)
    print(xml_output.decode("utf-8"))

    # 3. Iterate records from a large XML document
    # for s in polyxml.iterparse(open("huge.xml", "rb"), Sensor, tag="Sensor"):
    #     print(s.name)

    # 4. Zero-GIL binary serialization for key-value DBs (MDBX, Redis) & IPC
    binary_blob = polyxml.dumps_binary(sensor)
    restored = polyxml.loads_binary(binary_blob, Sensor)
    assert restored.name == sensor.name
    ```

=== "Rust"

    ### Cargo Dependency
    ```toml
    [dependencies]
    polyxml = "0.1"
    ```

    ### Example
    ```rust
    use std::sync::Arc;
    use polyxml::schema::{ModelSchema, FieldSchema, FieldKind, ScalarType, ValueType};
    use polyxml::{deserialize, serialize};

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let schema = ModelSchema::builder("Sensor")
            .field(FieldSchema::new("id", b"id", FieldKind::Attribute, ValueType::Scalar(ScalarType::Int)))
            .field(FieldSchema::new("name", b"name", FieldKind::Element, ValueType::Scalar(ScalarType::String)))
            .field(FieldSchema::new("reading", b"reading", FieldKind::Element, ValueType::Scalar(ScalarType::Float)))
            .field(FieldSchema::new("calibrated", b"calibrated", FieldKind::Element, ValueType::Scalar(ScalarType::Bool)))
            .build();

        let xml = br#"<Sensor id="101"><name>Barometric Altimeter</name><reading>1013.25</reading><calibrated>true</calibrated></Sensor>"#;

        // 1. Deserialize
        let val = deserialize(xml, Arc::clone(&schema))?;
        println!("Name: {}", val.get("name").unwrap().as_str().unwrap());

        // 2. Serialize
        let output = serialize("Sensor", &val, &schema, Some(2))?;
        println!("{}", std::str::from_utf8(&output)?);

        Ok(())
    }
    ```

=== "Modern C++20"

    ### Include Header
    ```cpp
    #include "polyxml.hpp"
    #include <iostream>

    int main() {
        auto schema = polyxml::SchemaBuilder("Sensor")
            .add_attribute("id", "id", POLYXML_SCALAR_INT)
            .add_element("name", "name", POLYXML_SCALAR_STRING)
            .add_element("reading", "reading", POLYXML_SCALAR_FLOAT)
            .add_element("calibrated", "calibrated", POLYXML_SCALAR_BOOL)
            .build();

        std::string xml = R"(<Sensor id="101"><name>Gyro</name><reading>99.5</reading><calibrated>true</calibrated></Sensor>)";

        // 1. Deserialize
        auto val = polyxml::deserialize(xml, schema);
        std::cout << "Name: " << val.get("name")->as_string().value() << "\n";

        // 2. Serialize
        std::string output = polyxml::serialize("Sensor", val, schema, 2);
        std::cout << output << "\n";

        return 0;
    }
    ```

=== "Go"

    ### Go Import
    ```go
    import (
        "fmt"
        "github.com/polyxml/PolyXML/bindings/go"
    )

    func main() {
        builder, _ := polyxml.NewSchemaBuilder("Sensor")
        builder.AddField("id", "id", polyxml.FieldAttribute, polyxml.ScalarInt)
        builder.AddField("name", "name", polyxml.FieldElement, polyxml.ScalarString)
        builder.AddField("reading", "reading", polyxml.FieldElement, polyxml.ScalarFloat)
        schema, _ := builder.Build()

        xml := []byte(`<Sensor id="101"><name>Altimeter</name><reading>1013.25</reading></Sensor>`)

        // 1. Deserialize
        val, _ := polyxml.Deserialize(xml, schema)
        name, _ := val.GetField("name").GetString()
        fmt.Println("Sensor Name:", name)

        // 2. Serialize
        output, _ := polyxml.Serialize("Sensor", val, schema, 2)
        fmt.Println(string(output))
    }
    ```

=== "TypeScript & Node.js"

    ### NPM Install
    ```bash
    npm install @polyxml/node
    ```

    ### Example
    ```typescript
    import { deserialize, serialize, ModelSchema } from '@polyxml/node';

    const schema: ModelSchema = {
      name: 'Sensor',
      fields: [
        { name: 'id', xmlName: 'id', kind: 'attribute', scalarType: 'int' },
        { name: 'name', xmlName: 'name', kind: 'element', scalarType: 'string' },
        { name: 'reading', xmlName: 'reading', kind: 'element', scalarType: 'float' },
        { name: 'calibrated', xmlName: 'calibrated', kind: 'element', scalarType: 'bool' },
      ],
    };

    const xml = '<Sensor id="101"><name>Barometric</name><reading>1013.25</reading><calibrated>true</calibrated></Sensor>';

    // 1. Deserialize
    const sensor = deserialize(xml, schema);
    console.log(`Sensor: ${sensor.name}, Reading: ${sensor.reading}`);

    // 2. Serialize
    const output = serialize('Sensor', sensor, schema, 2);
    console.log(new TextDecoder().decode(output));
    ```

=== "Java (Panama FFI)"

    ### Maven Dependency
    ```xml
    <dependency>
        <groupId>io.github.polyxml</groupId>
        <artifactId>polyxml</artifactId>
        <version>0.33.0</version>
    </dependency>
    ```

    ### Example
    ```java
    import io.polyxml.PolyXML;

    public class App {
        public static void main(String[] args) {
            try (var schema = new PolyXML.SchemaBuilder("Sensor")
                    .addField("id", "id", PolyXML.FieldKind.ATTRIBUTE, PolyXML.ScalarType.INT)
                    .addField("name", "name", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.STRING)
                    .build()) {

                System.out.println("PolyXML Native Version: " + PolyXML.version());
            }
        }
    }
    ```

=== "C# 12 / .NET 8+"

    ### Example Model & Serialization
    ```csharp
    using System;
    using System.IO;
    using System.Xml.Serialization;

    [XmlRoot("Sensor")]
    public record Sensor(
        [property: XmlAttribute("id")] int Id,
        [property: XmlElement("name")] string Name,
        [property: XmlElement("reading")] double Reading,
        [property: XmlElement("calibrated")] bool Calibrated = false
    )
    {
        public Sensor() : this(0, string.Empty, 0.0, false) { }
    }

    // 1. Deserialize XML
    var xml = "<Sensor id=\"101\"><name>Barometric</name><reading>1013.25</reading><calibrated>true</calibrated></Sensor>";
    var serializer = new XmlSerializer(typeof(Sensor));
    using var reader = new StringReader(xml);
    var sensor = (Sensor)serializer.Deserialize(reader)!;
    Console.WriteLine($"Sensor: {sensor.Name}, Reading: {sensor.Reading}");

    // 2. Serialize back to XML
    using var writer = new StringWriter();
    serializer.Serialize(writer, sensor);
    Console.WriteLine(writer.ToString());
    ```

=== "XSD-to-Code CLI"

    ### 0. Install the CLI
    ```bash
    # Universal one-line installer (Linux / macOS)
    curl -fsSL https://raw.githubusercontent.com/polyxml/PolyXML/main/scripts/install.sh | bash

    # Or via Homebrew
    brew install polyxml/polyxml/polyxml

    # Or pre-built .deb / .rpm from GitHub Releases:
    # sudo dpkg -i polyxml_amd64.deb
    # sudo dnf install ./polyxml.rpm
    ```

    ### 1. Compile Schema to Multiple Languages
    ```bash
    # Generate models for Python, Rust, and C# simultaneously
    polyxml generate \
      --lang python --backend pydantic \
      --lang rust --zero-copy --codecs \
      --lang csharp --namespace Sensors \
      --out ./generated \
      schemas/sensor.xsd

    # Or filter a large schema to specific root element(s) and their reachable dependencies
    polyxml generate --lang rust --root-element Entity --root-element PositionReport --out ./generated/uci schemas/uci.xsd
    ```

    ### 2. Declarative Workspace Build
    ```bash
    # Build all targets defined in polyxml.toml
    polyxml build --config polyxml.toml
    ```

    ### 3. Schema Static Analysis
    ```bash
    # Check schema validity and cycle topology
    polyxml validate schemas/*.xsd
    ```

---

## Consume the Generated Models Instantly

After compiling your schema with `polyxml generate`, each target ships ready-to-use models with built-in streaming XML codecs (plus native JSON on the same model):

=== "Python"

    ```python
    # Generated by polyxml generate --lang python
    from generated.python import Customer
    import polyxml

    # XML-to-dataclass parsing; see the linked benchmarks below for measured workloads
    customer = Customer.from_xml(xml_bytes)
    xml_output = customer.to_xml(indent=2)

    # Native JSON on the same model:
    json_bytes = customer.to_json(indent=2)
    customer = Customer.from_json(json_bytes)
    ```

    For measured XML comparisons, see the [Python benchmark results](https://github.com/polyxml/PolyXML/blob/main/benchmarks/python/results.md).

=== "Rust"

    ```rust
    // Generated by polyxml generate --lang rust --zero-copy --codecs
    use generated::rust::Customer;

    // Zero-copy deserialization: borrows text slices directly with Cow<'a, str>
    let customer = Customer::from_xml(xml_str)?;
    assert_eq!(customer.name.as_ref(), "Alice");

    // Stream back to XML or native JSON
    let output_xml = customer.to_xml_string()?;
    let output_json = customer.to_json_string()?;
    ```

=== "Modern C++20"

    ```cpp
    // Generated by polyxml generate --lang cpp --mode modules --backend glaze
    import crm;
    #include <glaze/glaze.hpp>
    #include <iostream>

    using namespace enterprise::crm;

    Customer customer{
        .name = "Global Logistics",
        .status = Status::Active,
        .id = 101
    };

    // Ultra-fast compile-time reflectionless serialization (multi-GB/s):
    std::string json;
    glz::write_json(customer, json);
    std::cout << "Serialized: " << json << std::endl;
    ```

=== "Go"

    ```go
    // Generated by polyxml generate --lang go
    // Models are dual-annotated with xml:"..." and json:"..." struct tags
    package main

    import (
        "encoding/json"
        "encoding/xml"
        "fmt"
        "generated/banking"
    )

    func main() {
        var customer banking.Customer
        _ = xml.Unmarshal(xmlBytes, &customer)

        // Marshal to JSON immediately — zero translation layer:
        jsonBytes, _ := json.MarshalIndent(customer, "", "  ")
        fmt.Println(string(jsonBytes))
    }
    ```

=== "TypeScript & Node.js"

    ```typescript
    // Generated by polyxml generate --lang typescript --backend zod
    import { deserialize, ModelSchema } from '@polyxml/node';
    import { type Customer, CustomerSchema } from './generated/typescript/customer';

    // Structural schema for the native napi-rs engine (see the Node.js Guide)
    const customerModel: ModelSchema = {
      name: 'Customer',
      fields: [
        { name: 'id', xmlName: 'id', kind: 'attribute', scalarType: 'int' },
        { name: 'name', xmlName: 'name', kind: 'element', scalarType: 'string' },
        { name: 'status', xmlName: 'status', kind: 'element', scalarType: 'string' },
      ],
    };

    // 1. Deserialize XML → typed JS object via the native Rust engine
    const raw = deserialize(xml, customerModel);

    // 2. Validate & narrow with the generated Zod schema
    const customer: Customer = CustomerSchema.parse(raw);

    // 3. Native JSON round-trip on the same generated model
    const json = JSON.stringify(customer, null, 2);
    ```

=== "Java (Jackson)"

    ```java
    // Generated by polyxml generate --lang java --backend jackson
    package com.enterprise.banking;

    import com.fasterxml.jackson.dataformat.xml.XmlMapper;
    import com.fasterxml.jackson.databind.ObjectMapper;

    // Java 22 record deserialization with Jackson XmlMapper:
    XmlMapper xmlMapper = new XmlMapper();
    Customer customer = xmlMapper.readValue(xmlString, Customer.class);

    // Direct JSON serialization with ObjectMapper:
    ObjectMapper jsonMapper = new ObjectMapper();
    String json = jsonMapper.writeValueAsString(customer);
    System.out.println("Customer serialized to JSON: " + json);
    ```

=== "C# 12 / .NET 8+"

    ```csharp
    // Generated by polyxml generate --lang csharp
    // Records feature both XmlSerializer and System.Text.Json attributes
    using Enterprise.Banking;
    using System.Text.Json;
    using System.Xml.Serialization;

    var serializer = new XmlSerializer(typeof(Customer));
    var customer = (Customer)serializer.Deserialize(new StringReader(xml))!;

    // Natively serialize to JSON with System.Text.Json:
    string jsonString = JsonSerializer.Serialize(customer);
    Console.WriteLine($"Customer {customer.Name} serialized to JSON.");
    ```

---

## 🌐 Real-World Polyglot Project Templates

Looking for production-grade project repositories with complete build setups across all 7 languages? Explore our open-source reference implementations:

| Domain & Repository | Standards & Integration | Key PolyXML Features Highlighted |
|---|---|---|
| **Defense & Avionics**<br>[polyxml-defense-examples](https://github.com/polyxml/polyxml-defense-examples) | **Anduril Lattice SDK** (Protobuf/JSON) ↔ **USAF UCI v2.5** (XML) | • `backend = "aot"` Ahead-of-Time PyO3 C-extension in the [telemetry bridge](https://github.com/polyxml/polyxml-defense-examples/blob/main/examples/python/bridge_aot.py)<br>• `features = ["rkyv"]` zero-copy binary serialization in Rust<br>• `xsd:extension` inheritance & base-field inlining<br>• Large schema validation on the [8.3 MB UCI schema](https://github.com/polyxml/polyxml-defense-examples#readme)<br>• Standard Library Java 22+ immutable records<br>• Edge C2 Wasm streaming via `@polyxml/wasm` |
| **Banking & FinTech**<br>[polyxml-finance-examples](https://github.com/polyxml/polyxml-finance-examples) | **FinTech Payments** (FedNow, Stripe, Plaid) ↔ **ISO 20022 `pacs.008`** (XML) | • `backend = "jackson"` Java annotations for enterprise microservices<br>• `backend = "source-gen"` C# 12 / .NET 8 Native AOT source-gen<br>• Strict facets & attributes on simple content (`Ccy="USD"`)<br>• Batch payment streaming via `@polyxml/wasm` |
| **Public Transit & Mobility**<br>[polyxml-transit-examples](https://github.com/polyxml/polyxml-transit-examples) | **Google GTFS-Realtime** (Protobuf/JSON) ↔ **European CEN SIRI & NeTEx** (XML) | • `backend = "sonic"` ByteDance JIT/AVX JSON engine for Go<br>• Deeply nested arrays & complex recursive collections<br>• C++20 `XmlModel` concept verification & `operator==` equality<br>• Passenger map live streaming via `@polyxml/wasm` |
