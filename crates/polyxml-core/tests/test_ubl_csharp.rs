use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions};
use polyxml::schema_parser::XsdParser;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
fn multi_namespace_roots_and_inherited_simple_content_compile_and_round_trip() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("base.xsd"), r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:base" xmlns:b="urn:base">
      <xs:complexType name="CodeType"><xs:simpleContent><xs:extension base="xs:string"><xs:attribute name="listID" type="xs:string"/></xs:extension></xs:simpleContent></xs:complexType>
      <xs:complexType name="DateType"><xs:simpleContent><xs:extension base="xs:date"/></xs:simpleContent></xs:complexType>
      <xs:element name="CodeType" type="b:CodeType"/>
    </xs:schema>"#).unwrap();
    fs::write(dir.path().join("root.xsd"), r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:root" elementFormDefault="qualified" xmlns:r="urn:root" xmlns:b="urn:base">
      <xs:import namespace="urn:base" schemaLocation="base.xsd"/>
      <xs:complexType name="CodeType"><xs:simpleContent><xs:extension base="b:CodeType"><xs:attribute name="language" type="xs:string"/></xs:extension></xs:simpleContent></xs:complexType>
      <xs:element name="Root" type="r:CodeType"/>
      <xs:element name="CodeType" type="xs:string"/>
      <xs:complexType name="RestrictedCode"><xs:simpleContent><xs:restriction base="b:CodeType"><xs:attribute name="listID" type="xs:string" use="required"/></xs:restriction></xs:simpleContent></xs:complexType>
      <xs:element name="RestrictedRoot" type="r:RestrictedCode"/>
      <xs:complexType name="RestrictedDate"><xs:simpleContent><xs:restriction base="b:DateType"/></xs:simpleContent></xs:complexType>
      <xs:element name="Container"><xs:complexType><xs:sequence>
        <xs:element ref="b:CodeType"/>
        <xs:element name="When" type="r:RestrictedDate"/>
        <xs:element name="Stamp" type="xs:dateTime"/>
        <xs:element name="Clock" type="xs:time"/>
        <xs:element name="Span" type="xs:duration"/>
      </xs:sequence></xs:complexType></xs:element>
    </xs:schema>"#).unwrap();
    let ir = XsdParser::new()
        .parse_file(dir.path().join("root.xsd"))
        .unwrap();
    for use_records in [true, false] {
        let project = dir
            .path()
            .join(if use_records { "records" } else { "classes" });
        fs::create_dir(&project).unwrap();
        fs::write(
            project.join("Models.cs"),
            CSharpCodegen::new(CSharpOptions {
                namespace: "Models".into(),
                use_records,
                ..Default::default()
            })
            .generate_module(&ir),
        )
        .unwrap();
        fs::write(project.join("App.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
        fs::write(project.join("Program.cs"), r#"
using Models;
using System.Xml;
using System.Xml.Schema;
using System.Xml.Serialization;
var serializer = new XmlSerializer(typeof(Root));
var root = (Root)serializer.Deserialize(new StringReader("<Root xmlns='urn:root' listID='catalog' language='en'>ABC</Root>"))!;
if (root.Value != "ABC" || root.ListId != "catalog" || root.Language != "en") throw new Exception("inherited text/attributes lost");
var writer = new StringWriter(); serializer.Serialize(writer, root);
var settings = new XmlReaderSettings { ValidationType = ValidationType.Schema };
settings.Schemas.XmlResolver = new XmlUrlResolver();
settings.Schemas.Add("urn:root", "../root.xsd");
settings.ValidationEventHandler += (_, e) => throw new Exception(e.Message);
using var reader = XmlReader.Create(new StringReader(writer.ToString()), settings); while(reader.Read()) {}
var again = (Root)serializer.Deserialize(new StringReader(writer.ToString()))!;
if (again.Value != "ABC") throw new Exception("round trip lost text");
var restrictedSerializer = new XmlSerializer(typeof(RestrictedRoot));
var restricted = (RestrictedRoot)restrictedSerializer.Deserialize(new StringReader("<RestrictedRoot xmlns='urn:root' listID='catalog'>DEF</RestrictedRoot>"))!;
if (restricted.Value != "DEF" || restricted.ListId != "catalog") throw new Exception("restricted inherited attribute lost");
var containerSerializer = new XmlSerializer(typeof(Container));
var xml = "<Container xmlns='urn:root'><CodeType xmlns='urn:base' listID='catalog'>XYZ</CodeType><When>2026-10-01+05:30</When><Stamp>2026-10-01T12:30:00.123</Stamp><Clock>12:30:00.123+05:30</Clock><Span>P1DT2H</Span></Container>";
var container = (Container)containerSerializer.Deserialize(new StringReader(xml))!;
if (container.CodeType.Value != "XYZ" || container.When.Value != new DateOnly(2026,10,1)) throw new Exception("namespaced value lost");
var containerWriter = new StringWriter(); containerSerializer.Serialize(containerWriter, container);
using(var check = XmlReader.Create(new StringReader(containerWriter.ToString()),settings)) { while(check.Read()){} }
if (!containerWriter.ToString().Contains("2026-10-01+05:30") || !containerWriter.ToString().Contains("2026-10-01T12:30:00.123")) throw new Exception("temporal lexical changed");
container.When.Value = new DateOnly(2027,1,2);
if (container.When.ValueXml != "2027-01-02") throw new Exception("stale temporal lexical");
foreach (var type in typeof(Root).Assembly.GetTypes()) {
    var attr = type.GetCustomAttributes(typeof(XmlRootAttribute), false).Cast<XmlRootAttribute>().FirstOrDefault();
    if (attr?.ElementName != "CodeType") continue;
    if (attr.Namespace != "urn:root" && attr.Namespace != "urn:base") throw new Exception("namespace renamed");
    new XmlSerializer(type);
}
"#).unwrap();
        let output = Command::new("dotnet")
            .args(["run", "--project", "App.csproj"])
            .env("DOTNET_NOLOGO", "1")
            .current_dir(&project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn unprefixed_xsd_builtin_simple_content_base_uses_default_namespace() {
    let ir = XsdParser::new().parse_str(r#"<schema xmlns="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:binary"><complexType name="Binary"><simpleContent><extension base="base64Binary"/></simpleContent></complexType></schema>"#).unwrap();
    let polyxml::ir::TypeDef::Struct(binary) =
        &ir.types[&polyxml::ir::QName::new(Some("urn:binary"), "Binary")]
    else {
        panic!("missing Binary")
    };
    assert_eq!(
        binary.base_type.as_ref().unwrap().namespace.as_deref(),
        Some("http://www.w3.org/2001/XMLSchema")
    );
}
