use std::sync::Arc;

use polyxml::error::PolyXmlError;
use polyxml::ir::PrimitiveType;
use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::value::PolyValue;
use polyxml::{deserialize, serialize};

#[test]
fn test_gregorian_partial_date_lexical_validation() {
    use polyxml::converters::ValueConverter;

    for (kind, valid, invalid) in [
        (PrimitiveType::GDay, "---15Z", "---32"),
        (PrimitiveType::GMonth, "--04+14:00", "--04+14:01"),
        (PrimitiveType::GYear, "-0045", "012345"),
        (PrimitiveType::GYearMonth, "2026-09-05:30", "2026-13"),
        (PrimitiveType::GMonthDay, "--02-29", "--02-30"),
    ] {
        let scalar = ScalarType::XmlGregorian(kind);
        assert_eq!(
            ValueConverter::parse_scalar(&scalar, valid.as_bytes(), "date").unwrap(),
            PolyValue::String(valid.into())
        );
        assert!(ValueConverter::parse_scalar(&scalar, invalid.as_bytes(), "date").is_err());
    }
}

#[test]
fn test_xsd_gregorian_fields_round_trip_and_reject_invalid_dates() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:complexType name="Calendar"><xs:sequence>
        <xs:element name="day" type="xs:gDay"/>
        <xs:element name="month" type="xs:gMonth"/>
        <xs:element name="year" type="xs:gYear"/>
        <xs:element name="yearMonth" type="xs:gYearMonth"/>
        <xs:element name="monthDay" type="xs:gMonthDay"/>
      </xs:sequence></xs:complexType>
      <xs:element name="calendar" type="Calendar"/>
    </xs:schema>"#;
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(xsd)
        .unwrap();
    assert_eq!(ir.types.len(), 6);
    let schema = ModelSchema::from_ir(&ir, Some("calendar")).unwrap();
    let xml = br#"<calendar><day>---15Z</day><month>--04+14:00</month><year>-0045</year><yearMonth>2026-09-05:30</yearMonth><monthDay>--02-29</monthDay></calendar>"#;
    let value = deserialize(xml, Arc::clone(&schema)).unwrap();
    let out = serialize("calendar", &value, &schema, None).unwrap();
    assert_eq!(deserialize(&out, Arc::clone(&schema)).unwrap(), value);
    let invalid = br#"<calendar><day>---32</day><month>--04</month><year>2026</year><yearMonth>2026-09</yearMonth><monthDay>--02-29</monthDay></calendar>"#;
    assert!(deserialize(invalid, Arc::clone(&schema)).is_err());
    let mut bad_value = std::collections::HashMap::new();
    bad_value.insert("day".into(), PolyValue::String("---32".into()));
    assert!(serialize("calendar", &PolyValue::Object(bad_value), &schema, None).is_err());
}

#[test]
fn test_all_scalar_types() {
    let schema = ModelSchema::builder("Sensors")
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::Int),
        ))
        .field(FieldSchema::new(
            "sensor_name",
            b"name",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "temperature",
            b"temp",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Float),
        ))
        .field(FieldSchema::new(
            "is_calibrated",
            b"calibrated",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Bool),
        ))
        .field(FieldSchema::new(
            "timestamp",
            b"timestamp",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::XmlDateTime),
        ))
        .field(FieldSchema::new(
            "reading_date",
            b"date",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::XmlDate),
        ))
        .field(FieldSchema::new(
            "duration",
            b"duration",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::XmlDuration),
        ))
        .field(FieldSchema::new(
            "precision_decimal",
            b"prec",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Decimal),
        ))
        .build();

    let xml = br#"
    <Sensors id="999">
        <name>Thermostat Alpha</name>
        <temp>-12.345</temp>
        <calibrated>1</calibrated>
        <timestamp>2026-09-09T10:00:00Z</timestamp>
        <date>2026-09-09</date>
        <duration>PT1H30M</duration>
        <prec>123456789.987654321</prec>
    </Sensors>
    "#;

    let val = deserialize(xml, Arc::clone(&schema)).expect("Failed to deserialize scalars");
    assert_eq!(val.get("id"), Some(&PolyValue::Int(999)));
    assert_eq!(
        val.get("sensor_name"),
        Some(&PolyValue::String("Thermostat Alpha".into()))
    );
    assert_eq!(val.get("temperature"), Some(&PolyValue::Float(-12.345)));
    assert_eq!(val.get("is_calibrated"), Some(&PolyValue::Bool(true)));
    assert_eq!(
        val.get("timestamp"),
        Some(&PolyValue::String("2026-09-09T10:00:00Z".into()))
    );
    assert_eq!(
        val.get("reading_date"),
        Some(&PolyValue::String("2026-09-09".into()))
    );
    assert_eq!(
        val.get("duration"),
        Some(&PolyValue::String("PT1H30M".into()))
    );
    assert_eq!(
        val.get("precision_decimal"),
        Some(&PolyValue::String("123456789.987654321".into()))
    );
}

