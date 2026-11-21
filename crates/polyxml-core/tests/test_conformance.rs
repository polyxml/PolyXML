use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;

use polyxml::schema::{FieldKind, FieldSchema, ModelSchema, ScalarType, ValueType};
use polyxml::value::PolyValue;
use polyxml::{deserialize, serialize_with_options, XmlItemStream};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/samples")
        .join(name)
}

#[test]
fn test_soap_envelope_conformance() {
    let path = fixture_path("soap_envelope.xml");
    let xml_bytes = fs::read(&path).expect("Failed to read soap_envelope.xml");

    let sec_token_schema = ModelSchema::builder("SecurityToken")
        .with_namespace("http://example.com/security")
        .field(
            FieldSchema::new(
                "must_understand",
                b"env:mustUnderstand",
                FieldKind::Attribute,
                ValueType::Scalar(ScalarType::Bool),
            )
            .with_namespace("http://www.w3.org/2003/05/soap-envelope"),
        )
        .field(FieldSchema::new(
            "token",
            b"token",
            FieldKind::Text,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    let header_schema = ModelSchema::builder("Header")
        .with_namespace("http://www.w3.org/2003/05/soap-envelope")
        .field(FieldSchema::new(
            "security_token",
            b"auth:SecurityToken",
            FieldKind::Element,
            ValueType::Nested(Arc::clone(&sec_token_schema)),
        ))
        .build();

    let stock_schema = ModelSchema::builder("GetStockPrice")
        .with_namespace("http://example.com/stock")
        .field(FieldSchema::new(
            "stock_name",
            b"m:StockName",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "currency",
            b"m:Currency",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "target_date",
            b"m:TargetDate",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::XmlDate),
        ))
        .build();

    let body_schema = ModelSchema::builder("Body")
        .with_namespace("http://www.w3.org/2003/05/soap-envelope")
        .field(FieldSchema::new(
            "get_stock_price",
            b"m:GetStockPrice",
            FieldKind::Element,
            ValueType::Nested(Arc::clone(&stock_schema)),
        ))
        .build();

    let envelope_schema = ModelSchema::builder("Envelope")
        .with_namespace("http://www.w3.org/2003/05/soap-envelope")
        .field(FieldSchema::new(
            "header",
            b"env:Header",
            FieldKind::Element,
            ValueType::Nested(header_schema),
        ))
        .field(FieldSchema::new(
            "body",
            b"env:Body",
            FieldKind::Element,
            ValueType::Nested(body_schema),
        ))
        .build();

    // 1. Deserialize
    let val = deserialize(&xml_bytes, Arc::clone(&envelope_schema))
        .expect("Failed to deserialize SOAP envelope");

    let header = val.get("header").unwrap();
    let sec = header.get("security_token").unwrap();
    assert_eq!(sec.get("must_understand"), Some(&PolyValue::Bool(true)));
    assert_eq!(
        sec.get("token"),
        Some(&PolyValue::String("sec-tok-987654321".into()))
    );

    let body = val.get("body").unwrap();
    let req = body.get("get_stock_price").unwrap();
    assert_eq!(
        req.get("stock_name"),
        Some(&PolyValue::String("ACME".into()))
    );
    assert_eq!(req.get("currency"), Some(&PolyValue::String("USD".into())));
    assert_eq!(
        req.get("target_date"),
        Some(&PolyValue::String("2026-09-17".into()))
    );

    // 2. Roundtrip serialize with prefixes
    let mut ns_map = HashMap::new();
    ns_map.insert(
        "http://www.w3.org/2003/05/soap-envelope".to_string(),
        "env".to_string(),
    );
    ns_map.insert("http://example.com/stock".to_string(), "m".to_string());
    ns_map.insert(
        "http://example.com/security".to_string(),
        "auth".to_string(),
    );

    let serialized = serialize_with_options(
        "Envelope",
        &val,
        &envelope_schema,
        Some(4),
        Some(true),
        Some(&ns_map),
    )
    .expect("Failed to serialize SOAP envelope");

    let serialized_str = std::str::from_utf8(&serialized).unwrap();
    assert!(serialized_str.contains("xmlns:env=\"http://www.w3.org/2003/05/soap-envelope\""));
    assert!(serialized_str.contains("<env:Envelope"));
    assert!(serialized_str.contains("<env:Header>"));
    assert!(serialized_str.contains("<env:Body>"));

    // 3. Re-deserialize to ensure semantic identity
    let val_re = deserialize(&serialized, Arc::clone(&envelope_schema))
        .expect("Failed to re-deserialize SOAP envelope");
    assert_eq!(val, val_re);
}

#[test]
fn test_atom_feed_conformance() {
    let path = fixture_path("atom_feed.xml");
    let xml_bytes = fs::read(&path).expect("Failed to read atom_feed.xml");

    let author_schema = ModelSchema::builder("author")
        .with_namespace("http://www.w3.org/2005/Atom")
        .field(FieldSchema::new(
            "name",
            b"name",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "email",
            b"email",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    let entry_schema = ModelSchema::builder("entry")
        .with_namespace("http://www.w3.org/2005/Atom")
        .field(FieldSchema::new(
            "title",
            b"title",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "updated",
            b"updated",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::XmlDateTime),
        ))
        .field(FieldSchema::new(
            "summary",
            b"summary",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "author",
            b"author",
            FieldKind::Element,
            ValueType::Nested(author_schema),
        ))
        .build();

    let feed_schema = ModelSchema::builder("feed")
        .with_namespace("http://www.w3.org/2005/Atom")
        .field(FieldSchema::new(
            "title",
            b"title",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "updated",
            b"updated",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::XmlDateTime),
        ))
        .field(FieldSchema::new(
            "entries",
            b"entry",
            FieldKind::Element,
            ValueType::List(Box::new(ValueType::Nested(entry_schema))),
        ))
        .build();

    // 1. Deserialize
    let val =
        deserialize(&xml_bytes, Arc::clone(&feed_schema)).expect("Failed to deserialize Atom feed");

    assert_eq!(
        val.get("title"),
        Some(&PolyValue::String("PolyXML Engineering Updates".into()))
    );

    if let Some(PolyValue::List(entries)) = val.get("entries") {
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].get("title"),
            Some(&PolyValue::String("Release v0.10.0".into()))
        );
        let author = entries[0].get("author").unwrap();
        assert_eq!(
            author.get("name"),
            Some(&PolyValue::String("Bailey Nguyen".into()))
        );
    } else {
        panic!("Expected entries list");
    }

    // 2. Roundtrip serialize with default namespace
    let mut ns_map = HashMap::new();
    ns_map.insert("http://www.w3.org/2005/Atom".to_string(), "".to_string());

    let serialized = serialize_with_options(
        "feed",
        &val,
        &feed_schema,
        Some(2),
        Some(true),
        Some(&ns_map),
    )
    .expect("Failed to serialize Atom feed");

    let val_re = deserialize(&serialized, Arc::clone(&feed_schema))
        .expect("Failed to re-deserialize Atom feed");
    assert_eq!(val, val_re);
}

