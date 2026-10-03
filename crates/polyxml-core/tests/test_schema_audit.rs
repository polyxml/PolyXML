//! Regression tests for edge-case XSD parsing and code generation.
//!
//! These tests cover named groups, duplicate enum values, nested inline
//! types, namespace collisions, simple content, and include behavior.

use std::collections::HashSet;
use std::fs;

use polyxml::codegen::{
    CSharpCodegen, CSharpOptions, CppCodegen, CppOptions, GoCodegen, GoOptions, JavaCodegen,
    JavaOptions, PythonBackend, PythonCodegen, PythonOptions, RustCodegen, RustOptions,
    TypeScriptCodegen, TypeScriptOptions,
};
use polyxml::ir::{
    FieldDef, FieldKind, PrimitiveType, QName, SchemaIR, StructDef, TypeDef, TypeRef,
};
use polyxml::schema_parser::XsdParser;
use tempfile::tempdir;

fn struct_of<'a>(ir: &'a SchemaIR, local: &str) -> &'a StructDef {
    let key = QName::new(ir.target_namespace.clone(), local);
    match ir.types.get(&key) {
        Some(TypeDef::Struct(s)) => s,
        other => panic!("expected struct `{local}`, got {other:?}"),
    }
}

fn has_field(s: &StructDef, wire_name: &str) -> bool {
    s.fields
        .iter()
        .any(|f| f.xml_name == wire_name || f.name == wire_name)
}

fn field_names(s: &StructDef) -> Vec<String> {
    s.fields
        .iter()
        .map(|f| format!("{} ({:?})", f.xml_name, f.kind))
        .collect()
}

// ---------------------------------------------------------------------------
// xs:group named model groups
// ---------------------------------------------------------------------------

#[test]
fn named_group_reference_fields_are_parsed() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:g" xmlns:t="urn:g" elementFormDefault="qualified">
        <xs:group name="HeaderParts">
            <xs:sequence>
                <xs:element name="MsgId" type="xs:string"/>
                <xs:element name="CreDt" type="xs:string"/>
                <xs:element name="MsgDefIdr" type="xs:string"/>
            </xs:sequence>
        </xs:group>
        <xs:complexType name="DocumentHeader">
            <xs:sequence>
                <xs:element name="OwnField" type="xs:string"/>
                <xs:group ref="t:HeaderParts"/>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse");
    let header = struct_of(&ir, "DocumentHeader");
    for expected in ["OwnField", "MsgId", "CreDt", "MsgDefIdr"] {
        assert!(
            has_field(header, expected),
            "field `{expected}` missing from DocumentHeader; got {:?}",
            field_names(header)
        );
    }
}

// ---------------------------------------------------------------------------
// duplicate enum variant deduplication
// ---------------------------------------------------------------------------

#[test]
fn duplicate_enumeration_values_are_deduped() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:e" elementFormDefault="qualified">
        <xs:simpleType name="SettlementStatus">
            <xs:restriction base="xs:string">
                <xs:enumeration value="settled"/>
                <xs:enumeration value="settled"/>
                <xs:enumeration value="failed"/>
            </xs:restriction>
        </xs:simpleType>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse");
    let key = QName::new(Some("urn:e"), "SettlementStatus");
    let TypeDef::Enum(e) = ir.types.get(&key).expect("SettlementStatus") else {
        panic!("SettlementStatus is not an enum: {:?}", ir.types.get(&key));
    };
    let values: Vec<&str> = e.variants.iter().map(|v| v.value.as_str()).collect();
    assert_eq!(
        values,
        ["settled", "failed"],
        "duplicate enumeration values must be dropped"
    );
}

