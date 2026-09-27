//! Execute generated validators, not just regex source assertions.
use polyxml::codegen::*;
use polyxml::ir::{QName, RestrictionFacets, SchemaIR, TypeDef};
use polyxml::schema_parser::XsdParser;
use std::{fs, path::Path, process::Command};
use tempfile::tempdir;

const SCHEMA: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:simpleType name="ZBase"><xs:restriction base="xs:string">
    <xs:pattern value="[A-Z]+"><xs:annotation><xs:documentation>letters</xs:documentation></xs:annotation></xs:pattern>
    <xs:pattern value="[0-9]+"/>
  </xs:restriction></xs:simpleType>
  <xs:simpleType name="Derived"><xs:restriction base="ZBase">
    <xs:pattern value="[A-Z]{3}"/><xs:pattern value="[0-9]{3}"/>
  </xs:restriction></xs:simpleType>
  <xs:simpleType name="Mixed"><xs:restriction base="Derived">
    <xs:pattern value=".*Z.*"/><xs:pattern value=".*9.*"/>
  </xs:restriction></xs:simpleType>
  <xs:simpleType name="Single"><xs:restriction base="xs:string"><xs:pattern value="[A-Z]+"/></xs:restriction></xs:simpleType>
  <xs:complexType name="Flag"><xs:simpleContent><xs:extension base="xs:boolean"/></xs:simpleContent></xs:complexType>
  <xs:complexType name="Document"><xs:sequence>
    <xs:element name="value" type="Mixed"/>
    <xs:element name="optional" type="Derived" minOccurs="0"/>
    <xs:element name="items" type="Derived" minOccurs="0" maxOccurs="unbounded"/>
  </xs:sequence></xs:complexType>
  <xs:element name="document" type="Document"/>
</xs:schema>"#;

const GREGORIAN_SCHEMA: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:complexType name="Calendar"><xs:sequence>
    <xs:element name="day" type="xs:gDay"/>
    <xs:element name="month" type="xs:gMonth"/>
    <xs:element name="year" type="xs:gYear"/>
    <xs:element name="yearMonth" type="xs:gYearMonth"/>
    <xs:element name="monthDay" type="xs:gMonthDay"/>
  </xs:sequence></xs:complexType>
</xs:schema>"#;

#[test]
fn gregorian_primitives_emit_validated_types_across_targets() {
    let ir = XsdParser::new().parse_str(GREGORIAN_SCHEMA).unwrap();
    assert_eq!(ir.types.len(), 6);
    let rust = RustCodegen::new(RustOptions::default()).generate_module(&ir);
    assert!(rust.contains("validate_GDay_patterns"));
    assert!(rust.contains("validate_GMonthDay_patterns"));
    let go = GoCodegen::new(GoOptions::default()).generate_module(&ir);
    assert!(go.contains("func (s GDay) Validate() error"));
    assert!(go.contains("func (s *GMonthDay) UnmarshalText(text []byte) error"));
    let cpp = CppCodegen::new(CppOptions::default()).generate_header(&ir);
    assert!(cpp.contains("validate_GDay_patterns"));
    let java = JavaCodegen::new(JavaOptions::default()).generate_module(&ir, "Models");
    assert!(java.contains("Pattern.compile(\"---"));
    let python = PythonCodegen::new(PythonOptions {
        backend: PythonBackend::Pydantic,
        ..Default::default()
    })
    .generate_module(&ir);
    assert!(python.contains("type GDay = Annotated[str, Field(pattern="));
    let ts = TypeScriptCodegen::new(TypeScriptOptions {
        backend: TypeScriptBackend::Zod,
        ..Default::default()
    })
    .generate_module(&ir);
    assert!(ts.contains("GDaySchema = z.string().regex("));
    let valibot = TypeScriptCodegen::new(TypeScriptOptions {
        backend: TypeScriptBackend::Valibot,
        ..Default::default()
    })
    .generate_module(&ir);
    assert!(valibot.contains("GDaySchema = v.pipe(v.string(), v.regex("));
    let typebox = TypeScriptCodegen::new(TypeScriptOptions {
        backend: TypeScriptBackend::TypeBox,
        ..Default::default()
    })
    .generate_module(&ir);
    assert!(typebox.contains("GDaySchema = Type.String({ pattern:"));
    let csharp = CSharpCodegen::new(CSharpOptions::default()).generate_module(&ir);
    assert!(csharp.contains("record GDay("));
    assert!(csharp.contains("Regex.IsMatch(Value.ToString()"));
}

