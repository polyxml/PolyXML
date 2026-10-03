use polyxml::codegen::{
    csharp::{CSharpCodegen, CSharpOptions},
    go::{GoCodegen, GoOptions},
};
use polyxml::schema_parser::XsdParser;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
fn global_attribute_references_round_trip_by_namespace() {
    let mut parser = XsdParser::new();
    let ir = parser
        .parse_file(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../research/fixtures/wave6/xlink_shared_attribute/root.xsd"
        ))
        .unwrap();
    assert_attribute_round_trip(&ir);
}

#[test]
fn imported_inline_global_attribute_enum_round_trips() {
    let fixture = tempdir().unwrap();
    fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../research/fixtures/wave6/xlink_shared_attribute/root.xsd"
        ),
        fixture.path().join("root.xsd"),
    )
    .unwrap();
    fs::write(fixture.path().join("xlink.xsd"), r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="http://www.w3.org/1999/xlink">
      <xs:attribute name="type"><xs:simpleType><xs:restriction base="xs:string"><xs:enumeration value="resource"/><xs:enumeration value="arc"/></xs:restriction></xs:simpleType></xs:attribute>
    </xs:schema>"#).unwrap();
    let ir = XsdParser::new()
        .parse_file(fixture.path().join("root.xsd"))
        .unwrap();
    assert_attribute_round_trip(&ir);
}

fn assert_attribute_round_trip(ir: &polyxml::ir::SchemaIR) {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("go.mod"), "module attrs\n\ngo 1.22\n").unwrap();
    fs::write(
        temp.path().join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(ir),
    )
    .unwrap();
    fs::write(temp.path().join("models_test.go"), r#"package models
import("encoding/xml";"testing";"strings";"io")
func TestAttributes(t *testing.T) {
 for _, document := range []string{`<Links xmlns:xl="http://www.w3.org/1999/xlink"><Resource xl:type="resource"/><Arc xl:type="arc"/></Links>`, `<Links><Resource xmlns:other="http://www.w3.org/1999/xlink" other:type="resource"/><Arc xmlns:a="http://www.w3.org/1999/xlink" a:type="arc"/></Links>`} {
 var root Links; if err:=xml.Unmarshal([]byte(document),&root);err!=nil{t.Fatal(err)}
 data,err:=xml.Marshal(root);if err!=nil{t.Fatal(err)}
 decoder:=xml.NewDecoder(strings.NewReader(string(data)));count:=0
 for {token,err:=decoder.Token();if err==io.EOF{break};if err!=nil{t.Fatal(err)};if start,ok:=token.(xml.StartElement);ok{for _,attr:=range start.Attr{if attr.Name.Space=="http://www.w3.org/1999/xlink"&&attr.Name.Local=="type"{count++;if attr.Value!="resource"&&attr.Value!="arc"{t.Fatal(attr)}}}}}
 if count!=2{t.Fatalf("attributes lost: %s",data)}
 }
 var absent Links; if err:=xml.Unmarshal([]byte(`<Links><Resource/><Arc/></Links>`),&absent);err!=nil{t.Fatal(err)}
 data,err:=xml.Marshal(absent);if err!=nil{t.Fatal(err)};if strings.Contains(string(data),`type=`){t.Fatalf("absent attribute emitted: %s",data)}
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
        fs::write(temp.path().join("App.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
        fs::write(
            temp.path().join("Models.cs"),
            CSharpCodegen::new(CSharpOptions {
                namespace: "Models".into(),
                use_records,
                ..Default::default()
            })
            .generate_module(ir),
        )
        .unwrap();
        fs::write(temp.path().join("Program.cs"), r#"using System.Xml.Serialization;using System.Xml.Linq;using Models;
var serializer=new XmlSerializer(typeof(Links));
foreach(var document in new[]{"<Links xmlns:xl='http://www.w3.org/1999/xlink'><Resource xl:type='resource'/><Arc xl:type='arc'/></Links>","<Links><Resource xmlns:b='http://www.w3.org/1999/xlink' b:type='resource'/><Arc xmlns:c='http://www.w3.org/1999/xlink' c:type='arc'/></Links>"}){
var root=(Links)serializer.Deserialize(new StringReader(document))!;
var writer=new StringWriter();serializer.Serialize(writer,root);
using(var jsonDocument=System.Text.Json.JsonSerializer.SerializeToDocument(root)){
if(!jsonDocument.RootElement.GetProperty("Resource").TryGetProperty("type",out _))throw new Exception("JSON wire name lost");
}
var xml=XDocument.Parse(writer.ToString());XNamespace ns="http://www.w3.org/1999/xlink";
if((string?)xml.Root!.Element("Resource")!.Attribute(ns+"type")!="resource"||(string?)xml.Root.Element("Arc")!.Attribute(ns+"type")!="arc")throw new Exception(writer.ToString());
}
var absent=(Links)serializer.Deserialize(new StringReader("<Links><Resource/><Arc/></Links>"))!;
var absentWriter=new StringWriter();serializer.Serialize(absentWriter,absent);
if(XDocument.Parse(absentWriter.ToString()).Descendants().Attributes().Any(a=>a.Name.LocalName=="type"))throw new Exception(absentWriter.ToString());
var json=System.Text.Json.JsonSerializer.Serialize(absent);
if(json.Contains("TypeXml"))throw new Exception("XML proxy leaked to JSON: "+json);
"#).unwrap();
        if ir
            .types
            .values()
            .any(|definition| matches!(definition, polyxml::ir::TypeDef::Enum(_)))
        {
            use std::io::Write;
            let mut program = fs::OpenOptions::new()
                .append(true)
                .open(temp.path().join("Program.cs"))
                .unwrap();
            writeln!(program, r#"bool rejected=false;try{{serializer.Deserialize(new StringReader("<Links xmlns:x='http://www.w3.org/1999/xlink'><Resource x:type='invalid'/><Arc/></Links>"));}}catch(InvalidOperationException){{rejected=true;}}if(!rejected)throw new Exception("invalid enum accepted");"#).unwrap();
        }
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
