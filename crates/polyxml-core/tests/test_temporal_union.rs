use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions};
use polyxml::codegen::go::{GoCodegen, GoOptions};
use polyxml::schema_parser::XsdParser;
use std::{fs, process::Command};
use tempfile::tempdir;

const XSD: &str = include_str!("../../../research/fixtures/wave5/date_union.xsd");

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn go_temporal_union_preserves_branches_and_lexicals() {
    let ir = XsdParser::new().parse_str(XSD).unwrap();
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("go.mod"), "module temporal\n\ngo 1.22\n").unwrap();
    fs::write(
        dir.path().join("models.go"),
        GoCodegen::new(GoOptions::default()).generate_module(&ir),
    )
    .unwrap();
    fs::write(dir.path().join("models_test.go"), r#"package models
import("encoding/xml"; "strings"; "testing")
func TestTemporalUnion(t *testing.T) {
 for _, text := range []string{"2023-03-15", "2023-03-15Z", "2023-03-15+05:30", "2023-03-15-04:00", "2023-03-15T12:30:00Z", "2023-03-15T12:30:00", "2023-03-15T12:30:00.1234567", "2023-03-15T12:30:00.1234567+05:30"} {
  var root Root
  if err := xml.Unmarshal([]byte("<Root><When>"+text+"</When></Root>"), &root); err != nil { t.Fatal(text, err) }
  if (root.When.DateValue != nil) != !strings.Contains(text,"T") || (root.When.DateTimeValue != nil) != strings.Contains(text,"T") { t.Fatal("wrong branch", text) }
  out, err := xml.Marshal(root); if err != nil { t.Fatal(err) }
  if !strings.Contains(string(out), ">"+text+"<") { t.Fatal("lexical changed", text, string(out)) }
  var again Root; if err := xml.Unmarshal(out, &again); err != nil { t.Fatal(err) }
 }
 for _, text := range []string{"2023-02-30", "03/15/2023", "2023-03-15T12:30:00garbage"} {
  var root Root; if xml.Unmarshal([]byte("<Root><When>"+text+"</When></Root>"), &root) == nil { t.Fatal("accepted",text) }
 }
}
"#).unwrap();
    run(Command::new("go")
        .arg("test")
        .arg("./...")
        .current_dir(dir.path()));
}

#[test]
fn csharp_temporal_union_preserves_branches_and_lexicals() {
    let ir = XsdParser::new().parse_str(XSD).unwrap();
    for use_records in [true, false] {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("App.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
        fs::write(
            dir.path().join("Models.cs"),
            CSharpCodegen::new(CSharpOptions {
                namespace: "Models".into(),
                use_records,
                ..Default::default()
            })
            .generate_module(&ir),
        )
        .unwrap();
        fs::write(dir.path().join("schema.xsd"), XSD).unwrap();
        fs::write(dir.path().join("Program.cs"), r#"
using Models;
using System.Xml;
using System.Xml.Schema;
using System.Xml.Serialization;
var serializer = new XmlSerializer(typeof(Root));
var settings = new XmlReaderSettings { ValidationType = ValidationType.Schema };
settings.Schemas.Add(null, "schema.xsd");
settings.ValidationEventHandler += (_, e) => throw new Exception(e.Message);
foreach (var text in new[] {"2023-03-15", "2023-03-15Z", "2023-03-15+05:30", "2023-03-15-04:00", "2023-03-15T12:30:00Z", "2023-03-15T12:30:00", "2023-03-15T12:30:00.1234567", "2023-03-15T12:30:00.1234567+05:30"}) {
    var root = (Root)serializer.Deserialize(new StringReader("<Root><When>"+text+"</When></Root>"))!;
    if ((root.When is DateOrDateTime.DateValue) != !text.Contains('T')) throw new Exception("wrong branch");
    if (root.When.ToXmlString() != text) throw new Exception("lexical changed");
    var writer = new StringWriter(); serializer.Serialize(writer, root);
    using var reader = XmlReader.Create(new StringReader(writer.ToString()), settings);
    while (reader.Read()) {}
    var again = (Root)serializer.Deserialize(new StringReader(writer.ToString()))!;
    if (again.When.ToXmlString() != text) throw new Exception("round trip changed");
}
foreach (var text in new[] {"2023-02-30", "03/15/2023", "2023-03-15T12:30:00garbage"}) {
    try { DateOrDateTime.Parse(text); throw new Exception("accepted " + text); } catch (FormatException) {}
}
var changed = DateOrDateTime.Parse("2023-03-15+05:30");
changed.GetType().GetProperty("Value")!.SetValue(changed, new DateOnly(2024, 1, 2));
if (changed.ToXmlString() != "2024-01-02") throw new Exception("stale lexical after mutation");
"#).unwrap();
        run(Command::new("dotnet")
            .args(["run", "--project", "App.csproj"])
            .env("DOTNET_NOLOGO", "1")
            .current_dir(dir.path()));
    }
}
