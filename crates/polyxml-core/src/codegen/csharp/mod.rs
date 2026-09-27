//! Modern C# 12 / .NET 8+ Code Generator for PolyXML-IR.
//!
//! Emits idiomatic C# 12 records with primary constructors, standard System.Xml.Serialization
//! attributes, polymorphic xs:choice abstract records, and IValidatableObject facet boundary checks.

use std::collections::HashSet;
use std::fmt::Write as FmtWrite;

use heck::{AsLowerCamelCase, AsPascalCase};
use serde::{Deserialize, Serialize};

use crate::codegen::{
    build_type_name_map, lookup_type_name, normalize_symbol_name, sanitize_keyword,
    set_type_name_map, LanguageContext,
};
use crate::ir::{
    EnumDef, FieldDef, FieldKind, PrimitiveType, QName, RestrictionFacets, SchemaIR, SimpleTypeDef,
    StructDef, TypeDef, TypeRef, UnionDef,
};

/// Record emission kind: class vs struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CSharpRecordKind {
    /// Emit `public sealed record` (reference type with value semantics)
    #[default]
    Class,
    /// Emit `public readonly record struct` (value type)
    Struct,
}

impl CSharpRecordKind {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "class" | "record" | "record-class" | "reference" => Some(Self::Class),
            "struct" | "record-struct" | "value" => Some(Self::Struct),
            _ => None,
        }
    }
}

/// Options configuring C# 12 / .NET 8+ code generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSharpOptions {
    /// Namespace declaration (e.g. "Crm.Models", "Generated")
    pub namespace: String,
    /// Emit System.Xml.Serialization attributes ([XmlElement], [XmlAttribute], etc.)
    pub emit_xml_attributes: bool,
    /// Emit System.Text.Json.Serialization attributes ([JsonPropertyName], etc.)
    pub emit_json_attributes: bool,
    /// Emit System.Text.Json compile-time source generation context (JsonSerializerContext)
    pub emit_source_gen: bool,
    /// Name of the generated JsonSerializerContext partial class (default: "PolyXmlJsonContext")
    pub source_gen_context_name: String,
    /// Emit IValidatableObject and restriction facet validation logic
    pub emit_validation: bool,
    /// Record emission kind (record class vs record struct)
    pub record_kind: CSharpRecordKind,
    /// Emit immutable records (default) instead of mutable classes.
    #[serde(default = "default_use_records")]
    pub use_records: bool,
    /// Use modern C# 10+ file-scoped namespaces (`namespace Foo;`)
    pub use_file_scoped_namespaces: bool,
    /// Emit root element wrapper records or aliases
    pub emit_root_records: bool,
    /// Custom header text to prepend to generated files
    pub custom_header: Option<String>,
}

fn default_use_records() -> bool {
    true
}

impl Default for CSharpOptions {
    fn default() -> Self {
        Self {
            namespace: "Generated".to_string(),
            emit_xml_attributes: true,
            emit_json_attributes: true,
            emit_source_gen: false,
            source_gen_context_name: "PolyXmlJsonContext".to_string(),
            emit_validation: true,
            record_kind: CSharpRecordKind::Class,
            use_records: true,
            use_file_scoped_namespaces: true,
            emit_root_records: true,
            custom_header: None,
        }
    }
}

/// Sanitizes an identifier into a PascalCase C# type name.
pub fn to_csharp_type_name(raw: &str) -> String {
    let pascal = AsPascalCase(raw).to_string();
    let safe = if pascal.is_empty() {
        "Type".to_string()
    } else if pascal.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Type{}", pascal)
    } else {
        pascal
    };
    sanitize_keyword(&safe, "csharp")
}

/// Sanitizes an identifier into a PascalCase C# property name.
/// If the property name matches the enclosing type name, suffixes "Value" to prevent CS0542.
pub fn to_csharp_property_name(raw: &str, enclosing_type: Option<&str>) -> String {
    let pascal = AsPascalCase(raw).to_string();
    let safe = if pascal.is_empty() {
        "Property".to_string()
    } else if pascal.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Prop{}", pascal)
    } else {
        pascal
    };

    let sanitized = sanitize_keyword(&safe, "csharp");
    if let Some(enclosing) = enclosing_type {
        if sanitized == enclosing || sanitized.trim_start_matches('@') == enclosing {
            return format!("{}Value", sanitized);
        }
    }
    sanitized
}

/// Sanitizes an identifier into a camelCase C# parameter name.
pub fn to_csharp_param_name(raw: &str) -> String {
    let camel = AsLowerCamelCase(raw).to_string();
    let safe = if camel.is_empty() {
        "item".to_string()
    } else if camel.starts_with(|c: char| c.is_ascii_digit()) {
        format!("p{}", camel)
    } else {
        camel
    };
    sanitize_keyword(&safe, "csharp")
}

/// Sanitizes an identifier into a PascalCase C# enum variant.
pub fn to_csharp_variant_name(raw: &str) -> String {
    let normalized = normalize_symbol_name(raw);
    let pascal = AsPascalCase(&normalized).to_string();
    let safe = if pascal.is_empty() {
        "Value".to_string()
    } else if pascal.starts_with(|c: char| c.is_ascii_digit()) {
        format!("V{}", pascal)
    } else {
        pascal
    };
    sanitize_keyword(&safe, "csharp")
}

/// Sanitizes a namespace into dotted PascalCase segments.
pub fn to_csharp_namespace(raw: &str) -> String {
    let segments: Vec<String> = raw
        .split(['.', '/', ':'])
        .filter(|s| !s.is_empty())
        .map(to_csharp_type_name)
        .collect();

    if segments.is_empty() {
        "Generated".to_string()
    } else {
        segments.join(".")
    }
}

/// Language context adapter for C# 12 / .NET 8+.
pub struct CSharpLanguageContext;

