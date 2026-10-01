use std::fs;
use std::process::Command;
use tempfile::tempdir;

use polyxml::codegen::go::{
    to_go_constant_name, to_go_field_name, to_go_package_name, to_go_type_name, GoBackend,
    GoCodegen, GoOptions,
};
use polyxml::ir::{
    Cardinality, EnumDef, EnumValue, FieldDef, FieldKind, PrimitiveType, QName, RestrictionFacets,
    SchemaIR, StructDef, TypeDef, TypeRef, UnionBranch, UnionDef,
};
use polyxml::schema_parser::XsdParser;

#[test]
fn referenced_element_uses_declared_type_after_includes() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("base.xsd"),
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test" xmlns:t="urn:test">
        <xs:complexType name="Base"><xs:sequence><xs:element ref="t:description"/></xs:sequence></xs:complexType>
        <xs:element name="description" type="xs:string"/>
        <xs:element name="day" type="xs:gDay"/>
        </xs:schema>"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("main.xsd"),
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test" xmlns:t="urn:test">
        <xs:include schemaLocation="base.xsd"/>
        <xs:complexType name="Derived"><xs:complexContent><xs:extension base="t:Base"/></xs:complexContent></xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();
    let ir = XsdParser::new()
        .parse_file(dir.path().join("main.xsd"))
        .unwrap();
    let base = match ir.types.get(&QName::new(Some("urn:test"), "Base")).unwrap() {
        TypeDef::Struct(value) => value,
        _ => panic!("expected struct"),
    };
    assert_eq!(
        base.fields[0].type_ref,
        TypeRef::Primitive(PrimitiveType::String)
    );
    let go = GoCodegen::new(GoOptions::default()).generate_module(&ir);
    assert!(go.contains("type GDay string"));
    assert!(!go.contains("type GDay GDay"));
}