fn schema() -> SchemaIR {
    let dir = tempdir().unwrap();
    let path = dir.path().join("pattern.xsd");
    fs::write(&path, SCHEMA).unwrap();
    XsdParser::new().parse_file(&path).unwrap()
}
fn patterns<'a>(ir: &'a SchemaIR, name: &str) -> &'a [String] {
    match &ir.types[&QName::new(None::<String>, name)] {
        TypeDef::Simple(s) => &s.facets.patterns,
        _ => panic!("expected simple type"),
    }
}
fn run(cmd: &mut Command) {
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "{cmd:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn available(program: &str) -> bool {
    Command::new(program).arg("--version").output().is_ok()
}

#[test]
fn pattern_restrictions_preserve_or_groups_and_inherited_and_steps() {
    let ir = schema();
    assert_eq!(patterns(&ir, "ZBase"), ["([A-Z]+)|([0-9]+)"]);
    assert_eq!(
        patterns(&ir, "Derived"),
        ["([A-Z]{3})|([0-9]{3})", "([A-Z]+)|([0-9]+)"]
    );
    assert_eq!(patterns(&ir, "Mixed").len(), 3);
    assert_eq!(patterns(&ir, "Single"), ["[A-Z]+"]);
    // Facets (not the whole IR, whose QName map keys serde_json rejects) must
    // survive serialization with their grouping intact.
    let facets = match &ir.types[&QName::new(None::<String>, "Mixed")] {
        TypeDef::Simple(s) => &s.facets,
        _ => panic!("expected simple type"),
    };
    let serialized = serde_json::to_string(facets).unwrap();
    let restored: RestrictionFacets = serde_json::from_str(&serialized).unwrap();
    assert_eq!(&restored.patterns, &facets.patterns);
}

#[test]
fn generated_go_pattern_validation_and_xml_roundtrip() {
    if !available("go") {
        return;
    }
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("go.mod"), "module patterns\n\ngo 1.22\n").unwrap();
    fs::write(
        dir.path().join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(&schema()),
    )
    .unwrap();
    fs::write(dir.path().join("models_test.go"), r#"package models
import("testing"; "encoding/xml")
func TestPatterns(t *testing.T) {
    for _, value := range []string{"ZZZ", "999"} {
        if err := Mixed(value).Validate(); err != nil { t.Fatal(value, err) }
        var doc Document
        if err := xml.Unmarshal([]byte("<document><value>"+value+"</value></document>"), &doc); err != nil { t.Fatal(err) }
        if err := doc.Validate(); err != nil { t.Fatal(err) }
        if _, err := xml.Marshal(doc); err != nil { t.Fatal(err) }
    }
    for _, value := range []string{"ABC", "123", "Z", "9", "abc", "xZZZy", ""} {
        if Mixed(value).Validate() == nil { t.Fatal("accepted", value) }
        var doc Document
        if xml.Unmarshal([]byte("<document><value>"+value+"</value></document>"), &doc) == nil { t.Fatal("read accepted", value) }
        if _, err := xml.Marshal(Document{Value: Mixed(value)}); err == nil { t.Fatal("write accepted", value) }
    }
    if ZBase("ABC").Validate()!=nil || ZBase("123").Validate()!=nil { t.Fatal("OR") }
    bad := Derived("x")
    if (Document{Value: Mixed("ZZZ"), Optional: &bad}).Validate()==nil {t.Fatal("optional")}
    if (Document{Value: Mixed("ZZZ"), Items: []Derived{bad}}).Validate()==nil {t.Fatal("list")}
}
"#).unwrap();
    run(Command::new("go").arg("test").current_dir(dir.path()));
}

#[test]
fn generated_cpp_pattern_validation() {
    if !available("g++") {
        return;
    }
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("models.hpp"),
        CppCodegen::new(CppOptions::default()).generate_header(&schema()),
    )
    .unwrap();
    fs::write(dir.path().join("main.cpp"), r#"#include "models.hpp"
#include <cassert>
using namespace polyxml::generated;
int main() {
    for (auto value : {"ZZZ", "999"}) { assert(validate_Mixed_patterns(value)); Document doc; doc.value=value; assert(doc.validate()); }
    for (auto value : {"ABC", "123", "Z", "9", "abc", "xZZZy", ""}) { assert(!validate_Mixed_patterns(value)); Document doc; doc.value=value; assert(!doc.validate()); }
    assert(validate_ZBase_patterns("ABC")); assert(validate_ZBase_patterns("123"));
    Document doc; doc.value="ZZZ"; doc.optional="x"; assert(!doc.validate());
    doc.optional.reset(); doc.items.push_back("x"); assert(!doc.validate());
}
"#).unwrap();
    run(Command::new("g++")
        .args(["-std=c++20", "main.cpp", "-o", "check"])
        .current_dir(dir.path()));
    run(&mut Command::new(dir.path().join("check")));
}

