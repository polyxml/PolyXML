use std::fs;
use std::process::Command;
use tempfile::tempdir;

use polyxml::codegen::cpp::{CppCodegen, CppOptions};
use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions};
use polyxml::codegen::go::{GoCodegen, GoOptions};
use polyxml::codegen::java::{JavaCodegen, JavaOptions};
use polyxml::codegen::python::{PythonCodegen, PythonOptions};
use polyxml::codegen::rust::{RustCodegen, RustOptions};
use polyxml::codegen::typescript::{TypeScriptCodegen, TypeScriptOptions};
use polyxml::ir::{TypeDef, TypeRef};
use polyxml::schema_parser::XsdParser;

const UNBOUNDED_CHOICE_XSD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:inventory"
           xmlns="urn:inventory"
           elementFormDefault="qualified">

  <xs:complexType name="Container">
    <xs:choice maxOccurs="unbounded">
      <xs:element name="itemA" type="xs:string"/>
      <xs:element name="itemB" type="xs:int"/>
    </xs:choice>
  </xs:complexType>

  <xs:element name="container" type="Container"/>
</xs:schema>
"#;

#[test]
fn test_unbounded_choice_ir_and_all_targets() {
    let mut parser = XsdParser::new();
    let ir = parser
        .parse_str(UNBOUNDED_CHOICE_XSD)
        .expect("Failed to parse XSD with unbounded choice");

    // 1. Verify Schema IR representation
    let container_choice = ir
        .types
        .values()
        .find(|t| t.qname().local == "ContainerChoice")
        .expect("Synthetic UnionDef ContainerChoice must exist");

    match container_choice {
        TypeDef::Union(u) => {
            assert_eq!(u.branches.len(), 2);
            assert_eq!(u.branches[0].xml_name, "itemA");
            assert_eq!(u.branches[1].xml_name, "itemB");
        }
        _ => panic!("ContainerChoice must be TypeDef::Union"),
    }

    let container_struct = ir
        .types
        .values()
        .find(|t| t.qname().local == "Container")
        .expect("StructDef Container must exist");

    match container_struct {
        TypeDef::Struct(s) => {
            assert_eq!(
                s.fields.len(),
                1,
                "Container should have 1 ordered items field instead of separate lists"
            );
            let items_field = &s.fields[0];
            assert_eq!(items_field.name, "items");
            assert!(
                items_field.cardinality.is_list(),
                "items must have list cardinality"
            );
            match &items_field.type_ref {
                TypeRef::Named(q) => assert_eq!(q.local, "ContainerChoice"),
                other => panic!("Expected TypeRef::Named(ContainerChoice), got {:?}", other),
            }
        }
        _ => panic!("Container must be TypeDef::Struct"),
    }

    // 2. Target: Rust
    let rust_opts = RustOptions::default();
    let rust_code = RustCodegen::new(rust_opts).generate_module(&ir);
    assert!(
        rust_code.contains("pub enum ContainerChoice"),
        "Rust must emit ContainerChoice enum"
    );
    assert!(
        rust_code.contains("ItemA(String)") || rust_code.contains("ItemA("),
        "Rust ContainerChoice must contain ItemA variant"
    );
    assert!(
        rust_code.contains("ItemB(i32)"),
        "Rust ContainerChoice must contain ItemB variant"
    );
    assert!(
        rust_code.contains("pub items: Vec<ContainerChoice"),
        "Rust Container must hold ordered Vec<ContainerChoice>"
    );
    assert!(
        rust_code.contains("item.encode_xml(writer, None)?"),
        "Rust serialization must preserve tag without wrapping"
    );

    // 3. Target: TypeScript
    let ts_opts = TypeScriptOptions::default();
    let ts_code = TypeScriptCodegen::new(ts_opts).generate_module(&ir);
    assert!(
        ts_code.contains("export type ContainerChoice ="),
        "TypeScript must emit ContainerChoice union"
    );
    assert!(
        ts_code.contains("items: ContainerChoice[]"),
        "TypeScript Container must hold ContainerChoice[]"
    );

    // 4. Target: Python
    let py_opts = PythonOptions::default();
    let py_code = PythonCodegen::new(py_opts).generate_module(&ir);
    assert!(
        py_code.contains("type ContainerChoice ="),
        "Python must emit ContainerChoice type alias"
    );
    assert!(
        py_code.contains("items: list[ContainerChoice]"),
        "Python Container must hold list[ContainerChoice]"
    );

    // 5. Target: C#
    let cs_opts = CSharpOptions::default();
    let cs_code = CSharpCodegen::new(cs_opts).generate_module(&ir);
    assert!(
        cs_code.contains("public abstract record ContainerChoice")
            || cs_code.contains("public abstract class ContainerChoice"),
        "C# must emit ContainerChoice abstract base"
    );
    assert!(
        cs_code.contains("XmlElement(\"itemA\", typeof(ContainerChoice.ItemA))"),
        "C# must emit polymorphic XmlElement attribute for itemA"
    );
    assert!(
        cs_code.contains("XmlElement(\"itemB\", typeof(ContainerChoice.ItemB))"),
        "C# must emit polymorphic XmlElement attribute for itemB"
    );
    assert!(
        cs_code.contains("List<ContainerChoice> Items"),
        "C# Container must hold List<ContainerChoice>"
    );

    // 6. Target: Go
    let go_opts = GoOptions {
        emit_xml_tags: true,
        emit_json_tags: true,
        validate_choice_exclusivity: true,
        ..GoOptions::default()
    };
    let go_code = GoCodegen::new(go_opts).generate_module(&ir);
    assert!(
        go_code.contains("type ContainerChoice struct"),
        "Go must emit ContainerChoice struct"
    );
    assert!(
        go_code.contains("Items []ContainerChoice `xml:\",any\""),
        "Go Container must hold []ContainerChoice with xml:\",any\""
    );
    assert!(
        go_code.contains("func (c ContainerChoice) MarshalXML"),
        "Go must emit MarshalXML for ContainerChoice"
    );
    assert!(
        go_code.contains("func (c *ContainerChoice) UnmarshalXML"),
        "Go must emit UnmarshalXML for ContainerChoice"
    );

    // 7. Target: C++
    let cpp_opts = CppOptions::default();
    let cpp_code = CppCodegen::new(cpp_opts).generate_module(&ir);
    assert!(
        cpp_code.contains("using ContainerChoice = std::variant<"),
        "C++ must emit ContainerChoice std::variant"
    );
    assert!(
        cpp_code.contains("std::vector<ContainerChoice> items"),
        "C++ Container must hold std::vector<ContainerChoice>"
    );

    // 8. Target: Java
    let java_opts = JavaOptions::default();
    let java_code = JavaCodegen::new(java_opts).generate_module(&ir, "Inventory");
    assert!(
        java_code.contains("sealed interface ContainerChoice"),
        "Java must emit sealed interface ContainerChoice"
    );
    assert!(
        java_code.contains("List<ContainerChoice> items"),
        "Java Container must hold List<ContainerChoice>"
    );

    // 9. Execute Go compilation and round-trip verification to ensure document order preservation
    let temp = tempdir().unwrap();
    let mod_path = temp.path().join("inventory.go");
    fs::write(&mod_path, &go_code).unwrap();

    let test_path = temp.path().join("inventory_test.go");
    fs::write(
        &test_path,
        r#"package models

import (
    "encoding/xml"
    "testing"
)

func TestUnboundedChoiceOrder(t *testing.T) {
    // Interleaved elements: itemA, then itemB, then itemA
    inputXml := `<container xmlns="urn:inventory"><itemA>first</itemA><itemB>42</itemB><itemA>third</itemA></container>`

    var c Container
    if err := xml.Unmarshal([]byte(inputXml), &c); err != nil {
        t.Fatalf("Unmarshal error: %v", err)
    }

    if len(c.Items) != 3 {
        t.Fatalf("expected 3 items in ordered choice list, got %d", len(c.Items))
    }

    // Verify item 0: itemA = "first"
    if c.Items[0].ItemA == nil || *c.Items[0].ItemA != "first" {
        t.Fatalf("items[0] should be itemA with value 'first', got itemA=%v, itemB=%v", c.Items[0].ItemA, c.Items[0].ItemB)
    }
    if c.Items[0].ItemB != nil {
        t.Fatalf("items[0] should not have itemB populated")
    }

    // Verify item 1: itemB = 42
    if c.Items[1].ItemB == nil || *c.Items[1].ItemB != 42 {
        t.Fatalf("items[1] should be itemB with value 42")
    }
    if c.Items[1].ItemA != nil {
        t.Fatalf("items[1] should not have itemA populated")
    }

    // Verify item 2: itemA = "third"
    if c.Items[2].ItemA == nil || *c.Items[2].ItemA != "third" {
        t.Fatalf("items[2] should be itemA with value 'third'")
    }
    if c.Items[2].ItemB != nil {
        t.Fatalf("items[2] should not have itemB populated")
    }

    // Re-serialize and verify exact interleaved order is preserved
    outBytes, err := xml.Marshal(&c)
    if err != nil {
        t.Fatalf("Marshal error: %v", err)
    }
    outStr := string(outBytes)

    expected := `<container xmlns="urn:inventory"><itemA>first</itemA><itemB>42</itemB><itemA>third</itemA></container>`
    if outStr != expected {
        t.Fatalf("Serialization did not preserve document order:\nGot:      %s\nExpected: %s", outStr, expected)
    }
}
"#,
    ).unwrap();

    let init_status = Command::new("go")
        .args(["mod", "init", "testmod"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run go mod init");
    assert!(init_status.success());

    let test_status = Command::new("go")
        .args(["test", "-v", "."])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run go test");
    assert!(
        test_status.success(),
        "Go test failed for unbounded choice ordering"
    );
}