#[test]
fn nested_lexical_unions_compile_and_round_trip_in_go() {
    if Command::new("go").arg("version").output().is_err() {
        return;
    }
    let schema = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test">
      <xs:simpleType name="Inner"><xs:union memberTypes="xs:int xs:string"/></xs:simpleType>
      <xs:simpleType name="Outer"><xs:union memberTypes="Inner xs:boolean"/></xs:simpleType>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(schema).unwrap();
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("go.mod"), "module nested\n\ngo 1.22\n").unwrap();
    fs::write(
        dir.path().join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(
        dir.path().join("models_test.go"),
        r#"package models
import ("encoding/xml"; "testing")
func TestNestedUnion(t *testing.T) {
    var value Outer
    if err := xml.Unmarshal([]byte("<value>42</value>"), &value); err != nil { t.Fatal(err) }
    if value.InnerValue == nil || value.InnerValue.IntValue == nil { t.Fatalf("wrong branch: %+v", value) }
    data, err := xml.Marshal(value)
    if err != nil { t.Fatal(err) }
    if string(data) != "<Outer>42</Outer>" { t.Fatalf("wrong XML: %s", data) }
}"#,
    )
    .unwrap();
    let result = Command::new("go")
        .args(["test", "./..."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn root_alias_names_are_unique_after_go_normalization() {
    let schema = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test">
      <xs:complexType name="FirstType"/>
      <xs:complexType name="SecondType"/>
      <xs:element name="item" type="FirstType"/>
      <xs:element name="item_" type="SecondType"/>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(schema).unwrap();
    let code = GoCodegen::new(GoOptions::default()).generate_module(&ir);
    assert_eq!(code.matches("type Item = ").count(), 1);
}

#[test]
fn test_go_sanitization() {
    assert_eq!(to_go_field_name("id"), "ID");
    assert_eq!(to_go_field_name("url"), "URL");
    assert_eq!(to_go_field_name("uri"), "URI");
    assert_eq!(to_go_field_name("xml_name"), "XMLName");
    assert_eq!(to_go_field_name("customer_id"), "CustomerID");
    assert_eq!(to_go_field_name("123_count"), "Field123Count");
    assert_eq!(to_go_field_name("default"), "Default");

    assert_eq!(to_go_type_name("customer_record"), "CustomerRecord");
    assert_eq!(to_go_type_name("123_type"), "Type123Type");
    assert_eq!(to_go_type_name("order_id"), "OrderID");

    assert_eq!(
        to_go_constant_name("OrderStatus", "pending"),
        "OrderStatusPending"
    );
    assert_eq!(
        to_go_constant_name("OrderStatus", "in-progress"),
        "OrderStatusInProgress"
    );
    assert_eq!(to_go_constant_name("Payment", "10_days"), "PaymentV10Days");

    assert_eq!(to_go_package_name("com.example.crm"), "comexamplecrm");
    assert_eq!(to_go_package_name("my_models"), "mymodels");
    assert_eq!(to_go_package_name("123_pkg"), "pkg123pkg");
}

#[test]
fn test_go_struct_and_enum_generation() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/crm");

    // Enum: OrderStatus
    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/crm"), "OrderStatus"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "pending".into(),
                value: "pending".into(),
                documentation: Some("Pending review".into()),
            },
            EnumValue {
                name: "shipped".into(),
                value: "shipped".into(),
                documentation: None,
            },
            EnumValue {
                name: "cancelled".into(),
                value: "cancelled".into(),
                documentation: None,
            },
        ],
        documentation: Some("Status of order processing".into()),
    }));

    // Struct: Customer
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/crm"), "Customer"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "id".into(),
                xml_name: "id".into(),
                namespace: None,
                kind: FieldKind::Attribute,
                type_ref: TypeRef::Primitive(PrimitiveType::Int),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "name".into(),
                xml_name: "name".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: Some(RestrictionFacets {
                    min_length: Some(1),
                    max_length: Some(100),
                    ..Default::default()
                }),
                is_cycle_cut: false,
            },
            FieldDef {
                name: "email".into(),
                xml_name: "email".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "tags".into(),
                xml_name: "tag".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::unbounded(0),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "status".into(),
                xml_name: "status".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(
                    Some("https://example.com/crm"),
                    "OrderStatus",
                )),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: Some("Customer record definition".into()),
    }));

    let options = GoOptions {
        backend: GoBackend::Standard,
        package_name: "crm".to_string(),
        emit_xml_tags: true,
        emit_json_tags: true,
        validate_choice_exclusivity: true,
        validate_facets: true,
        emit_root_aliases: true,
        custom_header: None,
    };

    let codegen = GoCodegen::new(options);
    let go_code = codegen.generate_module(&ir);

    assert!(go_code.contains("package crm"));
    assert!(go_code.contains("type OrderStatus string"));
    assert!(go_code.contains("OrderStatusPending OrderStatus = \"pending\""));
    assert!(go_code.contains("func (e OrderStatus) IsValid() bool"));
    assert!(go_code.contains("type Customer struct {"));
    assert!(go_code.contains("XMLName xml.Name `json:\"-\"`"));
    assert!(go_code.contains("ID int32 `xml:\"id,attr\" json:\"id\"`"));
    assert!(go_code.contains("Name string `xml:\"name\" json:\"name\"`"));
    assert!(go_code.contains("Email *string `xml:\"email,omitempty\" json:\"email,omitempty\"`"));
    assert!(go_code.contains("Tags []string `xml:\"tag\" json:\"tag\"`"));
    assert!(go_code.contains("Status OrderStatus `xml:\"status\" json:\"status\"`"));

    // Verify Go compilation and test execution
    let temp = tempdir().unwrap();
    let mod_path = temp.path().join("models.go");
    fs::write(&mod_path, &go_code).unwrap();

    let test_path = temp.path().join("models_test.go");
    fs::write(
        &test_path,
        r#"package crm

import (
    "encoding/json"
    "encoding/xml"
    "strings"
    "testing"
)

func TestCustomerRoundtrip(t *testing.T) {
    email := "alice@example.com"
    c := Customer{
        ID:     42,
        Name:   "Alice",
        Email:  &email,
        Tags:   []string{"vip", "retail"},
        Status: OrderStatusPending,
    }

    if !c.Status.IsValid() {
        t.Fatalf("expected status to be valid")
    }

    // 1. XML Roundtrip
    xmlData, err := xml.Marshal(c)
    if err != nil {
        t.Fatalf("xml marshal failed: %v", err)
    }

    var xmlDecoded Customer
    if err := xml.Unmarshal(xmlData, &xmlDecoded); err != nil {
        t.Fatalf("xml unmarshal failed: %v", err)
    }

    if xmlDecoded.ID != 42 || xmlDecoded.Name != "Alice" || xmlDecoded.Email == nil || *xmlDecoded.Email != "alice@example.com" {
        t.Fatalf("xml roundtrip mismatch: %+v", xmlDecoded)
    }
    if len(xmlDecoded.Tags) != 2 || xmlDecoded.Tags[0] != "vip" {
        t.Fatalf("xml tags mismatch: %+v", xmlDecoded.Tags)
    }

    if err := xmlDecoded.Validate(); err != nil {
        t.Fatalf("validation failed: %v", err)
    }

    // 2. JSON Roundtrip
    jsonData, err := json.Marshal(c)
    if err != nil {
        t.Fatalf("json marshal failed: %v", err)
    }

    jsonStr := string(jsonData)
    if strings.Contains(jsonStr, "XMLName") {
        t.Fatalf("json contains XMLName: %s", jsonStr)
    }
    if !strings.Contains(jsonStr, `"id":42`) {
        t.Fatalf("json missing id: %s", jsonStr)
    }

    var jsonDecoded Customer
    if err := json.Unmarshal(jsonData, &jsonDecoded); err != nil {
        t.Fatalf("json unmarshal failed: %v", err)
    }

    if jsonDecoded.ID != 42 || jsonDecoded.Name != "Alice" || jsonDecoded.Email == nil || *jsonDecoded.Email != "alice@example.com" {
        t.Fatalf("json roundtrip mismatch: %+v", jsonDecoded)
    }
}
"#,
    )
    .unwrap();

    // Init go module
    let init_status = Command::new("go")
        .args(["mod", "init", "crm"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to init go module");
    assert!(init_status.success(), "go mod init failed");

    // Run go test
    let test_status = Command::new("go")
        .args(["test", "-v", "."])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run go test");
    assert!(test_status.success(), "go test failed on generated models");
}

#[test]
fn test_go_choice_mutual_exclusivity() {
    let mut ir = SchemaIR::new().with_target_namespace("urn:payments");

    ir.add_type(TypeDef::Union(UnionDef {
        qname: QName::new(Some("urn:payments"), "ContactChoice"),
        branches: vec![
            UnionBranch {
                variant_name: "email".into(),
                xml_name: "email".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: None,
            },
            UnionBranch {
                variant_name: "phone".into(),
                xml_name: "phone".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: None,
            },
        ],
        documentation: Some("Sum type representing email or phone".into()),
    }));

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:payments"), "PaymentParty"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "name".into(),
                xml_name: "name".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "contact".into(),
                xml_name: "contact".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(Some("urn:payments"), "ContactChoice")),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: None,
    }));

    let options = GoOptions {
        backend: GoBackend::Standard,
        package_name: "payments".to_string(),
        emit_xml_tags: true,
        emit_json_tags: true,
        validate_choice_exclusivity: true,
        validate_facets: true,
        emit_root_aliases: true,
        custom_header: None,
    };

    let codegen = GoCodegen::new(options);
    let go_code = codegen.generate_module(&ir);

    assert!(go_code.contains("func (c *ContactChoice) UnmarshalXML("));
    assert!(go_code.contains("func (c ContactChoice) MarshalXML("));
    assert!(go_code.contains("func (c ContactChoice) Selected() string"));
    assert!(go_code.contains("func (c ContactChoice) Validate() error"));

    let temp = tempdir().unwrap();
    let mod_path = temp.path().join("payments.go");
    fs::write(&mod_path, &go_code).unwrap();

    let test_path = temp.path().join("payments_test.go");
    fs::write(
        &test_path,
        r#"package payments

import (
    "encoding/xml"
    "testing"
)

func TestChoiceMutualExclusivity(t *testing.T) {
    // 1. Valid: single email branch
    validXmlEmail := `<PaymentParty xmlns="urn:payments"><name>Alice</name><contact><email>alice@example.com</email></contact></PaymentParty>`
    var p1 PaymentParty
    if err := xml.Unmarshal([]byte(validXmlEmail), &p1); err != nil {
        t.Fatalf("unexpected unmarshal error: %v", err)
    }
    if p1.Contact.Email == nil || *p1.Contact.Email != "alice@example.com" {
        t.Fatalf("expected email branch to be populated")
    }
    if p1.Contact.Phone != nil {
        t.Fatalf("phone branch should be nil")
    }
    if p1.Contact.Selected() != "email" {
        t.Fatalf("expected Selected() to return email, got %s", p1.Contact.Selected())
    }

    // 2. Valid: single phone branch
    validXmlPhone := `<PaymentParty xmlns="urn:payments"><name>Bob</name><contact><phone>+123456789</phone></contact></PaymentParty>`
    var p2 PaymentParty
    if err := xml.Unmarshal([]byte(validXmlPhone), &p2); err != nil {
        t.Fatalf("unexpected unmarshal error: %v", err)
    }
    if p2.Contact.Phone == nil || *p2.Contact.Phone != "+123456789" {
        t.Fatalf("expected phone branch to be populated")
    }
    if p2.Contact.Selected() != "phone" {
        t.Fatalf("expected Selected() to return phone, got %s", p2.Contact.Selected())
    }

    // 3. Invalid: both email and phone populated -> MUST FAIL UnmarshalXML
    invalidXmlBoth := `<PaymentParty xmlns="urn:payments"><name>Eve</name><contact><email>eve@example.com</email><phone>999</phone></contact></PaymentParty>`
    var p3 PaymentParty
    if err := xml.Unmarshal([]byte(invalidXmlBoth), &p3); err == nil {
        t.Fatalf("expected mutual exclusivity error, but unmarshal succeeded")
    } else {
        t.Logf("Correctly rejected concurrent choice branches: %v", err)
    }
}
"#,
    )
    .unwrap();

    let init_status = Command::new("go")
        .args(["mod", "init", "payments"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to init go module");
    assert!(init_status.success());

    let test_status = Command::new("go")
        .args(["test", "-v", "."])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run go test");
    assert!(test_status.success(), "go test failed on choice validation");
}

#[test]
fn test_go_recursive_cycle_pointers() {
    let mut ir = SchemaIR::new().with_target_namespace("urn:tree");

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:tree"), "TreeNode"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "label".into(),
                xml_name: "label".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "next".into(),
                xml_name: "next".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(Some("urn:tree"), "TreeNode")),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: true, // Tarjan cycle cut -> *TreeNode
            },
        ],
        documentation: None,
    }));

    let options = GoOptions {
        package_name: "tree".to_string(),
        emit_xml_tags: true,
        ..Default::default()
    };

    let codegen = GoCodegen::new(options);
    let go_code = codegen.generate_module(&ir);

    assert!(go_code.contains("Next *TreeNode `xml:\"next,omitempty\" json:\"next,omitempty\"`"));

    let temp = tempdir().unwrap();
    let mod_path = temp.path().join("tree.go");
    fs::write(&mod_path, &go_code).unwrap();

    let test_path = temp.path().join("tree_test.go");
    fs::write(
        &test_path,
        r#"package tree

import (
    "encoding/xml"
    "testing"
)

func TestRecursiveTree(t *testing.T) {
    root := TreeNode{
        Label: "root",
        Next: &TreeNode{
            Label: "child1",
            Next: &TreeNode{
                Label: "child2",
            },
        },
    }

    data, err := xml.Marshal(root)
    if err != nil {
        t.Fatalf("marshal failed: %v", err)
    }

    var decoded TreeNode
    if err := xml.Unmarshal(data, &decoded); err != nil {
        t.Fatalf("unmarshal failed: %v", err)
    }

    if decoded.Label != "root" || decoded.Next == nil || decoded.Next.Label != "child1" {
        t.Fatalf("tree decoding mismatch: %+v", decoded)
    }
    if decoded.Next.Next == nil || decoded.Next.Next.Label != "child2" {
        t.Fatalf("deep tree child mismatch: %+v", decoded.Next.Next)
    }
}
"#,
    )
    .unwrap();

    let init_status = Command::new("go")
        .args(["mod", "init", "tree"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to init go module");
    assert!(init_status.success());

    let test_status = Command::new("go")
        .args(["test", "-v", "."])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run go test");
    assert!(test_status.success(), "go test failed on recursive tree");
}

#[test]
fn test_go_backend_from_str_loose() {
    assert_eq!(
        GoBackend::from_str_loose("standard"),
        Some(GoBackend::Standard)
    );
    assert_eq!(GoBackend::from_str_loose("std"), Some(GoBackend::Standard));
    assert_eq!(
        GoBackend::from_str_loose("default"),
        Some(GoBackend::Standard)
    );
    assert_eq!(
        GoBackend::from_str_loose("easyjson"),
        Some(GoBackend::EasyJson)
    );
    assert_eq!(
        GoBackend::from_str_loose("easy-json"),
        Some(GoBackend::EasyJson)
    );
    assert_eq!(
        GoBackend::from_str_loose("easy_json"),
        Some(GoBackend::EasyJson)
    );
    assert_eq!(GoBackend::from_str_loose("sonic"), Some(GoBackend::Sonic));
    assert_eq!(
        GoBackend::from_str_loose("bytedance"),
        Some(GoBackend::Sonic)
    );
    assert_eq!(GoBackend::from_str_loose("unknown"), None);
}

#[test]
fn test_go_easyjson_backend() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/easy");

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/easy"), "Payload"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef {
            name: "id".into(),
            xml_name: "id".into(),
            namespace: None,
            kind: FieldKind::Element,
            type_ref: TypeRef::Primitive(PrimitiveType::Int),
            cardinality: Cardinality::required_one(),
            nillable: false,
            default_value: None,
            fixed_value: None,
            documentation: None,
            facets: None,
            is_cycle_cut: false,
        }],
        documentation: None,
    }));

    let options = GoOptions {
        backend: GoBackend::EasyJson,
        package_name: "easy".to_string(),
        ..Default::default()
    };
    let codegen = GoCodegen::new(options);
    let code = codegen.generate_module(&ir);

    assert!(code.contains("//easyjson:json"));
    assert!(code.contains("type Payload struct {"));
}

