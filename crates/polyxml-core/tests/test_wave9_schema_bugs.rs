use std::fs;
use std::process::Command;
use tempfile::tempdir;

use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions};
use polyxml::codegen::go::{GoCodegen, GoOptions};
use polyxml::codegen::java::{JavaCodegen, JavaOptions};
use polyxml::codegen::python::{PythonCodegen, PythonOptions};
use polyxml::codegen::rust::{RustCodegen, RustOptions};
use polyxml::ir::{FieldKind, PrimitiveType, QName, TypeDef, TypeRef};
use polyxml::schema_parser::XsdParser;

#[test]
fn issue_163_substitution_group_inherits_head_type() {
    let schema = include_str!("../../../tests/fixtures/conformance/wave9/sub_notype.xsd");
    let ir = XsdParser::new().parse_str(schema).unwrap();

    let head_q = QName::local("head");
    let member_q = QName::local("member");
    let expected_type = TypeRef::Named(QName::local("HeadType"));

    assert_eq!(ir.elements[&head_q].type_ref, expected_type);
    assert_eq!(
        ir.elements[&member_q].type_ref, expected_type,
        "member without explicit type should inherit HeadType"
    );

    let rust = RustCodegen::new(RustOptions::default()).generate_module(&ir);
    assert!(
        rust.contains("Member(HeadType)"),
        "expected Member(HeadType) in ContainerItem:\n{rust}"
    );
    assert!(
        rust.contains("pub type Member = HeadType;"),
        "expected pub type Member = HeadType;:\n{rust}"
    );
}

#[test]
fn issue_163_chained_substitution_group_inherits_head_type() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:complexType name="HeadType">
    <xs:sequence>
      <xs:element name="code" type="xs:int"/>
    </xs:sequence>
  </xs:complexType>
  <xs:element name="head" type="HeadType"/>
  <xs:element name="mid" substitutionGroup="head"/>
  <xs:element name="leaf" substitutionGroup="mid"/>
</xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let expected = TypeRef::Named(QName::local("HeadType"));
    assert_eq!(ir.elements[&QName::local("mid")].type_ref, expected);
    assert_eq!(ir.elements[&QName::local("leaf")].type_ref, expected);
}

#[test]
fn issue_164_prohibited_attributes_removed_and_base_attrs_inherited() {
    let schema = include_str!("../../../tests/fixtures/conformance/wave9/prohibited_attr.xsd");
    let ir = XsdParser::new().parse_str(schema).unwrap();

    let TypeDef::Struct(modern) = &ir.types[&QName::local("ModernItem")] else {
        panic!("expected ModernItem struct");
    };
    let attr_names: Vec<&str> = modern
        .fields
        .iter()
        .filter(|f| f.kind == FieldKind::Attribute)
        .map(|f| f.xml_name.as_str())
        .collect();
    assert!(
        !attr_names.contains(&"legacyCode"),
        "prohibited legacyCode must be excluded: {attr_names:?}"
    );
    assert_eq!(
        attr_names,
        vec!["active"],
        "non-prohibited base attribute 'active' must be inherited"
    );
    let active_field = modern
        .fields
        .iter()
        .find(|f| f.xml_name == "active")
        .unwrap();
    assert_eq!(active_field.default_value.as_deref(), Some("true"));

    let rust = RustCodegen::new(RustOptions::default()).generate_module(&ir);
    let modern_rs_block = rust.split("pub struct ModernItem").nth(1).unwrap();
    assert!(!modern_rs_block.contains("legacy_code"));
    assert!(modern_rs_block.contains("pub active: Option<bool>"));

    let py = PythonCodegen::new(PythonOptions::default()).generate_module(&ir);
    let modern_py_block = py.split("class ModernItem").nth(1).unwrap();
    assert!(!modern_py_block.contains("legacy_code"));
    assert!(modern_py_block.contains("active: bool | None"));

    let cs = CSharpCodegen::new(CSharpOptions::default()).generate_module(&ir);
    let modern_cs_block = cs.split("public record ModernItem").nth(1).unwrap();
    assert!(!modern_cs_block.contains("LegacyCode"));
    assert!(modern_cs_block.contains("Active"));

    let java = JavaCodegen::new(JavaOptions::default())
        .generate_files(&ir)
        .into_iter()
        .find(|(name, _)| name == "ModernItem.java")
        .map(|(_, src)| src)
        .unwrap();
    assert!(!java.contains("legacyCode"));
    assert!(java.contains("active"));
}

