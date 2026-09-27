use std::fs;
use std::process::Command;
use tempfile::tempdir;

use polyxml::codegen::cpp::{
    to_cpp_enum_variant, to_cpp_field_name, to_cpp_namespace, to_cpp_type_name, CppBackend,
    CppCodegen, CppMode, CppOptions,
};
use polyxml::ir::{
    Cardinality, EnumDef, EnumValue, FieldDef, FieldKind, PrimitiveType, QName, RestrictionFacets,
    SchemaIR, StructDef, TypeDef, TypeRef, UnionBranch, UnionDef,
};

#[test]
fn test_cpp_sanitization() {
    assert_eq!(to_cpp_field_name("class"), "class_");
    assert_eq!(to_cpp_field_name("default"), "default_");
    assert_eq!(to_cpp_field_name("switch"), "switch_");
    assert_eq!(to_cpp_field_name("template"), "template_");
    assert_eq!(to_cpp_field_name("123_test"), "_123_test");
    assert_eq!(to_cpp_field_name("customer_id"), "customer_id");

    assert_eq!(to_cpp_type_name("customer_record"), "CustomerRecord");
    assert_eq!(to_cpp_type_name("123_test"), "Type_123Test");
    assert_eq!(to_cpp_type_name("class"), "Class");
    assert_eq!(to_cpp_type_name("struct"), "Struct");

    assert_eq!(to_cpp_enum_variant("pending"), "Pending");
    assert_eq!(to_cpp_enum_variant("in-progress"), "InProgress");
    assert_eq!(to_cpp_enum_variant("10_days"), "V10Days");
    assert_eq!(to_cpp_enum_variant(""), "Unknown");

    assert_eq!(to_cpp_namespace("com.example.crm"), "com::example::crm");
    assert_eq!(to_cpp_namespace("polyxml::generated"), "polyxml::generated");
    assert_eq!(to_cpp_namespace("123_crm.models"), "_123_crm::models");
    assert_eq!(to_cpp_namespace(""), "polyxml::generated");
}

#[test]
fn test_cpp_struct_and_enum_codegen_compilation() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/crm");

    // Enum: OrderStatus
    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/crm"), "OrderStatus"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "pending".into(),
                value: "pending".into(),
                documentation: Some("Order pending payment".into()),
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
        documentation: Some("Order status enum".into()),
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
                kind: FieldKind::Element,
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
                xml_name: "tags".into(),
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

    let options = CppOptions {
        namespace: "crm::models".to_string(),
        mode: CppMode::HeaderOnly,
        backend: CppBackend::Standard,
        standard: "c++20".to_string(),
        emit_equality_operators: true,
        emit_enum_converters: true,
        validate_facets: true,
        emit_root_aliases: true,
        emit_cmake: false,
        emit_meson: false,
        custom_header: None,
    };

    let codegen = CppCodegen::new(options);
    let header_code = codegen.generate_header(&ir);

    let temp = tempdir().unwrap();
    let header_path = temp.path().join("customer.hpp");
    fs::write(&header_path, &header_code).unwrap();

    let main_cpp = temp.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"
#include "customer.hpp"
#include <cassert>
#include <iostream>