#[test]
fn test_error_handling_invalid_scalars() {
    let schema = ModelSchema::builder("Device")
        .field(FieldSchema::new(
            "port",
            b"port",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Int),
        ))
        .field(FieldSchema::new(
            "ratio",
            b"ratio",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Float),
        ))
        .field(FieldSchema::new(
            "online",
            b"online",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Bool),
        ))
        .build();

    // Invalid integer
    let xml_bad_int = b"<Device><port>not-a-number</port></Device>";
    let err = deserialize(xml_bad_int, Arc::clone(&schema)).unwrap_err();
    match err {
        PolyXmlError::ScalarParseError {
            field,
            expected,
            value,
        } => {
            assert_eq!(field, "port");
            assert_eq!(expected, "integer");
            assert_eq!(value, "not-a-number");
        }
        other => panic!("Unexpected error variant: {:?}", other),
    }

    // Invalid float
    let xml_bad_float = b"<Device><ratio>bad_float</ratio></Device>";
    let err = deserialize(xml_bad_float, Arc::clone(&schema)).unwrap_err();
    match err {
        PolyXmlError::ScalarParseError {
            field, expected, ..
        } => {
            assert_eq!(field, "ratio");
            assert_eq!(expected, "float");
        }
        other => panic!("Unexpected error variant: {:?}", other),
    }

    // Invalid boolean
    let xml_bad_bool = b"<Device><online>maybe</online></Device>";
    let err = deserialize(xml_bad_bool, Arc::clone(&schema)).unwrap_err();
    match err {
        PolyXmlError::ScalarParseError {
            field, expected, ..
        } => {
            assert_eq!(field, "online");
            assert!(expected.contains("boolean"));
        }
        other => panic!("Unexpected error variant: {:?}", other),
    }
}