impl LanguageContext for CSharpLanguageContext {
    fn target_language(&self) -> &'static str {
        "csharp"
    }

    fn map_primitive(&self, prim: PrimitiveType) -> &'static str {
        match prim {
            PrimitiveType::String
            | PrimitiveType::NormalizedString
            | PrimitiveType::Token
            | PrimitiveType::Language
            | PrimitiveType::Name
            | PrimitiveType::NCName
            | PrimitiveType::NMTOKEN
            | PrimitiveType::NMTOKENS
            | PrimitiveType::Id
            | PrimitiveType::IdRef
            | PrimitiveType::IdRefs
            | PrimitiveType::Entity
            | PrimitiveType::Entities
            | PrimitiveType::AnyUri
            | PrimitiveType::QName
            | PrimitiveType::GYear
            | PrimitiveType::GYearMonth
            | PrimitiveType::GMonth
            | PrimitiveType::GMonthDay
            | PrimitiveType::GDay => "string",
            PrimitiveType::Boolean => "bool",
            PrimitiveType::Byte => "sbyte",
            PrimitiveType::UnsignedByte => "byte",
            PrimitiveType::Short => "short",
            PrimitiveType::UnsignedShort => "ushort",
            PrimitiveType::Int => "int",
            PrimitiveType::UnsignedInt => "uint",
            PrimitiveType::Long
            | PrimitiveType::Integer
            | PrimitiveType::NonPositiveInteger
            | PrimitiveType::NegativeInteger
            | PrimitiveType::NonNegativeInteger
            | PrimitiveType::PositiveInteger => "long",
            PrimitiveType::UnsignedLong => "ulong",
            PrimitiveType::Float => "float",
            PrimitiveType::Double => "double",
            PrimitiveType::Decimal => "decimal",
            PrimitiveType::DateTime => "DateTimeOffset",
            PrimitiveType::Date => "DateOnly",
            PrimitiveType::Time => "TimeOnly",
            PrimitiveType::Duration => "TimeSpan",
            PrimitiveType::Base64Binary | PrimitiveType::HexBinary => "byte[]",
            PrimitiveType::AnyType | PrimitiveType::AnySimpleType => "object",
        }
    }

    fn map_type_ref(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(prim) => self.map_primitive(*prim).to_string(),
            TypeRef::Named(qname) => type_ident(qname),
            TypeRef::Boxed(inner) => self.map_type_ref(inner),
            TypeRef::List(inner) => format!("List<{}>", self.map_type_ref(inner)),
        }
    }
}

/// Modern C# 12 / .NET 8+ Code Generator.
pub struct CSharpCodegen {
    options: CSharpOptions,
    context: CSharpLanguageContext,
}

/// Emitted C# identifier for a named type, disambiguated across namespaces
/// for the IR currently being generated.
fn type_ident(q: &QName) -> String {
    lookup_type_name(q, || to_csharp_type_name(&q.local))
}

impl CSharpCodegen {
    /// Creates a new C# code generator with the given configuration options.
    pub fn new(options: CSharpOptions) -> Self {
        Self {
            options,
            context: CSharpLanguageContext,
        }
    }

    /// Emits all types in the given SchemaIR as a single C# compilation unit.
    pub fn generate_module(&self, ir: &SchemaIR) -> String {
        set_type_name_map(build_type_name_map(ir, to_csharp_type_name));
        let mut out = String::new();

        if let Some(ref header) = self.options.custom_header {
            let trimmed = header.trim();
            if !trimmed.is_empty() {
                out.push_str(trimmed);
                out.push_str("\n\n");
            }
        }
        writeln!(out, "// <auto-generated/>").unwrap();
        writeln!(
            out,
            "// @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)"
        )
        .unwrap();
        writeln!(out, "#nullable enable\n").unwrap();
        writeln!(out, "using System;").unwrap();
        writeln!(out, "using System.Collections.Generic;").unwrap();
        if self.options.emit_validation {
            writeln!(out, "using System.ComponentModel.DataAnnotations;").unwrap();
            writeln!(out, "using System.Text.RegularExpressions;").unwrap();
        }
        if self.options.emit_xml_attributes {
            writeln!(out, "using System.Xml.Serialization;").unwrap();
        }
        if self.options.emit_json_attributes || self.options.emit_source_gen {
            writeln!(out, "using System.Text.Json.Serialization;").unwrap();
        }
        writeln!(out).unwrap();

        let ns = to_csharp_namespace(&self.options.namespace);
        if self.options.use_file_scoped_namespaces {
            writeln!(out, "namespace {};\n", ns).unwrap();
        } else {
            writeln!(out, "namespace {}\n{{\n", ns).unwrap();
        }

        let indent = if self.options.use_file_scoped_namespaces {
            ""
        } else {
            "    "
        };

        // Emit SimpleTypes / Enums
        for def in ir.types.values() {
            if let TypeDef::Enum(e) = def {
                self.emit_enum(&mut out, e, indent);
            } else if let TypeDef::Simple(s) = def {
                let mut simple = s.clone();
                if !simple.facets.patterns.is_empty() {
                    simple.base_type = super::primitive_base(&simple.base_type, ir).clone();
                }
                self.emit_simple(&mut out, &simple, indent);
            }
        }

        // Emit Unions (Choices)
        for def in ir.types.values() {
            if let TypeDef::Union(u) = def {
                self.emit_union(&mut out, u, ir, indent);
            }
        }

        // Emit Structs (ComplexTypes)
        for def in ir.types.values() {
            if let TypeDef::Struct(s) = def {
                self.emit_struct(&mut out, s, ir, indent);
            }
        }

        // Emit Root Element Records if requested
        if self.options.emit_root_records {
            self.emit_root_elements(&mut out, ir, indent);
        }

        // Emit Source Generator Context if requested
        if self.options.emit_source_gen {
            self.emit_source_gen_context(&mut out, ir, indent);
        }

        if !self.options.use_file_scoped_namespaces {
            writeln!(out, "}}").unwrap();
        }

        out
    }