#[test]
fn issue_165_choice_shared_tail_preserves_order_and_round_trips_go() {
    let schema = include_str!("../../../tests/fixtures/conformance/wave9/choice_shared_tail.xsd");
    let ir = XsdParser::new().parse_str(schema).unwrap();

    let TypeDef::Struct(response) = &ir.types[&QName::local("Response")] else {
        panic!("expected Response struct");
    };
    let elem_names: Vec<&str> = response
        .fields
        .iter()
        .filter(|f| f.kind == FieldKind::Element)
        .map(|f| f.xml_name.as_str())
        .collect();
    assert_eq!(elem_names, vec!["ok", "err", "detail"]);

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("go.mod"), "module wave9choice\n\ngo 1.22\n").unwrap();
    fs::write(
        dir.path().join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(
        dir.path().join("models_test.go"),
        r#"package models
import (
    "encoding/xml"
    "testing"
)
func TestChoiceSharedTailRoundTrip(t *testing.T) {
    for _, doc := range []string{
        "<Response><ok>200</ok><detail>done</detail></Response>",
        "<Response><err>500</err><detail>timeout</detail></Response>",
    } {
        var r Response
        if err := xml.Unmarshal([]byte(doc), &r); err != nil {
            t.Fatalf("unmarshal %s failed: %v", doc, err)
        }
        out, err := xml.Marshal(r)
        if err != nil {
            t.Fatalf("marshal %s failed: %v", doc, err)
        }
        if string(out) != doc {
            t.Fatalf("roundtrip mismatch: got %s want %s", string(out), doc)
        }
    }
}
"#,
    )
    .unwrap();
    let result = Command::new("go")
        .args(["test", "./..."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn issue_165_outer_sequence_around_choice_does_not_leapfrog_footer() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:complexType name="Envelope">
    <xs:sequence>
      <xs:element name="header" type="xs:string"/>
      <xs:choice>
        <xs:sequence>
          <xs:element name="ok" type="xs:string"/>
          <xs:element name="detail" type="xs:string"/>
        </xs:sequence>
        <xs:sequence>
          <xs:element name="err" type="xs:string"/>
          <xs:element name="detail" type="xs:string"/>
        </xs:sequence>
      </xs:choice>
      <xs:element name="footer" type="xs:string"/>
    </xs:sequence>
  </xs:complexType>
</xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let TypeDef::Struct(env) = &ir.types[&QName::local("Envelope")] else {
        panic!("expected Envelope struct");
    };
    let elem_names: Vec<&str> = env
        .fields
        .iter()
        .filter(|f| f.kind == FieldKind::Element)
        .map(|f| f.xml_name.as_str())
        .collect();
    assert_eq!(elem_names, vec!["header", "ok", "err", "detail", "footer"]);
}

#[test]
fn issue_163_substitution_group_inline_anonymous_head_and_polyglot_codegen() {
    let inline_xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="head">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="code" type="xs:int"/>
      </xs:sequence>
    </xs:complexType>
  </xs:element>
  <xs:element name="member" substitutionGroup="head"/>
</xs:schema>"#;

    let inline_ir = XsdParser::new().parse_str(inline_xsd).unwrap();
    let head_type = inline_ir.elements[&QName::local("head")].type_ref.clone();
    assert!(
        matches!(&head_type, TypeRef::Named(q) if q.local == "headType"),
        "expected inline anonymous complexType Named(headType), got {head_type:?}"
    );
    assert_eq!(
        inline_ir.elements[&QName::local("member")].type_ref,
        head_type,
        "member without explicit type should inherit head's anonymous TypeRef"
    );

    let sub_schema = include_str!("../../../tests/fixtures/conformance/wave9/sub_notype.xsd");
    let sub_ir = XsdParser::new().parse_str(sub_schema).unwrap();

    let go = GoCodegen::new(GoOptions::default()).generate_module(&sub_ir);
    assert!(
        go.contains("Member *HeadType") || go.contains("type Member HeadType"),
        "expected Go codegen to type Member as HeadType:\n{go}"
    );
    assert!(
        !go.contains("Member any") && !go.contains("Member interface{}"),
        "Go member must not degrade to untyped any/interface{{}}:\n{go}"
    );

    let py = PythonCodegen::new(PythonOptions::default()).generate_module(&sub_ir);
    assert!(
        py.contains("HeadType") && py.contains("Member(HeadType)"),
        "expected Python codegen to emit typed Member(HeadType) subclass/alias:\n{py}"
    );
    assert!(
        !py.contains("member: Any"),
        "Python member must not degrade to typing.Any:\n{py}"
    );
}

#[test]
fn issue_164_multi_level_restriction_chain_propagates_prohibitions_and_overrides() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:complexType name="GrandBase">
    <xs:sequence>
      <xs:element name="label" type="xs:string"/>
    </xs:sequence>
    <xs:attribute name="legacy" type="xs:string"/>
    <xs:attribute name="obsolete" type="xs:string"/>
    <xs:attribute name="mode" type="xs:string" default="v1"/>
    <xs:attribute name="active" type="xs:boolean"/>
  </xs:complexType>
  <xs:complexType name="MidItem">
    <xs:complexContent>
      <xs:restriction base="GrandBase">
        <xs:sequence>
          <xs:element name="label" type="xs:string"/>
        </xs:sequence>
        <xs:attribute name="legacy" use="prohibited"/>
        <xs:attribute name="mode" type="xs:string" use="required"/>
      </xs:restriction>
    </xs:complexContent>
  </xs:complexType>
  <xs:complexType name="LeafItem">
    <xs:complexContent>
      <xs:restriction base="MidItem">
        <xs:sequence>
          <xs:element name="label" type="xs:string"/>
        </xs:sequence>
        <xs:attribute name="obsolete" use="prohibited"/>
      </xs:restriction>
    </xs:complexContent>
  </xs:complexType>
</xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let TypeDef::Struct(leaf) = &ir.types[&QName::local("LeafItem")] else {
        panic!("expected LeafItem struct");
    };
    let attrs: Vec<_> = leaf
        .fields
        .iter()
        .filter(|f| f.kind == FieldKind::Attribute)
        .collect();
    let attr_names: Vec<&str> = attrs.iter().map(|f| f.xml_name.as_str()).collect();
    assert!(
        !attr_names.contains(&"legacy"),
        "legacy prohibited in MidItem: {attr_names:?}"
    );
    assert!(
        !attr_names.contains(&"obsolete"),
        "obsolete prohibited in LeafItem: {attr_names:?}"
    );
    assert_eq!(attr_names, vec!["mode", "active"]);

    let mode = attrs.iter().find(|f| f.xml_name == "mode").unwrap();
    assert_eq!(
        mode.cardinality.min_occurs, 1,
        "mode override must remain required"
    );
    assert_eq!(mode.type_ref, TypeRef::Primitive(PrimitiveType::String));

    let active = attrs.iter().find(|f| f.xml_name == "active").unwrap();
    assert_eq!(active.type_ref, TypeRef::Primitive(PrimitiveType::Boolean));
}

#[test]
fn issue_165_choice_conflicting_sequence_order_cycle_is_deterministic() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:complexType name="CyclicChoice">
    <xs:choice>
      <xs:sequence>
        <xs:element name="a" type="xs:string"/>
        <xs:element name="b" type="xs:string"/>
      </xs:sequence>
      <xs:sequence>
        <xs:element name="b" type="xs:string"/>
        <xs:element name="a" type="xs:string"/>
      </xs:sequence>
    </xs:choice>
  </xs:complexType>
</xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let TypeDef::Struct(cyclic) = &ir.types[&QName::local("CyclicChoice")] else {
        panic!("expected CyclicChoice struct");
    };
    let elem_names: Vec<&str> = cyclic
        .fields
        .iter()
        .filter(|f| f.kind == FieldKind::Element)
        .map(|f| f.xml_name.as_str())
        .collect();
    assert_eq!(elem_names, vec!["a", "b"]);
}
