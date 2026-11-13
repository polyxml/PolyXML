use polyxml::codegen::cpp::{CppCodegen, CppOptions};
use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions};
use polyxml::codegen::go::{GoCodegen, GoOptions};
use polyxml::codegen::java::{JavaCodegen, JavaOptions};
use polyxml::codegen::rust::{RustCodegen, RustOptions};
use polyxml::codegen::typescript::{TypeScriptCodegen, TypeScriptOptions};
use polyxml::ir::{PrimitiveType, QName, TypeDef, TypeRef, UnionBranch, UnionDef};
use polyxml::schema_parser::XsdParser as SchemaParser;

#[test]
fn go_avoids_enum_const_type_collision_any_receiver_and_abstract_base_collision() {
    let xsd = r#"
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
               targetNamespace="urn:test" xmlns:tns="urn:test" elementFormDefault="qualified">
        <xs:simpleType name="ActClass">
            <xs:restriction base="xs:token">
                <xs:enumeration value="BATTERY"/>
            </xs:restriction>
        </xs:simpleType>
        <xs:complexType name="ActClassBattery">
            <xs:sequence>
                <xs:element name="code" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>
        <xs:complexType name="Option" abstract="true">
            <xs:sequence>
                <xs:element name="id" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>
        <xs:complexType name="OptionBase">
            <xs:complexContent>
                <xs:extension base="tns:Option">
                    <xs:sequence>
                        <xs:element name="kind" type="xs:string"/>
                    </xs:sequence>
                </xs:extension>
            </xs:complexContent>
        </xs:complexType>
        <xs:element name="AbstractObject" type="xs:anyType"/>
    </xs:schema>"#;

    let ir = SchemaParser::new().parse_str(xsd).unwrap();
    let code = GoCodegen::new(GoOptions {
        emit_root_aliases: true,
        emit_xml_tags: true,
        ..Default::default()
    })
    .generate_module(&ir);

    assert!(code.contains("ActClassBattery_2 ActClass = \"BATTERY\""));
    assert!(code.contains("type AbstractObject = any"));
    assert!(!code.contains("func (r *AbstractObject) UnmarshalXML"));
    assert!(code.contains("type OptionBase_2 struct"));
    assert!(code.contains("type OptionBase struct"));
}

#[test]
fn java_deduplicates_union_variants_and_avoids_java_lang_string_shadowing() {
    let xsd = r#"
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
               targetNamespace="urn:test" xmlns:tns="urn:test" elementFormDefault="qualified">
        <xs:complexType name="String">
            <xs:sequence>
                <xs:element name="text" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#;

    let mut ir = SchemaParser::new().parse_str(xsd).unwrap();
    let union_qn = QName::new(Some("urn:test"), "ChoiceWithDuplicateBranch");
    ir.types.insert(
        union_qn.clone(),
        TypeDef::Union(UnionDef {
            qname: union_qn,
            branches: vec![
                UnionBranch {
                    variant_name: "barrier".to_string(),
                    xml_name: "barrier".to_string(),
                    namespace: Some("urn:test".to_string()),
                    type_ref: TypeRef::Primitive(PrimitiveType::String),
                    documentation: None,
                },
                UnionBranch {
                    variant_name: "barrier".to_string(),
                    xml_name: "barrier2".to_string(),
                    namespace: Some("urn:test".to_string()),
                    type_ref: TypeRef::Primitive(PrimitiveType::Int),
                    documentation: None,
                },
            ],
            documentation: None,
        }),
    );

    let code = JavaCodegen::new(JavaOptions {
        emit_direct_codec: true,
        use_records: true,
        ..Default::default()
    })
    .generate_module(&ir, "Models");

    assert!(!code.contains("record String("));
    assert!(code.contains("record StringType("));
    assert!(code.contains("record Barrier("));
    assert!(code.contains("record Barrier2("));
}