    /// Emits generated C# files suitable for multi-file project outputs.
    pub fn generate_files(&self, ir: &SchemaIR, base_name: &str) -> Vec<(String, String)> {
        let code = self.generate_module(ir);
        let filename = format!("{}.cs", to_csharp_type_name(base_name));
        vec![(filename, code)]
    }

    fn emit_source_gen_context(&self, out: &mut String, ir: &SchemaIR, indent: &str) {
        writeln!(out).unwrap();
        writeln!(
            out,
            "{}/// <summary>\n{}/// Source-generated JsonSerializerContext for Native AOT and high-throughput serialization.\n{}/// </summary>",
            indent, indent, indent
        )
        .unwrap();
        writeln!(
            out,
            "{}[JsonSourceGenerationOptions(WriteIndented = true)]",
            indent
        )
        .unwrap();

        // Enums
        for def in ir.types.values() {
            if let TypeDef::Enum(e) = def {
                let name = type_ident(&e.qname);
                writeln!(out, "{}[JsonSerializable(typeof({}))]", indent, name).unwrap();
            }
        }

        // Simple types
        for def in ir.types.values() {
            if let TypeDef::Simple(s) = def {
                let name = type_ident(&s.qname);
                writeln!(out, "{}[JsonSerializable(typeof({}))]", indent, name).unwrap();
            }
        }

        // Unions
        for def in ir.types.values() {
            if let TypeDef::Union(u) = def {
                let name = type_ident(&u.qname);
                writeln!(out, "{}[JsonSerializable(typeof({}))]", indent, name).unwrap();
                writeln!(out, "{}[JsonSerializable(typeof(List<{}>))]", indent, name).unwrap();
            }
        }

        // Structs
        for def in ir.types.values() {
            if let TypeDef::Struct(s) = def {
                let name = type_ident(&s.qname);
                writeln!(out, "{}[JsonSerializable(typeof({}))]", indent, name).unwrap();
                writeln!(out, "{}[JsonSerializable(typeof(List<{}>))]", indent, name).unwrap();
            }
        }

        if !self.options.use_records && self.options.emit_root_records {
            for elem in ir.elements.values() {
                let name = to_csharp_type_name(&elem.qname.local);
                if name != self.context.map_type_ref(&elem.type_ref) {
                    writeln!(out, "{}[JsonSerializable(typeof({}))]", indent, name).unwrap();
                }
            }
        }
        let ctx_name = &self.options.source_gen_context_name;
        writeln!(
            out,
            "{}public partial class {} : JsonSerializerContext\n{}{{",
            indent, ctx_name, indent
        )
        .unwrap();
        writeln!(out, "{}}}", indent).unwrap();
    }

    fn emit_enum(&self, out: &mut String, e: &EnumDef, indent: &str) {
        let enum_name = type_ident(&e.qname);
        if let Some(ref doc) = e.documentation {
            self.emit_docstring(out, doc, indent);
        }

        if self.options.emit_json_attributes {
            writeln!(
                out,
                "{}[JsonConverter(typeof(JsonStringEnumConverter<{}>))]",
                indent, enum_name
            )
            .unwrap();
        }
        writeln!(out, "{}public enum {}", indent, enum_name).unwrap();
        writeln!(out, "{}{{", indent).unwrap();

        let mut seen_variants = HashSet::new();
        let variant_names: Vec<String> = e
            .variants
            .iter()
            .map(|v| self.unique_variant_name(&v.name, &mut seen_variants))
            .collect();

        for (variant, variant_name) in e.variants.iter().zip(&variant_names) {
            if let Some(ref doc) = variant.documentation {
                self.emit_docstring(out, doc, &format!("{}    ", indent));
            }
            if self.options.emit_xml_attributes {
                writeln!(out, "{}    [XmlEnum(\"{}\")]", indent, variant.value).unwrap();
            }
            writeln!(out, "{}    {},", indent, variant_name).unwrap();
        }

        writeln!(out, "{}}}\n", indent).unwrap();

        // Emit helper extension methods
        writeln!(out, "{}public static class {}Extensions", indent, enum_name).unwrap();
        writeln!(out, "{}{{", indent).unwrap();

        // IsValid extension
        writeln!(
            out,
            "{}    public static bool IsValid(this {} value) => value switch",
            indent, enum_name
        )
        .unwrap();
        writeln!(out, "{}    {{", indent).unwrap();
        for variant_name in &variant_names {
            writeln!(
                out,
                "{}        {}.{} => true,",
                indent, enum_name, variant_name
            )
            .unwrap();
        }
        writeln!(out, "{}        _ => false", indent).unwrap();
        writeln!(out, "{}    }};\n", indent).unwrap();

        // ToXmlValue extension
        writeln!(
            out,
            "{}    public static string ToXmlValue(this {} value) => value switch",
            indent, enum_name
        )
        .unwrap();
        writeln!(out, "{}    {{", indent).unwrap();
        for (variant, variant_name) in e.variants.iter().zip(&variant_names) {
            writeln!(
                out,
                "{}        {}.{} => \"{}\",",
                indent, enum_name, variant_name, variant.value
            )
            .unwrap();
        }
        writeln!(
            out,
            "{}        _ => throw new ArgumentOutOfRangeException(nameof(value), value, null)",
            indent
        )
        .unwrap();
        writeln!(out, "{}    }};", indent).unwrap();

        writeln!(out, "{}}}\n", indent).unwrap();
    }