#[test]
fn test_ubl_invoice_conformance() {
    let path = fixture_path("ubl_invoice.xml");
    let xml_bytes = fs::read(&path).expect("Failed to read ubl_invoice.xml");

    let party_schema = ModelSchema::builder("Party")
        .with_namespace("urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2")
        .field(FieldSchema::new(
            "name",
            b"cbc:RegistrationName",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "company_id",
            b"cbc:CompanyID",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .build();

    let supplier_schema = ModelSchema::builder("AccountingSupplierParty")
        .with_namespace("urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2")
        .field(FieldSchema::new(
            "party",
            b"cac:Party",
            FieldKind::Element,
            ValueType::Nested(Arc::clone(&party_schema)),
        ))
        .build();

    let total_schema = ModelSchema::builder("LegalMonetaryTotal")
        .with_namespace("urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2")
        .field(FieldSchema::new(
            "payable_amount",
            b"cbc:PayableAmount",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Float),
        ))
        .build();

    let invoice_schema = ModelSchema::builder("Invoice")
        .with_namespace("urn:oasis:names:specification:ubl:schema:xsd:Invoice-2")
        .field(FieldSchema::new(
            "id",
            b"cbc:ID",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "issue_date",
            b"cbc:IssueDate",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::XmlDate),
        ))
        .field(FieldSchema::new(
            "currency",
            b"cbc:DocumentCurrencyCode",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "supplier",
            b"cac:AccountingSupplierParty",
            FieldKind::Element,
            ValueType::Nested(supplier_schema),
        ))
        .field(FieldSchema::new(
            "totals",
            b"cac:LegalMonetaryTotal",
            FieldKind::Element,
            ValueType::Nested(total_schema),
        ))
        .build();

    // 1. Deserialize
    let val = deserialize(&xml_bytes, Arc::clone(&invoice_schema))
        .expect("Failed to deserialize UBL Invoice");

    assert_eq!(
        val.get("id"),
        Some(&PolyValue::String("INV-2026-0042".into()))
    );
    assert_eq!(
        val.get("issue_date"),
        Some(&PolyValue::String("2026-09-17".into()))
    );
    assert_eq!(val.get("currency"), Some(&PolyValue::String("USD".into())));

    let supplier = val.get("supplier").unwrap();
    let party = supplier.get("party").unwrap();
    assert_eq!(
        party.get("name"),
        Some(&PolyValue::String("PolyXML Tech Inc.".into()))
    );

    let totals = val.get("totals").unwrap();
    assert_eq!(
        totals.get("payable_amount"),
        Some(&PolyValue::Float(1620.0))
    );

    // 2. Roundtrip serialize
    let mut ns_map = HashMap::new();
    ns_map.insert(
        "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2".to_string(),
        "".to_string(),
    );
    ns_map.insert(
        "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2".to_string(),
        "cac".to_string(),
    );
    ns_map.insert(
        "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2".to_string(),
        "cbc".to_string(),
    );

    let serialized = serialize_with_options(
        "Invoice",
        &val,
        &invoice_schema,
        Some(2),
        Some(true),
        Some(&ns_map),
    )
    .expect("Failed to serialize UBL Invoice");

    let val_re = deserialize(&serialized, Arc::clone(&invoice_schema))
        .expect("Failed to re-deserialize UBL Invoice");
    assert_eq!(val, val_re);
}