#[test]
fn distinct_values_with_colliding_variant_names_stay_unique() {
    // "failed" and "Failed" are distinct (legal) XSD values but sanitize to the
    // same variant identifier in every target language.
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:e" elementFormDefault="qualified">
        <xs:simpleType name="CaseStatus">
            <xs:restriction base="xs:string">
                <xs:enumeration value="failed"/>
                <xs:enumeration value="Failed"/>
            </xs:restriction>
        </xs:simpleType>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse");
    let key = QName::new(Some("urn:e"), "CaseStatus");
    let TypeDef::Enum(e) = ir.types.get(&key).expect("CaseStatus") else {
        panic!("CaseStatus is not an enum");
    };
    assert_eq!(
        e.variants.len(),
        2,
        "distinct values must both be kept; got {:?}",
        e.variants
    );
    let mut names = HashSet::new();
    for v in &e.variants {
        assert!(
            names.insert(v.name.clone()),
            "variant name `{}` collides across distinct values {:?}",
            v.value,
            e.variants
        );
    }
}

// ---------------------------------------------------------------------------
// cross-namespace same-local-name collision in flat output
// ---------------------------------------------------------------------------

fn two_namespace_address_ir() -> SchemaIR {
    let mut ir = SchemaIR::new().with_target_namespace("urn:a");
    for ns in ["urn:a", "urn:b"] {
        ir.add_type(TypeDef::Struct(StructDef {
            qname: QName::new(Some(ns), "Address"),
            base_type: None,
            is_abstract: false,
            is_mixed: false,
            fields: vec![FieldDef::new(
                "street",
                "Street",
                FieldKind::Element,
                TypeRef::Primitive(PrimitiveType::String),
            )],
            documentation: None,
        }));
    }
    ir
}

/// Extract declared type identifiers that follow any of `markers`, keeping
/// only those starting with `prefix` (drops outer wrapper classes, glz::meta
/// specializations, forward declarations, etc. from the comparison).
fn declared_prefixed_idents(code: &str, markers: &[&str], prefix: &str) -> HashSet<String> {
    let mut found = HashSet::new();
    for marker in markers {
        let mut rest = code;
        while let Some(idx) = rest.find(marker) {
            rest = &rest[idx + marker.len()..];
            let ident: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if ident.starts_with(prefix) {
                found.insert(ident);
            }
        }
    }
    found
}

#[test]
fn rust_flat_output_has_no_colliding_type_names() {
    let ir = two_namespace_address_ir();
    let code = RustCodegen::new(RustOptions::default()).generate_module(&ir);

    let marker = "pub struct ";
    let mut names: Vec<String> = Vec::new();
    let mut rest = code.as_str();
    while let Some(idx) = rest.find(marker) {
        rest = &rest[idx + marker.len()..];
        let ident: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        names.push(ident);
    }
    assert!(!names.is_empty(), "no structs emitted:\n{code}");
    let mut seen = HashSet::new();
    for n in &names {
        assert!(
            seen.insert(n.clone()),
            "duplicate type identifier `{n}` in flat output: {names:?}\n{code}"
        );
    }
    let address_idents = declared_prefixed_idents(&code, &[marker], "Address");
    assert_eq!(
        address_idents,
        ["Address".to_string(), "Address2".to_string()]
            .into_iter()
            .collect::<HashSet<_>>(),
        "rust must emit two distinct struct names for colliding types; got {address_idents:?}\n{code}"
    );
}

#[test]
fn all_languages_disambiguate_flat_collision() {
    let ir = two_namespace_address_ir();
    let outputs = [
        (
            "python",
            PythonCodegen::new(PythonOptions::default()).generate_module(&ir),
            &["class "] as &[&str],
        ),
        (
            "java",
            JavaCodegen::new(JavaOptions::default()).generate_module(&ir, "Audit"),
            &["record ", "class "],
        ),
        (
            "csharp",
            CSharpCodegen::new(CSharpOptions::default()).generate_module(&ir),
            &["record ", "class ", "struct "],
        ),
        (
            "typescript",
            TypeScriptCodegen::new(TypeScriptOptions::default()).generate_module(&ir),
            &["export interface ", "export type "],
        ),
        (
            "go",
            GoCodegen::new(GoOptions::default()).generate_module(&ir),
            &["type "],
        ),
        (
            "cpp",
            CppCodegen::new(CppOptions::default()).generate_module(&ir),
            &["struct "],
        ),
    ];
    let expected: HashSet<String> = ["Address".to_string(), "Address2".to_string()]
        .into_iter()
        .collect();
    for (lang, out, markers) in outputs {
        let address_idents = declared_prefixed_idents(&out, markers, "Address");
        assert_eq!(
            address_idents, expected,
            "{lang} must disambiguate the two colliding `Address` types into \
             `Address` + `Address2`; got {address_idents:?}\n{out}"
        );
    }
}