    fn emit_simple(&self, out: &mut String, s: &SimpleTypeDef, indent: &str) {
        if !s.facets.is_empty() || !self.options.use_records {
            let type_name = type_ident(&s.qname);
            let base_type = self.context.map_type_ref(&s.base_type);

            if let Some(ref doc) = s.documentation {
                self.emit_docstring(out, doc, indent);
            }

            if !self.options.use_records {
                writeln!(
                    out,
                    "{}public class {}{}\n{}{{",
                    indent,
                    type_name,
                    if self.options.emit_validation {
                        " : IValidatableObject"
                    } else {
                        ""
                    },
                    indent
                )
                .unwrap();
                if self.options.emit_xml_attributes {
                    writeln!(out, "{}    [XmlText]", indent).unwrap();
                }
                writeln!(
                    out,
                    "{}    public {} Value {{ get; set; }} = default!;",
                    indent, base_type
                )
                .unwrap();
                writeln!(
                    out,
                    "{}    public {}() {{ }}\n{}    public {}({} value) {{ Value = value; }}",
                    indent, type_name, indent, type_name, base_type
                )
                .unwrap();
                if self.options.emit_validation {
                    writeln!(out, "{}    public IEnumerable<ValidationResult> Validate(ValidationContext validationContext)\n{}    {{", indent, indent).unwrap();
                    self.emit_facet_checks(out, &s.facets, "Value", &format!("{}        ", indent));
                    writeln!(out, "{}        yield break;\n{}    }}", indent, indent).unwrap();
                }
                writeln!(out, "{}}}\n", indent).unwrap();
                return;
            }
            writeln!(
                out,
                "{}public sealed record {}([property: XmlText] {} Value) : IValidatableObject",
                indent, type_name, base_type
            )
            .unwrap();
            writeln!(out, "{}{{", indent).unwrap();
            writeln!(
                out,
                "{}    public {}() : this(default({})!) {{ }}",
                indent, type_name, base_type
            )
            .unwrap();
            writeln!(out).unwrap();

            // Validation
            writeln!(
                out,
                "{}    public IEnumerable<ValidationResult> Validate(ValidationContext validationContext)",
                indent
            )
            .unwrap();
            writeln!(out, "{}    {{", indent).unwrap();
            self.emit_facet_checks(out, &s.facets, "Value", &format!("{}        ", indent));
            writeln!(out, "{}        yield break;", indent).unwrap();
            writeln!(out, "{}    }}", indent).unwrap();
            writeln!(out, "{}}}\n", indent).unwrap();
        }
    }

    fn emit_union(&self, out: &mut String, u: &UnionDef, ir: &SchemaIR, indent: &str) {
        let choice_name = type_ident(&u.qname);
        if let Some(ref doc) = u.documentation {
            self.emit_docstring(out, doc, indent);
        }

        // XmlInclude attributes for element-choice polymorphism.
        if self.options.emit_xml_attributes && !u.is_lexical() {
            for branch in &u.branches {
                let variant_name = to_csharp_type_name(&branch.variant_name);
                writeln!(
                    out,
                    "{}[XmlInclude(typeof({}.{}))]",
                    indent, choice_name, variant_name
                )
                .unwrap();
            }
        }

        if !self.options.use_records && self.options.emit_json_attributes {
            for branch in &u.branches {
                writeln!(
                    out,
                    "{}[JsonDerivedType(typeof({}.{}), {:?})]",
                    indent,
                    choice_name,
                    to_csharp_type_name(&branch.variant_name),
                    branch.xml_name
                )
                .unwrap();
            }
        }
        writeln!(
            out,
            "{}public abstract {} {}",
            indent,
            if self.options.use_records {
                "record"
            } else {
                "class"
            },
            choice_name
        )
        .unwrap();
        writeln!(out, "{}{{", indent).unwrap();

        for branch in &u.branches {
            let variant_name = to_csharp_type_name(&branch.variant_name);
            let branch_type = self.context.map_type_ref(&branch.type_ref);

            if let Some(ref doc) = branch.documentation {
                self.emit_docstring(out, doc, &format!("{}    ", indent));
            }

            let mut branch_attrs = Vec::new();
            if self.options.emit_xml_attributes && !u.is_lexical() {
                branch_attrs.push(format!("XmlElement(\"{}\")", branch.xml_name));
            }
            if self.options.emit_json_attributes && !u.is_lexical() {
                branch_attrs.push(format!("JsonPropertyName(\"{}\")", branch.xml_name));
            }
            let xml_attr = if branch_attrs.is_empty() {
                String::new()
            } else {
                format!("[property: {}] ", branch_attrs.join(", "))
            };

            if !self.options.use_records {
                writeln!(
                    out,
                    "{}    public sealed class {} : {}\n{}    {{",
                    indent, variant_name, choice_name, indent
                )
                .unwrap();
                writeln!(
                    out,
                    "{}        {}public {} Value {{ get; set; }} = default!;",
                    indent,
                    xml_attr.replace("[property: ", "["),
                    branch_type
                )
                .unwrap();
                writeln!(out, "{}        public {}() {{ }}\n{}        public {}({} value) {{ Value = value; }}\n{}    }}", indent, variant_name, indent, variant_name, branch_type, indent).unwrap();
                continue;
            }
            writeln!(
                out,
                "{}    public sealed record {}({}{} Value) : {}",
                indent, variant_name, xml_attr, branch_type, choice_name
            )
            .unwrap();
            writeln!(out, "{}    {{", indent).unwrap();
            writeln!(
                out,
                "{}        public {}() : this(default({})!) {{ }}",
                indent, variant_name, branch_type
            )
            .unwrap();
            writeln!(out, "{}    }}", indent).unwrap();
            writeln!(out).unwrap();
        }

        if u.is_lexical() {
            self.emit_lexical_union_helpers(out, u, ir, indent);
        }
        writeln!(out, "{}}}\n", indent).unwrap();
    }

