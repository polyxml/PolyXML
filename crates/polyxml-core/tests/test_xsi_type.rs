//! `xsi:type` polymorphic dispatch for abstract complex types.
//!
//! Locks in the runtime registry strategy: `ModelSchema` carries the
//! concrete derivations of a type (keyed by QName local part), the parser
//! selects one from the wire `xsi:type` attribute, and the serializer
//! re-emits the selector so round trips preserve the concrete type.

use std::io::Cursor;
use std::sync::Arc;

use polyxml::schema::{ModelSchema, ValueType};
use polyxml::schema_parser::XsdParser;
use polyxml::value::PolyValue;
use polyxml::XmlItemStream;

const VEHICLE_XSD: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
        targetNamespace="urn:veh" xmlns:t="urn:veh" elementFormDefault="qualified">
    <xs:complexType name="Vehicle" abstract="true">
        <xs:sequence>
            <xs:element name="id" type="xs:string"/>
        </xs:sequence>
    </xs:complexType>
    <xs:complexType name="Car">
        <xs:complexContent>
            <xs:extension base="t:Vehicle">
                <xs:sequence>
                    <xs:element name="doors" type="xs:int"/>
                </xs:sequence>
            </xs:extension>
        </xs:complexContent>
    </xs:complexType>
    <xs:complexType name="Truck">
        <xs:complexContent>
            <xs:extension base="t:Vehicle">
                <xs:sequence>
                    <xs:element name="payload" type="xs:string"/>
                </xs:sequence>
            </xs:extension>
        </xs:complexContent>
    </xs:complexType>
    <xs:element name="Vehicle" type="t:Vehicle"/>
    <xs:element name="Root">
        <xs:complexType>
            <xs:sequence>
                <xs:element name="vehicle" type="t:Vehicle" nillable="true"/>
                <xs:element name="spare" type="t:Vehicle"
                            minOccurs="0" maxOccurs="unbounded" nillable="true"/>
            </xs:sequence>
        </xs:complexType>
    </xs:element>
</xs:schema>"#;

fn schema_for(root: Option<&str>) -> Arc<ModelSchema> {
    let ir = XsdParser::new().parse_str(VEHICLE_XSD).expect("parse XSD");
    ModelSchema::from_ir(&ir, root).expect("build runtime schema")
}

/// Declared schema of the `Root` document's `vehicle` field.
fn declared_vehicle(root: &Arc<ModelSchema>) -> Arc<ModelSchema> {
    let idx = root
        .fields
        .iter()
        .position(|f| f.name == "vehicle")
        .expect("vehicle field");
    match &root.fields[idx].val_type {
        ValueType::Nested(s) => Arc::clone(s),
        other => panic!("expected nested vehicle schema, got {other:?}"),
    }
}

fn rec_field<'a>(value: &'a PolyValue, field_name: &str) -> Option<&'a PolyValue> {
    match value {
        PolyValue::Record { schema, values } => {
            let idx = schema.fields.iter().position(|f| f.name == field_name)?;
            values.get(idx).and_then(|v| v.as_ref())
        }
        _ => None,
    }
}

fn rec_schema(value: &PolyValue) -> &ModelSchema {
    match value {
        PolyValue::Record { schema, .. } => schema,
        other => panic!("expected Record, got {other:?}"),
    }
}

fn assert_str(value: Option<&PolyValue>, expected: &str, ctx: &str) {
    match value {
        Some(PolyValue::String(s)) => assert_eq!(s, expected, "wrong value for {ctx}"),
        other => panic!("expected string {expected:?} for {ctx}, got {other:?}"),
    }
}

fn schema_local(schema: &ModelSchema) -> String {
    String::from_utf8_lossy(&schema.xml_name).into_owned()
}

#[test]
fn from_ir_flattens_base_fields_and_registers_variants() {
    let root = schema_for(Some("Root"));
    let declared = declared_vehicle(&root);

    assert!(declared.is_abstract, "Vehicle must be marked abstract");
    assert!(
        declared.has_variants(),
        "Vehicle must register its derivations"
    );
    let keys: Vec<String> = declared
        .variants()
        .iter()
        .map(|v| schema_local(v))
        .collect();
    assert_eq!(keys, ["Car", "Truck"], "variant keys by QName local part");

    // The declared (abstract) type keeps only its own content model.
    let base_fields: Vec<&str> = declared.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(base_fields, ["id"], "abstract base keeps only base fields");

    // Derived runtime schemas flatten the extension chain (base first).
    let car = declared.find_variant(b"Car").expect("Car variant");
    let car_fields: Vec<&str> = car.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        car_fields,
        ["id", "doors"],
        "Car must inherit id before its own doors"
    );
    assert!(!car.is_abstract, "concrete derivation is not abstract");

    let truck = declared.find_variant(b"Truck").expect("Truck variant");
    let truck_fields: Vec<&str> = truck.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(truck_fields, ["id", "payload"]);
}

