mod codec;
mod models;

use std::collections::HashSet;
use std::fmt::Write as FmtWrite;

use heck::{AsLowerCamelCase, AsPascalCase, AsShoutySnakeCase};
use serde::{Deserialize, Serialize};

use crate::codegen::{
    build_type_name_map, lookup_type_name, normalize_symbol_name, sanitize_keyword,
    set_type_name_map, LanguageContext,
};
use crate::ir::{
    EnumDef, PrimitiveType, QName, RestrictionFacets, SchemaIR, SimpleTypeDef, StructDef, TypeDef,
    TypeRef, UnionDef,
};

/// Target backend for Java code emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum JavaBackend {
    /// Zero-dependency Java 22+ records (default).
    #[default]
    Standard,
    /// Jackson-annotated records for Spring Boot / enterprise JSON+XML binding.
    Jackson,
}

impl JavaBackend {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "standard" | "std" | "default" => Some(Self::Standard),
            "jackson" | "spring" | "spring-boot" | "enterprise" => Some(Self::Jackson),
            _ => None,
        }
    }

    pub fn is_jackson(self) -> bool {
        matches!(self, Self::Jackson)
    }
}

/// Options configuring Java 22+ code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaOptions {
    /// Java package declaration for generated types (default: "generated.models")
    pub package_name: String,
    /// Target serialization backend (default: Standard / zero-dependency).
    pub backend: JavaBackend,
    /// Use Java 22 `record` instead of traditional classes (default: true)
    pub use_records: bool,
    /// Emit fluent builders for model types.
    #[serde(default)]
    pub emit_builder: bool,
    /// Emit reflection-free StAX XML companion codecs.
    #[serde(default)]
    pub emit_direct_codec: bool,
    /// Generate facet validation in compact constructors (default: true)
    pub validate_facets: bool,
    /// Emit Jakarta Bean Validation annotations (adds an optional API dependency).
    #[serde(default)]
    pub bean_validation: bool,
    /// Emit top-level aliases for root elements (default: true)
    pub emit_root_aliases: bool,
    /// Custom header text to prepend to generated files (default: None)
    pub custom_header: Option<String>,
}

impl Default for JavaOptions {
    fn default() -> Self {
        Self {
            package_name: "generated.models".to_string(),
            backend: JavaBackend::Standard,
            use_records: true,
            emit_builder: false,
            emit_direct_codec: false,
            validate_facets: true,
            bean_validation: false,
            emit_root_aliases: true,
            custom_header: None,
        }
    }
}

/// Language context adapter for Java 22+.
pub struct JavaLanguageContext;

impl LanguageContext for JavaLanguageContext {
    fn target_language(&self) -> &'static str {
        "java"
    }

    fn map_primitive(&self, prim: PrimitiveType) -> &'static str {
        match prim {
            PrimitiveType::Boolean => "boolean",
            PrimitiveType::Float => "float",
            PrimitiveType::Double => "double",
            PrimitiveType::Decimal => "java.math.BigDecimal",
            PrimitiveType::Byte => "byte",
            PrimitiveType::Short => "short",
            PrimitiveType::Int => "int",
            PrimitiveType::Integer
            | PrimitiveType::Long
            | PrimitiveType::PositiveInteger
            | PrimitiveType::NegativeInteger
            | PrimitiveType::NonPositiveInteger
            | PrimitiveType::NonNegativeInteger
            | PrimitiveType::UnsignedLong => "long",
            PrimitiveType::UnsignedByte
            | PrimitiveType::UnsignedShort
            | PrimitiveType::UnsignedInt => "int",
            PrimitiveType::String
            | PrimitiveType::NormalizedString
            | PrimitiveType::Token
            | PrimitiveType::Name
            | PrimitiveType::NCName
            | PrimitiveType::QName
            | PrimitiveType::Language
            | PrimitiveType::NMTOKEN
            | PrimitiveType::NMTOKENS
            | PrimitiveType::AnyUri
            | PrimitiveType::Id
            | PrimitiveType::IdRef
            | PrimitiveType::IdRefs
            | PrimitiveType::Entity
            | PrimitiveType::Entities => "String",
            PrimitiveType::Date => "java.time.LocalDate",
            PrimitiveType::Time => "java.time.LocalTime",
            PrimitiveType::DateTime => "java.time.Instant",
            PrimitiveType::Duration => "java.time.Duration",
            PrimitiveType::GYear
            | PrimitiveType::GYearMonth
            | PrimitiveType::GMonth
            | PrimitiveType::GMonthDay
            | PrimitiveType::GDay => "String",
            PrimitiveType::Base64Binary | PrimitiveType::HexBinary => "byte[]",
            PrimitiveType::AnyType | PrimitiveType::AnySimpleType => "Object",
        }
    }

    fn map_type_ref(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(prim) => self.map_primitive(*prim).to_string(),
            TypeRef::Named(qname) => type_ident(qname),
            TypeRef::Boxed(inner) | TypeRef::List(inner) => {
                let inner_str = self.boxed_type(inner);
                if matches!(type_ref, TypeRef::List(_)) {
                    format!("java.util.List<{}>", inner_str)
                } else {
                    inner_str
                }
            }
        }
    }
}

