use polyxml::codegen::{
    CSharpCodegen, CSharpOptions, CppBackend, CppCodegen, CppMode, CppOptions, GoBackend,
    GoCodegen, GoOptions, JavaBackend, JavaCodegen, JavaOptions, PythonBackend, PythonCodegen,
    PythonOptions, RustCodegen, RustOptions, TypeScriptBackend, TypeScriptCodegen,
    TypeScriptOptions,
};
use polyxml::ir::{SchemaIR, TypeDef};
use polyxml::schema_parser::XsdParser;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{error, validate_inline_xsd};

/// Maximum XSD byte size accepted by the browser playground compiler.
pub const MAX_CODEGEN_XSD_BYTES: usize = 256 * 1024;
/// Maximum XML element count accepted by the browser playground compiler.
pub const MAX_CODEGEN_XSD_ELEMENTS: usize = 4_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompileStats {
    target_namespace: Option<String>,
    types: usize,
    structs: usize,
    enums: usize,
    unions: usize,
    simple_types: usize,
    roots: Vec<String>,
}

#[derive(Debug, Serialize)]
struct GeneratedFile {
    name: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct CompileResponse {
    language: String,
    variant: String,
    files: Vec<GeneratedFile>,
    stats: CompileStats,
}

/// Compile a self-contained XSD in the browser and return JSON with generated files and IR stats.
#[wasm_bindgen]
pub fn compile_xsd(xsd: &str, lang: &str, variant: Option<String>) -> Result<String, JsValue> {
    compile_xsd_json(xsd, lang, variant.as_deref()).map_err(error)
}

fn collect_stats(ir: &SchemaIR) -> CompileStats {
    let mut structs = 0usize;
    let mut enums = 0usize;
    let mut unions = 0usize;
    let mut simple_types = 0usize;

    for def in ir.types.values() {
        match def {
            TypeDef::Struct(_) => structs += 1,
            TypeDef::Enum(_) => enums += 1,
            TypeDef::Union(_) => unions += 1,
            TypeDef::Simple(_) => simple_types += 1,
        }
    }

    CompileStats {
        target_namespace: ir.target_namespace.clone(),
        types: ir.types.len(),
        structs,
        enums,
        unions,
        simple_types,
        roots: ir.elements.keys().map(|q| q.local.clone()).collect(),
    }
}

fn unknown_variant(lang: &str, variant: &str) -> String {
    format!("Unknown variant '{variant}' for language '{lang}'")
}

fn validate_codegen_xsd(xsd: &str) -> Result<(), String> {
    if xsd.trim().is_empty() {
        return Err("Input is empty — paste a W3C XSD <xs:schema> document.".into());
    }
    if xsd.len() > MAX_CODEGEN_XSD_BYTES {
        return Err(format!(
            "Schema exceeds browser playground limit (256 KiB; got {:.1} KiB). For multi-megabyte schemas, use the native CLI: polyxml generate",
            xsd.len() as f64 / 1024.0
        ));
    }

    validate_inline_xsd(xsd)?;

    let mut reader = Reader::from_str(xsd);
    let mut element_count = 0usize;
    let mut saw_root = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(tag) | Event::Empty(tag)) => {
                if !saw_root {
                    saw_root = true;
                    if tag.local_name().as_ref() != "schema" {
                        return Err("Expected a W3C XSD <xs:schema> root element.".into());
                    }
                }
                element_count += 1;
                if element_count > MAX_CODEGEN_XSD_ELEMENTS {
                    return Err(format!(
                        "Schema exceeds browser playground limit ({MAX_CODEGEN_XSD_ELEMENTS} XML elements). Use the native CLI: polyxml generate"
                    ));
                }
            }
            Ok(Event::Eof) => {
                if !saw_root {
                    return Err("Expected a W3C XSD <xs:schema> root element.".into());
                }
                return Ok(());
            }
            Err(err) => return Err(err.to_string()),
            _ => {}
        }
    }
}

