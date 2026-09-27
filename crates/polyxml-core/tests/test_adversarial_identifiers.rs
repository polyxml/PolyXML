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
use polyxml::schema_parser::XsdParser;

const ADVERSARIAL_XSD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
           targetNamespace="https://example.com/adversarial"
           xmlns="https://example.com/adversarial"
           elementFormDefault="qualified">

  <xs:simpleType name="OperatorEnum">
    <xs:restriction base="xs:string">
      <xs:enumeration value="+"/>
      <xs:enumeration value="-"/>
      <xs:enumeration value="*"/>
      <xs:enumeration value="100%"/>
      <xs:enumeration value="2G"/>
      <xs:enumeration value="3G"/>
      <xs:enumeration value="urn:iso:std:123"/>
    </xs:restriction>
  </xs:simpleType>

  <xs:complexType name="Item">
    <xs:sequence>
      <!-- Property matching enclosing class name -->
      <xs:element name="Item" type="xs:string"/>
      <!-- Element and attribute name collision with 'status' -->
      <xs:element name="status" type="OperatorEnum"/>
      <!-- Language keywords -->
      <xs:element name="self" type="xs:string" minOccurs="0"/>
      <xs:element name="type" type="xs:string" minOccurs="0"/>
      <xs:element name="record" type="xs:string" minOccurs="0"/>
      <xs:element name="default" type="xs:string" minOccurs="0"/>
      <xs:element name="match" type="xs:string" minOccurs="0"/>
      <xs:element name="yield" type="xs:string" minOccurs="0"/>
      <xs:element name="import" type="xs:string" minOccurs="0"/>
    </xs:sequence>
    <!-- Attribute with same name as element 'status' -->
    <xs:attribute name="status" type="xs:string"/>
    <!-- Attribute with keyword name -->
    <xs:attribute name="class" type="xs:string"/>
  </xs:complexType>

  <xs:element name="root" type="Item"/>
</xs:schema>
"#;

#[test]
fn test_csharp_adversarial_compilation() {
    let ir = XsdParser::new()
        .parse_str(ADVERSARIAL_XSD)
        .expect("parse XSD");
    let codegen = CSharpCodegen::new(CSharpOptions {
        use_records: true,
        namespace: "Adversarial.Models".to_string(),
        ..Default::default()
    });
    let code = codegen.generate_module(&ir);

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Models.cs"), &code).unwrap();
    fs::write(
        dir.path().join("Test.csproj"),
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <TreatWarningsAsErrors>true</TreatWarningsAsErrors>
  </PropertyGroup>
</Project>"#,
    )
    .unwrap();

    let output = Command::new("dotnet")
        .current_dir(dir.path())
        .args(["build"])
        .output()
        .expect("failed to run dotnet build");

    assert!(
        output.status.success(),
        "dotnet build failed on adversarial identifiers:\nSTDOUT:\n{}\nSTDERR:\n{}\nCODE:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        code
    );
}

#[test]
fn test_go_adversarial_compilation() {
    let ir = XsdParser::new()
        .parse_str(ADVERSARIAL_XSD)
        .expect("parse XSD");
    let codegen = GoCodegen::new(GoOptions {
        package_name: "adversarial".to_string(),
        emit_xml_tags: true,
        ..Default::default()
    });
    let code = codegen.generate_module(&ir);

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("go.mod"), "module adversarial\n\ngo 1.22\n").unwrap();
    fs::write(dir.path().join("models.go"), &code).unwrap();

    let output = Command::new("go")
        .current_dir(dir.path())
        .args(["vet", "./..."])
        .output()
        .expect("failed to run go vet");

    assert!(
        output.status.success(),
        "go vet failed on adversarial identifiers:\nSTDOUT:\n{}\nSTDERR:\n{}\nCODE:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        code
    );
}

