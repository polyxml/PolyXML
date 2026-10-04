//! Compile generated enum consumers and compare runtime values with independent
//! data files. Source substring checks cannot catch escaping that changes values.
use std::{fs, path::Path, process::Command};

use polyxml::codegen::*;
use polyxml::ir::{QName, TypeDef};
use polyxml::schema_parser::XsdParser;

fn run(directory: &Path, program: &str, args: &[&str]) {
    let output = Command::new(program)
        .args(args)
        .current_dir(directory)
        .env("CARGO_BUILD_JOBS", "1")
        .output()
        .unwrap_or_else(|error| panic!("running {program}: {error}"));
    assert!(
        output.status.success(),
        "{program} {args:?}:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn enum_literals_compile_and_preserve_values_in_all_seven_languages() {
    let ir = XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/enum_literal_escaping.xsd"
        ))
        .unwrap();
    let TypeDef::Enum(status) = &ir.types[&QName::local("Status")] else {
        panic!("expected Status enum");
    };
    let expected = [
        "R&D",
        "\"quoted\" 'value' <tag>",
        "café 😀",
        "literal &amp;",
        "slash\\path\\u000a",
        "tab\tline\ncarriage\rtail",
        "\u{85}next\u{2028}line",
    ];
    assert_eq!(
        status
            .variants
            .iter()
            .map(|v| v.value.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    let root = tempfile::tempdir().unwrap();
    let json = serde_json::to_vec(&expected).unwrap();
    let mut binary = Vec::new();
    for value in expected {
        binary.extend_from_slice(&(value.len() as u32).to_le_bytes());
        binary.extend_from_slice(value.as_bytes());
    }
    let consumer = |name: &str| {
        let path = root.path().join(name);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("expected.json"), &json).unwrap();
        fs::write(path.join("expected.bin"), &binary).unwrap();
        path
    };

    let rust = consumer("rust");
    fs::create_dir(rust.join("src")).unwrap();
    let core = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::write(rust.join("Cargo.toml"), format!("[package]\nname=\"enum-literal-test\"\nversion=\"0.0.0\"\nedition=\"2021\"\n[dependencies]\npolyxml={{path={core:?}}}\nquick-xml=\"0.42\"\nserde={{version=\"1\",features=[\"derive\"]}}\nserde_json=\"1\"\n")).unwrap();
    fs::write(
        rust.join("src/models.rs"),
        RustCodegen::new(RustOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(rust.join("src/main.rs"), r#"mod models;
fn main() {
 let expected: Vec<String> = serde_json::from_slice(&std::fs::read("expected.json").unwrap()).unwrap();
 for text in expected {
   let value: models::Status = text.parse().unwrap();
   assert_eq!(value.as_str(), text);
   let json = serde_json::to_string(&value).unwrap();
   assert_eq!(serde_json::from_str::<String>(&json).unwrap(), text);
   assert_eq!(serde_json::from_str::<models::Status>(&json).unwrap(), value);
 }
}"#).unwrap();
    let output = Command::new("cargo")
        .args(["run", "--offline", "--quiet"])
        .env(
            "CARGO_TARGET_DIR",
            core.join("../../target/enum-literal-tests"),
        )
        .env("CARGO_BUILD_JOBS", "1")
        .current_dir(&rust)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Rust: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let python = consumer("python");
    fs::write(
        python.join("models.py"),
        PythonCodegen::new(PythonOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(python.join("check.py"), "import json\nfrom models import Status\nwith open('expected.json', encoding='utf-8') as f:\n    expected = json.load(f)\nassert [v.value for v in Status] == expected\nfor value in expected:\n    assert Status(value).value == value\n").unwrap();
    run(&python, "python3", &["check.py"]);

    let go = consumer("go");
    fs::write(go.join("go.mod"), "module literals\n\ngo 1.22\n").unwrap();
    fs::write(
        go.join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(go.join("models_test.go"), r#"package models
import("encoding/json";"os";"testing")
func TestValues(t *testing.T) {
 data,err:=os.ReadFile("expected.json");if err!=nil{t.Fatal(err)};var expected []string
 if err=json.Unmarshal(data,&expected);err!=nil{t.Fatal(err)}
 for _,text:=range expected{value:=Status(text);if !value.IsValid(){t.Fatal(text)};encoded,err:=json.Marshal(value);if err!=nil{t.Fatal(err)};var actual string;if err=json.Unmarshal(encoded,&actual);err!=nil{t.Fatal(err)};if actual!=text{t.Fatal(actual,text)}}
}"#).unwrap();
    run(&go, "go", &["test", "./..."]);

    let cpp = consumer("cpp");
    fs::write(
        cpp.join("models.hpp"),
        CppCodegen::new(CppOptions::default()).generate_header(&ir),
    )
    .unwrap();
    fs::write(
        cpp.join("main.cpp"),
        r#"#include "models.hpp"
#include <fstream>
#include <stdexcept>
int main(){std::ifstream input("expected.bin",std::ios::binary);unsigned char size[4];
 while(input.read(reinterpret_cast<char*>(size),4)){
 unsigned count=size[0]|(unsigned(size[1])<<8)|(unsigned(size[2])<<16)|(unsigned(size[3])<<24);
 std::string expected(count,'\0');input.read(expected.data(),count);
 auto value=polyxml::generated::status_from_string(expected);
 if(!value||polyxml::generated::to_string(*value)!=expected)throw std::runtime_error(expected);
 }}"#,
    )
    .unwrap();
    run(
        &cpp,
        "c++",
        &[
            "-std=c++20",
            "-Wall",
            "-Wextra",
            "-Werror",
            "main.cpp",
            "-o",
            "check",
        ],
    );
    run(&cpp, "./check", &[]);

    let java = consumer("java");
    fs::write(
        java.join("Models.java"),
        JavaCodegen::new(JavaOptions {
            package_name: String::new(),
            ..Default::default()
        })
        .generate_module(&ir, "Models"),
    )
    .unwrap();
    fs::write(java.join("Check.java"), r#"import java.nio.*;import java.nio.file.*;import java.nio.charset.*;
public class Check {public static void main(String[] args)throws Exception{
 ByteBuffer input=ByteBuffer.wrap(Files.readAllBytes(Path.of("expected.bin"))).order(ByteOrder.LITTLE_ENDIAN);
 while(input.hasRemaining()){byte[] bytes=new byte[input.getInt()];input.get(bytes);String text=new String(bytes,StandardCharsets.UTF_8);
 if(!Models.Status.fromValue(text).getValue().equals(text))throw new AssertionError(text);
 }}}
"#).unwrap();
    run(&java, "javac", &["Models.java", "Check.java"]);
    run(&java, "java", &["Check"]);

    let csharp = consumer("csharp");
    fs::write(csharp.join("Check.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>").unwrap();
    fs::write(
        csharp.join("Models.cs"),
        CSharpCodegen::new(CSharpOptions {
            namespace: "Models".into(),
            ..Default::default()
        })
        .generate_module(&ir),
    )
    .unwrap();
    fs::write(
        csharp.join("Program.cs"),
        r#"using Models;using System.Text;using System.Xml.Serialization;using System.Reflection;
using var reader=new BinaryReader(File.OpenRead("expected.bin"));
foreach(var value in Enum.GetValues<Status>()){
 string expected=Encoding.UTF8.GetString(reader.ReadBytes(reader.ReadInt32()));
 if(value.ToXmlValue()!=expected)throw new Exception(expected);
 var attribute=typeof(Status).GetField(value.ToString())!.GetCustomAttribute<XmlEnumAttribute>()!;
 if(attribute.Name!=expected)throw new Exception(attribute.Name);
}
if(reader.BaseStream.Position!=reader.BaseStream.Length)throw new Exception("missing variants");
"#,
    )
    .unwrap();
    run(
        &csharp,
        "dotnet",
        &["run", "--project", "Check.csproj", "--verbosity", "quiet"],
    );

    let typescript = consumer("typescript");
    fs::write(
        typescript.join("models.ts"),
        TypeScriptCodegen::new(TypeScriptOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(typescript.join("check.cjs"), "const fs=require('fs');const {Status}=require('./models.js');const assert=require('assert');assert.deepStrictEqual(Object.values(Status),JSON.parse(fs.readFileSync('expected.json','utf8')));\n").unwrap();
    run(
        &typescript,
        "tsc",
        &[
            "--target",
            "es2020",
            "--module",
            "commonjs",
            "--strict",
            "models.ts",
        ],
    );
    run(&typescript, "node", &["check.cjs"]);
}