// ---------------------------------------------------------------------------
// simpleContent extension text field
// ---------------------------------------------------------------------------

#[test]
fn simple_content_extension_emits_value_field() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:amt" elementFormDefault="qualified">
        <xs:complexType name="AmountWithCurrency">
            <xs:simpleContent>
                <xs:extension base="xs:decimal">
                    <xs:attribute name="ccy" type="xs:string" use="required"/>
                </xs:extension>
            </xs:simpleContent>
        </xs:complexType>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse");
    let s = struct_of(&ir, "AmountWithCurrency");
    let text = match s.fields.iter().find(|f| f.kind == FieldKind::Text) {
        Some(f) => f,
        None => panic!(
            "no Text value field emitted for simpleContent; got {:?}",
            field_names(s)
        ),
    };
    assert_eq!(text.name, "value");
    assert_eq!(
        text.type_ref,
        TypeRef::Primitive(PrimitiveType::Decimal),
        "value field must carry the extension base type"
    );
    assert!(
        s.fields
            .iter()
            .any(|f| f.kind == FieldKind::Attribute && f.xml_name == "ccy"),
        "`ccy` attribute missing; got {:?}",
        field_names(s)
    );
}

// ---------------------------------------------------------------------------
// anonymous inline complexType extraction (nested)
// ---------------------------------------------------------------------------

#[test]
fn nested_inline_complex_type_is_extracted_not_leaked() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:ord" elementFormDefault="qualified">
        <xs:complexType name="Order">
            <xs:sequence>
                <xs:element name="Item">
                    <xs:complexType>
                        <xs:sequence>
                            <xs:element name="Sku" type="xs:string"/>
                        </xs:sequence>
                    </xs:complexType>
                </xs:element>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse");
    let order = struct_of(&ir, "Order");
    assert!(
        !has_field(order, "Sku"),
        "nested inline type fields leaked into parent struct; got {:?}",
        field_names(order)
    );
    assert!(has_field(order, "Item"), "Item field missing");
    let item = order
        .fields
        .iter()
        .find(|f| f.xml_name == "Item")
        .expect("Item field");
    assert!(
        matches!(item.type_ref, TypeRef::Named(_)),
        "Item must reference the extracted inline type, got {:?}",
        item.type_ref
    );
    let carrier = ir.types.iter().find(|(q, t)| {
        q.local != "Order" && matches!(t, TypeDef::Struct(s) if has_field(s, "Sku"))
    });
    assert!(
        carrier.is_some(),
        "no extracted type holds the nested `Sku`; types: {:?}",
        ir.types.keys().collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// unknown element tolerance in generated serde codecs
// ---------------------------------------------------------------------------

#[test]
fn generated_rust_has_no_deny_unknown_fields() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:p" elementFormDefault="qualified">
        <xs:complexType name="Person">
            <xs:sequence>
                <xs:element name="Name" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse");
    let code = RustCodegen::new(RustOptions::default()).generate_module(&ir);
    assert!(
        !code.contains("deny_unknown_fields"),
        "generated code must tolerate unknown elements:\n{code}"
    );
}