int main() {
    using namespace crm::models;

    // Test enum converters
    assert(to_string(OrderStatus::Pending) == "pending");
    assert(to_string(OrderStatus::Shipped) == "shipped");
    auto parsed = order_status_from_string("cancelled");
    assert(parsed.has_value());
    assert(*parsed == OrderStatus::Cancelled);

    // Test C++20 aggregate initialization with designated initializers
    Customer c1{
        .id = 42,
        .name = "Alice",
        .email = "alice@example.com",
        .tags = {"vip", "retail"},
        .status = OrderStatus::Pending
    };

    Customer c2{
        .id = 42,
        .name = "Alice",
        .email = "alice@example.com",
        .tags = {"vip", "retail"},
        .status = OrderStatus::Pending
    };

    Customer c3{
        .id = 43,
        .name = "Bob",
        .email = std::nullopt,
        .tags = {},
        .status = OrderStatus::Shipped
    };

    // Test C++20 defaulted operator==
    assert(c1 == c2);
    assert(!(c1 == c3));

    // Test constraint validator
    assert(c1.validate() == true);

    Customer invalid_name{
        .id = 1,
        .name = "", // Fails min_length = 1
        .email = std::nullopt,
        .tags = {},
        .status = OrderStatus::Pending
    };
    assert(invalid_name.validate() == false);

    std::cout << "All C++20 tests passed successfully." << std::endl;
    return 0;
}
"#,
    )
    .unwrap();

    let out_bin = temp.path().join("test_app");
    let compile_status = Command::new("g++")
        .args([
            "-std=c++20",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
            "-I",
            temp.path().to_str().unwrap(),
            main_cpp.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute g++");

    assert!(compile_status.success(), "g++ compilation failed");

    let run_status = Command::new(&out_bin)
        .status()
        .expect("Failed to run test binary");
    assert!(run_status.success(), "C++ test binary execution failed");
}

#[test]
fn test_cpp_choice_variant_compilation() {
    let mut ir = SchemaIR::new().with_target_namespace("urn:payments");

    // Choice: ContactChoice
    ir.add_type(TypeDef::Union(UnionDef {
        qname: QName::new(Some("urn:payments"), "ContactChoice"),
        branches: vec![
            UnionBranch {
                variant_name: "email".into(),
                xml_name: "email".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: Some("Email address".into()),
            },
            UnionBranch {
                variant_name: "phone".into(),
                xml_name: "phone".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: Some("Phone number".into()),
            },
        ],
        documentation: Some("Contact choice sum type".into()),
    }));

    // Struct: Payer
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:payments"), "Payer"),
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

    let options = CppOptions {
        namespace: "payments".to_string(),
        ..Default::default()
    };

    let codegen = CppCodegen::new(options);
    let header_code = codegen.generate_header(&ir);

    let temp = tempdir().unwrap();
    let header_path = temp.path().join("payments.hpp");
    fs::write(&header_path, &header_code).unwrap();

    let main_cpp = temp.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"
#include "payments.hpp"
#include <cassert>
#include <iostream>

int main() {
    using namespace payments;

    Payer p1{
        .name = "Alice",
        .contact = ContactChoiceEmail{.value = "alice@example.com"}
    };

    Payer p2{
        .name = "Bob",
        .contact = ContactChoicePhone{.value = "+1-555-0199"}
    };

    assert(p1.contact.index() == 0);
    assert(p2.contact.index() == 1);

    // Test C++20 pattern matching via overloaded visitor
    bool email_matched = false;
    std::visit(overloaded {
        [&](const ContactChoiceEmail& e) {
            if (e.value == "alice@example.com") {
                email_matched = true;
            }
        },
        [&](const ContactChoicePhone&) {
            assert(false);
        }
    }, p1.contact);
    assert(email_matched);

    bool phone_matched = false;
    std::visit(overloaded {
        [&](const ContactChoiceEmail&) {
            assert(false);
        },
        [&](const ContactChoicePhone& p) {
            if (p.value == "+1-555-0199") {
                phone_matched = true;
            }
        }
    }, p2.contact);
    assert(phone_matched);

    std::cout << "Choice pattern matching passed." << std::endl;
    return 0;
}
"#,
    )
    .unwrap();

    let out_bin = temp.path().join("choice_app");
    let compile_status = Command::new("g++")
        .args([
            "-std=c++20",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
            "-I",
            temp.path().to_str().unwrap(),
            main_cpp.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute g++");

    assert!(compile_status.success(), "g++ compilation failed");

    let run_status = Command::new(&out_bin)
        .status()
        .expect("Failed to run test binary");
    assert!(run_status.success(), "Choice test binary execution failed");
}

#[test]
fn test_cpp_recursive_cycle_unique_ptr() {
    let mut ir = SchemaIR::new().with_target_namespace("urn:tree");

    // Self-recursive Struct: Node
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:tree"), "Node"),
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
                type_ref: TypeRef::Named(QName::new(Some("urn:tree"), "Node")),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: true, // Tarjan cycle cut -> std::unique_ptr<Node>
            },
        ],
        documentation: None,
    }));

    let options = CppOptions {
        namespace: "tree".to_string(),
        ..Default::default()
    };

    let codegen = CppCodegen::new(options);
    let header_code = codegen.generate_header(&ir);

    let temp = tempdir().unwrap();
    let header_path = temp.path().join("tree.hpp");
    fs::write(&header_path, &header_code).unwrap();

    let main_cpp = temp.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"