#[test]
fn test_cpp_adversarial_compilation() {
    let ir = XsdParser::new()
        .parse_str(ADVERSARIAL_XSD)
        .expect("parse XSD");
    let codegen = CppCodegen::new(CppOptions {
        namespace: "adversarial".to_string(),
        emit_enum_converters: true,
        ..Default::default()
    });
    let code = codegen.generate_header(&ir);

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("models.hpp"), &code).unwrap();
    fs::write(
        dir.path().join("main.cpp"),
        "#include \"models.hpp\"\nint main() { adversarial::Item item{}; return 0; }\n",
    )
    .unwrap();

    let output = Command::new("g++")
        .current_dir(dir.path())
        .args(["-std=c++20", "-c", "main.cpp", "-o", "main.o"])
        .output()
        .expect("failed to run g++");

    assert!(
        output.status.success(),
        "g++ compilation failed on adversarial identifiers:\nSTDOUT:\n{}\nSTDERR:\n{}\nCODE:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        code
    );
}

#[test]
fn test_python_adversarial_compilation() {
    let ir = XsdParser::new()
        .parse_str(ADVERSARIAL_XSD)
        .expect("parse XSD");
    let codegen = PythonCodegen::new(PythonOptions::default());
    let code = codegen.generate_module(&ir);

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("models.py");
    fs::write(&file_path, &code).unwrap();

    let output = Command::new("python3")
        .current_dir(dir.path())
        .args(["-m", "py_compile", "models.py"])
        .output()
        .expect("failed to run python3");

    assert!(
        output.status.success(),
        "python3 compilation failed on adversarial identifiers:\nSTDOUT:\n{}\nSTDERR:\n{}\nCODE:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        code
    );
}

#[test]
fn test_typescript_adversarial_compilation() {
    let ir = XsdParser::new()
        .parse_str(ADVERSARIAL_XSD)
        .expect("parse XSD");
    let codegen = TypeScriptCodegen::new(TypeScriptOptions::default());
    let code = codegen.generate_module(&ir);

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("models.ts");
    fs::write(&file_path, &code).unwrap();

    let output = Command::new("tsc")
        .current_dir(dir.path())
        .args(["--noEmit", "models.ts"])
        .output()
        .expect("failed to run tsc");

    assert!(
        output.status.success(),
        "tsc type check failed on adversarial identifiers:\nSTDOUT:\n{}\nSTDERR:\n{}\nCODE:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        code
    );
}

#[test]
fn test_java_adversarial_compilation() {
    let ir = XsdParser::new()
        .parse_str(ADVERSARIAL_XSD)
        .expect("parse XSD");
    let codegen = JavaCodegen::new(JavaOptions {
        package_name: "adversarial".to_string(),
        use_records: true,
        ..Default::default()
    });
    let files = codegen.generate_files(&ir);

    let dir = tempdir().unwrap();
    let mut file_names = Vec::new();
    for (filename, content) in files {
        fs::write(dir.path().join(&filename), &content).unwrap();
        file_names.push(filename);
    }

    let mut args = vec!["-d", "."];
    for f in &file_names {
        args.push(f.as_str());
    }

    let output = Command::new("javac")
        .current_dir(dir.path())
        .args(&args)
        .output()
        .expect("failed to run javac");

    assert!(
        output.status.success(),
        "javac compilation failed on adversarial identifiers:\nSTDOUT:\n{}\nSTDERR:\n{}\n",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_rust_adversarial_compilation() {
    let ir = XsdParser::new()
        .parse_str(ADVERSARIAL_XSD)
        .expect("parse XSD");
    let codegen = RustCodegen::new(RustOptions {
        derive_serde: false,
        emit_codecs: false,
        ..Default::default()
    });
    let code = codegen.generate_module(&ir);

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("lib.rs");
    fs::write(&file_path, &code).unwrap();

    let output = Command::new("rustc")
        .current_dir(dir.path())
        .args(["--crate-type=lib", "--emit=metadata", "lib.rs"])
        .output()
        .expect("failed to run rustc");

    assert!(
        output.status.success(),
        "rustc compilation failed on adversarial identifiers:\nSTDOUT:\n{}\nSTDERR:\n{}\nCODE:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        code
    );
}