#[test]
fn nested_xsi_type_dispatch_and_round_trip() {
    let root = schema_for(Some("Root"));
    let xml = br#"<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:veh">
        <vehicle xsi:type="t:Car"><id>V1</id><doors>4</doors></vehicle>
        <spare xsi:type="t:Truck"><id>S1</id><payload>ore</payload></spare>
        <spare xsi:type="t:Car"/>
    </Root>"#;

    let value = polyxml::deserialize(xml, Arc::clone(&root)).expect("dispatch parse");

    let vehicle = rec_field(&value, "vehicle").expect("vehicle value");
    assert_eq!(schema_local(rec_schema(vehicle)), "Car");
    assert_str(rec_field(vehicle, "id"), "V1", "vehicle.id (inherited)");
    match rec_field(vehicle, "doors") {
        Some(PolyValue::Int(i)) => assert_eq!(*i, 4, "vehicle.doors"),
        other => panic!("expected int doors, got {other:?}"),
    }

    let spares = rec_field(&value, "spare").expect("spare list");
    let items = match spares {
        PolyValue::List(items) => items,
        other => panic!("expected spare list, got {other:?}"),
    };
    assert_eq!(items.len(), 2);
    assert_eq!(schema_local(rec_schema(&items[0])), "Truck");
    assert_str(
        rec_field(&items[0], "payload"),
        "ore",
        "spare.payload (inherited nesting)",
    );
    assert_eq!(schema_local(rec_schema(&items[1])), "Car");

    // Round trip: xsi:type selectors are re-emitted for every dispatch.
    let out = polyxml::serialize_with_options("Root", &value, &root, None, Some(true), None)
        .expect("serialize");
    let out_str = String::from_utf8_lossy(&out);
    assert!(out_str.contains("xsi:type="), "missing selector: {out_str}");
    assert!(out_str.contains("Car"), "missing Car selector: {out_str}");
    assert!(
        out_str.contains("Truck"),
        "missing Truck selector: {out_str}"
    );
    assert!(
        out_str.contains("xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\""),
        "missing xsi declaration: {out_str}"
    );

    let reparsed = polyxml::deserialize(&out, Arc::clone(&root)).expect("re-parse");
    assert_eq!(
        reparsed, value,
        "round trip must preserve dispatched records"
    );
}

#[test]
fn root_xsi_type_dispatch_including_empty_element() {
    let root = schema_for(Some("Vehicle"));
    assert!(root.is_abstract);
    assert!(root.has_variants());

    let xml = br#"<Vehicle xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:veh" xsi:type="t:Truck"><id>T9</id><payload>ore</payload></Vehicle>"#;
    let value = polyxml::deserialize(xml, Arc::clone(&root)).expect("root dispatch");
    assert_eq!(schema_local(rec_schema(&value)), "Truck");
    assert_str(rec_field(&value, "id"), "T9", "root id");
    assert_str(rec_field(&value, "payload"), "ore", "root payload");

    let out = polyxml::serialize_with_options("Vehicle", &value, &root, None, Some(true), None)
        .expect("serialize root");
    let out_str = String::from_utf8_lossy(&out);
    assert!(
        out_str.starts_with("<Vehicle") || out_str.contains(":Vehicle "),
        "root tag: {out_str}"
    );
    assert!(
        out_str.contains("xsi:type=") && out_str.contains("Truck"),
        "root selector: {out_str}"
    );
    let reparsed = polyxml::deserialize(&out, Arc::clone(&root)).expect("re-parse root");
    assert_eq!(reparsed, value);

    // Empty elements dispatch through the root Empty branch too.
    let empty = br#"<Vehicle xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:veh" xsi:type="t:Car"/>"#;
    let value = polyxml::deserialize(empty, root).expect("empty dispatch");
    assert_eq!(schema_local(rec_schema(&value)), "Car");
}

#[test]
fn unknown_xsi_type_on_abstract_type_errors() {
    let root = schema_for(Some("Root"));
    let xml = br#"<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:veh"><vehicle xsi:type="t:Plane"><id>X</id></vehicle></Root>"#;
    let err = polyxml::deserialize(xml, root).expect_err("unknown derivation must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("xsi:type=\"t:Plane\"") && msg.contains("known derivation"),
        "error must name the offending selector: {msg}"
    );
    assert!(
        msg.contains("'Vehicle'") && msg.contains("Car, Truck"),
        "error must name the base and known derivations: {msg}"
    );
}

#[test]
fn abstract_type_without_derivations_errors() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:solo" xmlns:t="urn:solo">
        <xs:complexType name="Solo" abstract="true">
            <xs:sequence>
                <xs:element name="id" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>
        <xs:element name="Solo" type="t:Solo"/>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).expect("parse XSD");
    let root = ModelSchema::from_ir(&ir, Some("Solo")).expect("build runtime schema");
    assert!(root.is_abstract);
    assert!(!root.has_variants());

    let xml = br#"<Solo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:solo" xsi:type="t:Other"><id>X</id></Solo>"#;
    let err = polyxml::deserialize(xml, root).expect_err("must fail loudly");
    let msg = err.to_string();
    assert!(
        msg.contains("no registered derivations") && msg.contains("escape hatch"),
        "error must explain the escape hatch: {msg}"
    );
}