#include "tree.hpp"
#include <cassert>
#include <iostream>

int main() {
    using namespace tree;

    Node head{
        .label = "root",
        .next = std::make_unique<Node>(Node{
            .label = "child1",
            .next = std::make_unique<Node>(Node{
                .label = "child2",
                .next = nullptr
            })
        })
    };

    assert(head.label == "root");
    assert(head.next != nullptr);
    assert(head.next->label == "child1");
    assert(head.next->next != nullptr);
    assert(head.next->next->label == "child2");
    assert(head.next->next->next == nullptr);

    std::cout << "Recursive tree unique_ptr passed." << std::endl;
    return 0;
}
"#,
    )
    .unwrap();

    let out_bin = temp.path().join("tree_app");
    let compile_status = Command::new("g++")
        .args([
            "-std=c++20",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
            "-I",
            temp.path().to_str().unwrap(),
            main_cpp.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute g++");

    assert!(compile_status.success(), "g++ compilation failed");

    let run_status = Command::new(&out_bin)
        .status()
        .expect("Failed to run test binary");
    assert!(run_status.success(), "Tree test binary execution failed");
}

#[test]
fn test_cpp_modules_compilation() {
    let mut ir = SchemaIR::new().with_target_namespace("urn:calc");

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:calc"), "Operation"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "op_name".into(),
                xml_name: "opName".into(),
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
                name: "value".into(),
                xml_name: "value".into(),
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
        ],
        documentation: None,
    }));

    let options = CppOptions {
        namespace: "calc::models".to_string(),
        mode: CppMode::Module,
        ..Default::default()
    };

    let codegen = CppCodegen::new(options);
    let module_code = codegen.generate_module(&ir);

    assert!(module_code.contains("export module calc.models;"));
    assert!(module_code.contains("export namespace calc::models {"));
    assert!(module_code.contains("struct Operation {"));

    let temp = tempdir().unwrap();
    let mod_file = temp.path().join("calc.cppm");
    fs::write(&mod_file, &module_code).unwrap();

    let compile_status = Command::new("g++")
        .args([
            "-std=c++20",
            "-fmodules-ts",
            "-c",
            mod_file.to_str().unwrap(),
            "-o",
            temp.path().join("calc.o").to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute g++ module compile");

    assert!(
        compile_status.success(),
        "g++ C++20 module compilation failed"
    );
}

#[test]
fn test_cpp_cmake_and_meson_generation() {
    let ir = SchemaIR::new().with_target_namespace("urn:finance");

    let options = CppOptions {
        namespace: "finance".to_string(),
        emit_cmake: true,
        emit_meson: true,
        ..Default::default()
    };

    let codegen = CppCodegen::new(options);
    let files = codegen.generate_files(&ir, "finance_models");

    let file_map: std::collections::HashMap<_, _> = files.into_iter().collect();

    assert!(file_map.contains_key("finance_models.hpp"));
    assert!(file_map.contains_key("CMakeLists.txt"));
    assert!(file_map.contains_key("PolyXMLConfig.cmake"));
    assert!(file_map.contains_key("meson.build"));

    let cmake_lists = file_map.get("CMakeLists.txt").unwrap();
    assert!(cmake_lists.contains("project(finance_models_models LANGUAGES CXX)"));
    assert!(cmake_lists.contains("add_library(finance_models INTERFACE)"));
    assert!(cmake_lists.contains("CMAKE_CXX_STANDARD 20"));

    let cmake_config = file_map.get("PolyXMLConfig.cmake").unwrap();
    assert!(cmake_config.contains("PolyXML::FinanceModels"));

    let meson_build = file_map.get("meson.build").unwrap();
    assert!(meson_build.contains("project('finance_models_models', 'cpp'"));
    assert!(meson_build.contains("cpp_std=c++20"));
    assert!(meson_build.contains("declare_dependency"));
}

#[test]
fn test_cpp_inheritance_codegen() {
    let mut ir = SchemaIR::new().with_target_namespace("urn:org");

    // Base struct
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:org"), "Person"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef {
            name: "id".into(),
            xml_name: "id".into(),
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
        }],
        documentation: None,
    }));

    // Derived struct
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:org"), "Employee"),
        base_type: Some(QName::new(Some("urn:org"), "Person")),
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef {
            name: "dept".into(),
            xml_name: "dept".into(),
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
        }],
        documentation: None,
    }));

    let options = CppOptions {
        namespace: "org".to_string(),
        ..Default::default()
    };

    let codegen = CppCodegen::new(options);
    let header_code = codegen.generate_header(&ir);

    let temp = tempdir().unwrap();
    let header_path = temp.path().join("org.hpp");
    fs::write(&header_path, &header_code).unwrap();

    let main_cpp = temp.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"