#[test]
fn test_malformed_xml_syntax_error() {
    let schema = ModelSchema::builder("Root")
        .field(FieldSchema::new(
            "val",
            b"val",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    let unclosed_xml = b"<Root><val>test";
    let err = deserialize(unclosed_xml, schema);
    assert!(err.is_err());
}

#[test]
fn test_xsi_nil_handling() {
    let schema = ModelSchema::builder("Payload")
        .field(FieldSchema::new(
            "nullable_val",
            b"val",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "nullable_attr",
            b"attr",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::Int),
        ))
        .build();

    let xml = br#"<Payload><val xsi:nil="true"/></Payload>"#;
    let val = deserialize(xml, schema).expect("Deserialization failed for nil element");
    assert_eq!(val.get("nullable_val"), Some(&PolyValue::Null));
}

#[test]
fn test_roundtrip_serialization_with_indentation() {
    let child_schema = ModelSchema::builder("Child")
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::Int),
        ))
        .field(FieldSchema::new(
            "name",
            b"name",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    let parent_schema = ModelSchema::builder("Parent")
        .field(FieldSchema::new(
            "title",
            b"title",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "children",
            b"child",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Nested(Arc::clone(&child_schema)))),
        ))
        .build();

    let xml = br#"
    <Parent>
        <title>Family Tree</title>
        <child id="1"><name>Alice</name></child>
        <child id="2"><name>Bob</name></child>
    </Parent>
    "#;

    let val = deserialize(xml, Arc::clone(&parent_schema)).expect("Failed to deserialize parent");
    let serialized = serialize("Parent", &val, &parent_schema, Some(2))
        .expect("Failed to serialize with indent");
    let serialized_str = std::str::from_utf8(&serialized).unwrap();

    assert!(serialized_str.contains("  <title>Family Tree</title>"));
    assert!(serialized_str.contains(r#"<child id="1">"#));
    assert!(serialized_str.contains("<name>Alice</name>"));

    // Re-deserialize to verify data preservation
    let val_re =
        deserialize(&serialized, Arc::clone(&parent_schema)).expect("Failed to re-deserialize");
    assert_eq!(val, val_re);
}

#[test]
fn unbounded_integers_round_trip_and_compare_exact_bounds() {
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/wave5/unbounded_integer.xsd"
        ))
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    for digits in [
        "1234567890123456789012345678901234567890",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "-1234567890123456789012345678901234567890",
        "+000123",
    ] {
        let xml = format!("<Root><Value>{digits}</Value></Root>");
        let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
        let output = serialize("Root", &value, &schema, None).unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), xml);
    }
    for bad in ["1.0", "1e30", "", "+", "1 2", "١"] {
        assert!(deserialize(
            format!("<Root><Value>{bad}</Value></Root>").as_bytes(),
            Arc::clone(&schema)
        )
        .is_err());
    }
    use polyxml::converters::ValueConverter;
    let bound = "1234567890123456789012345678901234567890";
    let scalar = ScalarType::Restricted(
        Box::new(ScalarType::Integer(PrimitiveType::Integer)),
        Box::new(polyxml::ir::RestrictionFacets {
            min_inclusive: Some(bound.into()),
            max_inclusive: Some(bound.into()),
            ..Default::default()
        }),
    );
    assert!(ValueConverter::parse_scalar(&scalar, bound.as_bytes(), "value").is_ok());
    for outside in [
        "1234567890123456789012345678901234567889",
        "1234567890123456789012345678901234567891",
    ] {
        assert!(ValueConverter::parse_scalar(&scalar, outside.as_bytes(), "value").is_err());
    }
    for (kind, good, bad) in [
        (PrimitiveType::PositiveInteger, "1", "0"),
        (PrimitiveType::NonNegativeInteger, "-0", "-1"),
        (PrimitiveType::NegativeInteger, "-1", "-0"),
        (PrimitiveType::NonPositiveInteger, "+0", "1"),
    ] {
        assert!(
            ValueConverter::parse_scalar(&ScalarType::Integer(kind), good.as_bytes(), "value")
                .is_ok()
        );
        assert!(
            ValueConverter::parse_scalar(&ScalarType::Integer(kind), bad.as_bytes(), "value")
                .is_err()
        );
    }
    assert!(
        ValueConverter::parse_scalar(&ScalarType::Int, b"9223372036854775808", "bounded").is_err()
    );
}

#[test]
fn integer_mappings_keep_fixed_width_builtins() {
    use polyxml::codegen::{
        cpp::CppLanguageContext, csharp::CSharpLanguageContext, go::GoLanguageContext,
        java::JavaLanguageContext, python::PythonLanguageContext, rust::RustLanguageContext,
        typescript::TypeScriptLanguageContext, LanguageContext,
    };
    let rust = RustLanguageContext::new(false);
    let contexts: [(&dyn LanguageContext, &str, &str); 7] = [
        (&rust, "String", "i64"),
        (&GoLanguageContext, "PolyxmlInteger", "int64"),
        (&CSharpLanguageContext, "string", "long"),
        (&JavaLanguageContext, "java.math.BigInteger", "long"),
        (&CppLanguageContext, "std::string", "std::int64_t"),
        (&TypeScriptLanguageContext, "string", "number"),
        (&PythonLanguageContext, "int", "int"),
    ];
    for (context, unbounded, bounded) in contexts {
        assert_eq!(context.map_primitive(PrimitiveType::Integer), unbounded);
        assert_eq!(context.map_primitive(PrimitiveType::Long), bounded);
    }
}
