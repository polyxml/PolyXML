use polyxml::codegen::{
    csharp::{CSharpCodegen, CSharpOptions},
    go::{GoCodegen, GoOptions},
};
use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use std::{fs, process::Command, sync::Arc};
use tempfile::tempdir;

#[test]
fn typed_lexical_lists_round_trip_across_codecs() {
    let ir = XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/list_simple_types.xsd"
        ))
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let value = polyxml::deserialize(
        b"<Root><Numbers>1 -2 3</Numbers><Words>a b</Words></Root>",
        Arc::clone(&schema),
    )
    .unwrap();
    assert_eq!(
        value.get("numbers"),
        Some(&polyxml::value::PolyValue::List(vec![
            polyxml::value::PolyValue::Int(1),
            polyxml::value::PolyValue::Int(-2),
            polyxml::value::PolyValue::Int(3)
        ]))
    );
    let output = polyxml::serialize("Root", &value, &schema, None).unwrap();
    assert!(String::from_utf8(output)
        .unwrap()
        .contains("<Numbers>1 -2 3</Numbers>"));
    assert!(
        polyxml::deserialize(b"<Root><Numbers>one two</Numbers><Words/></Root>", schema).is_err()
    );
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("go.mod"), "module lists\n\ngo 1.22\n").unwrap();
    fs::write(
        temp.path().join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(temp.path().join("models_test.go"),r#"package models
import("encoding/xml";"testing";"reflect";"strings")
func TestLists(t *testing.T){
 for _,text:=range []string{"1 -2 3","  1\t-2\n3  ",""}{var root Root;if err:=xml.Unmarshal([]byte(`<Root><Numbers>`+text+`</Numbers><Words>a b</Words></Root>`),&root);err!=nil{t.Fatal(err)};if text!=""&&!reflect.DeepEqual(root.Numbers,IntList{1,-2,3}){t.Fatal(root.Numbers)};data,err:=xml.Marshal(root);if err!=nil{t.Fatal(err)};if text!=""&&!strings.Contains(string(data),`<Numbers>1 -2 3</Numbers>`){t.Fatal(string(data))};var second Root;if err:=xml.Unmarshal(data,&second);err!=nil{t.Fatal(err)}}
 var root Root;if err:=xml.Unmarshal([]byte(`<Root><Numbers>one two</Numbers><Words/></Root>`),&root);err==nil{t.Fatal("invalid items accepted")}
 root.Words=WordList{"a b"};if _,err:=xml.Marshal(root);err==nil{t.Fatal("invalid output accepted")}
}"#).unwrap();
    let result = Command::new("go")
        .args(["test", "./..."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    for use_records in [true, false] {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("App.csproj"),r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
        fs::write(
            temp.path().join("Models.cs"),
            CSharpCodegen::new(CSharpOptions {
                namespace: "Models".into(),
                use_records,
                ..Default::default()
            })
            .generate_module(&ir),
        )
        .unwrap();
        fs::write(temp.path().join("Program.cs"),r#"using Models;using System.Xml.Serialization;
var serializer=new XmlSerializer(typeof(Root));
foreach(var text in new[]{"1 -2 3","  1\t-2\n3  ",""}){
var root=(Root)serializer.Deserialize(new StringReader("<Root><Numbers>"+text+"</Numbers><Words>a b</Words></Root>"))!;
if(text.Length>0&&!root.Numbers.Value.SequenceEqual(new[]{1,-2,3}))throw new Exception("not typed items");
var writer=new StringWriter();serializer.Serialize(writer,root);
if(text.Length>0&&!writer.ToString().Contains("<Numbers>1 -2 3</Numbers>"))throw new Exception(writer.ToString());
}
try{serializer.Deserialize(new StringReader("<Root><Numbers>one two</Numbers><Words/></Root>"));throw new Exception("invalid input accepted");}catch(InvalidOperationException){}
"#).unwrap();
        let result = Command::new("dotnet")
            .arg("run")
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn named_list_items_enforce_facets_and_enumerations() {
    let ir = XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/restricted_list_items.xsd"
        ))
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    for document in [
        "<Root><Numbers>-1</Numbers><Colors>A</Colors></Root>",
        "<Root><Numbers>2</Numbers><Colors>INVALID</Colors></Root>",
    ] {
        assert!(polyxml::deserialize(document.as_bytes(), Arc::clone(&schema)).is_err());
    }
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("go.mod"),
        "module restrictedlists\n\ngo 1.22\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(temp.path().join("models_test.go"),r#"package models
import("encoding/xml";"testing")
func TestRestrictions(t *testing.T){for _,doc:=range []string{`<Root><Numbers>-1</Numbers><Colors>A</Colors></Root>`,`<Root><Numbers>2</Numbers><Colors>INVALID</Colors></Root>`}{var root Root;if err:=xml.Unmarshal([]byte(doc),&root);err==nil{t.Fatal("invalid item accepted")}};var root Root;if err:=xml.Unmarshal([]byte(`<Root><Numbers>0 2</Numbers><Colors>A B</Colors></Root>`),&root);err!=nil{t.Fatal(err)};if _,err:=xml.Marshal(root);err!=nil{t.Fatal(err)}}"#).unwrap();
    let result = Command::new("go")
        .args(["test", "./..."])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("App.csproj"),r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
    fs::write(
        temp.path().join("Models.cs"),
        CSharpCodegen::new(CSharpOptions {
            namespace: "Models".into(),
            ..Default::default()
        })
        .generate_module(&ir),
    )
    .unwrap();
    fs::write(temp.path().join("Program.cs"),r#"using Models;using System.Xml.Serialization;
var serializer=new XmlSerializer(typeof(Root));foreach(var xml in new[]{"<Root><Numbers>-1</Numbers><Colors>A</Colors></Root>","<Root><Numbers>2</Numbers><Colors>INVALID</Colors></Root>"}){try{serializer.Deserialize(new StringReader(xml));throw new Exception("invalid item accepted");}catch(InvalidOperationException){}}
var root=(Root)serializer.Deserialize(new StringReader("<Root><Numbers>0 2</Numbers><Colors>A B</Colors></Root>"))!;serializer.Serialize(new StringWriter(),root);
"#).unwrap();
    let result = Command::new("dotnet")
        .arg("run")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
