use std::fs;
use std::process::Command;

use polyxml::codegen::csharp::{
    to_csharp_namespace, to_csharp_param_name, to_csharp_property_name, to_csharp_type_name,
    to_csharp_variant_name, CSharpCodegen, CSharpOptions, CSharpRecordKind,
};
use polyxml::ir::{
    Cardinality, EnumDef, EnumValue, FieldDef, FieldKind, PrimitiveType, QName, RestrictionFacets,
    SchemaIR, StructDef, TypeDef, TypeRef, UnionBranch, UnionDef,
};
use std::sync::Mutex;
use tempfile::tempdir;

#[test]
fn emits_unrestricted_simple_types_and_validates_empty_records() {
    let mut ir = SchemaIR::new().with_target_namespace("urn:test");
    ir.add_type(TypeDef::Simple(Box::new(polyxml::ir::SimpleTypeDef {
        qname: QName::new(Some("urn:test"), "DistanceType"),
        base_type: TypeRef::Primitive(PrimitiveType::Double),
        facets: RestrictionFacets::default(),
        documentation: None,
    })));
    ir.add_type(TypeDef::Simple(Box::new(polyxml::ir::SimpleTypeDef {
        qname: QName::new(Some("urn:test"), "PositiveDistanceType"),
        base_type: TypeRef::Named(QName::new(Some("urn:test"), "DistanceType")),
        facets: RestrictionFacets {
            min_inclusive: Some("0".into()),
            ..Default::default()
        },
        documentation: None,
    })));
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:test"), "EmptyType"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![],
        documentation: None,
    }));
    let code = CSharpCodegen::new(CSharpOptions::default()).generate_module(&ir);
    assert!(code.contains("record DistanceType("));
    assert!(code.contains("if (Value.Value < 0)"));
    assert!(code.contains("record EmptyType : IValidatableObject"));
    assert!(code.contains("IEnumerable<ValidationResult> Validate"));
}

#[test]
fn choice_variants_avoid_record_member_names() {
    let mut ir = SchemaIR::new().with_target_namespace("urn:test");
    ir.add_type(TypeDef::Union(UnionDef {
        qname: QName::new(Some("urn:test"), "ParameterValueType"),
        branches: vec![
            UnionBranch {
                variant_name: "Value".into(),
                xml_name: "Value".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: None,
            },
            UnionBranch {
                variant_name: "Equals".into(),
                xml_name: "Equals".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::Int),
                documentation: None,
            },
        ],
        documentation: None,
    }));
    let code = CSharpCodegen::new(CSharpOptions::default()).generate_module(&ir);
    assert!(code.contains("record ValueBranch("));
    assert!(code.contains("record EqualsBranch("));
    assert!(code.contains("XmlElement(\"Value\")"));
}

