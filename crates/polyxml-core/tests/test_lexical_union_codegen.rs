use polyxml::codegen::cpp::{CppCodegen, CppOptions};
use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions};
use polyxml::codegen::go::{GoCodegen, GoOptions};
use polyxml::codegen::java::{JavaCodegen, JavaOptions};
use polyxml::codegen::python::{PythonCodegen, PythonOptions};
use polyxml::codegen::rust::{RustCodegen, RustOptions};
use polyxml::codegen::typescript::{TypeScriptBackend, TypeScriptCodegen, TypeScriptOptions};
use polyxml::ir::{QName, TypeDef};
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize, ModelSchema, PolyValue};
use std::sync::Arc;

#[test]
fn lexical_union_xml_selects_first_valid_member_and_round_trips() {
    let ir = XsdParser::new()
        .parse_str(include_str!("fixtures/lexical_union.xsd"))
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("payload")).unwrap();
    for (number, code, expected_number) in [
        ("42", "LATE", PolyValue::Int(42)),
        ("---27", "ABC12", PolyValue::String("---27".into())),
    ] {
        let xml = format!("<payload><number>{number}</number><code>{code}</code></payload>");
        let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
        let PolyValue::Record { values, .. } = &value else {
            panic!("expected record")
        };
        let number_index = schema
            .fields
            .iter()
            .position(|f| f.name == "number")
            .unwrap();
        assert_eq!(values[number_index], Some(expected_number));
        let encoded = serialize("payload", &value, &schema, None).unwrap();
        assert_eq!(deserialize(&encoded, Arc::clone(&schema)).unwrap(), value);
    }
    let invalid = b"<payload><number>---99</number><code>abc</code></payload>";
    assert!(deserialize(invalid, schema).is_err());

    let dated = ModelSchema::from_ir(&ir, Some("datedPayload")).unwrap();
    for (lexical, expected) in [
        ("42", PolyValue::Int(42)),
        ("2026-09-27", PolyValue::String("2026-09-27".into())),
    ] {
        let xml = format!("<datedPayload><dateValue>{lexical}</dateValue></datedPayload>");
        let value = deserialize(xml.as_bytes(), Arc::clone(&dated)).unwrap();
        let PolyValue::Record { values, .. } = value else {
            panic!("expected record")
        };
        assert_eq!(values[0], Some(expected));
    }
    assert!(deserialize(
        b"<datedPayload><dateValue>2026-02-30</dateValue></datedPayload>",
        dated
    )
    .is_err());
}

#[test]
fn lexical_unions_are_typed_in_all_targets() {
    let ir = XsdParser::new()
        .parse_str(include_str!("fixtures/lexical_union.xsd"))
        .unwrap();
    let number = match &ir.types[&QName::new(Some("urn:polyxml:lexical-union"), "NumberOrDay")] {
        TypeDef::Union(u) => u,
        _ => panic!("NumberOrDay must be a union"),
    };
    assert!(number.is_lexical());
    assert_eq!(number.branches.len(), 2);

    let rust = RustCodegen::new(RustOptions::default()).generate_module(&ir);
    assert!(rust.contains("pub enum NumberOrDay"));
    assert!(rust.contains("if let Ok(value) = s.parse::<i32>()"));
    assert!(rust.contains("validate_GDay_patterns(s).is_ok()"));
    assert!(rust.contains("ValueConverter::parse_scalar(&polyxml::ScalarType::XmlDate"));
    assert!(rust.contains("#[serde(untagged)]"));

    let ts = TypeScriptCodegen::new(TypeScriptOptions {
        backend: TypeScriptBackend::Zod,
        ..Default::default()
    })
    .generate_module(&ir);
    assert!(ts.contains("export type NumberOrDay = number | GDay;"));
    assert!(ts.contains("export const NumberOrDaySchema = z.union(["));
    assert!(ts.contains("export type IntegerOrDate = number | string;"));

    let python = PythonCodegen::new(PythonOptions::default()).generate_module(&ir);
    assert!(python.contains("type NumberOrDay = int | GDay"));

    let go = GoCodegen::new(GoOptions::default()).generate_module(&ir);
    assert!(go.contains("type NumberOrDay struct"));
    assert!(go.contains("func (c *NumberOrDay) UnmarshalXML"));
    assert!(go.contains("strconv.ParseInt(value, 10, 64)"));
    assert!(go.contains("time.Parse(\"2006-01-02\", value)"));

    let cpp = CppCodegen::new(CppOptions::default()).generate_module(&ir);
    assert!(cpp.contains("using NumberOrDay = std::variant<"));

    let csharp = CSharpCodegen::new(CSharpOptions::default()).generate_module(&ir);
    assert!(csharp.contains("public abstract record NumberOrDay"));
    assert!(csharp.contains("public static NumberOrDay Parse(string text)"));
    assert!(csharp.contains("public string? NumberXml"));
    assert!(csharp.contains("DateOnly.TryParseExact(value, \"yyyy-MM-dd\""));

    let java = JavaCodegen::new(JavaOptions::default()).generate_module(&ir, "Models");
    assert!(java.contains("sealed interface NumberOrDay"));
}
