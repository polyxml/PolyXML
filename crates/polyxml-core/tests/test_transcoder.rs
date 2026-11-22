use std::sync::Arc;

use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::schema_parser::XsdParser;
use polyxml::{json_to_xml, xml_to_json};

#[test]
fn test_schemaless_xml_to_json_and_back() {
    let xml = br#"<order id="42" vip="true"><customer>Alice</customer><item>Book</item><item>Pen</item></order>"#;

    let json_bytes = xml_to_json(xml, None, None, true).expect("xml_to_json");
    let json_str = String::from_utf8(json_bytes.clone()).unwrap();

    assert!(json_str.contains("\"@id\":42"));
    assert!(json_str.contains("\"@vip\":true"));
    assert!(json_str.contains("\"customer\":\"Alice\""));
    assert!(json_str.contains("\"item\":[\"Book\",\"Pen\"]"));

    // Transcode back from JSON to XML
    let roundtrip_xml =
        json_to_xml(&json_bytes, None, None, None, None, None).expect("json_to_xml");
    let roundtrip_str = String::from_utf8(roundtrip_xml).unwrap();

    assert!(
        roundtrip_str.contains("<order")
            && roundtrip_str.contains("id=\"42\"")
            && roundtrip_str.contains("vip=\"true\"")
    );
    assert!(roundtrip_str.contains("<customer>Alice</customer>"));
    assert!(roundtrip_str.contains("<item>Book</item>"));
    assert!(roundtrip_str.contains("<item>Pen</item>"));
}

#[test]
fn test_schemaless_pretty_transcoding() {
    let xml = br#"<root><greeting>Hello World</greeting></root>"#;
    let json_bytes = xml_to_json(xml, None, Some(2), true).unwrap();
    let json_str = String::from_utf8(json_bytes).unwrap();

    assert!(json_str.contains("{\n  \"root\": {\n    \"greeting\": \"Hello World\"\n  }\n}"));

    let xml_roundtrip = json_to_xml(json_str.as_bytes(), None, None, Some(2), None, None).unwrap();
    let xml_str = String::from_utf8(xml_roundtrip).unwrap();
    assert!(xml_str.contains("<root>\n  <greeting>Hello World</greeting>\n</root>"));
}

#[test]
fn test_schema_directed_transcoding() {
    let item_schema = ModelSchema::builder("Item")
        .field(FieldSchema::new(
            "name",
            b"name",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "sku",
            b"sku",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    let order_schema = ModelSchema::builder("Order")
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::Int),
        ))
        .field(FieldSchema::new(
            "active",
            b"active",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Bool),
        ))
        .field(FieldSchema::new(
            "item",
            b"item",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Nested(item_schema))),
        ))
        .build();

    let xml = br#"<Order id="999"><active>true</active><item sku="SKU-1"><name>Keyboard</name></item><item sku="SKU-2"><name>Mouse</name></item></Order>"#;

    let json_bytes = xml_to_json(xml, Some(Arc::clone(&order_schema)), Some(2), true).unwrap();
    let json_str = String::from_utf8(json_bytes.clone()).unwrap();

    assert!(json_str.contains("\"id\": 999"));
    assert!(json_str.contains("\"active\": true"));
    assert!(json_str.contains("\"sku\": \"SKU-1\""));
    assert!(json_str.contains("\"name\": \"Keyboard\""));

    let roundtrip_xml = json_to_xml(
        &json_bytes,
        Some(order_schema),
        Some("Order"),
        None,
        None,
        None,
    )
    .unwrap();
    let roundtrip_str = String::from_utf8(roundtrip_xml).unwrap();

    assert!(roundtrip_str.contains("id=\"999\""));
    assert!(roundtrip_str.contains("<active>true</active>"));
    assert!(roundtrip_str.contains("<name>Keyboard</name>"));
    assert!(roundtrip_str.contains("<name>Mouse</name>"));
}

#[test]
fn test_schema_from_ir_transcoding() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="Product" type="ProductType"/>
  <xs:complexType name="ProductType">
    <xs:sequence>
      <xs:element name="title" type="xs:string"/>
      <xs:element name="price" type="xs:decimal"/>
    </xs:sequence>
    <xs:attribute name="code" type="xs:string" use="required"/>
  </xs:complexType>