#[test]
fn generated_rust_pattern_validation_and_xml_roundtrip() {
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();
    let core = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        dir.path().join("Cargo.toml"),
        format!(
            r#"[package]
name="pattern-test"
version="0.0.0"
edition="2021"
[dependencies]
polyxml={{path={:?}}}
quick-xml={{version="0.42",features=["serialize"]}}
serde={{version="1",features=["derive"]}}
serde_json="1"
regex="1"
"#,
            core
        ),
    )
    .unwrap();
    fs::write(
        dir.path().join("src/models.rs"),
        RustCodegen::new(RustOptions::default()).generate_module(&schema()),
    )
    .unwrap();
    fs::write(
        dir.path().join("src/main.rs"),
        r#"mod models;
use models::*;
fn main() {
    for value in ["ZZZ", "999"] {
        assert!(validate_Mixed_patterns(value).is_ok());
        let xml=format!("<document><value>{value}</value></document>");
        let doc=Document::from_xml(&xml).unwrap();
        assert!(doc.to_xml().is_ok());
    }
    for value in ["ABC", "123", "Z", "9", "abc", "xZZZy", ""] {
        assert!(validate_Mixed_patterns(value).is_err());
        let xml=format!("<document><value>{value}</value></document>");
        assert!(Document::from_xml(&xml).is_err());
        let doc=Document { value: value.into(), ..Default::default() };
        assert!(doc.to_xml().is_err());
    }
    assert!(Document::from_xml("<document><value/></document>").is_err());
    assert!(Document::from_xml("<document/>").is_err());
    assert!(validate_ZBase_patterns("ABC").is_ok());
    assert!(validate_ZBase_patterns("123").is_ok());
    assert!(Flag::from_xml("<Flag>true</Flag>").is_ok());
    assert!(Flag::from_xml("<Flag>0</Flag>").is_ok());
    assert!(Flag::from_xml("<Flag>garbage</Flag>").is_err());
}
"#,
    )
    .unwrap();
    run(Command::new("cargo")
        .args(["run", "--quiet", "--offline"])
        .env(
            "CARGO_TARGET_DIR",
            core.join("../../target/pattern-codegen-tests"),
        )
        .current_dir(dir.path()));
}

#[test]
fn generated_python_pattern_validation() {
    let python = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.venv/bin/python");
    if !python.exists() {
        return;
    }
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("models.py"),
        PythonCodegen::new(PythonOptions {
            backend: PythonBackend::Pydantic,
            ..Default::default()
        })
        .generate_module(&schema()),
    )
    .unwrap();
    fs::write(
        dir.path().join("check.py"),
        r#"from models import Mixed, Derived, ZBase
from pydantic import TypeAdapter, ValidationError
mixed = TypeAdapter(Mixed)
for value in ('ZZZ', '999'):
    assert mixed.validate_python(value) == value
for value in ('ABC', '123', 'Z', '9', 'abc', 'xZZZy', ''):
    try:
        mixed.validate_python(value)
    except ValidationError:
        pass
    else:
        raise AssertionError(value)
for value in ('ABC', '123'):
    assert TypeAdapter(ZBase).validate_python(value) == value
"#,
    )
    .unwrap();
    run(Command::new(python).arg("check.py").current_dir(dir.path()));
}