    fn emit_lexical_union_helpers(
        &self,
        out: &mut String,
        u: &UnionDef,
        ir: &SchemaIR,
        indent: &str,
    ) {
        let name = type_ident(&u.qname);
        let _ = writeln!(
            out,
            "{}    public static {} Parse(string text)",
            indent, name
        );
        let _ = writeln!(out, "{}    {{", indent);
        let _ = writeln!(out, "{}        var value = text.Trim();", indent);
        for (idx, branch) in u.branches.iter().enumerate() {
            let variant = to_csharp_type_name(&branch.variant_name);
            let mapped = self.context.map_type_ref(&branch.type_ref);
            if let TypeRef::Named(qname) = &branch.type_ref {
                if let Some(TypeDef::Enum(def)) = ir.types.get(qname) {
                    for item in &def.variants {
                        let item_name = to_csharp_variant_name(&item.name);
                        let _ = writeln!(
                            out,
                            "{}        if (value == {:?}) return new {}({}.{});",
                            indent, item.value, variant, mapped, item_name
                        );
                    }
                    continue;
                }
            }
            let simple = match &branch.type_ref {
                TypeRef::Named(qname) => ir.types.get(qname).and_then(|def| match def {
                    TypeDef::Simple(s) => Some(s.as_ref()),
                    _ => None,
                }),
                _ => None,
            };
            let base = super::primitive_base(&branch.type_ref, ir);
            let base_name = self.context.map_type_ref(base);
            let wrap = if simple.is_some() {
                format!("new {}(candidate{})", mapped, idx)
            } else {
                format!("candidate{}", idx)
            };
            if base_name == "string" {
                if let Some(simple) = simple.filter(|s| !s.facets.patterns.is_empty()) {
                    let checks = simple
                        .facets
                        .patterns
                        .iter()
                        .map(|pattern| {
                            format!("Regex.IsMatch(value, {:?})", format!(r"\A(?:{pattern})\z"))
                        })
                        .collect::<Vec<_>>()
                        .join(" && ");
                    let _ = writeln!(
                        out,
                        "{}        if ({}) {{ var candidate{} = value; return new {}({}); }}",
                        indent, checks, idx, variant, wrap
                    );
                } else {
                    let _ = writeln!(
                        out,
                        "{}        {{ var candidate{} = value; return new {}({}); }}",
                        indent, idx, variant, wrap
                    );
                }
            } else if base_name == "bool" {
                let _ = writeln!(out, "{}        if (value == \"true\" || value == \"1\") {{ var candidate{} = true; return new {}({}); }}", indent, idx, variant, wrap);
                let _ = writeln!(out, "{}        if (value == \"false\" || value == \"0\") {{ var candidate{} = false; return new {}({}); }}", indent, idx, variant, wrap);
            } else if base_name == "DateOnly" {
                let _ = writeln!(out, "{}        if (DateOnly.TryParseExact(value, \"yyyy-MM-dd\", System.Globalization.CultureInfo.InvariantCulture, System.Globalization.DateTimeStyles.None, out var candidate{})) return new {}({});", indent, idx, variant, wrap);
            } else {
                let _ = writeln!(
                    out,
                    "{}        if ({}.TryParse(value, out var candidate{})) return new {}({});",
                    indent, base_name, idx, variant, wrap
                );
            }
        }
        let _ = writeln!(
            out,
            "{}        throw new FormatException(\"No {} union member accepts the value\");",
            indent, name
        );
        let _ = writeln!(out, "{}    }}", indent);
        let _ = writeln!(
            out,
            "{}    public string ToXmlString() => this switch",
            indent
        );
        let _ = writeln!(out, "{}    {{", indent);
        for branch in &u.branches {
            let variant = to_csharp_type_name(&branch.variant_name);
            let value = if matches!(
                super::primitive_base(&branch.type_ref, ir),
                TypeRef::Primitive(PrimitiveType::Date)
            ) {
                "item.Value.ToString(\"yyyy-MM-dd\", System.Globalization.CultureInfo.InvariantCulture)".to_string()
            } else {
                match &branch.type_ref {
                TypeRef::Named(qname) if matches!(ir.types.get(qname), Some(TypeDef::Enum(_))) => "item.Value.ToXmlValue()".to_string(),
                TypeRef::Named(qname) if matches!(ir.types.get(qname), Some(TypeDef::Simple(_))) => "Convert.ToString(item.Value.Value, System.Globalization.CultureInfo.InvariantCulture) ?? \"\"".to_string(),
                _ => "Convert.ToString(item.Value, System.Globalization.CultureInfo.InvariantCulture) ?? \"\"".to_string(),
            }
            };
            let _ = writeln!(out, "{}        {} item => {},", indent, variant, value);
        }
        let _ = writeln!(
            out,
            "{}        _ => throw new InvalidOperationException()",
            indent
        );
        let _ = writeln!(out, "{}    }};", indent);
    }