#[test]
fn quick_xml_serde_skips_unknown_elements() {
    // Mirrors the derive set emitted by the Rust codegen; verifies quick-xml's
    // default Deserialize behavior skips elements not present on the struct.
    #[derive(Debug, serde::Deserialize)]
    struct Person {
        #[serde(rename = "Name")]
        name: String,
    }

    let xml = r#"<Person><Name>Alex</Name><Nickname>Al</Nickname><extra><deep/></extra></Person>"#;
    let parsed: Person = quick_xml::de::from_str(xml).expect("unknown elements must be tolerated");
    assert_eq!(parsed.name, "Alex");
}

// `xsi:type` polymorphic dispatch is covered in `tests/test_xsi_type.rs`.

// ---------------------------------------------------------------------------
// pattern facet AND/OR derivation semantics
// ---------------------------------------------------------------------------

#[test]
fn derived_simple_type_inherits_base_patterns() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:pat" xmlns:t="urn:pat" elementFormDefault="qualified">
        <xs:simpleType name="BaseCode">
            <xs:restriction base="xs:string">
                <xs:pattern value="[A-Z]{6}"/>
            </xs:restriction>
        </xs:simpleType>
        <xs:simpleType name="DerivedCode">
            <xs:restriction base="t:BaseCode">
                <xs:pattern value="[A-Z]{6}[A-Z0-9]{3}"/>
            </xs:restriction>
        </xs:simpleType>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse");
    let key = QName::new(Some("urn:pat"), "DerivedCode");
    let TypeDef::Simple(d) = ir.types.get(&key).expect("DerivedCode") else {
        panic!("DerivedCode is not a simple type: {:?}", ir.types.get(&key));
    };
    assert_eq!(
        d.facets.patterns.len(),
        2,
        "derived type must inherit base patterns (AND across derivation steps); got {:?}",
        d.facets.patterns
    );
    assert_eq!(
        d.base_type,
        TypeRef::Named(QName::new(Some("urn:pat"), "BaseCode"))
    );

    // Pydantic must enforce ALL inherited patterns, not just the first:
    // `Field(pattern=...)` holds a single regex, so multiple patterns are
    // AND-combined through an AfterValidator over the module helper.
    let py = PythonCodegen::new(PythonOptions {
        backend: PythonBackend::Pydantic,
        ..PythonOptions::default()
    })
    .generate_module(&ir);
    assert!(py.contains("import re\n"), "`import re` missing:\n{py}");
    assert!(
        py.contains("def _polyxml_patterns("),
        "module pattern helper missing:\n{py}"
    );
    assert!(
        py.contains(r#"AfterValidator(_polyxml_patterns(r"[A-Z]{6}[A-Z0-9]{3}", r"[A-Z]{6}"))"#),
        "both patterns must be AND-combined via AfterValidator:\n{py}"
    );
    // Single-pattern types keep Field(pattern=...), with full-value anchors.
    assert!(
        py.contains(r#"Field(pattern=r"\A(?:[A-Z]{6})\z")"#),
        "single-pattern Field kwarg missing:\n{py}"
    );
}

// ---------------------------------------------------------------------------
// include cache / chameleon namespace attribution
// ---------------------------------------------------------------------------

#[test]
fn chameleon_include_adopts_including_namespace_per_includer() {
    let dir = tempdir().expect("tempdir");
    let common = dir.path().join("common.xsd");
    fs::write(
        &common,
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
        <xs:complexType name="CommonThing">
            <xs:sequence>
                <xs:element name="Payload" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#,
    )
    .expect("write common.xsd");

    let a = dir.path().join("a.xsd");
    fs::write(
        &a,
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                targetNamespace="urn:a" xmlns:t="urn:a" elementFormDefault="qualified">
        <xs:include schemaLocation="common.xsd"/>
        <xs:complexType name="AEnvelope">
            <xs:sequence>
                <xs:element name="Body" type="t:CommonThing"/>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#,
    )
    .expect("write a.xsd");

    let b = dir.path().join("b.xsd");
    fs::write(
        &b,
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
                targetNamespace="urn:b" xmlns:t="urn:b" elementFormDefault="qualified">
        <xs:include schemaLocation="common.xsd"/>
        <xs:complexType name="BEnvelope">
            <xs:sequence>
                <xs:element name="Body" type="t:CommonThing"/>
            </xs:sequence>
        </xs:complexType>
    </xs:schema>"#,
    )
    .expect("write b.xsd");

    // One parser instance: the visited-file cache must not drop the second
    // includer's chameleon copy.
    let mut parser = XsdParser::new();

    let ir_a = parser.parse_file(&a).expect("parse a.xsd");
    assert!(
        ir_a.types
            .contains_key(&QName::new(Some("urn:a"), "CommonThing")),
        "chameleon include types must adopt includer namespace urn:a; keys: {:?}",
        ir_a.types.keys().collect::<Vec<_>>()
    );

    let ir_b = parser.parse_file(&b).expect("parse b.xsd");
    assert!(
        ir_b.types
            .contains_key(&QName::new(Some("urn:b"), "CommonThing")),
        "second include of the chameleon file must be re-keyed to urn:b; keys: {:?}",
        ir_b.types.keys().collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// named aliases/newtypes for restricted simple types
// ---------------------------------------------------------------------------

#[test]
fn restricted_simple_types_emit_named_aliases() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:bic" elementFormDefault="qualified">
        <xs:simpleType name="BicCode">
            <xs:restriction base="xs:string">
                <xs:pattern value="[A-Z]{6}"/>
            </xs:restriction>
        </xs:simpleType>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse");

    // zero-copy mode may emit `pub type BicCode<'a> = Cow<'a, str>;`
    let rust = RustCodegen::new(RustOptions::default()).generate_module(&ir);
    assert!(
        rust.contains("pub type BicCode"),
        "rust alias missing:\n{rust}"
    );

    let python = PythonCodegen::new(PythonOptions::default()).generate_module(&ir);
    assert!(
        python.contains("type BicCode ="),
        "python alias missing:\n{python}"
    );

    let ts = TypeScriptCodegen::new(TypeScriptOptions::default()).generate_module(&ir);
    assert!(
        ts.contains("export type BicCode ="),
        "typescript alias missing:\n{ts}"
    );

    let go = GoCodegen::new(GoOptions::default()).generate_module(&ir);
    assert!(go.contains("type BicCode "), "go alias missing:\n{go}");

    let cpp = CppCodegen::new(CppOptions::default()).generate_module(&ir);
    assert!(cpp.contains("using BicCode ="), "cpp alias missing:\n{cpp}");

    // Java and C# have no alias syntax for this; wrapper types are by design —
    // the requirement is that the named type exists at all.
    let java = JavaCodegen::new(JavaOptions::default()).generate_module(&ir, "Audit");
    assert!(java.contains("BicCode"), "java type missing:\n{java}");

    let csharp = CSharpCodegen::new(CSharpOptions::default()).generate_module(&ir);
    assert!(csharp.contains("BicCode"), "csharp type missing:\n{csharp}");
}

#[test]
fn diamond_includes_do_not_duplicate_upa_source_documents() {
    let dir = tempdir().unwrap();
    for index in 0usize..16 {
        let includes = (index.saturating_sub(2)..index)
            .map(|previous| format!("<xs:include schemaLocation=\"{previous}.xsd\"/>"))
            .collect::<String>();
        fs::write(dir.path().join(format!("{index}.xsd")), format!("<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">{includes}<xs:complexType name=\"T{index}\"><xs:sequence><xs:element name=\"item{index}\" type=\"xs:string\"/></xs:sequence></xs:complexType></xs:schema>")).unwrap();
    }
    let mut parser = XsdParser::new();
    let ir = parser.parse_file(dir.path().join("15.xsd")).unwrap();
    assert_eq!(ir.types.len(), 16);
    assert_eq!(
        ir.upa_documents.len(),
        16,
        "each unique source document is retained once"
    );
    assert_eq!(
        parser
            .parse_file(dir.path().join("15.xsd"))
            .unwrap()
            .upa_documents
            .len(),
        16
    );
}