impl JavaLanguageContext {
    /// Return boxed reference type corresponding to a given TypeRef (e.g. `Integer` for `int`).
    pub fn boxed_type(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(prim) => match prim {
                PrimitiveType::Boolean => "Boolean".to_string(),
                PrimitiveType::Float => "Float".to_string(),
                PrimitiveType::Double => "Double".to_string(),
                PrimitiveType::Decimal => "java.math.BigDecimal".to_string(),
                PrimitiveType::Byte => "Byte".to_string(),
                PrimitiveType::Short => "Short".to_string(),
                PrimitiveType::Int
                | PrimitiveType::UnsignedByte
                | PrimitiveType::UnsignedShort
                | PrimitiveType::UnsignedInt => "Integer".to_string(),
                PrimitiveType::Integer
                | PrimitiveType::Long
                | PrimitiveType::PositiveInteger
                | PrimitiveType::NegativeInteger
                | PrimitiveType::NonPositiveInteger
                | PrimitiveType::NonNegativeInteger
                | PrimitiveType::UnsignedLong => "Long".to_string(),
                PrimitiveType::Base64Binary | PrimitiveType::HexBinary => "byte[]".to_string(),
                _ => self.map_primitive(*prim).to_string(),
            },
            TypeRef::Named(qname) => type_ident(qname),
            TypeRef::Boxed(inner) => self.boxed_type(inner),
            TypeRef::List(inner) => format!("java.util.List<{}>", self.boxed_type(inner)),
        }
    }
}

/// Convert an XML field/attribute name to a valid Java lowerCamelCase identifier.
pub fn to_java_field_identifier(name: &str) -> String {
    let raw = AsLowerCamelCase(name).to_string();
    let sanitized = if raw.is_empty() {
        "field".to_string()
    } else if raw.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("_{}", raw)
    } else {
        raw
    };
    sanitize_keyword(&sanitized, "java")
}

/// Convert an XML type name to a PascalCase Java class/record/interface identifier.
pub fn to_java_type_name(name: &str) -> String {
    let raw = AsPascalCase(name).to_string();
    let sanitized = if raw.is_empty() {
        "Type".to_string()
    } else if raw.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("Type{}", raw)
    } else {
        raw
    };
    sanitize_keyword(&sanitized, "java")
}

/// Emitted Java identifier for a named type, disambiguated across
/// namespaces for the IR currently being generated.
pub(super) fn type_ident(q: &QName) -> String {
    lookup_type_name(q, || to_java_type_name(&q.local))
}

/// Convert an XML enumeration variant name to SCREAMING_SNAKE_CASE for Java enum constants.
pub fn to_java_enum_constant(name: &str) -> String {
    let normalized = normalize_symbol_name(name);
    let raw = AsShoutySnakeCase(&normalized).to_string();
    let sanitized = if raw.is_empty() {
        "EMPTY".to_string()
    } else if raw.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("VALUE_{}", raw)
    } else {
        raw
    };
    sanitize_keyword(&sanitized, "java")
}

/// Code generator producing Java 22+ records or mutable classes, choices, and enums.
pub struct JavaCodegen {
    pub options: JavaOptions,
    pub context: JavaLanguageContext,
}

impl JavaCodegen {
    pub fn new(options: JavaOptions) -> Self {
        Self {
            options,
            context: JavaLanguageContext,
        }
    }

    /// Generate individual compilation units (`{TypeName}.java`), one for each schema type.
    pub fn generate_files(&self, ir: &SchemaIR) -> Vec<(String, String)> {
        set_type_name_map(build_type_name_map(ir, to_java_type_name));
        let mut files = Vec::new();

        for type_def in ir.types.values() {
            let (type_name, content) = self.generate_single_type(type_def, false, ir);
            let filename = format!("{}.java", type_name);
            files.push((filename, content));
            if self.options.emit_direct_codec {
                files.push((
                    format!("{}Codec.java", type_name),
                    self.generate_codec(type_def, ir),
                ));
            }
        }

        files
    }