    fn emit_struct(&self, out: &mut String, s: &StructDef, ir: &SchemaIR, indent: &str) {
        let struct_name = type_ident(&s.qname);
        if let Some(ref doc) = s.documentation {
            self.emit_docstring(out, doc, indent);
        }

        // XmlRoot attribute if enabled
        if self.options.emit_xml_attributes {
            if let Some(ref ns) = s.qname.namespace {
                writeln!(
                    out,
                    "{}[XmlRoot(\"{}\", Namespace = \"{}\")]",
                    indent, s.qname.local, ns
                )
                .unwrap();
            } else {
                writeln!(out, "{}[XmlRoot(\"{}\")]", indent, s.qname.local).unwrap();
            }
        }

        let record_keyword = match self.options.record_kind {
            CSharpRecordKind::Class => "record",
            CSharpRecordKind::Struct => "readonly record struct",
        };

        // Determine inheritance / interface implementation
        let mut base_clause = Vec::new();
        if let Some(ref base_qname) = s.base_type {
            if matches!(ir.types.get(base_qname), Some(TypeDef::Struct(_))) {
                let base_name = type_ident(base_qname);
                base_clause.push(base_name);
            }
        }
        if self.options.emit_validation {
            base_clause.push("IValidatableObject".to_string());
        }

        let implements_str = if base_clause.is_empty() {
            String::new()
        } else {
            format!(" : {}", base_clause.join(", "))
        };

        let mut seen_props = HashSet::new();
        let prop_names: Vec<String> = s
            .fields
            .iter()
            .map(|f| self.unique_property_name(&f.name, &mut seen_props, Some(&struct_name)))
            .collect();

        let has_lexical_union = self.options.emit_xml_attributes && s.fields.iter().any(|f| {
            matches!(&f.type_ref, TypeRef::Named(q) if matches!(ir.types.get(q), Some(TypeDef::Union(u)) if u.is_lexical()))
        });

        if self.options.use_records && has_lexical_union {
            writeln!(
                out,
                "{}public record {}{}\n{}{{",
                indent, struct_name, implements_str, indent
            )
            .unwrap();
            for (f, name) in s.fields.iter().zip(&prop_names) {
                let ty = self.map_field_type(f, ir);
                if let TypeRef::Named(q) = &f.type_ref {
                    if matches!(ir.types.get(q), Some(TypeDef::Union(u)) if u.is_lexical()) {
                        let _ = writeln!(out, "{}    [XmlIgnore]", indent);
                        if self.options.emit_json_attributes {
                            let _ =
                                writeln!(out, "{}    [JsonPropertyName({:?})]", indent, f.xml_name);
                        }
                        let _ = writeln!(
                            out,
                            "{}    public {} {} {{ get; set; }} = default!;",
                            indent, ty, name
                        );
                        let _ = writeln!(out, "{}    [XmlElement({:?})]", indent, f.xml_name);
                        if self.options.emit_json_attributes {
                            let _ = writeln!(out, "{}    [JsonIgnore]", indent);
                        }
                        let _ = writeln!(out, "{}    public string? {}Xml {{ get => {}?.ToXmlString(); set => {} = value is null ? default! : {}.Parse(value); }}", indent, name, name, name, type_ident(q));
                        continue;
                    }
                }
                let attrs = self
                    .build_field_attributes(f, ir)
                    .replace("[property: ", "[");
                let _ = writeln!(
                    out,
                    "{}    {}public {} {} {{ get; set; }} = default!;",
                    indent, attrs, ty, name
                );
            }
            if self.options.emit_validation {
                self.emit_struct_validator(out, s, &prop_names, indent);
            }
            writeln!(out, "{}}}\n", indent).unwrap();
            return;
        }

        if !self.options.use_records {
            writeln!(
                out,
                "{}public class {}{}\n{}{{",
                indent, struct_name, implements_str, indent
            )
            .unwrap();
            writeln!(out, "{}    public {}() {{ }}", indent, struct_name).unwrap();
            for (f, name) in s.fields.iter().zip(&prop_names) {
                let ty = self.map_field_type(f, ir);
                if let TypeRef::Named(q) = &f.type_ref {
                    if self.options.emit_xml_attributes
                        && matches!(ir.types.get(q), Some(TypeDef::Union(u)) if u.is_lexical())
                    {
                        let _ = writeln!(out, "{}    [XmlIgnore]", indent);
                        if self.options.emit_json_attributes {
                            let _ =
                                writeln!(out, "{}    [JsonPropertyName({:?})]", indent, f.xml_name);
                        }
                        let _ = writeln!(
                            out,
                            "{}    public {} {} {{ get; set; }} = default!;",
                            indent, ty, name
                        );
                        let _ = writeln!(out, "{}    [XmlElement({:?})]", indent, f.xml_name);
                        if self.options.emit_json_attributes {
                            let _ = writeln!(out, "{}    [JsonIgnore]", indent);
                        }
                        let _ = writeln!(out, "{}    public string? {}Xml {{ get => {}?.ToXmlString(); set => {} = value is null ? default! : {}.Parse(value); }}", indent, name, name, name, type_ident(q));
                        continue;
                    }
                }
                let attrs = self
                    .build_field_attributes(f, ir)
                    .replace("[property: ", "[");
                let initial = if f.cardinality.is_list() || f.type_ref.is_list() {
                    "new()"
                } else {
                    "default!"
                };
                writeln!(
                    out,
                    "{}    {}public {} {} {{ get; set; }} = {};",
                    indent, attrs, ty, name, initial
                )
                .unwrap();
            }
            if self.options.emit_validation {
                self.emit_struct_validator(out, s, &prop_names, indent);
            }
            writeln!(out, "{}}}\n", indent).unwrap();
            return;
        }

        // Collect fields and parameters
        if s.fields.is_empty() {
            writeln!(
                out,
                "{}public {} {}{};",
                indent, record_keyword, struct_name, implements_str
            )
            .unwrap();
            writeln!(out).unwrap();
            return;
        }

        writeln!(out, "{}public {} {}(", indent, record_keyword, struct_name).unwrap();

        for (i, (f, prop_name)) in s.fields.iter().zip(&prop_names).enumerate() {
            let field_type = self.map_field_type(f, ir);
            let is_opt = f.cardinality.is_optional()
                || f.nillable
                || (f.cardinality.is_list() && f.cardinality.min_occurs == 0);

            // A parameter in C# primary constructor can only have a default value (= null)
            // if all subsequent parameters also have default values (CS1737).
            let can_have_default = is_opt
                && s.fields[i + 1..].iter().all(|next_f| {
                    next_f.cardinality.is_optional()
                        || next_f.nillable
                        || (next_f.cardinality.is_list() && next_f.cardinality.min_occurs == 0)
                });

            let is_last = i == s.fields.len() - 1;
            let comma = if is_last { "" } else { "," };

            let default_val = if can_have_default { " = null" } else { "" };

            let field_attrs = self.build_field_attributes(f, ir);

            if let Some(ref doc) = f.documentation {
                self.emit_docstring(out, doc, &format!("{}    ", indent));
            }

            writeln!(
                out,
                "{}    {}{} {}{}{}",
                indent, field_attrs, field_type, prop_name, default_val, comma
            )
            .unwrap();
        }

        writeln!(out, "{}){}", indent, implements_str).unwrap();
        writeln!(out, "{}{{", indent).unwrap();

        // Parameterless constructor for XmlSerializer compatibility
        write!(out, "{}    public {}() : this(", indent, struct_name).unwrap();
        for (i, f) in s.fields.iter().enumerate() {
            let field_type = self.map_field_type(f, ir);
            let is_opt = f.cardinality.is_optional()
                || f.nillable
                || (f.cardinality.is_list() && f.cardinality.min_occurs == 0);
            let default_arg = if is_opt {
                format!("default({})", field_type)
            } else {
                format!("default({})!", field_type)
            };

            let comma = if i == s.fields.len() - 1 { "" } else { ", " };
            write!(out, "{}{}", default_arg, comma).unwrap();
        }
        writeln!(out, ") {{ }}\n").unwrap();

        // IValidatableObject implementation
        if self.options.emit_validation {
            self.emit_struct_validator(out, s, &prop_names, indent);
        }

        writeln!(out, "{}}}\n", indent).unwrap();
    }