</xs:schema>"#;

    let mut parser = XsdParser::new();
    let ir = parser.parse_str(xsd).expect("parse xsd");
    let model_schema =
        ModelSchema::from_ir(&ir, Some("Product")).expect("build model schema from ir");

    assert_eq!(model_schema.name, "ProductType");

    let xml =
        br#"<Product code="PROD-101"><title>Rust in Action</title><price>39.99</price></Product>"#;
    let json_bytes =
        xml_to_json(xml, Some(model_schema), None, true).expect("transcode with ir schema");
    let json_str = String::from_utf8(json_bytes).unwrap();

    assert!(json_str.contains("\"code\":\"PROD-101\""));
    assert!(json_str.contains("\"title\":\"Rust in Action\""));
    assert!(json_str.contains("\"price\":\"39.99\""));
}

#[test]
fn test_transcoder_errors() {
    // Empty XML
    let err = xml_to_json(b"", None, None, true).unwrap_err();
    assert!(err.to_string().contains("Empty XML input"));

    // Malformed JSON
    let err = json_to_xml(b"{ not json }", None, None, None, None, None).unwrap_err();
    assert!(err.to_string().contains("Invalid JSON"));
}

/// quick-xml splits text at entity references (Text/GeneralRef/Text) and reports
/// CDATA as its own event. The transcoder used to fall through to `_ => {}` for
/// general refs (dropping them) and left attribute entities raw, corrupting
/// xml -> json -> xml round trips.
#[test]
fn test_schemaless_preserves_refs_cdata_and_attr_entities() {
    let xml = br#"<root><a>x &amp; y</a><b><![CDATA[raw & <text>]]></b><c>&#65;&#x42;</c><d name="A &amp; B"/></root>"#;
    let json_bytes = xml_to_json(xml, None, None, true).expect("xml_to_json");
    let json_str = String::from_utf8(json_bytes).unwrap();

    for expected in [
        "\"a\":\"x & y\"",
        "\"b\":\"raw & <text>\"",
        "\"c\":\"AB\"",
        "\"@name\":\"A & B\"",
    ] {
        assert!(
            json_str.contains(expected),
            "missing {expected} in:\n{json_str}"
        );
    }

    let roundtrip =
        json_to_xml(json_str.as_bytes(), None, None, None, None, None).expect("json_to_xml");
    let rt = String::from_utf8(roundtrip).unwrap();
    for expected in ["x &amp; y", "A &amp; B"] {
        assert!(rt.contains(expected), "roundtrip lost {expected} in:\n{rt}");
    }
}

#[test]
fn test_polymorphic_xsi_type_xml_json_xml_roundtrip() {
    use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
    use polyxml::PolyValue;

    let vehicle = ModelSchema::builder("Vehicle")
        .is_abstract(true)
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();
    let car = ModelSchema::builder("Car")
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "doors",
            b"doors",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Int),
        ))
        .build();
    let truck = ModelSchema::builder("Truck")
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "payload",
            b"payload",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();
    vehicle.set_variants(vec![car, truck]);

    let fleet = ModelSchema::builder("Fleet")
        .field(FieldSchema::new(
            "vehicle",
            b"vehicle",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Nested(vehicle.clone()))),
        ))
        .build();

    let xml = br#"<Fleet xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><vehicle xsi:type="Car"><id>c1</id><doors>4</doors></vehicle><vehicle xsi:type="Truck"><id>t1</id><payload>gravel</payload></vehicle></Fleet>"#;
    let json_bytes = xml_to_json(xml, Some(fleet.clone()), None, true).expect("xml_to_json");
    let json_str = String::from_utf8(json_bytes.clone()).unwrap();
    for expected in [
        "\"@xsi:type\":\"Car\"",
        "\"doors\":4",
        "\"@xsi:type\":\"Truck\"",
        "\"payload\":\"gravel\"",
    ] {
        assert!(
            json_str.contains(expected),
            "missing {expected} in:\n{json_str}"
        );
    }

    let rt_xml = json_to_xml(
        &json_bytes,
        Some(fleet.clone()),
        Some("Fleet"),
        None,
        None,
        None,
    )
    .expect("json_to_xml");
    let rt_str = String::from_utf8(rt_xml.clone()).unwrap();
    for expected in [
        "xsi:type=\"Car\"",
        "<doors>4</doors>",
        "xsi:type=\"Truck\"",
        "<payload>gravel</payload>",
    ] {
        assert!(
            rt_str.contains(expected),
            "roundtrip XML missing {expected} in:\n{rt_str}"
        );
    }

    let parsed = polyxml::deserialize(&rt_xml, fleet).expect("deserialize roundtrip XML");
    let vehicles = match parsed.get("vehicle") {
        Some(PolyValue::List(items)) => items,
        other => panic!("expected List, got {other:?}"),
    };
    assert_eq!(vehicles.len(), 2);
    assert_eq!(vehicles[0].get("doors"), Some(&PolyValue::Int(4)));
    assert_eq!(
        vehicles[1].get("payload"),
        Some(&PolyValue::String("gravel".to_string()))
    );

    let err = json_to_xml(
        br#"{"id":"x"}"#,
        Some(vehicle),
        Some("vehicle"),
        None,
        None,
        None,
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("abstract schema"),
        "unexpected error: {err}"
    );
}

