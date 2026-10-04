//! Compile inherited text consumers; verify codecs where the target supplies them.
use polyxml::codegen::*;
use polyxml::schema_parser::XsdParser;
use std::{fs, path::Path, process::Command};

fn run(directory: &Path, program: &str, args: &[&str]) {
    let output = Command::new(program)
        .args(args)
        .current_dir(directory)
        .env("CARGO_BUILD_JOBS", "1")
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
fn inherited_scalar_text_compiles_in_all_seven_languages() {
    let ir = XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/inherited_simple_content.xsd"
        ))
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let consumer = |name: &str| {
        let directory = root.path().join(name);
        fs::create_dir(&directory).unwrap();
        directory
    };
    let core = Path::new(env!("CARGO_MANIFEST_DIR"));
    for zero_copy in [true, false] {
        let rust = consumer(if zero_copy { "borrowed" } else { "owned" });
        fs::create_dir(rust.join("src")).unwrap();
        fs::write(rust.join("Cargo.toml"), format!("[package]\nname=\"inherited-text-test\"\nversion=\"0.0.0\"\nedition=\"2021\"\n[dependencies]\npolyxml={{path={core:?}}}\nquick-xml=\"0.42\"\nserde={{version=\"1\",features=[\"derive\"]}}\nserde_json=\"1\"\nregex=\"1\"\n")).unwrap();
        fs::write(
            rust.join("src/models.rs"),
            RustCodegen::new(RustOptions {
                zero_copy,
                ..Default::default()
            })
            .generate_module(&ir),
        )
        .unwrap();
        fs::write(rust.join("src/main.rs"), r##"mod models;
use models::*;
fn main() {
 let value=Leaf::from_xml("<Leaf unit='m' rank='4' tag='T'>7</Leaf>").unwrap();
 assert_eq!(value.value,7); assert_eq!(value.unit.as_deref(),Some("m"));
 assert_eq!(value.rank,4); assert_eq!(value.tag.as_deref(),Some("T"));
 let xml=value.to_xml_string().unwrap();assert_eq!(Leaf::from_xml(&xml).unwrap(),value);
 assert!(xml.contains(">7</Leaf>"));
 let json=serde_json::to_string(&value).unwrap();
 let expected:serde_json::Value=serde_json::from_str(r#"{"unit":"m","rank":4,"value":7,"tag":"T"}"#).unwrap();
 assert_eq!(serde_json::from_str::<serde_json::Value>(&json).unwrap(),expected);
 assert_eq!(serde_json::from_str::<Leaf>(&json).unwrap(),value);
 assert!(Leaf::from_xml("<Leaf>bad</Leaf>").is_err());
}"##).unwrap();
        let output = Command::new("cargo")
            .args(["run", "--offline", "--quiet"])
            .current_dir(&rust)
            .env("CARGO_BUILD_JOBS", "1")
            .env(
                "CARGO_TARGET_DIR",
                core.join("../../target/inherited-text-consumers"),
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Rust: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for backend in [PythonBackend::Dataclass, PythonBackend::Pydantic] {
        let python = consumer(if backend == PythonBackend::Dataclass {
            "python"
        } else {
            "pydantic"
        });
        fs::write(
            python.join("models.py"),
            PythonCodegen::new(PythonOptions {
                backend,
                ..Default::default()
            })
            .generate_module(&ir),
        )
        .unwrap();
        fs::write(
            python.join("check.py"),
            r#"import json
from models import Leaf
value=Leaf.from_xml('<Leaf unit="m" rank="4" tag="T">7</Leaf>')
assert (value.value,value.unit,value.rank,value.tag)==(7,'m',4,'T')
xml=value.to_xml()
assert b'>7</Leaf>' in xml, xml
assert Leaf.from_xml(xml)==value
assert json.loads(value.to_json())==dict(unit='m',rank=4,value=7,tag='T')
assert Leaf.from_json(value.to_json())==value
"#,
        )
        .unwrap();
        run(&python, "python3", &["check.py"]);
    }
    let go = consumer("go");
    fs::write(go.join("go.mod"), "module inherited\n\ngo 1.22\n").unwrap();
    fs::write(
        go.join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(go.join("models_test.go"),r#"package models
import("encoding/xml";"encoding/json";"testing";"bytes")
func TestInherited(t *testing.T){
 var value Leaf;if err:=xml.Unmarshal([]byte(`<Leaf unit="m" rank="4" tag="T">7</Leaf>`),&value);err!=nil{t.Fatal(err)}
 if value.Value!=7||value.Unit==nil||*value.Unit!="m"||value.Rank!=4||value.Tag==nil||*value.Tag!="T"{t.Fatalf("%+v",value)}
 data,err:=xml.Marshal(value);if err!=nil{t.Fatal(err)};if !bytes.Contains(data,[]byte(`>7</Leaf>`)){t.Fatal(string(data))}
 var again Leaf;if err=xml.Unmarshal(data,&again);err!=nil{t.Fatal(err)};if again.Value!=7{t.Fatal(again)}
 data,err=json.Marshal(value);if err!=nil{t.Fatal(err)};var object map[string]any;if err=json.Unmarshal(data,&object);err!=nil{t.Fatal(err)}
 if object["value"]!=float64(7)||object["unit"]!="m"||object["rank"]!=float64(4)||object["tag"]!="T"{t.Fatal(string(data))}
}
"#).unwrap();
    run(&go, "go", &["test", "./..."]);
    let cpp = consumer("cpp");
    fs::write(
        cpp.join("models.hpp"),
        CppCodegen::new(CppOptions::default()).generate_header(&ir),
    )
    .unwrap();
    fs::write(cpp.join("check.cpp"),r#"#include "models.hpp"
#include <type_traits>
#include <stdexcept>
int main(){polyxml::generated::Leaf value;static_assert(std::is_same_v<decltype(value.value),std::int32_t>);
 value.value=7;value.unit="m";value.rank=4;value.tag="T";
 if(value.value!=7||value.unit!="m"||value.rank!=4||value.tag!="T")throw std::runtime_error("inherited fields");}
"#).unwrap();
    run(
        &cpp,
        "c++",
        &[
            "-std=c++20",
            "-Wall",
            "-Wextra",
            "-Werror",
            "check.cpp",
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
    fs::write(java.join("Check.java"),r#"import java.util.Optional;
public class Check{public static void main(String[] args){
 var value=new Models.Leaf(Optional.of("m"),4,7,Optional.of("T"));
 if(value.value()!=7||!value.unit().orElseThrow().equals("m")||value.rank()!=4||!value.tag().orElseThrow().equals("T"))throw new AssertionError(value);
}}
"#).unwrap();
    run(&java, "javac", &["Models.java", "Check.java"]);
    run(&java, "java", &["Check"]);
    for use_records in [true, false] {
        let cs = consumer(if use_records { "cs-record" } else { "cs-class" });
        fs::write(cs.join("Check.csproj"),r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable><TreatWarningsAsErrors>true</TreatWarningsAsErrors></PropertyGroup></Project>"#).unwrap();
        fs::write(
            cs.join("Models.cs"),
            CSharpCodegen::new(CSharpOptions {
                namespace: "Models".into(),
                use_records,
                ..Default::default()
            })
            .generate_module(&ir),
        )
        .unwrap();
        fs::write(cs.join("Program.cs"),r#"using Models;using System.Xml.Serialization;using System.Text.Json;using System.Xml.Linq;
var serializer=new XmlSerializer(typeof(Leaf));
var value=(Leaf)serializer.Deserialize(new StringReader("<Leaf unit='m' rank='4' tag='T'>7</Leaf>"))!;
if(value.Value!=7||value.Unit!="m"||value.Rank!=4||value.Tag!="T")throw new Exception("inherited fields");
var writer=new StringWriter();serializer.Serialize(writer,value);
var xml=XDocument.Parse(writer.ToString()).Root!;if(xml.Value!="7")throw new Exception(writer.ToString());
var again=(Leaf)serializer.Deserialize(new StringReader(writer.ToString()))!;if(again.Value!=7||again.Unit!="m"||again.Rank!=4||again.Tag!="T")throw new Exception("round trip");
var json=JsonDocument.Parse(JsonSerializer.Serialize(value)).RootElement;
if(json.GetProperty("value").GetInt32()!=7||json.GetProperty("unit").GetString()!="m"||json.GetProperty("rank").GetInt32()!=4||json.GetProperty("tag").GetString()!="T")throw new Exception("JSON");
"#).unwrap();
        run(
            &cs,
            "dotnet",
            &["run", "--project", "Check.csproj", "--verbosity", "quiet"],
        );
    }
    let ts = consumer("typescript");
    fs::write(
        ts.join("models.ts"),
        TypeScriptCodegen::new(TypeScriptOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(ts.join("check.ts"),r#"import type {Leaf} from './models';
const value:Leaf={unit:'m',rank:4,value:7,tag:'T'};
if(value.value!==7||value.unit!=='m'||value.rank!==4||value.tag!=='T')throw Error('inherited fields');
"#).unwrap();
    run(
        &ts,
        "tsc",
        &[
            "--strict",
            "--target",
            "es2020",
            "--module",
            "commonjs",
            "models.ts",
            "check.ts",
        ],
    );
    run(&ts, "node", &["check.js"]);
}