#[test]
fn plain_type_attribute_is_not_mistaken_for_dispatch() {
    // A content attribute literally named "type" on a non-abstract type
    // must never be treated as an xsi:type selector.
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:tag" xmlns:t="urn:tag">
        <xs:complexType name="Tagged">
            <xs:sequence>
                <xs:element name="label" type="xs:string"/>
            </xs:sequence>
            <xs:attribute name="type" type="xs:string"/>
        </xs:complexType>
        <xs:element name="Tagged" type="t:Tagged"/>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).expect("parse XSD");
    let root = ModelSchema::from_ir(&ir, Some("Tagged")).expect("build runtime schema");

    let xml = br#"<Tagged type="gear"><label>L</label></Tagged>"#;
    let value = polyxml::deserialize(xml, Arc::clone(&root)).expect("parse as declared");
    assert_str(rec_field(&value, "type"), "gear", "content attribute");
    assert_str(rec_field(&value, "label"), "L", "label");

    let out = polyxml::serialize_with_options("Tagged", &value, &root, None, None, None)
        .expect("serialize");
    let out_str = String::from_utf8_lossy(&out);
    assert!(
        !out_str.contains("xsi:type="),
        "no selector may be invented for plain types: {out_str}"
    );
}

#[test]
fn iterparse_dispatches_each_streamed_item() {
    let root = schema_for(Some("Root"));
    let declared = declared_vehicle(&root);
    let xml = br#"<wrap xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:veh">
        <vehicle xsi:type="t:Car"><id>A</id><doors>2</doors></vehicle>
        <vehicle xsi:type="t:Truck"><id>B</id><payload>p</payload></vehicle>
        <vehicle xsi:type="t:Car"/>
    </wrap>"#;

    let mut stream = XmlItemStream::new(Cursor::new(&xml[..]), declared, b"vehicle");
    let first = stream.next_item().expect("first item").expect("item 1");
    assert_eq!(schema_local(rec_schema(&first)), "Car");
    match rec_field(&first, "doors") {
        Some(PolyValue::Int(i)) => assert_eq!(*i, 2, "streamed doors"),
        other => panic!("expected int doors, got {other:?}"),
    }

    let second = stream.next_item().expect("second item").expect("item 2");
    assert_eq!(schema_local(rec_schema(&second)), "Truck");

    let third = stream.next_item().expect("third item").expect("item 3");
    assert_eq!(schema_local(rec_schema(&third)), "Car");

    assert!(stream.next_item().expect("eof").is_none());
}

#[test]
fn abstract_type_requires_concrete_selector() {
    let root = schema_for(Some("Vehicle"));
    for xml in [
        b"<Vehicle/>".as_slice(),
        b"<Vehicle><id>X</id></Vehicle>".as_slice(),
    ] {
        let err = polyxml::deserialize(xml, Arc::clone(&root)).unwrap_err();
        assert!(err.to_string().contains("requires xsi:type"), "{err}");
    }
}

#[test]
fn nil_abstract_complex_type_skips_concrete_dispatch() {
    let root = schema_for(Some("Root"));
    for xml in [
        br#"<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><vehicle xsi:nil="true"/><spare xsi:nil="true"/></Root>"#.as_slice(),
        br#"<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><vehicle xsi:nil="true"></vehicle><spare xsi:nil="true"></spare></Root>"#.as_slice(),
    ] {
        let value = polyxml::deserialize(xml, Arc::clone(&root)).expect("nil abstract field");
        assert!(matches!(rec_field(&value, "vehicle"), Some(PolyValue::Null)));
        assert!(matches!(rec_field(&value, "spare"), Some(PolyValue::List(items)) if matches!(items.as_slice(), [PolyValue::Null])));
    }
}

#[test]
fn xsi_type_resolves_namespace_inherited_from_parent() {
    let root = schema_for(Some("Root"));
    let valid = br#"<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:veh"><vehicle xsi:type="t:Car"><id>X</id><doors>2</doors></vehicle></Root>"#;
    assert!(polyxml::deserialize(valid, Arc::clone(&root)).is_ok());
    let wrong = br#"<Root xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:other"><vehicle xsi:type="t:Car"><id>X</id><doors>2</doors></vehicle></Root>"#;
    assert!(polyxml::deserialize(wrong, Arc::clone(&root)).is_err());
    let wrong_attribute = br#"<Root xmlns:o="urn:other" xmlns:t="urn:veh"><vehicle o:type="t:Car"><id>X</id><doors>2</doors></vehicle></Root>"#;
    assert!(polyxml::deserialize(wrong_attribute, root).is_err());
}

#[test]
fn variant_serialization_requires_namespaces() {
    let root = schema_for(Some("Vehicle"));
    let xml = br#"<Vehicle xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:t="urn:veh" xsi:type="t:Car"><id>X</id><doors>2</doors></Vehicle>"#;
    let value = polyxml::deserialize(xml, Arc::clone(&root)).unwrap();
    let err = polyxml::serialize_with_options("Vehicle", &value, &root, None, Some(false), None)
        .unwrap_err();
    assert!(err.to_string().contains("requires namespaces"), "{err}");
}