#[test]
fn test_streaming_catalog_conformance() {
    let path = fixture_path("streaming_catalog.xml");
    let file = fs::File::open(&path).expect("Failed to open streaming_catalog.xml");
    let reader = BufReader::new(file);

    let item_schema = ModelSchema::builder("Item")
        .field(FieldSchema::new(
            "id",
            b"id",
            FieldKind::Attribute,
            ValueType::Scalar(ScalarType::Int),
        ))
        .field(FieldSchema::new(
            "name",
            b"Name",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::String),
        ))
        .field(FieldSchema::new(
            "price",
            b"Price",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Float),
        ))
        .field(FieldSchema::new(
            "in_stock",
            b"InStock",
            FieldKind::Element,
            ValueType::Scalar(ScalarType::Bool),
        ))
        .build();

    let mut stream = XmlItemStream::new(reader, Arc::clone(&item_schema), b"Item");
    let mut count = 0;
    let mut first_item = None;
    let mut mid_item = None;
    let mut last_item = None;

    while let Some(item) = stream.next_item().expect("Failed to stream item") {
        count += 1;
        if count == 1 {
            first_item = Some(item.clone());
        } else if count == 500 {
            mid_item = Some(item.clone());
        } else if count == 1000 {
            last_item = Some(item);
        }
    }

    assert_eq!(count, 1000);

    let item1 = first_item.unwrap();
    assert_eq!(item1.get("id"), Some(&PolyValue::Int(1)));
    assert_eq!(
        item1.get("name"),
        Some(&PolyValue::String("Widget #1".into()))
    );
    assert_eq!(item1.get("price"), Some(&PolyValue::Float(1.50)));
    assert_eq!(item1.get("in_stock"), Some(&PolyValue::Bool(false)));

    let item500 = mid_item.unwrap();
    assert_eq!(item500.get("id"), Some(&PolyValue::Int(500)));
    assert_eq!(
        item500.get("name"),
        Some(&PolyValue::String("Widget #500".into()))
    );
    assert_eq!(item500.get("price"), Some(&PolyValue::Float(750.0)));
    assert_eq!(item500.get("in_stock"), Some(&PolyValue::Bool(true)));

    let item1000 = last_item.unwrap();
    assert_eq!(item1000.get("id"), Some(&PolyValue::Int(1000)));
    assert_eq!(
        item1000.get("name"),
        Some(&PolyValue::String("Widget #1000".into()))
    );
    assert_eq!(item1000.get("price"), Some(&PolyValue::Float(1500.0)));
    assert_eq!(item1000.get("in_stock"), Some(&PolyValue::Bool(true)));
}