#include "org.hpp"
#include <cassert>
#include <iostream>

int main() {
    using namespace org;

    Employee e1{{"emp-001"}, "Engineering"};
    Employee e2{{"emp-001"}, "Engineering"};
    Employee e3{{"emp-002"}, "Engineering"};

    assert(e1.id == "emp-001");
    assert(e1.dept == "Engineering");
    assert(e1 == e2);
    assert(!(e1 == e3));

    std::cout << "Inheritance test passed." << std::endl;
    return 0;
}
"#,
    )
    .unwrap();

    let out_bin = temp.path().join("org_app");
    let compile_status = Command::new("g++")
        .args([
            "-std=c++20",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
            "-I",
            temp.path().to_str().unwrap(),
            main_cpp.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute g++");

    assert!(compile_status.success(), "g++ compilation failed");

    let run_status = Command::new(&out_bin)
        .status()
        .expect("Failed to run test binary");
    assert!(
        run_status.success(),
        "Inheritance test binary execution failed"
    );
}

#[test]
fn test_cpp_from_xsd_schema() {
    use polyxml::schema_parser::XsdParser;

    let xsd_src = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
        <xs:simpleType name="ActiveCurrencyCode">
            <xs:restriction base="xs:string">
                <xs:pattern value="[A-Z]{3,3}"/>
            </xs:restriction>
        </xs:simpleType>
        <xs:complexType name="Amount">
            <xs:simpleContent>
                <xs:extension base="xs:decimal">
                    <xs:attribute name="Ccy" type="ActiveCurrencyCode" use="required"/>
                </xs:extension>
            </xs:simpleContent>
        </xs:complexType>
        <xs:complexType name="CreditTransfer">
            <xs:sequence>
                <xs:element name="EndToEndId" type="xs:string"/>
                <xs:element name="InstdAmt" type="Amount"/>
            </xs:sequence>
        </xs:complexType>
        <xs:element name="Document" type="CreditTransfer"/>
    </xs:schema>"#;

    let mut parser = XsdParser::new();
    let ir = parser.parse_str(xsd_src).expect("Failed to parse XSD");

    let options = CppOptions {
        namespace: "iso20022::pacs008".to_string(),
        emit_root_aliases: true,
        ..Default::default()
    };

    let codegen = CppCodegen::new(options);
    let header_code = codegen.generate_header(&ir);

    assert!(header_code.contains("namespace iso20022::pacs008 {"));
    assert!(header_code.contains("using Document = CreditTransfer;"));

    let temp = tempdir().unwrap();
    let header_path = temp.path().join("pacs008.hpp");
    fs::write(&header_path, &header_code).unwrap();

    let main_cpp = temp.path().join("main.cpp");
    fs::write(
        &main_cpp,
        r#"
#include "pacs008.hpp"
#include <cassert>
#include <iostream>

int main() {
    using namespace iso20022::pacs008;

    Document doc{
        .end_to_end_id = "E2E-987654321",
        .instd_amt = Amount{
            .ccy = "USD"
        }
    };

    assert(doc.end_to_end_id == "E2E-987654321");
    assert(doc.instd_amt.ccy == "USD");

    std::cout << "ISO 20022 pacs.008 C++20 test passed." << std::endl;
    return 0;
}
"#,
    )
    .unwrap();

    let out_bin = temp.path().join("iso_app");
    let compile_status = Command::new("g++")
        .args([
            "-std=c++20",
            "-Wall",
            "-Wextra",
            "-Wpedantic",
            "-Werror",
            "-I",
            temp.path().to_str().unwrap(),
            main_cpp.to_str().unwrap(),
            "-o",
            out_bin.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to execute g++");

    assert!(compile_status.success(), "g++ compilation failed");

    let run_status = Command::new(&out_bin)
        .status()
        .expect("Failed to run test binary");
    assert!(
        run_status.success(),
        "ISO 20022 test binary execution failed"
    );
}

#[test]
fn test_cpp_mode_and_backend_from_str_loose() {
    assert_eq!(CppMode::from_str_loose("header"), Some(CppMode::HeaderOnly));
    assert_eq!(
        CppMode::from_str_loose("header-only"),
        Some(CppMode::HeaderOnly)
    );
    assert_eq!(CppMode::from_str_loose("hpp"), Some(CppMode::HeaderOnly));
    assert_eq!(CppMode::from_str_loose("modules"), Some(CppMode::Module));
    assert_eq!(CppMode::from_str_loose("module"), Some(CppMode::Module));
    assert_eq!(CppMode::from_str_loose("cppm"), Some(CppMode::Module));
    assert_eq!(CppMode::from_str_loose("unknown"), None);

    assert_eq!(
        CppBackend::from_str_loose("standard"),
        Some(CppBackend::Standard)
    );
    assert_eq!(
        CppBackend::from_str_loose("default"),
        Some(CppBackend::Standard)
    );
    assert_eq!(CppBackend::from_str_loose("glaze"), Some(CppBackend::Glaze));
    assert_eq!(CppBackend::from_str_loose("glz"), Some(CppBackend::Glaze));
    assert_eq!(CppBackend::from_str_loose("unknown"), None);
}

#[test]
fn test_cpp_glaze_backend_meta_generation() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/shop");

    // Enum
    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/shop"), "OrderStatus"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "pending".into(),
                value: "pending".into(),
                documentation: None,
            },
            EnumValue {
                name: "shipped".into(),
                value: "shipped".into(),
                documentation: None,
            },
        ],
        documentation: None,
    }));

    // Struct
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/shop"), "Order"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "order_id".into(),
                xml_name: "orderId".into(),
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
            },
            FieldDef {
                name: "status".into(),
                xml_name: "status".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(
                    Some("https://example.com/shop"),
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
        documentation: None,
    }));

    let options = CppOptions {
        namespace: "shop".to_string(),
        backend: CppBackend::Glaze,
        ..Default::default()
    };

    let codegen = CppCodegen::new(options);
    let header_code = codegen.generate_header(&ir);

    // Includes Glaze header
    assert!(header_code.contains("#include <glaze/glaze.hpp>"));

    // Glaze enum reflection
    assert!(header_code.contains("template <>"));
    assert!(header_code.contains("struct glz::meta<shop::OrderStatus> {"));
    assert!(header_code.contains("using T = shop::OrderStatus;"));
    assert!(header_code.contains("static constexpr auto value = enumerate("));
    assert!(header_code.contains("\"pending\", T::Pending,"));
    assert!(header_code.contains("\"shipped\", T::Shipped"));

    // Glaze struct reflection
    assert!(header_code.contains("struct glz::meta<shop::Order> {"));
    assert!(header_code.contains("using T = shop::Order;"));
    assert!(header_code.contains("static constexpr auto value = object("));
    assert!(header_code.contains("\"orderId\", &T::order_id,"));
    assert!(header_code.contains("\"status\", &T::status"));
}

#[test]
fn test_cpp_cmake_modules_file_set() {
    let _ir = SchemaIR::new();

    let options = CppOptions {
        namespace: "telemetry".to_string(),
        mode: CppMode::Module,
        emit_cmake: true,
        ..Default::default()
    };

    let codegen = CppCodegen::new(options);
    let cmake = codegen.generate_cmake("telemetry");

    assert!(cmake.contains("cmake_minimum_required(VERSION 3.28)"));
    assert!(cmake.contains("FILE_SET CXX_MODULES FILES"));
    assert!(cmake.contains("telemetry.cppm"));
}