#[test]
fn test_ir_schema_recursive_extension_and_choice_roundtrip() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:example:telemetry"
           xmlns:tns="urn:example:telemetry"
           elementFormDefault="qualified">
  <xs:complexType name="TelemetryHeader">
    <xs:sequence>
      <xs:element name="PlatformId" type="xs:string"/>
      <xs:element name="Seq" type="xs:unsignedLong"/>
    </xs:sequence>
  </xs:complexType>
  <xs:complexType name="TrackNode">
    <xs:complexContent>
      <xs:extension base="tns:TelemetryHeader">
        <xs:sequence>
          <xs:choice>
            <xs:element name="BearingDeg" type="xs:double"/>
            <xs:element name="LostSignal" type="xs:boolean"/>
          </xs:choice>
          <xs:element name="Child" type="tns:TrackNode" minOccurs="0" maxOccurs="unbounded"/>
        </xs:sequence>
      </xs:extension>
    </xs:complexContent>
  </xs:complexType>
  <xs:element name="TrackNode" type="tns:TrackNode"/>
</xs:schema>"#;

    let xml = br#"<TrackNode xmlns="urn:example:telemetry">
  <PlatformId>UAV-ALPHA</PlatformId>
  <Seq>9012</Seq>
  <BearingDeg>137.5</BearingDeg>
  <Child>
    <PlatformId>RELAY-02</PlatformId>
    <Seq>9013</Seq>
    <LostSignal>false</LostSignal>
  </Child>
</TrackNode>"#;

    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(xsd)
        .expect("parse telemetry xsd");
    let schema = ModelSchema::from_ir(&ir, Some("TrackNode")).expect("ModelSchema::from_ir");

    let json_bytes =
        xml_to_json(xml, Some(Arc::clone(&schema)), Some(2), true).expect("xml_to_json");
    let parsed: serde_json::Value = serde_json::from_slice(&json_bytes).expect("valid json");
    assert_eq!(parsed["PlatformId"], "UAV-ALPHA");
    assert_eq!(parsed["Seq"], 9012);
    assert_eq!(parsed["BearingDeg"], 137.5);
    assert_eq!(parsed["Child"][0]["PlatformId"], "RELAY-02");
    assert_eq!(parsed["Child"][0]["Seq"], 9013);
    assert_eq!(parsed["Child"][0]["LostSignal"], false);

    let rt_xml = json_to_xml(
        &json_bytes,
        Some(Arc::clone(&schema)),
        Some("TrackNode"),
        Some(2),
        None,
        None,
    )
    .expect("json_to_xml");
    let rt_str = String::from_utf8(rt_xml).expect("utf8");
    for snippet in [
        "<ns0:PlatformId>UAV-ALPHA</ns0:PlatformId>",
        "<ns0:Seq>9012</ns0:Seq>",
        "<ns0:BearingDeg>137.5</ns0:BearingDeg>",
        "<ns0:PlatformId>RELAY-02</ns0:PlatformId>",
        "<ns0:Seq>9013</ns0:Seq>",
        "<ns0:LostSignal>false</ns0:LostSignal>",
    ] {
        assert!(rt_str.contains(snippet), "missing {snippet} in:\n{rt_str}");
    }
}