static DOTNET_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn particle_wire_structure_round_trips() {
    for (schema, root, documents) in [
        (include_str!("../../../research/fixtures/nested_sequence_choice.xsd"), "Root", vec!["<Root><First>A</First><Second>B</Second></Root>", "<Root><Alternative>C</Alternative></Root>"]),
        (include_str!("../../../research/fixtures/repeated_sequence.xsd"), "Root", vec!["<Root><First>A1</First><Second>B1</Second><First>A2</First><Second>B2</Second></Root>"]),
        (include_str!("../../../research/fixtures/substitution_group.xsd"), "Portfolio", vec!["<Portfolio xmlns='urn:audit:substitution'><Bond>A1</Bond><Equity>B1</Equity><Bond>A2</Bond></Portfolio>"]),
        (include_str!("../../../research/fixtures/choice_branch_cardinality.xsd"), "Root", vec!["<Root><Timing>A1</Timing><Timing>A2</Timing></Root>"]),
        (include_str!("../../../research/fixtures/wave6/duplicate_choice_branch_name.xsd"), "Person", vec!["<Person><MinAge>18</MinAge><MaxAge>25</MaxAge></Person>", "<Person><MaxAge>25</MaxAge></Person>"]),
    ] {
        let ir = polyxml::schema_parser::XsdParser::new().parse_str(schema).unwrap();
        for use_records in [true, false] {
            let temp = tempdir().unwrap();
            let code = CSharpCodegen::new(CSharpOptions { namespace: "ParticleModels".into(), use_records, ..Default::default() }).generate_module(&ir);
            fs::write(temp.path().join("Models.cs"), code).unwrap();
            fs::write(temp.path().join("App.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
            let documents = documents.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>().join(",");
            fs::write(temp.path().join("Program.cs"), format!(r#"
using System.Xml.Serialization;
using System.Xml.Linq;
using ParticleModels;
var serializer = new XmlSerializer(typeof({root}));
string Structure(XElement e) => e.Name.ToString()+"["+(e.HasElements ? string.Concat(e.Elements().Select(Structure)) : e.Value)+"]";
foreach (var document in new [] {{ {documents} }}) {{
 var value = serializer.Deserialize(new StringReader(document))!;
 var writer = new StringWriter(); serializer.Serialize(writer, value);
 if (Structure(XElement.Parse(document)) != Structure(XElement.Parse(writer.ToString()))) throw new Exception("wire data lost: "+writer);
}}
"#)).unwrap();
            let _lock = DOTNET_LOCK.lock().unwrap();
            let result = dotnet_command().args(["run"]).current_dir(temp.path()).output().unwrap();
            assert!(result.status.success(), "{root}, records={use_records}: {}\n{}", String::from_utf8_lossy(&result.stdout), String::from_utf8_lossy(&result.stderr));
        }
    }
}

#[test]
fn lexical_union_attributes_round_trip() {
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/lexical_union_attribute.xsd"
        ))
        .unwrap();
    for use_records in [true, false] {
        let code = CSharpCodegen::new(CSharpOptions {
            namespace: "AttrModels".into(),
            use_records,
            ..Default::default()
        })
        .generate_module(&ir);
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("App.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
        fs::write(temp.path().join("Models.cs"), code).unwrap();
        fs::write(temp.path().join("Program.cs"), r#"
using System.Xml.Serialization;
using AttrModels;
var serializer = new XmlSerializer(typeof(Root));
foreach (var value in new [] { "42", "" }) {
 var root = (Root)serializer.Deserialize(new StringReader("<Root count='"+value+"'/>"))!;
 if (root.Count is null || root.Count.ToXmlString() != value) throw new Exception("attribute lost");
 var writer = new StringWriter(); serializer.Serialize(writer, root);
 if (!writer.ToString().Contains("count=\""+value+"\"") || writer.ToString().Contains("<count>")) throw new Exception("wrong wire kind");
}
try { serializer.Deserialize(new StringReader("<Root count='invalid'/>")); throw new Exception("invalid union accepted"); } catch (InvalidOperationException) {}
"#).unwrap();
        let _lock = DOTNET_LOCK.lock().unwrap();
        let result = dotnet_command()
            .args(["run"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn nillable_enum_collection_round_trips() {
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/nillable_enum_collection.xsd"
        ))
        .unwrap();
    for use_records in [true, false] {
        let code = CSharpCodegen::new(CSharpOptions {
            namespace: "NilModels".into(),
            use_records,
            ..Default::default()
        })
        .generate_module(&ir);
        assert!(code.contains("List<Code?>"));
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("App.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><ImplicitUsings>enable</ImplicitUsings><Nullable>enable</Nullable></PropertyGroup></Project>"#).unwrap();
        fs::write(temp.path().join("Models.cs"), code).unwrap();
        fs::write(temp.path().join("Program.cs"), r#"
using System.Xml.Serialization;
using NilModels;
var serializer = new XmlSerializer(typeof(Root));
var xml = "<Root xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance'><Item>A</Item><Item xsi:nil='true'/><Item>B</Item></Root>";
var root = (Root)serializer.Deserialize(new StringReader(xml))!;
if (root.Item is not { Count: 3 } || root.Item[0] != Code.A || root.Item[1] != null || root.Item[2] != Code.B) throw new Exception("nil item lost");
var writer = new StringWriter();
serializer.Serialize(writer, root);
var decoded = (Root)serializer.Deserialize(new StringReader(writer.ToString()))!;
if (decoded.Item is not { Count: 3 } || decoded.Item[1] != null || decoded.Item[2] != Code.B) throw new Exception("nil round trip lost");
"#).unwrap();
        let _lock = DOTNET_LOCK.lock().unwrap();
        let result = dotnet_command()
            .args(["run", "--project", "App.csproj"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

fn dotnet_command() -> Command {
    let mut cmd = Command::new("dotnet");
    cmd.env("DOTNET_NOLOGO", "1");
    cmd.env("DOTNET_CLI_TELEMETRY_OPTOUT", "1");
    cmd.env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1");
    cmd
}

#[test]
fn test_csharp_sanitization() {
    assert_eq!(to_csharp_type_name("event"), "Event");
    assert_eq!(to_csharp_type_name("class"), "Class");
    assert_eq!(to_csharp_type_name("3d_point"), "Type3dPoint");
    assert_eq!(to_csharp_type_name("customer_account"), "CustomerAccount");

    // Property name sanitization avoiding collision with enclosing type (CS0542)
    assert_eq!(
        to_csharp_property_name("customer", Some("Customer")),
        "CustomerValue"
    );
    assert_eq!(to_csharp_property_name("name", Some("Customer")), "Name");

    // Keyword parameter name sanitization
    assert_eq!(to_csharp_param_name("event"), "@event");
    assert_eq!(to_csharp_param_name("params"), "@params");
    assert_eq!(to_csharp_param_name("first_name"), "firstName");

    // Variant names
    assert_eq!(to_csharp_variant_name("in_progress"), "InProgress");
    assert_eq!(to_csharp_variant_name("10_days"), "V10Days");

    // Dotted namespaces
    assert_eq!(
        to_csharp_namespace("com.example.crm-models"),
        "Com.Example.CrmModels"
    );
}

#[test]
fn test_csharp_records_and_enums_generation() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/crm");

    // Enum: OrderStatus
    let enum_qname = QName::new(Some("https://example.com/crm"), "OrderStatus");
    ir.add_type(TypeDef::Enum(EnumDef {
        qname: enum_qname.clone(),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "pending".into(),
                value: "pending".into(),
                documentation: Some("Pending processing".into()),
            },
            EnumValue {
                name: "in-progress".into(),
                value: "in-progress".into(),
                documentation: None,
            },
            EnumValue {
                name: "completed".into(),
                value: "completed".into(),
                documentation: None,
            },
        ],
        documentation: Some("Order processing state".into()),
    }));

    // Struct: Customer
    let customer_qname = QName::new(Some("https://example.com/crm"), "Customer");
    let facets = RestrictionFacets {
        min_length: Some(3),
        ..Default::default()
    };

    ir.add_type(TypeDef::Struct(StructDef {
        qname: customer_qname.clone(),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "id".into(),
                xml_name: "id".into(),
                namespace: None,
                kind: FieldKind::Attribute,
                type_ref: TypeRef::Primitive(PrimitiveType::Int),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "name".into(),
                xml_name: "name".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: Some(facets),
                is_cycle_cut: false,
            },
            FieldDef {
                name: "email".into(),
                xml_name: "email".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "tag".into(),
                xml_name: "tag".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::unbounded(0),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "status".into(),
                xml_name: "status".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(enum_qname),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: Some("Customer record definition".into()),
    }));

    let options = CSharpOptions {
        namespace: "Crm.Models".to_string(),
        emit_xml_attributes: true,
        emit_json_attributes: true,
        emit_validation: true,
        record_kind: CSharpRecordKind::Class,
        use_file_scoped_namespaces: true,
        emit_root_records: true,
        ..Default::default()
    };

    let codegen = CSharpCodegen::new(options);
    let cs_code = codegen.generate_module(&ir);

    assert!(cs_code.contains("using System.Text.Json.Serialization;"));
    assert!(cs_code.contains("namespace Crm.Models;"));
    assert!(cs_code.contains("[JsonConverter(typeof(JsonStringEnumConverter<OrderStatus>))]"));
    assert!(cs_code.contains("public enum OrderStatus"));
    assert!(cs_code.contains("[XmlEnum(\"pending\")]"));
    assert!(cs_code.contains("Pending,"));
    assert!(cs_code.contains("public static bool IsValid(this OrderStatus value)"));
    assert!(cs_code.contains("public static string ToXmlValue(this OrderStatus value)"));
    assert!(cs_code.contains("public record Customer("));
    assert!(cs_code.contains("[property: XmlAttribute(\"id\"), JsonPropertyName(\"id\")] int Id,"));
    assert!(cs_code
        .contains("[property: XmlElement(\"name\"), JsonPropertyName(\"name\")] string Name,"));
    assert!(cs_code
        .contains("[property: XmlElement(\"email\"), JsonPropertyName(\"email\")] string? Email,"));
    assert!(cs_code
        .contains("[property: XmlElement(\"tag\"), JsonPropertyName(\"tag\")] List<string>? Tag,"));
    assert!(cs_code.contains(
        "[property: XmlElement(\"status\"), JsonPropertyName(\"status\")] OrderStatus Status"
    ));
    assert!(cs_code.contains("public Customer() : this("));
    assert!(cs_code.contains("IValidatableObject"));

    // Verify .NET compilation, serialization roundtrip, and validation
    let temp = tempdir().unwrap();
    let csproj = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>"#;
    fs::write(temp.path().join("CrmApp.csproj"), csproj).unwrap();

    let models_cs = temp.path().join("Models.cs");
    fs::write(&models_cs, &cs_code).unwrap();

    let program_cs = temp.path().join("Program.cs");
    fs::write(
        &program_cs,
        r#"using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.IO;
using System.Text.Json;
using System.Xml.Serialization;
using Crm.Models;

public class Program
{
    public static int Main()
    {
        var cust = new Customer(
            Id: 42,
            Name: "Alice",
            Email: "alice@example.com",
            Tag: new List<string> { "vip", "retail" },
            Status: OrderStatus.Pending
        );

        if (!cust.Status.IsValid())
        {
            Console.WriteLine("OrderStatus should be valid");
            return 1;
        }

        if (cust.Status.ToXmlValue() != "pending")
        {
            Console.WriteLine("ToXmlValue mismatch");
            return 1;
        }

        // Test XmlSerializer roundtrip
        var serializer = new XmlSerializer(typeof(Customer));
        using var sw = new StringWriter();
        serializer.Serialize(sw, cust);
        var xml = sw.ToString();

        using var sr = new StringReader(xml);
        var decoded = (Customer?)serializer.Deserialize(sr);
        if (decoded == null)
        {
            Console.WriteLine("Deserialization produced null");
            return 1;
        }

        if (decoded.Id != 42 || decoded.Name != "Alice" || decoded.Email != "alice@example.com" || decoded.Status != OrderStatus.Pending)
        {
            Console.WriteLine("Field mismatch in deserialized object");
            return 1;
        }

        if (decoded.Tag == null || decoded.Tag.Count != 2 || decoded.Tag[0] != "vip")
        {
            Console.WriteLine("Tag list mismatch");
            return 1;
        }

        // Test System.Text.Json roundtrip
        var json = JsonSerializer.Serialize(cust);
        if (!json.Contains("\"id\":42") || !json.Contains("\"name\":\"Alice\"") || !json.Contains("\"Pending\""))
        {
            Console.WriteLine("JSON serialization missing fields: " + json);
            return 1;
        }

        var jsonDecoded = JsonSerializer.Deserialize<Customer>(json);
        if (jsonDecoded == null || jsonDecoded.Id != 42 || jsonDecoded.Name != "Alice" || jsonDecoded.Email != "alice@example.com" || jsonDecoded.Status != OrderStatus.Pending)
        {
            Console.WriteLine("JSON deserialization mismatch");
            return 1;
        }

        // Test IValidatableObject validation
        var validResults = new List<ValidationResult>();
        bool isValid = Validator.TryValidateObject(cust, new ValidationContext(cust), validResults, true);
        if (!isValid)
        {
            Console.WriteLine("Customer should be valid");
            return 1;
        }

        var invalidCust = new Customer(1, "Al", null, null, OrderStatus.Completed);
        var invalidResults = new List<ValidationResult>();
        bool isInvalid = !Validator.TryValidateObject(invalidCust, new ValidationContext(invalidCust), invalidResults, true);
        if (!isInvalid || invalidResults.Count == 0)
        {
            Console.WriteLine("Customer with short name should fail validation");
            return 1;
        }

        Console.WriteLine("Customer C# tests passed cleanly!");
        return 0;
    }
}
"#,
    )
    .unwrap();

    let _dotnet_lock = DOTNET_LOCK.lock().unwrap();
    let build_status = dotnet_command()
        .args(["build", "--warnaserror"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run dotnet build");
    assert!(
        build_status.success(),
        "dotnet build failed on generated records"
    );

    let run_status = dotnet_command()
        .args(["run"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run dotnet run");
    assert!(
        run_status.success(),
        "dotnet run failed on generated records"
    );
}

#[test]
fn test_csharp_choice_polymorphic_hierarchy() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/payments");

    // Choice union: ContactChoice
    let choice_qname = QName::new(Some("https://example.com/payments"), "ContactChoice");
    ir.add_type(TypeDef::Union(UnionDef {
        qname: choice_qname.clone(),
        branches: vec![
            UnionBranch {
                variant_name: "email".into(),
                xml_name: "email".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: None,
            },
            UnionBranch {
                variant_name: "phone".into(),
                xml_name: "phone".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: None,
            },
        ],
        documentation: Some("Preferred contact channel".into()),
    }));

    // Struct: PaymentContact
    let payment_qname = QName::new(Some("https://example.com/payments"), "PaymentContact");
    ir.add_type(TypeDef::Struct(StructDef {
        qname: payment_qname.clone(),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "payer".into(),
                xml_name: "payer".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "contact".into(),
                xml_name: "contact".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(choice_qname),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: None,
    }));

    let options = CSharpOptions {
        namespace: "Payments".to_string(),
        emit_xml_attributes: true,
        emit_json_attributes: true,
        emit_validation: true,
        record_kind: CSharpRecordKind::Class,
        use_file_scoped_namespaces: true,
        emit_root_records: true,
        ..Default::default()
    };

    let codegen = CSharpCodegen::new(options);
    let cs_code = codegen.generate_module(&ir);

    assert!(cs_code.contains("public abstract record ContactChoice"));
    assert!(cs_code.contains("[XmlInclude(typeof(ContactChoice.Email))]"));
    assert!(cs_code.contains("[XmlInclude(typeof(ContactChoice.Phone))]"));
    assert!(cs_code.contains("public sealed record Email("));
    assert!(cs_code.contains("public sealed record Phone("));
    assert!(cs_code.contains("XmlElement(\"email\", typeof(ContactChoice.Email))"));
    assert!(cs_code.contains("XmlElement(\"phone\", typeof(ContactChoice.Phone))"));
    assert!(cs_code.contains("JsonPropertyName(\"contact\")"));

    let temp = tempdir().unwrap();
    let csproj = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>"#;
    fs::write(temp.path().join("PaymentsApp.csproj"), csproj).unwrap();

    fs::write(temp.path().join("Models.cs"), &cs_code).unwrap();

    fs::write(
        temp.path().join("Program.cs"),
        r#"using System;
using System.IO;
using System.Xml.Serialization;
using Payments;

public class Program
{
    public static int Main()
    {
        var p1 = new PaymentContact(
            Payer: "Acme Corp",
            Contact: new ContactChoice.Email("billing@acme.com")
        );

        string channel = p1.Contact switch
        {
            ContactChoice.Email e => $"email:{e.Value}",
            ContactChoice.Phone p => $"phone:{p.Value}",
            _ => "unknown"
        };

        if (channel != "email:billing@acme.com")
        {
            Console.WriteLine($"Unexpected pattern match channel: {channel}");
            return 1;
        }

        // Test XmlSerializer roundtrip
        var serializer = new XmlSerializer(typeof(PaymentContact));
        using var sw = new StringWriter();
        serializer.Serialize(sw, p1);
        var xml = sw.ToString();

        using var sr = new StringReader(xml);
        var decoded = (PaymentContact?)serializer.Deserialize(sr);
        if (decoded == null || decoded.Payer != "Acme Corp")
        {
            Console.WriteLine("Deserialization failed");
            return 1;
        }

        if (decoded.Contact is not ContactChoice.Email emailChoice || emailChoice.Value != "billing@acme.com")
        {
            Console.WriteLine("Polymorphic choice deserialization failed");
            return 1;
        }

        Console.WriteLine("Choice pattern matching and XML serialization passed!");
        return 0;
    }
}
"#,
    )
    .unwrap();

    let _dotnet_lock = DOTNET_LOCK.lock().unwrap();
    let build_status = dotnet_command()
        .args(["build", "--warnaserror"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run dotnet build");
    assert!(
        build_status.success(),
        "dotnet build failed on choice models"
    );

    let run_status = dotnet_command()
        .args(["run"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run dotnet run");
    assert!(run_status.success(), "dotnet run failed on choice models");
}

#[test]
fn test_csharp_recursive_cycle() {
    let mut ir = SchemaIR::new();
    let tree_qname = QName::new(None::<String>, "TreeNode");

    ir.add_type(TypeDef::Struct(StructDef {
        qname: tree_qname.clone(),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "label".into(),
                xml_name: "label".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "next".into(),
                xml_name: "next".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(tree_qname),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: true,
            },
        ],
        documentation: None,
    }));

    let options = CSharpOptions {
        namespace: "Tree".to_string(),
        emit_xml_attributes: true,
        emit_json_attributes: true,
        emit_validation: true,
        record_kind: CSharpRecordKind::Class,
        use_file_scoped_namespaces: true,
        emit_root_records: true,
        ..Default::default()
    };

    let codegen = CSharpCodegen::new(options);
    let cs_code = codegen.generate_module(&ir);

    assert!(cs_code.contains("public record TreeNode("));
    assert!(cs_code
        .contains("[property: XmlElement(\"label\"), JsonPropertyName(\"label\")] string Label,"));
    assert!(cs_code.contains(
        "[property: XmlElement(\"next\"), JsonPropertyName(\"next\")] TreeNode? Next = null"
    ));

    let temp = tempdir().unwrap();
    let csproj = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>"#;
    fs::write(temp.path().join("TreeApp.csproj"), csproj).unwrap();

    fs::write(temp.path().join("Models.cs"), &cs_code).unwrap();

    fs::write(
        temp.path().join("Program.cs"),
        r#"using System;
using System.IO;
using System.Xml.Serialization;
using Tree;

public class Program
{
    public static int Main()
    {
        var root = new TreeNode(
            Label: "root",
            Next: new TreeNode(
                Label: "child1",
                Next: new TreeNode(
                    Label: "child2"
                )
            )
        );

        var serializer = new XmlSerializer(typeof(TreeNode));
        using var sw = new StringWriter();
        serializer.Serialize(sw, root);
        var xml = sw.ToString();

        using var sr = new StringReader(xml);
        var decoded = (TreeNode?)serializer.Deserialize(sr);
        if (decoded == null || decoded.Label != "root")
        {
            Console.WriteLine("Root node mismatch");
            return 1;
        }

        if (decoded.Next == null || decoded.Next.Label != "child1" || decoded.Next.Next?.Label != "child2")
        {
            Console.WriteLine("Recursive child node mismatch");
            return 1;
        }

        Console.WriteLine("Recursive tree C# tests passed cleanly!");
        return 0;
    }
}
"#,
    )
    .unwrap();

    let _dotnet_lock = DOTNET_LOCK.lock().unwrap();
    let build_status = dotnet_command()
        .args(["build", "--warnaserror"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run dotnet build");
    assert!(
        build_status.success(),
        "dotnet build failed on recursive tree models"
    );

    let run_status = dotnet_command()
        .args(["run"])
        .current_dir(temp.path())
        .status()
        .expect("Failed to run dotnet run");
    assert!(
        run_status.success(),
        "dotnet run failed on recursive tree models"
    );
}

#[test]
fn test_csharp_record_kind_from_str_loose() {
    assert_eq!(
        CSharpRecordKind::from_str_loose("class"),
        Some(CSharpRecordKind::Class)
    );
    assert_eq!(
        CSharpRecordKind::from_str_loose("record"),
        Some(CSharpRecordKind::Class)
    );
    assert_eq!(
        CSharpRecordKind::from_str_loose("struct"),
        Some(CSharpRecordKind::Struct)
    );
    assert_eq!(
        CSharpRecordKind::from_str_loose("record-struct"),
        Some(CSharpRecordKind::Struct)
    );
    assert_eq!(CSharpRecordKind::from_str_loose("invalid"), None);
}

#[test]
fn test_csharp_source_gen_context() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/crm");

    // Enum
    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/crm"), "OrderStatus"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "pending".into(),
                value: "pending".into(),
                documentation: None,
            },
            EnumValue {
                name: "shipped".into(),
                value: "shipped".into(),
                documentation: None,
            },
        ],
        documentation: None,
    }));

    // Struct
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/crm"), "Customer"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "id".into(),
                xml_name: "id".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::Int),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "name".into(),
                xml_name: "name".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: None,
    }));

    let options = CSharpOptions {
        namespace: "Crm.Models".to_string(),
        emit_source_gen: true,
        source_gen_context_name: "CrmJsonContext".to_string(),
        record_kind: CSharpRecordKind::Struct,
        ..Default::default()
    };

    let codegen = CSharpCodegen::new(options);
    let code = codegen.generate_module(&ir);

    // Record struct
    assert!(code.contains("public readonly record struct Customer("));

    // Source generation context
    assert!(code.contains("[JsonSourceGenerationOptions(WriteIndented = true)]"));
    assert!(code.contains("[JsonSerializable(typeof(Customer))]"));
    assert!(code.contains("[JsonSerializable(typeof(List<Customer>))]"));
    assert!(code.contains("[JsonSerializable(typeof(OrderStatus))]"));
    assert!(code.contains("public partial class CrmJsonContext : JsonSerializerContext"));
}

#[test]
fn test_csharp_mutable_inheritance_roundtrip() {
    let ir = polyxml::schema_parser::XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:complexType name="Base"><xs:attribute name="id" type="xs:int" use="required"/></xs:complexType>
      <xs:complexType name="Item"><xs:complexContent><xs:extension base="Base"><xs:sequence>
        <xs:element name="label" type="xs:string"/>
        <xs:element name="tags" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
        <xs:element name="child" type="Item" minOccurs="0"/>
      </xs:sequence></xs:extension></xs:complexContent></xs:complexType>
      <xs:complexType name="Empty"/>
      <xs:element name="document" type="Item"/>
      <xs:element name="greeting" type="xs:string"/>
    </xs:schema>"#).unwrap();
    let generator = CSharpCodegen::new(CSharpOptions {
        use_records: false,
        emit_source_gen: true,
        namespace: "Models".into(),
        ..Default::default()
    });
    let code = generator.generate_module(&ir);
    assert!(code.contains("public class Item : Base, IValidatableObject"));
    assert!(code.contains("get; set;"));
    assert!(!code.contains("public record"));
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Models.cs"), code).unwrap();
    fs::write(dir.path().join("Test.csproj"),r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework><OutputType>Exe</OutputType><Nullable>enable</Nullable><TreatWarningsAsErrors>true</TreatWarningsAsErrors></PropertyGroup></Project>"#).unwrap();
    fs::write(dir.path().join("Program.cs"),r#"using System; using System.IO; using System.Text.Json; using System.Xml.Serialization; using Models;
class Program {
 static void Main() {
  var item = new Document { Id = 42, Label = "a<&", Child = new Item { Label = "child" } };
  item.Tags!.Add("one"); item.Tags.Add("two");
  var serializer = new XmlSerializer(typeof(Document));
  var writer = new StringWriter(); serializer.Serialize(writer,item);
  var copy = (Document)serializer.Deserialize(new StringReader(writer.ToString()))!;
  if(copy.Id != 42 || copy.Label != item.Label || copy.Tags!.Count != 2 || copy.Child!.Label != "child") throw new Exception(writer.ToString());
  copy.Label = "changed";
  var json = JsonSerializer.Serialize(copy, PolyXmlJsonContext.Default.Document);
  var restored = JsonSerializer.Deserialize(json, PolyXmlJsonContext.Default.Document)!;
  if(restored.Label != "changed" || restored.Id != 42) throw new Exception(json);
  _ = new Empty();
  var greeting = new Greeting { Value = "hello" };
  var gs = new XmlSerializer(typeof(Greeting)); var gw = new StringWriter(); gs.Serialize(gw,greeting);
  if (((Greeting)gs.Deserialize(new StringReader(gw.ToString()))!).Value != "hello") throw new Exception(gw.ToString());
 }
}"#).unwrap();
    let _dotnet_lock = DOTNET_LOCK.lock().unwrap();
    if Command::new("dotnet").arg("--version").output().is_err() {
        return;
    }
    let result = Command::new("dotnet")
        .current_dir(dir.path())
        .args(["run", "--project", "Test.csproj"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_csharp_mutable_wrappers_and_choices_without_validation() {
    let ir=polyxml::schema_parser::XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="Alias"><xs:restriction base="xs:string"/></xs:simpleType>
      <xs:complexType name="Contact"><xs:choice><xs:element name="email" type="xs:string"/><xs:element name="phone" type="xs:string"/></xs:choice></xs:complexType>
      <xs:complexType name="Person"><xs:sequence><xs:element name="name" type="Alias"/><xs:element name="contact" type="Contact"/></xs:sequence></xs:complexType>
      <xs:element name="root" type="Contact"/>
    </xs:schema>"#).unwrap();
    let generator = CSharpCodegen::new(CSharpOptions {
        use_records: false,
        emit_validation: false,
        emit_source_gen: true,
        namespace: "Models".into(),
        ..Default::default()
    });
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Models.cs"), generator.generate_module(&ir)).unwrap();
    fs::write(dir.path().join("Test.csproj"),r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework><OutputType>Exe</OutputType><Nullable>enable</Nullable><TreatWarningsAsErrors>true</TreatWarningsAsErrors></PropertyGroup></Project>"#).unwrap();
    fs::write(dir.path().join("Program.cs"),r#"using System; using System.IO; using System.Text.Json; using System.Xml.Serialization; using Models;
class Program { static void Main() {
 var person = new Person { Name = new Alias("Alice"), Contact = new Contact.Email("a@example.org") };
 ((Contact.Email)person.Contact).Value = "b@example.org";
 var xml = new XmlSerializer(typeof(Person)); var writer = new StringWriter(); xml.Serialize(writer,person);
 var copy = (Person)xml.Deserialize(new StringReader(writer.ToString()))!;
 if(copy.Name.Value != "Alice" || ((Contact.Email)copy.Contact).Value != "b@example.org") throw new Exception(writer.ToString());
 var json = JsonSerializer.Serialize(person, PolyXmlJsonContext.Default.Person);
 if(JsonSerializer.Deserialize(json, PolyXmlJsonContext.Default.Person)!.Contact is not Contact.Email) throw new Exception(json);
 var root = new Root { Value = new Contact.Phone("123") }; var rx = new XmlSerializer(typeof(Root)); var rw = new StringWriter(); rx.Serialize(rw,root);
 if(((Root)rx.Deserialize(new StringReader(rw.ToString()))!).Value is not Contact.Phone) throw new Exception(rw.ToString());
} }
"#).unwrap();
    let _dotnet_lock = DOTNET_LOCK.lock().unwrap();
    if Command::new("dotnet").arg("--version").output().is_err() {
        return;
    }
    let result = Command::new("dotnet")
        .current_dir(dir.path())
        .args(["run", "--project", "Test.csproj"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_csharp_date_or_datetime_lexical_union() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="DateOrDateTime"><xs:union memberTypes="xs:date xs:dateTime"/></xs:simpleType>
      <xs:element name="Root"><xs:complexType><xs:sequence>
        <xs:element name="When" type="DateOrDateTime"/>
      </xs:sequence></xs:complexType></xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");
    let codegen = CSharpCodegen::new(CSharpOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("System.Xml.XmlConvert.ToString(item.Value)"));
    assert!(code.contains(
        "item.Value.ToString(\"yyyy-MM-dd\", System.Globalization.CultureInfo.InvariantCulture)"
    ));
}

#[test]
fn test_csharp_simple_content_class_validator_and_unsealed_records() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="PartyType">
        <xs:restriction base="xs:string">
          <xs:maxLength value="50"/>
        </xs:restriction>
      </xs:simpleType>
      <xs:element name="Party" type="PartyType"/>

      <xs:complexType name="CodeType">
        <xs:simpleContent>
          <xs:extension base="xs:string">
            <xs:attribute name="listID" type="xs:string"/>
          </xs:extension>
        </xs:simpleContent>
      </xs:complexType>

      <xs:element name="Root">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="Code" type="CodeType"/>
            <xs:element ref="Party"/>
          </xs:sequence>
        </xs:complexType>
      </xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");

    // Test mutable class mode with validation enabled
    let class_codegen = CSharpCodegen::new(CSharpOptions {
        use_records: false,
        emit_validation: true,
        emit_root_records: true,
        namespace: "UblSample".into(),
        ..Default::default()
    });
    let class_code = class_codegen.generate_module(&ir);

    assert!(class_code.contains("public class CodeType : IValidatableObject"));
    assert!(class_code.contains("public virtual IEnumerable<ValidationResult> Validate("));
    assert!(!class_code.contains("base.Validate("));
    assert!(class_code.contains("public class PartyType"));

    // Also test record mode to ensure Party derives from PartyType without sealed error
    let record_codegen = CSharpCodegen::new(CSharpOptions {
        use_records: true,
        emit_validation: true,
        emit_root_records: true,
        namespace: "UblSampleRecords".into(),
        ..Default::default()
    });
    let record_code = record_codegen.generate_module(&ir);
    assert!(record_code.contains("public record PartyType([property: XmlText] string Value)"));
    assert!(record_code.contains("public sealed record Party : PartyType"));

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("UblClass.cs"), class_code).unwrap();
    fs::write(dir.path().join("Test.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net8.0</TargetFramework><OutputType>Library</OutputType><Nullable>enable</Nullable><TreatWarningsAsErrors>true</TreatWarningsAsErrors></PropertyGroup></Project>"#).unwrap();

    let _dotnet_lock = DOTNET_LOCK.lock().unwrap();
    if Command::new("dotnet").arg("--version").output().is_err() {
        return;
    }
    let result = Command::new("dotnet")
        .current_dir(dir.path())
        .args(["build", "Test.csproj"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_csharp_any_attribute_codegen() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:element name="Extensible">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="name" type="xs:string"/>
          </xs:sequence>
          <xs:anyAttribute processContents="lax"/>
        </xs:complexType>
      </xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");

    // Test record mode
    let record_codegen = CSharpCodegen::new(CSharpOptions::default());
    let record_code = record_codegen.generate_module(&ir);
    assert!(record_code
        .contains("[property: XmlAnyAttribute] System.Xml.XmlAttribute[]? AnyAttribute = null"));

    // Test class mode
    let class_codegen = CSharpCodegen::new(CSharpOptions {
        use_records: false,
        ..Default::default()
    });
    let class_code = class_codegen.generate_module(&ir);
    assert!(class_code.contains("[XmlAnyAttribute]"));
    assert!(class_code
        .contains("public System.Xml.XmlAttribute[]? AnyAttribute { get; set; } = default!;"));
}

#[test]
fn test_csharp_sequence_nested_inside_choice_codegen() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:element name="Root"><xs:complexType><xs:choice>
        <xs:sequence>
          <xs:element name="First" type="xs:string"/>
          <xs:element name="Second" type="xs:string"/>
        </xs:sequence>
        <xs:element name="Alternative" type="xs:string"/>
      </xs:choice></xs:complexType></xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new()
        .parse_str(xsd)
        .expect("schema parse failed");
    let codegen = CSharpCodegen::new(CSharpOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("public record RootType("));
    assert!(code.contains("string? Alternative = null"));
    assert!(code.contains("public record RootTypeSequence"));
}

#[test]
fn element_defaults_preserve_absence_and_empty_presence() {
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/element_defaults.xsd"
        ))
        .unwrap();
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
            .generate_module(&ir),
        )
        .unwrap();
        fs::write(temp.path().join("Program.cs"), r#"using Models;using System.Xml.Serialization;
var serializer=new XmlSerializer(typeof(Root));
foreach(var document in new[]{"<Root/>","<Root><Flag/><Count></Count><Label/></Root>"}){
var root=(Root)serializer.Deserialize(new StringReader(document))!;
var present=document.Contains("Flag");
if(present&&(root.Flag!=false||root.Count!=42||root.Label!="fallback"))throw new Exception("defaults missing");
if(!present&&(root.Flag!=null||root.Count!=null||root.Label!=null))throw new Exception("absence lost");
var writer=new StringWriter();serializer.Serialize(writer,root);
if(!present&&writer.ToString().Contains("<Flag"))throw new Exception("absent value serialized");
if(present&&!writer.ToString().Contains("<Flag>false</Flag>"))throw new Exception(writer.ToString());
}"#).unwrap();
        let _lock = DOTNET_LOCK.lock().unwrap();
        let result = dotnet_command()
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