    /// Generate a single outer class (`{outer_class_name}.java`) enclosing all types as static members.
    pub fn generate_module(&self, ir: &SchemaIR, outer_class_name: &str) -> String {
        set_type_name_map(build_type_name_map(ir, to_java_type_name));
        let mut out = String::new();

        self.emit_file_header(&mut out);

        let _ = writeln!(out, "public final class {} {{", outer_class_name);
        let _ = writeln!(out, "    private {}() {{}}\n", outer_class_name);

        for type_def in ir.types.values() {
            let (_, body) = self.generate_type_body(type_def, "    ", ir);
            out.push_str(&body);
            out.push('\n');
        }

        if self.options.emit_root_aliases && !ir.elements.is_empty() {
            out.push_str("    // Root element aliases\n");
            let mut declared_names = HashSet::new();
            for el in ir.elements.values() {
                let el_name = to_java_type_name(&el.qname.local);
                let target_type = self.context.map_type_ref(&el.type_ref);

                if !declared_names.contains(&el_name) && el_name != target_type {
                    if !self.options.use_records {
                        self.emit_value_class(&mut out, &el_name, &el.type_ref, None, None, "    ");
                        declared_names.insert(el_name);
                        continue;
                    }
                    let _ = writeln!(
                        out,
                        "    public record {}({} value) {{}}",
                        el_name, target_type
                    );
                    declared_names.insert(el_name);
                }
            }
            out.push('\n');
        }

        if self.options.emit_direct_codec {
            for def in ir.types.values() {
                let codec = self.generate_codec(def, ir);
                let body = &codec[codec.find("public final class ").unwrap()..];
                out.push_str(&body.replacen(
                    "public final class ",
                    "public static final class ",
                    1,
                ));
            }
        }
        out.push_str("}\n");
        out
    }

    fn emit_file_header(&self, out: &mut String) {
        if let Some(ref header) = self.options.custom_header {
            let trimmed = header.trim();
            if !trimmed.is_empty() {
                out.push_str(trimmed);
                out.push_str("\n\n");
            }
        }
        out.push_str("// @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)\n");
        if !self.options.use_records {
            out.push_str("// Target: Java 22+ (Mutable JavaBeans)\n\n");
        } else if self.options.backend.is_jackson() {
            out.push_str("// Target: Java 22+ (Records, Jackson XML/JSON Annotations)\n\n");
        } else {
            out.push_str("// Target: Java 22+ (Records, Sealed Interfaces, Pattern Matching)\n\n");
        }

        if !self.options.package_name.is_empty() {
            let _ = writeln!(out, "package {};\n", self.options.package_name);
        }

        if self.options.emit_direct_codec {
            out.push_str("import javax.xml.stream.*;\nimport java.io.*;\n");
        }
        out.push_str("import java.util.*;\n");
        out.push_str("import java.time.*;\n");
        out.push_str("import java.math.*;\n");
        out.push_str("import java.util.regex.Pattern;\n");
        if self.options.bean_validation {
            out.push_str("import jakarta.validation.constraints.*;\n");
        }

        if self.options.backend.is_jackson() {
            out.push('\n');
            out.push_str("import com.fasterxml.jackson.annotation.*;\n");
            out.push_str("import com.fasterxml.jackson.dataformat.xml.annotation.*;\n");
        }

        out.push('\n');
    }

    fn generate_single_type(
        &self,
        type_def: &TypeDef,
        is_nested: bool,
        ir: &SchemaIR,
    ) -> (String, String) {
        let mut out = String::new();
        if !is_nested {
            self.emit_file_header(&mut out);
        }

        let (type_name, body) = self.generate_type_body(type_def, "", ir);
        out.push_str(&body);
        (type_name, out)
    }

    fn generate_type_body(
        &self,
        type_def: &TypeDef,
        indent: &str,
        ir: &SchemaIR,
    ) -> (String, String) {
        let mut out = String::new();
        let type_name = match type_def {
            TypeDef::Struct(s) => {
                let name = type_ident(&s.qname);
                self.emit_struct(&mut out, s, &name, indent, ir);
                name
            }
            TypeDef::Enum(e) => {
                let name = type_ident(&e.qname);
                self.emit_enum(&mut out, e, &name, indent);
                name
            }
            TypeDef::Union(u) => {
                let name = type_ident(&u.qname);
                self.emit_union(&mut out, u, &name, indent);
                name
            }
            TypeDef::Simple(s) => {
                let name = type_ident(&s.qname);
                {
                    let mut simple = s.clone();
                    if !simple.facets.patterns.is_empty() {
                        simple.base_type = super::primitive_base(&simple.base_type, ir).clone();
                    }
                    self.emit_simple(&mut out, &simple, &name, indent);
                }
                name
            }
        };

        (type_name, out)
    }

