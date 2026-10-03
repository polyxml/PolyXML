---
title: C#
description: Modern XML data binding, record generation, and facet validation in C# 12 and .NET 8+ using PolyXML.
---

# C#

PolyXML compiles W3C XML schemas directly into idiomatic, high-performance C# 12 and .NET 8+ data models.

PolyXML generates **records with primary constructors** by default, or **mutable classes** with `--style class`. Both representations support XML/JSON annotations and `IValidatableObject` facet validation.

---

## 🚀 Key Capabilities

- **Immutable Records with Primary Constructors**: Emits `public record TypeName(...)` with `init`-only properties.
- **Full `XmlSerializer` Interoperability**: Includes disambiguated parameterless constructors with default values and standard `System.Xml.Serialization` attributes (`[XmlRoot]`, `[XmlElement]`, `[XmlAttribute]`, `[XmlEnum]`, `[XmlInclude]`).
- **Polymorphic `xs:choice` Models**: Generates an abstract record base with nested sealed records, enabling natural switch pattern matching.
- **Facet Validation**: Implements `IValidatableObject` to validate string lengths, numeric ranges, and regex patterns.
- **Keyword & Collision Defense**: Automatically prefixes C# keywords with `@` and prevents property names from colliding with enclosing classes (CS0542).

---

## 🛠️ Generating C# Models

Generate C# models from an XML schema using the `polyxml` CLI:

```bash
# Generate C# models with custom namespace
polyxml generate \
  --lang csharp \
  --namespace Enterprise.Banking.Iso20022 \
  --out ./src/Generated \
  schemas/pain.001.001.09.xsd
```

Or configure it in your workspace manifest `polyxml.toml`:

```toml
[[generate]]
target = "csharp"
output = "src/Generated"
namespace = "Enterprise.Banking.Iso20022"
```

---

## Mutable classes

```bash
polyxml generate schema.xsd --lang csharp --style class --backend source-gen --out generated
```

`--style pojo` is an alias for `class`; `--style record` retains the default.
Mutable complex types have parameterless constructors and public `get; set;`
properties. Repeated properties start with an empty list, optional properties
remain nullable, and complex-type extensions preserve base-class inheritance.
Simple restriction wrappers and choice branches are also mutable. XML/JSON
attributes and the optional `JsonSerializerContext` continue to work. Derived
validators include the base type's validation results.

```csharp
var entity = new EntityMt();
entity.Id = "UUID-1234";
entity.Status = "PROCESSED";
```

```toml
[codegen.csharp]
output = "generated/csharp"
namespace = "Enterprise.Models"
style = "class"
backend = "source-gen"
```

These fields also work in `[[generate]]`. `--style record-struct` and
`--style class` are mutually exclusive values of the same option. Java's
`--feature builder,direct-codec` options do not apply; C# uses object
initializers and its standard XML/JSON serializers.

---

## 1. Generated Data Models

Given an XML Schema defining complex types, attributes, and optional fields:

```csharp
namespace Enterprise.Banking.Iso20022;

using System;
using System.Collections.Generic;
using System.ComponentModel.DataAnnotations;
using System.Text.Json.Serialization;
using System.Text.RegularExpressions;
using System.Xml.Serialization;

[XmlRoot("Customer", Namespace = "https://example.com/crm")]
public record Customer(
    [property: XmlAttribute("id"), JsonPropertyName("id")] int Id,
    [property: XmlElement("name"), JsonPropertyName("name")] string Name,
    [property: XmlElement("email"), JsonPropertyName("email")] string? Email = null,
    [property: XmlElement("tag"), JsonPropertyName("tag")] List<string>? Tag = null,
    [property: XmlElement("status"), JsonPropertyName("status")] OrderStatus Status = OrderStatus.Pending
) : IValidatableObject
{
    // Parameterless constructor ensures compatibility with XmlSerializer
    public Customer() : this(default(int)!, string.Empty, null, null, default(OrderStatus)!) { }

    public IEnumerable<ValidationResult> Validate(ValidationContext validationContext)
    {
        if (Name != null && Name.Length < 2)
        {
            yield return new ValidationResult(
                "Name must be at least 2 characters",
                new[] { nameof(Name) }
            );
        }
    }
}
```

---

## 2. Polymorphic `xs:choice` Support

XML Schema `<xs:choice>` groups are synthesized into type-safe polymorphic record hierarchies:

```csharp
[XmlInclude(typeof(ContactChoice.Email))]
[XmlInclude(typeof(ContactChoice.Phone))]
public abstract record ContactChoice
{
    public sealed record Email([property: XmlText] string Value) : ContactChoice
    {
        public Email() : this(string.Empty) { }
    }

    public sealed record Phone([property: XmlText] string Value) : ContactChoice
    {
        public Phone() : this(string.Empty) { }
    }
}
```

Enclosing types annotate choice properties with `XmlElement` variants:

```csharp
public record ContactInfo(
    [property: XmlElement("email", typeof(ContactChoice.Email))]
    [property: XmlElement("phone", typeof(ContactChoice.Phone))]
    ContactChoice? Contact = null
);
```