#[test]
fn test_go_sonic_backend() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/sonic");

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/sonic"), "Metric"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "cpuUsage".into(),
                xml_name: "cpuUsage".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::Double),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "notes".into(),
                xml_name: "notes".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: None,
    }));

    let options = GoOptions {
        backend: GoBackend::Sonic,
        package_name: "metrics".to_string(),
        emit_xml_tags: true,
        emit_json_tags: true,
        ..Default::default()
    };
    let codegen = GoCodegen::new(options);
    let code = codegen.generate_module(&ir);

    assert!(code.contains("XMLName xml.Name `json:\"-\" sonic:\"-\"`"));
    assert!(
        code.contains("CpuUsage float64 `xml:\"cpuUsage\" json:\"cpuUsage\" sonic:\"cpuUsage\"`")
    );
    assert!(code.contains("Notes *string `xml:\"notes,omitempty\" json:\"notes,omitempty\" sonic:\"notes,omitempty\"`"));
}

#[test]
fn test_go_enum_only_schema_omits_unused_xml_import() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="YesNo"><xs:restriction base="xs:string">
        <xs:enumeration value="YES"/><xs:enumeration value="yes"/>
        <xs:enumeration value="NO"/><xs:enumeration value="no"/>
      </xs:restriction></xs:simpleType>
      <xs:element name="Root" type="YesNo"/>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");
    let codegen = GoCodegen::new(GoOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(
        !code.contains("encoding/xml"),
        "Enum-only schema must not import unused encoding/xml"
    );
    assert!(code.contains("type YesNo string"));
    assert!(code.contains("YesNoYes YesNo = \"YES\""));
}