    fn unique_property_name(
        &self,
        name: &str,
        seen: &mut HashSet<String>,
        enclosing: Option<&str>,
    ) -> String {
        let base = to_csharp_property_name(name, enclosing);
        let mut candidate = base.clone();
        let mut counter = 1;
        while seen.contains(&candidate) {
            counter += 1;
            candidate = format!("{}{}", base, counter);
        }
        seen.insert(candidate.clone());
        candidate
    }

    fn unique_variant_name(&self, name: &str, seen: &mut HashSet<String>) -> String {
        let base = to_csharp_variant_name(name);
        let mut candidate = base.clone();
        let mut counter = 1;
        while seen.contains(&candidate) {
            counter += 1;
            candidate = format!("{}{}", base, counter);
        }
        seen.insert(candidate.clone());
        candidate
    }

    fn emit_struct_validator(
        &self,
        out: &mut String,
        s: &StructDef,
        prop_names: &[String],
        indent: &str,
    ) {
        let _struct_name = type_ident(&s.qname);
        let new_kw = if !self.options.use_records {
            if s.base_type.is_some() {
                "override "
            } else {
                "virtual "
            }
        } else if s.base_type.is_some() {
            "new "
        } else {
            ""
        };
        writeln!(
            out,
            "{}    public {}IEnumerable<ValidationResult> Validate(ValidationContext validationContext)",
            indent, new_kw
        )
        .unwrap();
        writeln!(out, "{}    {{", indent).unwrap();

        if !self.options.use_records && s.base_type.is_some() {
            writeln!(out, "{}        foreach (var result in base.Validate(validationContext)) yield return result;", indent).unwrap();
        }
        let mut has_checks = false;
        for (f, prop_name) in s.fields.iter().zip(prop_names) {
            let is_opt = f.cardinality.is_optional()
                || f.nillable
                || (f.cardinality.is_list() && f.cardinality.min_occurs == 0);

            if let Some(ref facets) = f.facets {
                if is_opt {
                    writeln!(out, "{}        if ({} is not null)", indent, prop_name).unwrap();
                    writeln!(out, "{}        {{", indent).unwrap();
                    self.emit_facet_checks(
                        out,
                        facets,
                        prop_name,
                        &format!("{}            ", indent),
                    );
                    writeln!(out, "{}        }}", indent).unwrap();
                } else {
                    self.emit_facet_checks(out, facets, prop_name, &format!("{}        ", indent));
                }
                has_checks = true;
            }
        }

        let _ = has_checks;
        writeln!(out, "{}        yield break;", indent).unwrap();
        writeln!(out, "{}    }}", indent).unwrap();
    }

    fn emit_facet_checks(
        &self,
        out: &mut String,
        facets: &RestrictionFacets,
        target: &str,
        indent: &str,
    ) {
        if let Some(min_len) = facets.min_length {
            writeln!(
                out,
                "{}if ({}.Length < {}) yield return new ValidationResult(\"{} length must be >= {}\", [nameof({})]);",
                indent, target, min_len, target, min_len, target
            )
            .unwrap();
        }
        if let Some(max_len) = facets.max_length {
            writeln!(
                out,
                "{}if ({}.Length > {}) yield return new ValidationResult(\"{} length must be <= {}\", [nameof({})]);",
                indent, target, max_len, target, max_len, target
            )
            .unwrap();
        }
        for pattern in &facets.patterns {
            let escaped = format!(r"\A(?:{pattern})\z")
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            writeln!(
                out,
                "{}if (!Regex.IsMatch({}.ToString() ?? \"\", \"{}\")) yield return new ValidationResult(\"{} does not match pattern {}\", [nameof({})]);",
                indent, target, escaped, target, escaped, target
            )
            .unwrap();
        }
        if let Some(ref min_inc) = facets.min_inclusive {
            writeln!(
                out,
                "{}if ({} < {}) yield return new ValidationResult(\"{} must be >= {}\", [nameof({})]);",
                indent, target, min_inc, target, min_inc, target
            )
            .unwrap();
        }
        if let Some(ref max_inc) = facets.max_inclusive {
            writeln!(
                out,
                "{}if ({} > {}) yield return new ValidationResult(\"{} must be <= {}\", [nameof({})]);",
                indent, target, max_inc, target, max_inc, target
            )
            .unwrap();
        }
        if let Some(ref min_exc) = facets.min_exclusive {
            writeln!(
                out,
                "{}if ({} <= {}) yield return new ValidationResult(\"{} must be > {}\", [nameof({})]);",
                indent, target, min_exc, target, min_exc, target
            )
            .unwrap();
        }
        if let Some(ref max_exc) = facets.max_exclusive {
            writeln!(
                out,
                "{}if ({} >= {}) yield return new ValidationResult(\"{} must be < {}\", [nameof({})]);",
                indent, target, max_exc, target, max_exc, target
            )
            .unwrap();
        }
    }