    fn emit_struct(
        &self,
        out: &mut String,
        s: &StructDef,
        java_name: &str,
        indent: &str,
        ir: &SchemaIR,
    ) {
        if !self.options.use_records {
            self.emit_pojo(out, s, java_name, indent, ir);
            return;
        }
        if let Some(ref doc) = s.documentation {
            self.emit_docstring(out, doc, indent);
        }

        let jackson = self.options.backend.is_jackson();

        // Jackson class-level annotations
        if jackson {
            let _ = writeln!(out, "{}@JsonIgnoreProperties(ignoreUnknown = true)", indent);
            let _ = writeln!(out, "{}@JsonInclude(JsonInclude.Include.NON_EMPTY)", indent);
            let ns_attr = s
                .qname
                .namespace
                .as_deref()
                .map(|ns| format!(", namespace = {:?}", ns))
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "{}@JacksonXmlRootElement(localName = {:?}{})",
                indent, s.qname.local, ns_attr
            );
        }

        let mut components = Vec::new();
        let mut validation_checks = Vec::new();

        for (field, field_id) in self.model_fields(s, ir) {
            let is_list = field.cardinality.is_list() || field.type_ref.is_list();
            let is_optional = field.cardinality.is_optional() || field.nillable;

            let field_type = self.field_type(field);

            let bean_annotations = self.bean_annotations(field).join(" ");
            let bean_prefix = if bean_annotations.is_empty() {
                String::new()
            } else {
                format!("{bean_annotations} ")
            };
            if jackson {
                let mut annotations = Vec::new();
                // @JsonProperty
                annotations.push(format!("@JsonProperty({:?})", field.xml_name));

                // @JacksonXmlProperty
                let is_attr = field.kind == crate::ir::FieldKind::Attribute;
                let ns_part = field
                    .namespace
                    .as_deref()
                    .map(|ns| format!(", namespace = {:?}", ns))
                    .unwrap_or_default();
                annotations.push(format!(
                    "@JacksonXmlProperty(localName = {:?}, isAttribute = {}{})",
                    field.xml_name, is_attr, ns_part
                ));

                // List fields: @JacksonXmlElementWrapper(useWrapping = false)
                if is_list {
                    annotations.push("@JacksonXmlElementWrapper(useWrapping = false)".to_string());
                }

                // Optional/list fields: @JsonInclude(NON_EMPTY)
                if is_optional || is_list {
                    annotations.push("@JsonInclude(JsonInclude.Include.NON_EMPTY)".to_string());
                }

                let annotation_str = annotations.join(" ");
                components.push(format!(
                    "{}{} {} {}",
                    bean_prefix, annotation_str, field_type, field_id
                ));
            } else {
                components.push(format!("{}{} {}", bean_prefix, field_type, field_id));
            }

            if self.options.validate_facets {
                if let Some(ref facets) = field.facets {
                    let checks = self.build_facet_checks(
                        &field_id,
                        &field.type_ref,
                        facets,
                        is_optional,
                        is_list,
                    );
                    validation_checks.extend(checks);
                }
            }
        }

        let modifier = if indent.is_empty() {
            "public "
        } else {
            "public static "
        };

        if components.is_empty() && !self.options.emit_builder {
            let _ = writeln!(out, "{}{}record {}() {{}}", indent, modifier, java_name);
            return;
        }

        let _ = writeln!(out, "{}{}record {}(", indent, modifier, java_name);
        for (i, comp) in components.iter().enumerate() {
            let comma = if i + 1 < components.len() { "," } else { "" };
            let _ = writeln!(out, "{}    {}{}", indent, comp, comma);
        }
        let _ = writeln!(out, "{}) {{", indent);

        if !validation_checks.is_empty() {
            let _ = writeln!(out, "{}    public {} {{", indent, java_name);
            for check in validation_checks {
                let _ = writeln!(out, "{}        {}", indent, check);
            }
            let _ = writeln!(out, "{}    }}", indent);
        }