#[test]
fn generated_java_pattern_validation_and_direct_codecs() {
    if !available("javac") {
        return;
    }
    let dir = tempdir().unwrap();
    let generator = JavaCodegen::new(JavaOptions {
        package_name: "patterns".into(),
        emit_direct_codec: true,
        ..Default::default()
    });
    let mut files = Vec::new();
    for (name, contents) in generator.generate_files(&schema()) {
        fs::write(dir.path().join(&name), contents).unwrap();
        files.push(name);
    }
    fs::write(dir.path().join("Check.java"), r#"package patterns;
import java.io.*;
public class Check {
    public static void main(String[] args) throws Exception {
        for (String value : new String[]{"ZZZ", "999"}) {
            new Mixed(value);
            var parsed = MixedCodec.readXml(new ByteArrayInputStream(("<Mixed>"+value+"</Mixed>").getBytes()));
            if (!parsed.value().equals(value)) throw new AssertionError();
            MixedCodec.writeXml(parsed, new ByteArrayOutputStream());
        }
        for (String value : new String[]{"ABC", "123", "Z", "9", "abc", "xZZZy", ""}) {
            try { new Mixed(value); throw new AssertionError(value); }
            catch (IllegalArgumentException expected) { }
        }
        new ZBase("ABC"); new ZBase("123");
    }
}
"#).unwrap();
    files.push("Check.java".into());
    run(Command::new("javac")
        .args(["-d", "."])
        .args(&files)
        .current_dir(dir.path()));
    run(Command::new("java")
        .args(["-cp", ".", "patterns.Check"])
        .current_dir(dir.path()));
}

#[test]
fn generated_csharp_pattern_validation() {
    if !available("dotnet") {
        return;
    }
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Check.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
    fs::write(
        dir.path().join("Models.cs"),
        CSharpCodegen::new(CSharpOptions::default()).generate_module(&schema()),
    )
    .unwrap();
    fs::write(dir.path().join("Program.cs"), r#"using Generated;
using System.ComponentModel.DataAnnotations;
static bool Valid(IValidatableObject value) => !value.Validate(new ValidationContext(value)).Any();
foreach(var value in new[]{"ZZZ", "999"}) if(!Valid(new Mixed(value))) throw new Exception(value);
foreach(var value in new[]{"ABC", "123", "Z", "9", "abc", "xZZZy", ""}) if(Valid(new Mixed(value))) throw new Exception(value);
if(!Valid(new ZBase("ABC")) || !Valid(new ZBase("123"))) throw new Exception("OR");
"#).unwrap();
    run(Command::new("dotnet")
        .args(["run", "--project", "Check.csproj", "--verbosity", "quiet"])
        .current_dir(dir.path()));
}

#[test]
fn generated_typescript_pattern_validation_all_backends() {
    // Optional installed toolchain: npm install --prefix <dir> zod@3 valibot@1
    // @sinclair/typebox@0.34 typescript@5; set POLYXML_TS_TEST_MODULES=<dir>/node_modules.
    let Ok(modules) = std::env::var("POLYXML_TS_TEST_MODULES") else {
        return;
    };
    let dir = tempdir().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&modules, dir.path().join("node_modules")).unwrap();
    for (name, backend) in [
        ("zod", TypeScriptBackend::Zod),
        ("valibot", TypeScriptBackend::Valibot),
        ("typebox", TypeScriptBackend::TypeBox),
    ] {
        fs::write(
            dir.path().join(format!("{name}.ts")),
            TypeScriptCodegen::new(TypeScriptOptions {
                backend,
                ..Default::default()
            })
            .generate_module(&schema()),
        )
        .unwrap();
    }
    fs::write(dir.path().join("check.ts"), r#"import { MixedSchema as z, ZBaseSchema as zb } from './zod';
import { MixedSchema as v, ZBaseSchema as vb } from './valibot';
import { MixedSchema as t, ZBaseSchema as tb } from './typebox';
import * as valibot from 'valibot';
import { Value } from '@sinclair/typebox/value';
for(const value of ['ZZZ','999']) {
    if(!z.safeParse(value).success || !valibot.safeParse(v,value).success || !Value.Check(t,value)) throw Error(value);
}
for(const value of ['ABC','123','Z','9','abc','xZZZy','']) {
    if(z.safeParse(value).success || valibot.safeParse(v,value).success || Value.Check(t,value)) throw Error(value);
}
for(const value of ['ABC','123']) {
    if(!zb.safeParse(value).success || !valibot.safeParse(vb,value).success || !Value.Check(tb,value)) throw Error('OR');
}
"#).unwrap();
    run(Command::new(Path::new(&modules).join(".bin/tsc"))
        .args([
            "check.ts",
            "--target",
            "es2020",
            "--module",
            "commonjs",
            "--strict",
            "--skipLibCheck",
        ])
        .current_dir(dir.path()));
    run(Command::new("node").arg("check.js").current_dir(dir.path()));
}