#[test]
fn test_go_date_or_datetime_lexical_union() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="DateOrDateTime"><xs:union memberTypes="xs:date xs:dateTime"/></xs:simpleType>
      <xs:element name="Root"><xs:complexType><xs:sequence>
        <xs:element name="When" type="DateOrDateTime"/>
      </xs:sequence></xs:complexType></xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");
    let codegen = GoCodegen::new(GoOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("time.Parse(time.RFC3339, value)"));
    assert!(code.contains("time.Parse(\"2006-01-02\", value)"));
    assert!(code.contains("c.DateTimeValue.Format(time.RFC3339)"));
    assert!(code.contains("c.DateValue.Format(\"2006-01-02\")"));
}

#[test]
fn test_go_any_attribute_codegen() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:element name="Extensible">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="name" type="xs:string"/>
          </xs:sequence>
          <xs:anyAttribute processContents="lax"/>
        </xs:complexType>
      </xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");
    let codegen = GoCodegen::new(GoOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("AnyAttribute []xml.Attr `xml:\",any,attr\" json:\"-\"`"));
}

#[test]
fn test_go_abstract_xsi_type_dispatch() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:audit:abstract" xmlns:t="urn:audit:abstract" elementFormDefault="qualified">
      <xs:complexType name="AbstractDocument" abstract="true">
        <xs:sequence>
          <xs:element name="Id" type="xs:string"/>
        </xs:sequence>
      </xs:complexType>

      <xs:complexType name="InvoiceDocument">
        <xs:complexContent>
          <xs:extension base="t:AbstractDocument">
            <xs:sequence>
              <xs:element name="Amount" type="xs:decimal"/>
            </xs:sequence>
          </xs:extension>
        </xs:complexContent>
      </xs:complexType>

      <xs:complexType name="ReceiptDocument">
        <xs:complexContent>
          <xs:extension base="t:AbstractDocument">
            <xs:sequence>
              <xs:element name="Store" type="xs:string"/>
            </xs:sequence>
          </xs:extension>
        </xs:complexContent>
      </xs:complexType>

      <xs:element name="Document" type="t:AbstractDocument"/>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");
    let codegen = GoCodegen::new(GoOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("type AbstractDocumentBase struct"));
    assert!(code.contains("type AbstractDocument struct"));
    assert!(code.contains("InvoiceDocument *InvoiceDocument"));
    assert!(code.contains("ReceiptDocument *ReceiptDocument"));
    assert!(code.contains("func (s AbstractDocument) Selected() string"));
    assert!(code.contains("func (s AbstractDocument) Value() any"));
    assert!(code.contains("func (s *AbstractDocument) UnmarshalXML"));
    assert!(code.contains("func (s AbstractDocument) MarshalXML"));

    if Command::new("go").arg("version").output().is_err() {
        return;
    }

    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("go.mod"),
        "module abstract_test\n\ngo 1.22\n",
    )
    .unwrap();
    fs::write(dir.path().join("models.go"), &code).unwrap();
    fs::write(
        dir.path().join("models_test.go"),
        r#"package models

import (
	"encoding/xml"
	"strings"
	"testing"
)

func TestAbstractDocumentRoundTrip(t *testing.T) {
	inputXml := `<Document xmlns="urn:audit:abstract" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="InvoiceDocument"><Id>INV-001</Id><Amount>199.99</Amount></Document>`
	var doc Document
	if err := xml.Unmarshal([]byte(inputXml), &doc); err != nil {
		t.Fatalf("Unmarshal failed: %v", err)
	}

	if doc.Selected() != "InvoiceDocument" {
		t.Fatalf("expected Selected() to be InvoiceDocument, got %q", doc.Selected())
	}
	if doc.InvoiceDocument == nil {
		t.Fatalf("InvoiceDocument is nil")
	}
	if doc.InvoiceDocument.ID != "INV-001" {
		t.Fatalf("expected ID 'INV-001', got %q", doc.InvoiceDocument.ID)
	}
	if doc.InvoiceDocument.Amount != 199.99 {
		t.Fatalf("expected Amount 199.99, got %v", doc.InvoiceDocument.Amount)
	}

	marshaled, err := xml.Marshal(&doc)
	if err != nil {
		t.Fatalf("Marshal failed: %v", err)
	}
	marshaledStr := string(marshaled)
	if !strings.Contains(marshaledStr, `xsi:type="InvoiceDocument"`) {
		t.Fatalf("marshaled XML missing xsi:type: %s", marshaledStr)
	}
	if !strings.Contains(marshaledStr, `Amount`) {
		t.Fatalf("marshaled XML missing Amount element: %s", marshaledStr)
	}

	// Missing xsi:type should fail
	var missingDoc Document
	if err := xml.Unmarshal([]byte(`<Document xmlns="urn:audit:abstract"><Id>INV-001</Id></Document>`), &missingDoc); err == nil {
		t.Fatalf("expected error unmarshaling abstract type without xsi:type, got nil")
	}

	// Unknown xsi:type should fail
	var unknownDoc Document
	if err := xml.Unmarshal([]byte(`<Document xmlns="urn:audit:abstract" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="UnknownDoc"><Id>INV-001</Id></Document>`), &unknownDoc); err == nil {
		t.Fatalf("expected error unmarshaling unknown xsi:type, got nil")
	}
}
"#,
    )
    .unwrap();

    let output = Command::new("go")
        .args(["test", "-v", "./..."])
        .current_dir(dir.path())
        .output()
        .expect("go test failed to execute");

    assert!(
        output.status.success(),
        "go test failed: stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