/// Compile an inline XSD into generated source files and IR metadata serialized as JSON.
pub fn compile_xsd_json(xsd: &str, lang: &str, variant: Option<&str>) -> Result<String, String> {
    validate_codegen_xsd(xsd)?;
    let ir = XsdParser::new().parse_str(xsd).map_err(|e| e.to_string())?;
    let stats = collect_stats(&ir);

    let normalized_lang = lang.trim().to_ascii_lowercase();
    let normalized_variant = variant.map(|v| v.trim().to_ascii_lowercase());

    let (canonical_variant, files) = match normalized_lang.as_str() {
        "rust" | "rs" => {
            let v = normalized_variant.as_deref().unwrap_or("zero-copy");
            let opts = match v {
                "zero-copy" => RustOptions::default(),
                "owned" => RustOptions {
                    zero_copy: false,
                    ..RustOptions::default()
                },
                "phf" => RustOptions {
                    phf: true,
                    ..RustOptions::default()
                },
                _ => return Err(unknown_variant("rust", v)),
            };
            (
                v.to_string(),
                vec![GeneratedFile {
                    name: "models.rs".into(),
                    content: RustCodegen::new(opts).generate_module(&ir),
                }],
            )
        }
        "python" | "py" => {
            let v = normalized_variant.as_deref().unwrap_or("dataclass");
            let backend = match v {
                "dataclass" => PythonBackend::Dataclass,
                "pydantic" => PythonBackend::Pydantic,
                _ => return Err(unknown_variant("python", v)),
            };
            let opts = PythonOptions {
                backend,
                ..PythonOptions::default()
            };
            (
                v.to_string(),
                vec![GeneratedFile {
                    name: "models.py".into(),
                    content: PythonCodegen::new(opts).generate_module(&ir),
                }],
            )
        }
        "cpp" | "c++" => {
            let v = normalized_variant.as_deref().unwrap_or("header");
            let (opts, filename) = match v {
                "header" => (CppOptions::default(), "models.hpp"),
                "glaze" => (
                    CppOptions {
                        backend: CppBackend::Glaze,
                        ..CppOptions::default()
                    },
                    "models.hpp",
                ),
                "module" => (
                    CppOptions {
                        mode: CppMode::Module,
                        ..CppOptions::default()
                    },
                    "models.cppm",
                ),
                _ => return Err(unknown_variant("cpp", v)),
            };
            let codegen = CppCodegen::new(opts);
            let content = if v == "module" {
                codegen.generate_module_unit(&ir, "polyxml.models")
            } else {
                codegen.generate_module(&ir)
            };
            (
                v.to_string(),
                vec![GeneratedFile {
                    name: filename.into(),
                    content,
                }],
            )
        }
        "java" => {
            let v = normalized_variant.as_deref().unwrap_or("record");
            let opts = match v {
                "record" => JavaOptions::default(),
                "jackson3" => JavaOptions {
                    backend: JavaBackend::Jackson3,
                    use_records: true,
                    bean_validation: true,
                    ..JavaOptions::default()
                },
                "pojo" => JavaOptions {
                    backend: JavaBackend::Jackson3,
                    use_records: false,
                    emit_builder: true,
                    ..JavaOptions::default()
                },
                _ => return Err(unknown_variant("java", v)),
            };
            let codegen = JavaCodegen::new(opts);
            let generated = codegen.generate_files(&ir);
            let files = if generated.is_empty() {
                vec![GeneratedFile {
                    name: "Models.java".into(),
                    content: codegen.generate_module(&ir, "Models"),
                }]
            } else {
                generated
                    .into_iter()
                    .map(|(name, content)| GeneratedFile { name, content })
                    .collect()
            };
            (v.to_string(), files)
        }
        "go" => {
            let v = normalized_variant.as_deref().unwrap_or("standard");
            let backend = match v {
                "standard" => GoBackend::Standard,
                "sonic" => GoBackend::Sonic,
                "easyjson" => GoBackend::EasyJson,
                _ => return Err(unknown_variant("go", v)),
            };
            let opts = GoOptions {
                backend,
                ..GoOptions::default()
            };
            (
                v.to_string(),
                vec![GeneratedFile {
                    name: "models.go".into(),
                    content: GoCodegen::new(opts).generate_module(&ir),
                }],
            )
        }
        "typescript" | "ts" => {
            let v = normalized_variant.as_deref().unwrap_or("interface");
            let opts = match v {
                "interface" => TypeScriptOptions::default(),
                "zod" => TypeScriptOptions {
                    backend: TypeScriptBackend::Zod,
                    emit_zod: true,
                    ..TypeScriptOptions::default()
                },
                "valibot" => TypeScriptOptions {
                    backend: TypeScriptBackend::Valibot,
                    ..TypeScriptOptions::default()
                },
                "typebox" => TypeScriptOptions {
                    backend: TypeScriptBackend::TypeBox,
                    ..TypeScriptOptions::default()
                },
                _ => return Err(unknown_variant("typescript", v)),
            };
            (
                v.to_string(),
                vec![GeneratedFile {
                    name: "models.ts".into(),
                    content: TypeScriptCodegen::new(opts).generate_module(&ir),
                }],
            )
        }
        "csharp" | "cs" | "c#" => {
            let v = normalized_variant.as_deref().unwrap_or("record");
            let opts = match v {
                "record" => CSharpOptions::default(),
                "source-gen" => CSharpOptions {
                    use_records: true,
                    emit_source_gen: true,
                    ..CSharpOptions::default()
                },
                "class" => CSharpOptions {
                    use_records: false,
                    ..CSharpOptions::default()
                },
                _ => return Err(unknown_variant("csharp", v)),
            };
            (
                v.to_string(),
                vec![GeneratedFile {
                    name: "Models.cs".into(),
                    content: CSharpCodegen::new(opts).generate_module(&ir),
                }],
            )
        }
        _ => return Err(format!("Unsupported target language '{lang}'")),
    };

    let response = CompileResponse {
        language: normalized_lang,
        variant: canonical_variant,
        files,
        stats,
    };
    serde_json::to_string(&response).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SAMPLE_XSD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="urn:example:payments"
           xmlns:tns="urn:example:payments"
           elementFormDefault="qualified">
  <xs:simpleType name="CurrencyCode">
    <xs:restriction base="xs:string">
      <xs:enumeration value="USD"/>
      <xs:enumeration value="EUR"/>
    </xs:restriction>
  </xs:simpleType>
  <xs:complexType name="Amount">
    <xs:simpleContent>
      <xs:extension base="xs:decimal">
        <xs:attribute name="currency" type="tns:CurrencyCode" use="required"/>
      </xs:extension>
    </xs:simpleContent>
  </xs:complexType>
  <xs:complexType name="PaymentTransfer">
    <xs:sequence>
      <xs:element name="TransferId" type="xs:string"/>
      <xs:element name="Amount" type="tns:Amount"/>
      <xs:choice>
        <xs:element name="Iban" type="xs:string"/>
        <xs:element name="RoutingNumber" type="xs:string"/>
      </xs:choice>
    </xs:sequence>
  </xs:complexType>
  <xs:element name="PaymentTransfer" type="tns:PaymentTransfer"/>
</xs:schema>"#;

    fn parse_response(lang: &str, variant: Option<&str>) -> Value {
        let json = compile_xsd_json(SAMPLE_XSD, lang, variant).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn compiles_all_seven_languages_and_reports_stats() {
        for (lang, expected_file, snippet) in [
            ("rust", "models.rs", "pub struct PaymentTransfer"),
            ("python", "models.py", "class PaymentTransfer"),
            ("cpp", "models.hpp", "struct PaymentTransfer"),
            (
                "java",
                "PaymentTransfer.java",
                "public record PaymentTransfer",
            ),
            ("go", "models.go", "type PaymentTransfer struct"),
            (
                "typescript",
                "models.ts",
                "export interface PaymentTransfer",
            ),
            ("csharp", "Models.cs", "public record PaymentTransfer"),
        ] {
            let parsed = parse_response(lang, None);
            assert_eq!(parsed["stats"]["targetNamespace"], "urn:example:payments");
            assert_eq!(parsed["stats"]["types"], 3);
            assert_eq!(parsed["stats"]["structs"], 2);
            assert_eq!(parsed["stats"]["enums"], 1);
            assert_eq!(parsed["stats"]["roots"][0], "PaymentTransfer");
            let files = parsed["files"].as_array().unwrap();
            assert!(!files.is_empty(), "{lang} emitted no files");
            let combined: String = files
                .iter()
                .map(|f| f["content"].as_str().unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                files.iter().any(|f| f["name"] == expected_file),
                "{lang} missing {expected_file}: {files:?}"
            );
            assert!(combined.contains(snippet), "{lang} missing {snippet}");
        }
    }

    #[test]
    fn supports_backend_variants_for_each_language() {
        let rust_owned = parse_response("rust", Some("owned"));
        assert!(!rust_owned["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Cow<'a, str>"));

        let rust_phf = parse_response("rust", Some("phf"));
        assert!(rust_phf["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("::phf::Map"));

        let py_pydantic = parse_response("python", Some("pydantic"));
        assert!(py_pydantic["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("BaseModel"));

        let cpp_glaze = parse_response("cpp", Some("glaze"));
        assert!(cpp_glaze["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("glz::meta"));

        let cpp_module = parse_response("cpp", Some("module"));
        assert_eq!(cpp_module["files"][0]["name"], "models.cppm");
        assert!(cpp_module["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("export module polyxml.models;"));

        let java_pojo = parse_response("java", Some("pojo"));
        assert!(java_pojo["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["content"]
                .as_str()
                .unwrap()
                .contains("public class PaymentTransfer")));

        let java_jackson3 = parse_response("java", Some("jackson3"));
        assert!(java_jackson3["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["content"]
                .as_str()
                .unwrap()
                .contains("jakarta.validation")));

        let go_sonic = parse_response("go", Some("sonic"));
        assert_eq!(go_sonic["variant"], "sonic");
        assert!(go_sonic["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("sonic:\""));

        let go_easyjson = parse_response("go", Some("easyjson"));
        assert_eq!(go_easyjson["variant"], "easyjson");
        assert!(go_easyjson["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("//easyjson:json"));

        let ts_zod = parse_response("typescript", Some("zod"));
        assert!(ts_zod["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("z.object"));

        let ts_valibot = parse_response("typescript", Some("valibot"));
        assert!(ts_valibot["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("v.object"));

        let ts_typebox = parse_response("typescript", Some("typebox"));
        assert!(ts_typebox["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Type.Object"));

        let cs_sg = parse_response("csharp", Some("source-gen"));
        assert!(cs_sg["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("JsonSerializerContext"));

        let cs_class = parse_response("csharp", Some("class"));
        assert!(cs_class["files"][0]["content"]
            .as_str()
            .unwrap()
            .contains("public class PaymentTransfer"));
    }

    #[test]
    fn rejects_external_references_upa_violations_and_unknown_options() {
        let imported = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
            <xs:include schemaLocation="other.xsd"/>
        </xs:schema>"#;
        let err = compile_xsd_json(imported, "rust", None).unwrap_err();
        assert!(err.contains("Inline XSD must not use"), "{err}");

        let upa_bad = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
            <xs:element name="Root">
              <xs:complexType>
                <xs:sequence>
                  <xs:element name="Item" type="xs:string" minOccurs="0"/>
                  <xs:element name="Item" type="xs:string"/>
                </xs:sequence>
              </xs:complexType>
            </xs:element>
        </xs:schema>"#;
        let err = compile_xsd_json(upa_bad, "rust", None).unwrap_err();
        assert!(err.contains("Unique Particle Attribution"), "{err}");

        let err = compile_xsd_json(SAMPLE_XSD, "ruby", None).unwrap_err();
        assert!(err.contains("Unsupported target language"), "{err}");

        let err = compile_xsd_json(SAMPLE_XSD, "rust", Some("unknown")).unwrap_err();
        assert!(err.contains("Unknown variant"), "{err}");
    }

    #[test]
    fn enforces_playground_size_depth_element_and_grammar_guardrails() {
        let err = compile_xsd_json("   \n\t ", "rust", None).unwrap_err();
        assert!(err.contains("Input is empty"), "{err}");

        let err = compile_xsd_json("<root/>", "rust", None).unwrap_err();
        assert!(err.contains("<xs:schema> root element"), "{err}");

        let doctype = r#"<!DOCTYPE xs:schema [<!ENTITY xxe "boom">]>
            <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"/>"#;
        let err = compile_xsd_json(doctype, "rust", None).unwrap_err();
        assert!(err.contains("DOCTYPE"), "{err}");

        let huge_payload = format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><!-- {} --></xs:schema>"#,
            "a".repeat(MAX_CODEGEN_XSD_BYTES + 128)
        );
        let err = compile_xsd_json(&huge_payload, "rust", None).unwrap_err();
        assert!(err.contains("256 KiB"), "{err}");

        let many_elements = format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{}</xs:schema>"#,
            r#"<xs:element name="e" type="xs:string"/>"#.repeat(MAX_CODEGEN_XSD_ELEMENTS + 1)
        );
        let err = compile_xsd_json(&many_elements, "rust", None).unwrap_err();
        assert!(err.contains("XML elements"), "{err}");

        let deep_open = "<xs:sequence>".repeat(crate::MAX_XSD_DEPTH + 1);
        let deep_close = "</xs:sequence>".repeat(crate::MAX_XSD_DEPTH + 1);
        let deeply_nested = format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="T">{deep_open}{deep_close}</xs:complexType></xs:schema>"#
        );
        let err = compile_xsd_json(&deeply_nested, "rust", None).unwrap_err();
        assert!(err.contains("nesting depth"), "{err}");
    }
}