### Pattern Matching on Choices

You can process choices with exhaustive C# switch expressions:

```csharp
string recipient = contactInfo.Contact switch
{
    ContactChoice.Email email => $"Send email to: {email.Value}",
    ContactChoice.Phone phone => $"Call phone at: {phone.Value}",
    null                      => "No contact information provided",
    _                         => throw new InvalidOperationException()
};
```

---

## 3. Serialization & Deserialization

PolyXML models feature dual-format annotations, working seamlessly with both .NET's built-in `XmlSerializer` and `System.Text.Json`.

### XML Serialization (`System.Xml.Serialization`)

```csharp
using System.IO;
using System.Xml.Serialization;
using Enterprise.Banking.Iso20022;

// Deserialization
var serializer = new XmlSerializer(typeof(Customer));
using var reader = new StringReader(xmlString);
var customer = (Customer)serializer.Deserialize(reader)!;

Console.WriteLine($"Customer {customer.Name} (ID: {customer.Id}) loaded.");

// Serialization
using var writer = new StringWriter();
serializer.Serialize(writer, customer);
string outputXml = writer.ToString();
```

### JSON Serialization (`System.Text.Json`)

Because PolyXML emits `[property: JsonPropertyName("...")]` on record properties and `[JsonConverter(typeof(JsonStringEnumConverter))]` on enums, models serialize and deserialize natively to JSON without mapping code:

```csharp
using System.Text.Json;
using Enterprise.Banking.Iso20022;

// Direct JSON serialization
string jsonString = JsonSerializer.Serialize(customer, new JsonSerializerOptions { WriteIndented = true });
Console.WriteLine(jsonString);

// Direct JSON deserialization back into immutable record
Customer restored = JsonSerializer.Deserialize<Customer>(jsonString)!;
assert(restored.Name == customer.Name);
```

### Compile-Time Source Generation (`--backend source-gen`)

For Native AOT, high-throughput microservices, and reflection-free environments, PolyXML can emit a compile-time `JsonSerializerContext`:

```bash
polyxml generate --lang csharp --backend source-gen --style record-struct --out ./src/Generated schema.xsd
```

This generates `[JsonSourceGenerationOptions]` and `[JsonSerializable(typeof(T))]` annotations:

```csharp
[JsonSourceGenerationOptions(WriteIndented = true)]
[JsonSerializable(typeof(Customer))]
public partial class CustomerJsonContext : JsonSerializerContext
{
}
```

Usage in .NET 8 / 9 Native AOT:

```csharp
// Zero-reflection, Native AOT-friendly JSON serialization
string json = JsonSerializer.Serialize(customer, CustomerJsonContext.Default.Customer);
Customer restored = JsonSerializer.Deserialize(json, CustomerJsonContext.Default.Customer);
```

### Record Structs (`--style record-struct`)

By default, PolyXML emits reference `record class` types. For zero-allocation, cache-friendly scenarios where data contracts are small or short-lived, pass `--style record-struct`:

```csharp
public readonly record struct Customer(
    [property: XmlAttribute("id"), JsonPropertyName("id")] int Id,
    [property: XmlElement("name"), JsonPropertyName("name")] string Name
) : IValidatableObject;
```

---

## 4. Restriction Facet Validation

PolyXML generates `IValidatableObject` implementations to validate constraint facets:

```csharp
using System.ComponentModel.DataAnnotations;

var customer = new Customer(
    Id: 42,
    Name: "A", // Violates minLength=2
    Email: "invalid-email"
);

var context = new ValidationContext(customer);
var results = new List<ValidationResult>();

bool isValid = Validator.TryValidateObject(customer, context, results, validateAllProperties: true);

if (!isValid)
{
    foreach (var validationError in results)
    {
        Console.WriteLine($"Validation error: {validationError.ErrorMessage}");
    }
}
```

---

## 5. Summary Table

| Feature | PolyXML C# Output | Advantage |
|---|---|---|
| **Class Model** | `public record Type(...)` | Immutability, value equality, concise syntax |
| **XML Serialization** | `System.Xml.Serialization` | Zero third-party runtime package dependencies |
| **JSON Serialization**| `System.Text.Json` | Native `[JsonPropertyName]` & `[JsonConverter]` attributes |
| **`xs:choice`** | `abstract record` + nested sealed records | Type-safe pattern matching with switch expressions |
| **Facets** | `IValidatableObject.Validate()` | Built-in .NET `DataAnnotations` standard integration |
| **Enums** | `public enum EnumName` with `[XmlEnum]` | Autocomplete, strongly typed string & JSON mappings |


## Generate once for XML and JSON

```bash
polyxml generate customer.xsd --lang csharp --out Models
```

Include the generated `.cs` files in your project. The [serialization examples](#3-serialization-deserialization)
apply directly to the generated root class: `XmlSerializer` reads/writes XML and
`System.Text.Json.JsonSerializer` reads/writes JSON. Use the generated root name
when it differs from the reusable type name. Select `--style class` if your
application requires mutable models; the default is record classes. This path
uses .NET serializers with emitted attributes and XML proxies and does not need
the native Rust binding or Rust Serde.