        if self.options.emit_builder {
            self.emit_builder(out, s, java_name, indent, ir);
        }
        let _ = writeln!(out, "{}}}", indent);
    }

    fn emit_enum(&self, out: &mut String, e: &EnumDef, java_name: &str, indent: &str) {
        if let Some(ref doc) = e.documentation {
            self.emit_docstring(out, doc, indent);
        }

        let jackson = self.options.backend.is_jackson();

        let modifier = if indent.is_empty() {
            "public "
        } else {
            "public static "
        };
        let _ = writeln!(out, "{}{}enum {} {{", indent, modifier, java_name);

        let mut seen_constants = HashSet::new();
        for (i, v) in e.variants.iter().enumerate() {
            let mut const_name = to_java_enum_constant(&v.name);
            let mut counter = 1;
            while seen_constants.contains(&const_name) {
                const_name = format!("{}_{}", to_java_enum_constant(&v.name), counter);
                counter += 1;
            }
            seen_constants.insert(const_name.clone());

            let semi_or_comma = if i + 1 == e.variants.len() { ";" } else { "," };
            let _ = writeln!(
                out,
                "{}    {}({:?}){}",
                indent, const_name, v.value, semi_or_comma
            );
        }

        out.push('\n');
        let _ = writeln!(out, "{}    private final String value;\n", indent);
        let _ = writeln!(out, "{}    {}(String value) {{", indent, java_name);
        let _ = writeln!(out, "{}        this.value = value;", indent);
        let _ = writeln!(out, "{}    }}\n", indent);

        // @JsonValue on getValue()
        if jackson {
            let _ = writeln!(out, "{}    @JsonValue", indent);
        }
        let _ = writeln!(out, "{}    public String getValue() {{", indent);
        let _ = writeln!(out, "{}        return this.value;", indent);
        let _ = writeln!(out, "{}    }}\n", indent);

        // @JsonCreator on fromValue()
        if jackson {
            let _ = writeln!(out, "{}    @JsonCreator", indent);
        }
        let _ = writeln!(
            out,
            "{}    public static {} fromValue(String value) {{",
            indent, java_name
        );
        let _ = writeln!(out, "{}        for ({} v : values()) {{", indent, java_name);
        let _ = writeln!(out, "{}            if (v.value.equals(value)) {{", indent);
        let _ = writeln!(out, "{}                return v;", indent);
        let _ = writeln!(out, "{}            }}", indent);
        let _ = writeln!(out, "{}        }}", indent);
        let _ = writeln!(
            out,
            "{}        throw new IllegalArgumentException(\"Unknown {} value: \" + value);",
            indent, java_name
        );
        let _ = writeln!(out, "{}    }}", indent);

        let _ = writeln!(out, "{}}}", indent);
    }

    fn emit_union(&self, out: &mut String, u: &UnionDef, java_name: &str, indent: &str) {
        if let Some(ref doc) = u.documentation {
            self.emit_docstring(out, doc, indent);
        }

        let jackson = self.options.backend.is_jackson();

        let modifier = if indent.is_empty() {
            "public "
        } else {
            "public static "
        };

        let variant_names: Vec<String> = u
            .branches
            .iter()
            .map(|b| to_java_type_name(&b.variant_name))
            .collect();

        let permits_clause = variant_names
            .iter()
            .map(|v| format!("{}.{}", java_name, v))
            .collect::<Vec<_>>()
            .join(", ");

        // Jackson polymorphic type annotations
        if jackson && !u.is_lexical() {
            let _ = writeln!(
                out,
                "{}@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = \"@type\")",
                indent
            );

            let subtypes: Vec<String> = u
                .branches
                .iter()
                .map(|b| {
                    let vn = to_java_type_name(&b.variant_name);
                    format!(
                        "@JsonSubTypes.Type(value = {}.{}.class, name = {:?})",
                        java_name, vn, b.xml_name
                    )
                })
                .collect();

            let _ = writeln!(out, "{}@JsonSubTypes({{{}}})", indent, subtypes.join(", "));
        }

        let _ = writeln!(
            out,
            "{}{}sealed interface {} permits {} {{",
            indent, modifier, java_name, permits_clause
        );

        for branch in &u.branches {
            let variant_name = to_java_type_name(&branch.variant_name);
            let branch_type = self.context.map_type_ref(&branch.type_ref);

            if let Some(ref doc) = branch.documentation {
                self.emit_docstring(out, doc, &format!("{}    ", indent));
            }

            if jackson && !u.is_lexical() {
                let _ = writeln!(out, "{}    @JsonTypeName({:?})", indent, branch.xml_name);
            }

            if !self.options.use_records {
                self.emit_value_class(
                    out,
                    &variant_name,
                    &branch.type_ref,
                    None,
                    Some(java_name),
                    &format!("{}    ", indent),
                );
                continue;
            }
            let _ = writeln!(
                out,
                "{}    record {}({} value) implements {} {{}}",
                indent, variant_name, branch_type, java_name
            );
        }

        let _ = writeln!(out, "{}}}", indent);
    }

    fn emit_simple(&self, out: &mut String, s: &SimpleTypeDef, java_name: &str, indent: &str) {
        if let Some(ref doc) = s.documentation {
            self.emit_docstring(out, doc, indent);
        }

        if !self.options.use_records {
            self.emit_value_class(out, java_name, &s.base_type, Some(&s.facets), None, indent);
            return;
        }
        let jackson = self.options.backend.is_jackson();
        let base_type = self.context.map_type_ref(&s.base_type);
        let modifier = if indent.is_empty() {
            "public "
        } else {
            "public static "
        };

        let checks = if self.options.validate_facets {
            self.build_facet_checks("value", &s.base_type, &s.facets, false, false)
        } else {
            Vec::new()
        };

        // Build the value component with optional Jackson annotations
        let mut value_field = crate::ir::FieldDef::new(
            "value",
            "value",
            crate::ir::FieldKind::Text,
            s.base_type.clone(),
        );
        value_field.facets = Some(s.facets.clone());
        let bean_prefix = self.bean_annotations(&value_field).join(" ");
        let bean_prefix = if bean_prefix.is_empty() {
            String::new()
        } else {
            format!("{bean_prefix} ")
        };
        let value_component = if jackson {
            format!(
                "{bean_prefix}@JsonValue @JacksonXmlText {} value",
                base_type
            )
        } else {
            format!("{bean_prefix}{} value", base_type)
        };

        if checks.is_empty() && !jackson {
            let _ = writeln!(
                out,
                "{}{}record {}({}) {{}}",
                indent, modifier, java_name, value_component
            );
            return;
        }

        let _ = writeln!(
            out,
            "{}{}record {}({}) {{",
            indent, modifier, java_name, value_component
        );

        // Jackson @JsonCreator factory method
        if jackson {
            let _ = writeln!(out, "{}    @JsonCreator", indent);
            let _ = writeln!(
                out,
                "{}    public static {} of({} value) {{",
                indent, java_name, base_type
            );
            let _ = writeln!(out, "{}        return new {}(value);", indent, java_name);
            let _ = writeln!(out, "{}    }}", indent);
        }

        if !checks.is_empty() {
            let _ = writeln!(out, "{}    public {} {{", indent, java_name);
            for check in checks {
                let _ = writeln!(out, "{}        {}", indent, check);
            }
            let _ = writeln!(out, "{}    }}", indent);
        }

        let _ = writeln!(out, "{}}}", indent);
    }

    fn bean_annotations(&self, field: &crate::ir::FieldDef) -> Vec<String> {
        if !self.options.bean_validation {
            return Vec::new();
        }
        let mut annotations = Vec::new();
        let is_list = field.cardinality.is_list() || field.type_ref.is_list();
        let is_optional = field.cardinality.is_optional() || field.nillable;
        if !is_optional
            && !matches!(
                self.field_type(field).as_str(),
                "boolean" | "byte" | "short" | "int" | "long" | "float" | "double"
            )
        {
            // Java primitive fields cannot be null; reference fields can.
            annotations.push("@NotNull".to_string());
        }
        if matches!(&field.type_ref, TypeRef::Named(_) | TypeRef::List(_)) {
            annotations.push("@jakarta.validation.Valid".to_string());
        }
        let facets = field.facets.as_ref();
        let Some(facets) = facets else {
            return annotations;
        };
        let primitive = &field.type_ref;
        let is_string = matches!(
            primitive,
            TypeRef::Primitive(
                PrimitiveType::String
                    | PrimitiveType::NormalizedString
                    | PrimitiveType::Token
                    | PrimitiveType::Name
                    | PrimitiveType::NCName
                    | PrimitiveType::QName
                    | PrimitiveType::Language
                    | PrimitiveType::NMTOKEN
                    | PrimitiveType::NMTOKENS
                    | PrimitiveType::AnyUri
                    | PrimitiveType::Id
                    | PrimitiveType::IdRef
                    | PrimitiveType::IdRefs
                    | PrimitiveType::Entity
                    | PrimitiveType::Entities
                    | PrimitiveType::GDay
                    | PrimitiveType::GMonth
                    | PrimitiveType::GYear
                    | PrimitiveType::GYearMonth
                    | PrimitiveType::GMonthDay
            )
        );
        let is_numeric = matches!(
            primitive,
            TypeRef::Primitive(
                PrimitiveType::Byte
                    | PrimitiveType::Short
                    | PrimitiveType::Int
                    | PrimitiveType::Integer
                    | PrimitiveType::Long
                    | PrimitiveType::Decimal
                    | PrimitiveType::UnsignedByte
                    | PrimitiveType::UnsignedShort
                    | PrimitiveType::UnsignedInt
                    | PrimitiveType::UnsignedLong
                    | PrimitiveType::PositiveInteger
                    | PrimitiveType::NegativeInteger
                    | PrimitiveType::NonPositiveInteger
                    | PrimitiveType::NonNegativeInteger
                    | PrimitiveType::Float
                    | PrimitiveType::Double
            )
        );
        if is_list || (is_string && !is_optional) {
            if let Some(length) = facets.length {
                annotations.push(format!("@Size(min = {length}, max = {length})"));
            } else if facets.min_length.is_some() || facets.max_length.is_some() {
                annotations.push(format!(
                    "@Size(min = {}, max = {})",
                    facets.min_length.unwrap_or(0),
                    facets.max_length.unwrap_or(i32::MAX as usize)
                ));
            }
        }
        if is_string && !is_optional {
            for pattern in &facets.patterns {
                annotations.push(format!(
                    "@jakarta.validation.constraints.Pattern(regexp = {pattern:?})"
                ));
            }
        }
        if is_numeric
            && !is_list
            && !is_optional
            && !matches!(
                primitive,
                TypeRef::Primitive(PrimitiveType::Float | PrimitiveType::Double)
            )
        {
            for (value, minimum, inclusive) in [
                (facets.min_inclusive.as_ref(), true, true),
                (facets.max_inclusive.as_ref(), false, true),
                (facets.min_exclusive.as_ref(), true, false),
                (facets.max_exclusive.as_ref(), false, false),
            ] {
                if let Some(value) = value {
                    if inclusive && !matches!(primitive, TypeRef::Primitive(PrimitiveType::Decimal))
                    {
                        if let Ok(integer) = value.parse::<i64>() {
                            annotations.push(format!(
                                "@{}({integer})",
                                if minimum { "Min" } else { "Max" }
                            ));
                            continue;
                        }
                    }
                    let annotation = if minimum { "DecimalMin" } else { "DecimalMax" };
                    annotations.push(format!(
                        "@{annotation}(value = {value:?}, inclusive = {inclusive})"
                    ));
                }
            }
        }
        annotations
    }

    fn build_facet_checks(
        &self,
        var_name: &str,
        type_ref: &TypeRef,
        facets: &RestrictionFacets,
        is_optional: bool,
        is_list: bool,
    ) -> Vec<String> {
        let mut checks = Vec::new();
        if is_list {
            return checks;
        }

        let is_string = match type_ref {
            TypeRef::Primitive(prim) => matches!(
                prim,
                PrimitiveType::String
                    | PrimitiveType::NormalizedString
                    | PrimitiveType::Token
                    | PrimitiveType::Name
                    | PrimitiveType::NCName
                    | PrimitiveType::QName
                    | PrimitiveType::Language
                    | PrimitiveType::NMTOKEN
                    | PrimitiveType::NMTOKENS
                    | PrimitiveType::AnyUri
                    | PrimitiveType::Id
                    | PrimitiveType::IdRef
                    | PrimitiveType::IdRefs
                    | PrimitiveType::Entity
                    | PrimitiveType::Entities
                    | PrimitiveType::GDay
                    | PrimitiveType::GMonth
                    | PrimitiveType::GYear
                    | PrimitiveType::GYearMonth
                    | PrimitiveType::GMonthDay
            ),
            _ => false,
        };

        let is_num = match type_ref {
            TypeRef::Primitive(prim) => matches!(
                prim,
                PrimitiveType::Byte
                    | PrimitiveType::Short
                    | PrimitiveType::Int
                    | PrimitiveType::Integer
                    | PrimitiveType::Long
                    | PrimitiveType::UnsignedByte
                    | PrimitiveType::UnsignedShort
                    | PrimitiveType::UnsignedInt
                    | PrimitiveType::UnsignedLong
                    | PrimitiveType::PositiveInteger
                    | PrimitiveType::NegativeInteger
                    | PrimitiveType::NonPositiveInteger
                    | PrimitiveType::NonNegativeInteger
                    | PrimitiveType::Float
                    | PrimitiveType::Double
            ),
            _ => false,
        };

        if is_string {
            if is_optional {
                if let Some(min_len) = facets.min_length {
                    checks.push(format!(
                        "{}.ifPresent(v -> {{ if (v.length() < {}) throw new IllegalArgumentException(\"{} minLength is {}\"); }});",
                        var_name, min_len, var_name, min_len
                    ));
                }
                if let Some(max_len) = facets.max_length {
                    checks.push(format!(
                        "{}.ifPresent(v -> {{ if (v.length() > {}) throw new IllegalArgumentException(\"{} maxLength is {}\"); }});",
                        var_name, max_len, var_name, max_len
                    ));
                }
                if let Some(len) = facets.length {
                    checks.push(format!(
                        "{}.ifPresent(v -> {{ if (v.length() != {}) throw new IllegalArgumentException(\"{} length must be {}\"); }});",
                        var_name, len, var_name, len
                    ));
                }
                for pat in &facets.patterns {
                    checks.push(format!(
                        "{}.ifPresent(v -> {{ if (!Pattern.compile({:?}).matcher(v).matches()) throw new IllegalArgumentException(\"{} does not match pattern: \" + {:?}); }});",
                        var_name, pat, var_name, pat
                    ));
                }
            } else {
                checks.push(format!(
                    "Objects.requireNonNull({}, \"{} must not be null\");",
                    var_name, var_name
                ));
                if let Some(min_len) = facets.min_length {
                    checks.push(format!(
                        "if ({}.length() < {}) throw new IllegalArgumentException(\"{} minLength is {}\");",
                        var_name, min_len, var_name, min_len
                    ));
                }
                if let Some(max_len) = facets.max_length {
                    checks.push(format!(
                        "if ({}.length() > {}) throw new IllegalArgumentException(\"{} maxLength is {}\");",
                        var_name, max_len, var_name, max_len
                    ));
                }
                if let Some(len) = facets.length {
                    checks.push(format!(
                        "if ({}.length() != {}) throw new IllegalArgumentException(\"{} length must be {}\");",
                        var_name, len, var_name, len
                    ));
                }
                for pat in &facets.patterns {
                    checks.push(format!(
                        "if (!Pattern.compile({:?}).matcher({}).matches()) throw new IllegalArgumentException(\"{} does not match pattern: \" + {:?});",
                        pat, var_name, var_name, pat
                    ));
                }
            }
        }

        if is_num {
            if is_optional {
                if let Some(ref min_inc) = facets.min_inclusive {
                    checks.push(format!(
                        "{}.ifPresent(v -> {{ if (v < {}) throw new IllegalArgumentException(\"{} minInclusive is {}\"); }});",
                        var_name, min_inc, var_name, min_inc
                    ));
                }
                if let Some(ref max_inc) = facets.max_inclusive {
                    checks.push(format!(
                        "{}.ifPresent(v -> {{ if (v > {}) throw new IllegalArgumentException(\"{} maxInclusive is {}\"); }});",
                        var_name, max_inc, var_name, max_inc
                    ));
                }
            } else {
                if let Some(ref min_inc) = facets.min_inclusive {
                    checks.push(format!(
                        "if ({} < {}) throw new IllegalArgumentException(\"{} minInclusive is {}\");",
                        var_name, min_inc, var_name, min_inc
                    ));
                }
                if let Some(ref max_inc) = facets.max_inclusive {
                    checks.push(format!(
                        "if ({} > {}) throw new IllegalArgumentException(\"{} maxInclusive is {}\");",
                        var_name, max_inc, var_name, max_inc
                    ));
                }
            }
        }

        checks
    }

    fn unique_field_name(&self, name: &str, seen: &mut HashSet<String>) -> String {
        let mut field_id = to_java_field_identifier(name);
        let mut counter = 1;
        while seen.contains(&field_id) {
            field_id = format!("{}_{}", to_java_field_identifier(name), counter);
            counter += 1;
        }
        seen.insert(field_id.clone());
        field_id
    }

    fn emit_docstring(&self, out: &mut String, doc: &str, indent: &str) {
        let clean = doc.trim();
        if clean.contains('\n') {
            let _ = writeln!(out, "{}/**", indent);
            for line in clean.lines() {
                let _ = writeln!(out, "{} * {}", indent, line.trim());
            }
            let _ = writeln!(out, "{} */", indent);
        } else {
            let _ = writeln!(out, "{}/** {} */", indent, clean);
        }
    }
}
