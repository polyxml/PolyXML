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

#[test]
fn inherited_attribute_and_distinct_element_keep_separate_property_names() {
    let ir = XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:complexType name="Entity"><xs:attribute name="nameOfClass" type="xs:string"/></xs:complexType>
      <xs:complexType name="Assignment"><xs:complexContent><xs:extension base="Entity"><xs:sequence><xs:element name="NameOfClass" type="xs:string" minOccurs="0"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType>
      <xs:complexType name="SpecialAssignment"><xs:complexContent><xs:extension base="Assignment"><xs:sequence><xs:element name="name_of_class" type="xs:string" minOccurs="0"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType>
    </xs:schema>"#).unwrap();
    for use_records in [true, false] {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("App.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable><WarningsAsErrors>CS0108;CS8866;CS0657;CS8907</WarningsAsErrors></PropertyGroup></Project>"#).unwrap();
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
        fs::write(temp.path().join("Program.cs"), r#"using System.Xml.Serialization;using System.Xml.Linq;using Models;
var serializer=new XmlSerializer(typeof(SpecialAssignment));
var value=(SpecialAssignment)serializer.Deserialize(new StringReader("<SpecialAssignment nameOfClass='attribute'><NameOfClass>element</NameOfClass><name_of_class>grandchild</name_of_class></SpecialAssignment>"))!;
if(((Entity)value).NameOfClass!="attribute"||value.NameOfClass2!="element"||value.NameOfClass3!="grandchild")throw new Exception("inherited values lost");
var writer=new StringWriter();serializer.Serialize(writer,value);var xml=XDocument.Parse(writer.ToString()).Root!;
if((string?)xml.Attribute("nameOfClass")!="attribute"||(string?)xml.Element("NameOfClass")!="element"||(string?)xml.Element("name_of_class")!="grandchild")throw new Exception(writer.ToString());
var json=System.Text.Json.JsonSerializer.Serialize(value);if(!json.Contains("attribute")||!json.Contains("element")||!json.Contains("grandchild"))throw new Exception(json);
"#).unwrap();
        let result = Command::new("dotnet")
            .args(["run", "--disable-build-servers"])
            .env("DOTNET_CLI_USE_MSBUILD_SERVER", "0")
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "records={use_records}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn derived_validators_preserve_base_constraints() {
    use polyxml::ir::{QName, RestrictionFacets, TypeDef};
    let mut ir=XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="Base"><xs:attribute name="code" type="xs:string"/></xs:complexType><xs:complexType name="Derived"><xs:complexContent><xs:extension base="Base"><xs:attribute name="extra" type="xs:string"/></xs:extension></xs:complexContent></xs:complexType></xs:schema>"#).unwrap();
    let TypeDef::Struct(base) = ir.types.get_mut(&QName::local("Base")).unwrap() else {
        panic!()
    };
    base.fields[0].facets = Some(RestrictionFacets {
        min_length: Some(3),
        ..Default::default()
    });
    for use_records in [true, false] {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("App.csproj"),r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable><WarningsAsErrors>CS0108;CS0109;CS0114</WarningsAsErrors></PropertyGroup></Project>"#).unwrap();
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
        fs::write(temp.path().join("Program.cs"),r#"using Models;using System.ComponentModel.DataAnnotations;
var value=new Derived {Code="x",Extra="leaf"};
if(!value.Validate(new ValidationContext(value)).Any())throw new Exception("base constraint lost");
if(!((Base)value).Validate(new ValidationContext(value)).Any())throw new Exception("base dispatch lost");
var results=new List<ValidationResult>();if(Validator.TryValidateObject(value,new ValidationContext(value),results,true))throw new Exception("interface validation lost");
var valid=new Derived {Code="valid",Extra="leaf"};if(valid.Validate(new ValidationContext(valid)).Any())throw new Exception("valid value rejected");
"#).unwrap();
        let output = Command::new("dotnet")
            .args(["run", "--disable-build-servers"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "records={use_records}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn lexical_dependencies_compile_and_round_trip_without_implicit_imports() {
    let ir=XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="Code"><xs:restriction base="xs:string"><xs:enumeration value="A"/><xs:enumeration value="B"/><xs:enumeration value="LONG"/></xs:restriction></xs:simpleType>
      <xs:simpleType name="ShortCode"><xs:restriction base="Code"><xs:maxLength value="1"/></xs:restriction></xs:simpleType><xs:simpleType name="Inner"><xs:union memberTypes="Code xs:int"/></xs:simpleType>
      <xs:simpleType name="Outer"><xs:union memberTypes="Inner xs:token xs:anyURI"/></xs:simpleType>
      <xs:simpleType name="Numbers"><xs:list itemType="xs:int"/></xs:simpleType>
      <xs:complexType name="Payload"><xs:sequence><xs:element name="Flag" type="xs:boolean" default="true"/><xs:element name="Count" type="xs:int" fixed="7"/><xs:element name="Huge" type="xs:integer"/><xs:element name="Values" type="Numbers"/><xs:element name="Reason" type="Outer"/></xs:sequence><xs:attribute name="mode" type="Code" use="required" fixed="A"/></xs:complexType>
    </xs:schema>"#).unwrap();
    for use_records in [true, false] {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("App.csproj"),r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><Nullable>enable</Nullable><TreatWarningsAsErrors>true</TreatWarningsAsErrors></PropertyGroup></Project>"#).unwrap();
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
        fs::write(temp.path().join("Program.cs"),r#"using System;using System.IO;using System.Xml.Serialization;using System.Xml.Linq;using Models;using System.ComponentModel.DataAnnotations;
var invalidCode=new ShortCode((Code)2);if(!invalidCode.Validate(new ValidationContext(invalidCode)).GetEnumerator().MoveNext())throw new Exception("enum lexical facet ignored");
var validCode=new ShortCode(Code.A);if(validCode.Validate(new ValidationContext(validCode)).GetEnumerator().MoveNext())throw new Exception("valid enum rejected");
foreach(var lexical in new[]{"A","42","fallback"}) {
var serializer=new XmlSerializer(typeof(Payload));var value=(Payload)serializer.Deserialize(new StringReader("<Payload mode='A'><Flag/><Count>7</Count><Huge>123456789012345678901234567890</Huge><Values>1 2</Values><Reason>"+lexical+"</Reason></Payload>"))!;
if(!value.Flag||value.Count!=7||value.Huge!="123456789012345678901234567890"||value.Values.Value.Count!=2||value.Values.Value[1]!=2||value.Reason.ToXmlString()!=lexical)throw new Exception("lexical input lost");
var writer=new StringWriter();serializer.Serialize(writer,value);var root=XDocument.Parse(writer.ToString()).Root!;
if(root.Attribute("mode")!.Value!="A"||root.Element("Flag")!.Value!="true"||root.Element("Count")!.Value!="7"||root.Element("Values")!.Value!="1 2"||root.Element("Reason")!.Value!=lexical)throw new Exception(writer.ToString());
value.Huge=null!;try{serializer.Serialize(new StringWriter(),value);throw new Exception("missing required integer emitted");}catch(InvalidOperationException){}
}
"#).unwrap();
        let output = Command::new("dotnet")
            .args(["run", "--disable-build-servers"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "records={use_records}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
