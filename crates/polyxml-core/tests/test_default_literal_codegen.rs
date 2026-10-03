//! Execute consumers: escaping must preserve values as well as valid source.
use polyxml::codegen::*;
use polyxml::schema_parser::XsdParser;
use std::{fs, path::Path, process::Command};

fn run(directory: &Path, program: &str, args: &[&str]) {
    let output = Command::new(program)
        .args(args)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{program} {args:?}:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_string_defaults_and_fixed_values_preserve_xml_characters() {
    let ir = XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/default_literal_escaping.xsd"
        ))
        .unwrap();
    let expected = "\"quoted\"\\path\\u000a\t\n\r\u{85}\u{2028}café 😀";
    let root = tempfile::tempdir().unwrap();
    let consumer = |name: &str| {
        let path = root.path().join(name);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("expected.txt"), expected).unwrap();
        path
    };
    let python = consumer("python");
    fs::write(
        python.join("models.py"),
        PythonCodegen::new(PythonOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(
        python.join("check.py"),
        r#"import dataclasses
from models import RootType
with open('expected.txt', encoding='utf-8', newline='') as f:
    expected = f.read()
model = RootType(fixed=expected)
assert model.value == expected, repr(model.value)
assert model.label == expected, repr(model.label)
fields = {f.name: f for f in dataclasses.fields(RootType)}
assert fields['value'].metadata['default'] == expected
assert fields['label'].metadata['default'] == expected
assert fields['fixed'].metadata['fixed'] == expected
"#,
    )
    .unwrap();
    run(&python, "python3", &["check.py"]);
    let pydantic = consumer("pydantic");
    fs::write(
        pydantic.join("models.py"),
        PythonCodegen::new(PythonOptions {
            backend: PythonBackend::Pydantic,
            ..Default::default()
        })
        .generate_module(&ir),
    )
    .unwrap();
    fs::write(
        pydantic.join("check.py"),
        r#"from models import RootType
with open('expected.txt', encoding='utf-8', newline='') as f:
    expected = f.read()
model = RootType(fixed=expected)
assert model.value == expected
assert model.label == expected
assert RootType.model_fields['value'].json_schema_extra['default'] == expected
assert RootType.model_fields['fixed'].json_schema_extra['fixed'] == expected
"#,
    )
    .unwrap();
    run(&pydantic, "python3", &["check.py"]);
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
#include <iterator>
#include <stdexcept>
int main() {
 std::ifstream input("expected.txt", std::ios::binary);
 std::string expected((std::istreambuf_iterator<char>(input)), std::istreambuf_iterator<char>());
 polyxml::generated::RootType model;
 if(model.value != expected) throw std::runtime_error("default mismatch");
}
"#,
    )
    .unwrap();
    run(&cpp, "g++", &["-std=c++20", "main.cpp", "-o", "check"]);
    run(&cpp, "./check", &[]);
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
        r#"using Models;
using System.ComponentModel.DataAnnotations;
var expected = File.ReadAllText("expected.txt");
var model = new RootType();
if(model.Label != expected || model.Fixed != expected) throw new Exception("initializer mismatch");
model.ValueXml = "";
if(model.Value != expected) throw new Exception("element default mismatch");
model.Fixed = expected;
try { model.Fixed = expected + "wrong"; throw new Exception("fixed value accepted"); }
catch(ValidationException) {}
model.LabelXml = expected;
if(model.Label != expected) throw new Exception("attribute proxy mismatch");
"#,
    )
    .unwrap();
    run(
        &csharp,
        "dotnet",
        &["run", "--project", "Check.csproj", "--verbosity", "quiet"],
    );
}