    fn map_field_type(&self, f: &FieldDef, ir: &SchemaIR) -> String {
        let base_type = match &f.type_ref {
            TypeRef::Primitive(p) => self.context.map_primitive(*p).to_string(),
            TypeRef::Named(qn) => {
                if let Some(TypeDef::Union(u)) = ir.types.get(qn) {
                    type_ident(&u.qname)
                } else {
                    type_ident(qn)
                }
            }
            TypeRef::Boxed(inner) => self.context.map_type_ref(inner),
            TypeRef::List(inner) => format!("List<{}>", self.context.map_type_ref(inner)),
        };

        if f.cardinality.is_list() {
            if f.cardinality.min_occurs == 0 || f.nillable {
                format!("List<{}>?", base_type)
            } else {
                format!("List<{}>", base_type)
            }
        } else if f.cardinality.is_optional() || f.nillable {
            format!("{}?", base_type)
        } else {
            base_type
        }
    }

    fn build_field_attributes(&self, f: &FieldDef, ir: &SchemaIR) -> String {
        let mut parts = Vec::new();

        if self.options.emit_xml_attributes {
            // If the field is a choice (UnionDef), emit XmlElement("branchXml", typeof(BranchType))
            let named_qname = match &f.type_ref {
                TypeRef::Named(q) => Some(q),
                TypeRef::List(inner) | TypeRef::Boxed(inner) => match &**inner {
                    TypeRef::Named(q) => Some(q),
                    _ => None,
                },
                _ => None,
            };
            if let Some(qname) = named_qname {
                if let Some(TypeDef::Union(u)) = ir.types.get(qname) {
                    let choice_name = type_ident(&u.qname);
                    for branch in &u.branches {
                        let variant_name = to_csharp_type_name(&branch.variant_name);
                        parts.push(format!(
                            "XmlElement(\"{}\", typeof({}.{}))",
                            branch.xml_name, choice_name, variant_name
                        ));
                    }
                }
            }
            if parts.is_empty() {
                match f.kind {
                    FieldKind::Attribute => parts.push(format!("XmlAttribute(\"{}\")", f.xml_name)),
                    FieldKind::Text => parts.push("XmlText".to_string()),
                    FieldKind::Any => parts.push("XmlAnyElement".to_string()),
                    FieldKind::AnyAttribute => parts.push("XmlAnyAttribute".to_string()),
                    FieldKind::Element => parts.push(format!("XmlElement(\"{}\")", f.xml_name)),
                }
            }
        }

        if self.options.emit_json_attributes {
            let json_name = match f.kind {
                FieldKind::Text => "value",
                _ => {
                    if f.xml_name.is_empty() {
                        &f.name
                    } else {
                        &f.xml_name
                    }
                }
            };
            if f.kind != FieldKind::Any && f.kind != FieldKind::AnyAttribute {
                parts.push(format!("JsonPropertyName(\"{}\")", json_name));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("[property: {}] ", parts.join(", "))
        }
    }

    fn emit_root_elements(&self, out: &mut String, ir: &SchemaIR, indent: &str) {
        if !self.options.emit_root_records || ir.elements.is_empty() {
            return;
        }

        for elem in ir.elements.values() {
            let elem_name = to_csharp_type_name(&elem.qname.local);
            let target_type = self.context.map_type_ref(&elem.type_ref);

            if elem_name != target_type {
                if self.options.emit_xml_attributes {
                    if let Some(ref ns) = elem.qname.namespace {
                        writeln!(
                            out,
                            "{}[XmlRoot(\"{}\", Namespace = \"{}\")]",
                            indent, elem.qname.local, ns
                        )
                        .unwrap();
                    } else {
                        writeln!(out, "{}[XmlRoot(\"{}\")]", indent, elem.qname.local).unwrap();
                    }
                }
                if !self.options.use_records {
                    let can_extend = matches!(&elem.type_ref, TypeRef::Named(q) if matches!(ir.types.get(q), Some(TypeDef::Struct(_) | TypeDef::Simple(_))));
                    if !can_extend {
                        writeln!(
                            out,
                            "{}public sealed class {}\n{}{{",
                            indent, elem_name, indent
                        )
                        .unwrap();
                        if self.options.emit_xml_attributes {
                            let attribute = if matches!(&elem.type_ref, TypeRef::Named(q)
                                if matches!(ir.types.get(q), Some(TypeDef::Union(_))))
                            {
                                "XmlElement"
                            } else {
                                "XmlText"
                            };
                            writeln!(out, "{}    [{}]", indent, attribute).unwrap();
                        }
                        writeln!(
                            out,
                            "{}    public {} Value {{ get; set; }} = default!;\n{}}}\n",
                            indent, target_type, indent
                        )
                        .unwrap();
                        continue;
                    }
                }
                writeln!(
                    out,
                    "{}public sealed {} {} : {}\n{}{{",
                    indent,
                    if self.options.use_records {
                        "record"
                    } else {
                        "class"
                    },
                    elem_name,
                    target_type,
                    indent
                )
                .unwrap();
                writeln!(out, "{}    public {}() : base() {{ }}", indent, elem_name).unwrap();
                writeln!(out, "{}}}\n", indent).unwrap();
            }
        }
    }

    fn emit_docstring(&self, out: &mut String, doc: &str, indent: &str) {
        writeln!(out, "{}/// <summary>", indent).unwrap();
        for line in doc.lines() {
            writeln!(out, "{}/// {}", indent, line.trim()).unwrap();
        }
        writeln!(out, "{}/// </summary>", indent).unwrap();
    }
}