#[test]
fn cpp_orders_enum_before_simple_list_guards_root_aliases_and_qualifies_enum_default() {
    let xsd = r#"
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
               targetNamespace="urn:test" xmlns:tns="urn:test" elementFormDefault="qualified">
        <xs:simpleType name="Color">
            <xs:restriction base="xs:token">
                <xs:enumeration value="red"/>
                <xs:enumeration value="blue"/>
            </xs:restriction>
        </xs:simpleType>
        <xs:simpleType name="ColorList">
            <xs:list itemType="tns:Color"/>
        </xs:simpleType>
        <xs:complexType name="Palette">
            <xs:sequence>
                <xs:element name="primary" type="tns:Color" default="red"/>
            </xs:sequence>
        </xs:complexType>
        <xs:element name="Palette" type="tns:Palette"/>
    </xs:schema>"#;

    let ir = SchemaParser::new().parse_str(xsd).unwrap();
    let code = CppCodegen::new(CppOptions {
        emit_root_aliases: true,
        ..Default::default()
    })
    .generate_module(&ir);

    let enum_pos = code.find("enum class Color").unwrap();
    let list_pos = code.find("using ColorList = std::vector<Color>").unwrap();
    assert!(enum_pos < list_pos);
    assert!(code.contains("#ifndef POLYXML_UTILITIES_DEFINED"));
    assert!(code.contains("Color primary = Color::Red;"));
    assert!(!code.contains("using Palette = Palette;"));
}

#[test]
fn typescript_empty_module_is_valid_esm_and_csharp_skips_scalar_proxy_on_enum_list() {
    let empty_xsd = r#"
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:empty"/>"#;
    let empty_ir = SchemaParser::new().parse_str(empty_xsd).unwrap();
    let ts_code = TypeScriptCodegen::new(TypeScriptOptions::default()).generate_module(&empty_ir);
    assert!(ts_code.contains("export {};"));

    let list_xsd = r#"
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
               targetNamespace="urn:test" xmlns:tns="urn:test" elementFormDefault="qualified">
        <xs:simpleType name="DayEnum">
            <xs:union memberTypes="xs:token">
                <xs:simpleType>
                    <xs:restriction base="xs:token">
                        <xs:enumeration value="MON"/>
                    </xs:restriction>
                </xs:simpleType>
            </xs:union>
        </xs:simpleType>
        <xs:complexType name="Schedule">
            <xs:sequence>
                <xs:element name="day" type="tns:DayEnum" maxOccurs="unbounded"/>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#;
    let list_ir = SchemaParser::new().parse_str(list_xsd).unwrap();
    let cs_code = CSharpCodegen::new(CSharpOptions::default()).generate_module(&list_ir);
    assert!(!cs_code.contains("Day?.ToXmlString()"));
}

#[test]
fn rust_avoids_prelude_and_event_collisions_and_handles_boxed_any_and_simple_lists() {
    let xsd = r#"
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
               targetNamespace="urn:test" xmlns:tns="urn:test" elementFormDefault="qualified">
        <xs:complexType name="Event">
            <xs:sequence>
                <xs:element name="payload" type="xs:anyType"/>
            </xs:sequence>
        </xs:complexType>
        <xs:complexType name="Option">
            <xs:sequence>
                <xs:element name="next" type="tns:Option" minOccurs="0"/>
            </xs:sequence>
        </xs:complexType>
        <xs:simpleType name="Flag">
            <xs:restriction base="xs:token">
                <xs:enumeration value="A"/>
            </xs:restriction>
        </xs:simpleType>
        <xs:simpleType name="FlagList">
            <xs:list itemType="tns:Flag"/>
        </xs:simpleType>
        <xs:complexType name="Holder">
            <xs:sequence>
                <xs:element name="flags" type="tns:FlagList"/>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#;

    let ir = SchemaParser::new().parse_str(xsd).unwrap();
    let files = RustCodegen::new(RustOptions {
        emit_codecs: true,
        split_units: Some(true),
        ..Default::default()
    })
    .generate_files(&ir, "test_mod");
    let combined: String = files.into_iter().map(|(_, c)| c).collect();

    assert!(combined.contains("Event as QuickXmlEvent"));
    assert!(combined.contains("pub struct OptionType"));
    assert!(!combined.contains("Box<Box<"));
    assert!(!combined.contains("Box::decode_xml"));
    assert!(!combined.contains("Cow::decode_xml"));
    assert!(!combined.contains("FlagList::decode_xml"));

    let union_xsd = r#"
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
               targetNamespace="urn:test" xmlns:tns="urn:test" elementFormDefault="qualified">
        <xs:simpleType name="NumOrBool">
            <xs:union memberTypes="xs:int xs:boolean"/>
        </xs:simpleType>
    </xs:schema>"#;
    let union_ir = SchemaParser::new().parse_str(union_xsd).unwrap();
    let zc_code = RustCodegen::new(RustOptions {
        emit_codecs: true,
        zero_copy: true,
        ..Default::default()
    })
    .generate_module(&union_ir);
    assert!(zc_code.contains("Self::parse_lexical(read_element_text(reader, &tag)?.into_owned())"));
}
