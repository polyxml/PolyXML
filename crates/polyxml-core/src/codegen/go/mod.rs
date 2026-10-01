use std::collections::HashSet;
use std::fmt::Write as FmtWrite;

use heck::{AsLowerCamelCase, AsPascalCase, AsSnakeCase};
use serde::{Deserialize, Serialize};

use crate::codegen::{
    build_type_name_map, lookup_type_name, normalize_symbol_name, sanitize_keyword,
    set_type_name_map, LanguageContext,
};
use crate::ir::{
    EnumDef, FieldDef, FieldKind, PrimitiveType, QName, RestrictionFacets, SchemaIR, SimpleTypeDef,
    StructDef, TypeDef, TypeRef, UnionDef,
};

/// Target serialization backend for Go codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GoBackend {
    /// Standard library `encoding/json` and `encoding/xml` (default)
    #[default]
    Standard,
    /// Mailru/EasyJSON static reflectionless serialization generator (`//easyjson:json`)
    EasyJson,
    /// ByteDance Sonic high-performance JIT serialization (`sonic:"..."` struct tags)
    Sonic,
}

impl GoBackend {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "standard" | "std" | "default" => Some(Self::Standard),
            "easyjson" | "easy-json" | "easy_json" => Some(Self::EasyJson),
            "sonic" | "bytedance" => Some(Self::Sonic),
            _ => None,
        }
    }
}

/// Options configuring Go 1.22+ code generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoOptions {
    /// Target serialization backend (default: GoBackend::Standard)
    pub backend: GoBackend,
    /// Package name for generated Go code (default: "models")
    pub package_name: String,
    /// Emit encoding/xml tags (default: true)
    pub emit_xml_tags: bool,
    /// Emit encoding/json tags (default: true)
    pub emit_json_tags: bool,
    /// Emit custom UnmarshalXML/MarshalXML for xs:choice mutual exclusivity validation (default: true)
    pub validate_choice_exclusivity: bool,
    /// Emit Validate() error method for restriction facets (default: true)
    pub validate_facets: bool,
    /// Emit top-level type aliases for root elements (default: true)
    pub emit_root_aliases: bool,
    /// Custom header text to prepend to generated files (default: None)
    pub custom_header: Option<String>,
}

impl Default for GoOptions {
    fn default() -> Self {
        Self {
            backend: GoBackend::Standard,
            package_name: "models".to_string(),
            emit_xml_tags: true,
            emit_json_tags: true,
            validate_choice_exclusivity: true,
            validate_facets: true,
            emit_root_aliases: true,
            custom_header: None,
        }
    }
}

/// Language context adapter for Go 1.22+.
pub struct GoLanguageContext;

impl LanguageContext for GoLanguageContext {
    fn target_language(&self) -> &'static str {
        "go"
    }

    fn map_primitive(&self, prim: PrimitiveType) -> &'static str {
        match prim {
            PrimitiveType::Boolean => "bool",
            PrimitiveType::Float => "float32",
            PrimitiveType::Double | PrimitiveType::Decimal => "float64",
            PrimitiveType::Byte => "int8",
            PrimitiveType::Short => "int16",
            PrimitiveType::Int => "int32",
            PrimitiveType::Integer
            | PrimitiveType::Long
            | PrimitiveType::PositiveInteger
            | PrimitiveType::NegativeInteger
            | PrimitiveType::NonPositiveInteger
            | PrimitiveType::NonNegativeInteger => "int64",
            PrimitiveType::UnsignedByte => "uint8",
            PrimitiveType::UnsignedShort => "uint16",
            PrimitiveType::UnsignedInt => "uint32",
            PrimitiveType::UnsignedLong => "uint64",
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
            | PrimitiveType::GYear
            | PrimitiveType::GYearMonth
            | PrimitiveType::GMonth
            | PrimitiveType::GMonthDay
            | PrimitiveType::GDay
            | PrimitiveType::Duration => "string",
            PrimitiveType::Date => "PolyxmlDate",
            PrimitiveType::Time => "PolyxmlTime",
            PrimitiveType::DateTime => "PolyxmlDateTime",
            PrimitiveType::Base64Binary | PrimitiveType::HexBinary => "[]byte",
            PrimitiveType::AnyType | PrimitiveType::AnySimpleType => "any",
        }
    }

    fn map_type_ref(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(prim) => self.map_primitive(*prim).to_string(),
            TypeRef::Named(qname) => type_ident(qname),
            TypeRef::Boxed(inner) => format!("*{}", self.map_type_ref(inner)),
            TypeRef::List(inner) => format!("[]{}", self.map_type_ref(inner)),
        }
    }
}

/// Normalizes common Go initialisms (e.g. `Id` -> `ID`, `Url` -> `URL`).
fn normalize_go_initialisms(s: &str) -> String {
    if s == "Id" {
        return "ID".to_string();
    }
    if s == "Url" {
        return "URL".to_string();
    }
    if s == "Xml" {
        return "XML".to_string();
    }
    if s == "Uri" {
        return "URI".to_string();
    }
    if s == "Uuid" {
        return "UUID".to_string();
    }

    let mut res = s.to_string();
    if res.ends_with("Id") {
        let len = res.len();
        res.replace_range(len - 2.., "ID");
    } else if res.ends_with("Url") {
        let len = res.len();
        res.replace_range(len - 3.., "URL");
    } else if res.ends_with("Xml") {
        let len = res.len();
        res.replace_range(len - 3.., "XML");
    } else if res.ends_with("Uri") {
        let len = res.len();
        res.replace_range(len - 3.., "URI");
    }

    if res.starts_with("Xml") {
        res.replace_range(..3, "XML");
    }
    res
}

/// Converts a raw identifier into an exported PascalCase Go type name.
pub fn to_go_type_name(raw: &str) -> String {
    let pascal = AsPascalCase(raw).to_string();
    let normalized = normalize_go_initialisms(&pascal);
    let safe = if normalized.is_empty() {
        "Type".to_string()
    } else if normalized.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Type{}", normalized)
    } else {
        normalized
    };
    sanitize_keyword(&safe, "go")
}

/// Converts a raw identifier into an exported PascalCase Go struct field name.
pub fn to_go_field_name(raw: &str) -> String {
    let pascal = AsPascalCase(raw).to_string();
    let normalized = normalize_go_initialisms(&pascal);
    let safe = if normalized.is_empty() {
        "Field".to_string()
    } else if normalized.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Field{}", normalized)
    } else {
        normalized
    };
    sanitize_keyword(&safe, "go")
}

/// Converts an enumeration variant into a typed Go constant identifier (`EnumNameVariant`).
pub fn to_go_constant_name(type_name: &str, raw: &str) -> String {
    let normalized_sym = normalize_symbol_name(raw);
    let pascal = AsPascalCase(&normalized_sym).to_string();
    let normalized = normalize_go_initialisms(&pascal);
    let safe_var = if normalized.is_empty() {
        "Value".to_string()
    } else if normalized.starts_with(|c: char| c.is_ascii_digit()) {
        format!("V{}", normalized)
    } else {
        normalized
    };
    format!("{}{}", type_name, safe_var)
}

/// Sanitizes a string into a valid Go package name (`crm`, `models`, etc.).
pub fn to_go_package_name(raw: &str) -> String {
    let s = AsSnakeCase(raw).to_string().replace('_', "");
    let safe = if s.is_empty() {
        "models".to_string()
    } else if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("pkg{}", s)
    } else {
        s
    };
    sanitize_keyword(&safe, "go")
}

/// Modern Go 1.22+ Code Generator.
pub struct GoCodegen {
    options: GoOptions,
    context: GoLanguageContext,
}

/// Emitted Go identifier for a named type, disambiguated across namespaces
/// for the IR currently being generated.
fn type_ident(q: &QName) -> String {
    lookup_type_name(q, || to_go_type_name(&q.local))
}

impl GoCodegen {
    fn emit_temporal_types(&self, out: &mut String) {
        for (name, layouts, default_layout) in [
            (
                "PolyxmlDate",
                &["2006-01-02Z07:00", "2006-01-02"][..],
                "2006-01-02Z07:00",
            ),
            (
                "PolyxmlTime",
                &["15:04:05Z07:00", "15:04:05"][..],
                "15:04:05.999999999Z07:00",
            ),
            (
                "PolyxmlDateTime",
                &["2006-01-02T15:04:05Z07:00", "2006-01-02T15:04:05"][..],
                "2006-01-02T15:04:05.999999999Z07:00",
            ),
        ] {
            writeln!(out, "// {name} retains the XSD lexical form, including an absent timezone.\ntype {name} struct {{ time.Time; lexical string }}").unwrap();
            writeln!(out, "func (v *{name}) UnmarshalText(text []byte) error {{\n    value := strings.TrimSpace(string(text))").unwrap();
            for layout in layouts {
                writeln!(out, "    if parsed, err := time.Parse({layout:?}, value); err == nil {{ *v = {name}{{Time: parsed, lexical: value}}; return nil }}").unwrap();
            }
            writeln!(out, "    return fmt.Errorf(\"invalid {name}: %q\", value)\n}}\nfunc (v {name}) MarshalText() ([]byte, error) {{\n    layout := {default_layout:?}").unwrap();
            for original_layout in layouts {
                writeln!(out, "    if parsed, err := time.Parse({original_layout:?}, v.lexical); err == nil {{\n        _, offset := v.Time.Zone(); _, originalOffset := parsed.Zone()\n        if v.Time.Equal(parsed) && offset == originalOffset {{ return []byte(v.lexical), nil }}").unwrap();
                if !original_layout.contains("Z07:00") {
                    writeln!(
                        out,
                        "        layout = {:?}",
                        default_layout.trim_end_matches("Z07:00")
                    )
                    .unwrap();
                }
                out.push_str("    }\n");
            }
            out.push_str("    return []byte(v.Time.Format(layout)), nil\n}\n\n");
        }
    }

    pub fn new(options: GoOptions) -> Self {
        Self {
            options,
            context: GoLanguageContext,
        }
    }

    /// Generate complete Go module content.
    pub fn generate_module(&self, ir: &SchemaIR) -> String {
        set_type_name_map(build_type_name_map(ir, to_go_type_name));
        let mut body = String::new();
        let mut has_time = false;
        let lexical_patterns = ir.emitted_types().any(|def| match def {
            TypeDef::Union(u) if u.is_lexical() => u.branches.iter().any(|branch| {
                matches!(&branch.type_ref, TypeRef::Named(q) if matches!(ir.types.get(q), Some(TypeDef::Simple(s)) if !s.facets.patterns.is_empty()))
            }),
            _ => false,
        });
        let has_patterns = lexical_patterns
            || (self.options.validate_facets
                && ir.emitted_types().any(|t| match t {
                    TypeDef::Simple(s) => !s.facets.patterns.is_empty(),
                    TypeDef::Struct(s) => s
                        .fields
                        .iter()
                        .any(|f| f.facets.as_ref().is_some_and(|f| !f.patterns.is_empty())),
                    _ => false,
                }));
        let mut has_fmt = has_patterns;
        let mut has_io = false;

        // Check types for time.Time
        for type_def in ir.emitted_types() {
            match type_def {
                TypeDef::Struct(s) => {
                    if ir.has_ordered_content(s) && self.options.emit_xml_tags {
                        has_fmt = true;
                        has_io = true;
                    }
                    for f in &s.fields {
                        if self.references_time(&f.type_ref) {
                            has_time = true;
                        }
                    }
                    if self.options.validate_facets
                        && s.fields.iter().any(|f| {
                            f.facets
                                .as_ref()
                                .map(|fac| !fac.is_empty())
                                .unwrap_or(false)
                        })
                    {
                        has_fmt = true;
                    }
                }
                TypeDef::Union(u) => {
                    if u.branches
                        .iter()
                        .any(|branch| self.references_time(&branch.type_ref))
                    {
                        has_time = true;
                    }
                    if self.options.validate_choice_exclusivity && !u.is_lexical() {
                        has_fmt = true;
                        has_io = true;
                    }
                    if u.is_lexical() {
                        has_fmt = true;
                    }
                }
                TypeDef::Simple(st) => {
                    if self.references_time(&st.base_type) {
                        has_time = true;
                    }
                }
                TypeDef::Enum(_) => {}
            }
        }

        // Generate types
        self.emit_types(&mut body, ir);
        if has_time {
            self.emit_temporal_types(&mut body);
        }
        self.emit_root_aliases(&mut body, ir);

        // Assemble final output with package and imports
        let mut out = String::new();
        if let Some(ref header) = self.options.custom_header {
            let trimmed = header.trim();
            if !trimmed.is_empty() {
                out.push_str(trimmed);
                out.push_str("\n\n");
            }
        }
        writeln!(
            out,
            "// Code generated by PolyXML Compiler (https://github.com/polyxml/PolyXML). DO NOT EDIT."
        )
        .unwrap();
        writeln!(
            out,
            "// @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)\n"
        )
        .unwrap();

        let pkg = to_go_package_name(&self.options.package_name);
        writeln!(out, "package {}\n", pkg).unwrap();

        let mut imports = Vec::new();
        if body.contains("xml.") {
            imports.push("\"encoding/xml\"");
        }
        if self.options.emit_xml_tags
            && ir
                .types
                .values()
                .any(|def| matches!(def, TypeDef::Struct(s) if ir.has_ordered_content(s)))
        {
            imports.push("\"bytes\"");
        }
        if has_fmt || body.contains("fmt.") {
            imports.push("\"fmt\"");
        }
        if has_io {
            imports.push("\"io\"");
        }
        if has_patterns {
            imports.push("\"regexp\"");
        }
        if ir
            .types
            .values()
            .any(|def| matches!(def, TypeDef::Union(u) if u.is_lexical()))
            && body.contains("strconv.")
        {
            imports.push("\"strconv\"");
        }
        if body.contains("strings.") {
            imports.push("\"strings\"");
        }
        if has_time {
            imports.push("\"time\"");
        }

        if !imports.is_empty() {
            writeln!(out, "import (").unwrap();
            for imp in imports {
                writeln!(out, "    {}", imp).unwrap();
            }
            writeln!(out, ")\n").unwrap();
        }

        out.push_str(&body);
        out
    }

    /// Generate bundle of files (source file named after base_name).
    pub fn generate_files(&self, ir: &SchemaIR, base_name: &str) -> Vec<(String, String)> {
        let filename = if base_name.is_empty() {
            "models.go".to_string()
        } else {
            format!("{}.go", AsSnakeCase(base_name))
        };

        vec![(filename, self.generate_module(ir))]
    }

    fn references_time(&self, type_ref: &TypeRef) -> bool {
        match type_ref {
            TypeRef::Primitive(
                PrimitiveType::Date | PrimitiveType::Time | PrimitiveType::DateTime,
            ) => true,
            TypeRef::List(inner) | TypeRef::Boxed(inner) => self.references_time(inner),
            _ => false,
        }
    }

    fn emit_types(&self, out: &mut String, ir: &SchemaIR) {
        // Emit simple types
        for type_def in ir.emitted_types() {
            if let TypeDef::Simple(simple) = type_def {
                self.emit_simple_type(out, simple, ir);
            }
        }

        // Emit enums
        for type_def in ir.emitted_types() {
            if let TypeDef::Enum(enum_def) = type_def {
                self.emit_enum(out, enum_def);
            }
        }

        // Emit choices (unions)
        for type_def in ir.emitted_types() {
            if let TypeDef::Union(u) = type_def {
                self.emit_union(out, u, ir);
            }
        }

        // Emit structs
        for type_def in ir.emitted_types() {
            if let TypeDef::Struct(s) = type_def {
                self.emit_struct(out, s, ir);
            }
        }

        let has_any_element = ir.emitted_types().any(|t| match t {
            TypeDef::Struct(s) => s.fields.iter().any(|f| f.kind == FieldKind::Any),
            _ => false,
        });
        if has_any_element {
            out.push_str("// AnyElement represents an unmapped wildcard XML element (xs:any).\ntype AnyElement struct {\n    XMLName xml.Name\n    Attrs   []xml.Attr `xml:\",any,attr\" json:\"attrs,omitempty\"`\n    Content string     `xml:\",innerxml\" json:\"content,omitempty\"`\n}\n\n");
        }
    }

    fn emit_simple_type(&self, out: &mut String, simple: &SimpleTypeDef, ir: &SchemaIR) {
        if let Some(ref doc) = simple.documentation {
            for line in doc.lines() {
                writeln!(out, "// {}", line).unwrap();
            }
        }

        let type_name = type_ident(&simple.qname);
        let base_type = self.context.map_type_ref(&simple.base_type);
        writeln!(out, "type {} {}\n", type_name, base_type).unwrap();
        if matches!(
            super::primitive_base(&simple.base_type, ir),
            TypeRef::Primitive(PrimitiveType::Date | PrimitiveType::Time | PrimitiveType::DateTime)
        ) {
            writeln!(out, "func (v *{type_name}) UnmarshalText(text []byte) error {{ return (*{base_type})(v).UnmarshalText(text) }}\nfunc (v {type_name}) MarshalText() ([]byte, error) {{ return {base_type}(v).MarshalText() }}\n").unwrap();
        }
        if self.options.validate_facets && !simple.facets.patterns.is_empty() {
            writeln!(out, "func (s {}) Validate() error {{", type_name).unwrap();
            for pattern in &simple.facets.patterns {
                writeln!(out, "    if matched, err := regexp.MatchString({:?}, fmt.Sprint(s)); err != nil || !matched {{ return fmt.Errorf(\"pattern constraint failed\") }}", format!("^(?:{pattern})$")).unwrap();
            }
            writeln!(out, "    return nil\n}}\n").unwrap();
            if self
                .context
                .map_type_ref(super::primitive_base(&simple.base_type, ir))
                == "string"
            {
                writeln!(out, "func (s *{}) UnmarshalText(text []byte) error {{\n    value := {}(text)\n    if err := value.Validate(); err != nil {{ return err }}\n    *s = value\n    return nil\n}}\n", type_name, type_name).unwrap();
                writeln!(out, "func (s {}) MarshalText() ([]byte, error) {{\n    if err := s.Validate(); err != nil {{ return nil, err }}\n    return []byte(s), nil\n}}\n", type_name).unwrap();
            }
        }
    }

    fn unique_field_name(&self, raw: &str, seen: &mut HashSet<String>) -> String {
        let base = to_go_field_name(raw);
        let mut name = base.clone();
        let mut index = 2;
        while seen.contains(&name) {
            name = format!("{}_{}", base, index);
            index += 1;
        }
        seen.insert(name.clone());
        name
    }

    fn unique_constant_name(
        &self,
        enum_name: &str,
        raw: &str,
        seen: &mut HashSet<String>,
    ) -> String {
        let base = to_go_constant_name(enum_name, raw);
        let mut name = base.clone();
        let mut index = 2;
        while seen.contains(&name) {
            name = format!("{}_{}", base, index);
            index += 1;
        }
        seen.insert(name.clone());
        name
    }

    fn emit_enum(&self, out: &mut String, enum_def: &EnumDef) {
        let enum_name = type_ident(&enum_def.qname);
        if let Some(ref doc) = enum_def.documentation {
            for line in doc.lines() {
                writeln!(out, "// {}", line).unwrap();
            }
        }

        writeln!(out, "type {} string\n", enum_name).unwrap();

        let mut seen = HashSet::new();
        let const_names: Vec<String> = enum_def
            .variants
            .iter()
            .map(|v| self.unique_constant_name(&enum_name, &v.name, &mut seen))
            .collect();

        writeln!(out, "const (").unwrap();
        for (variant, const_name) in enum_def.variants.iter().zip(&const_names) {
            if let Some(ref doc) = variant.documentation {
                writeln!(out, "    // {}", doc).unwrap();
            }
            writeln!(
                out,
                "    {} {} = \"{}\"",
                const_name, enum_name, variant.value
            )
            .unwrap();
        }
        writeln!(out, ")\n").unwrap();

        // IsValid() bool method
        writeln!(out, "func (e {}) IsValid() bool {{", enum_name).unwrap();
        writeln!(out, "    switch e {{").unwrap();
        writeln!(out, "    case {}:", const_names.join(", ")).unwrap();
        writeln!(out, "        return true").unwrap();
        writeln!(out, "    default:").unwrap();
        writeln!(out, "        return false").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}\n").unwrap();
    }

    fn emit_union(&self, out: &mut String, u: &UnionDef, ir: &SchemaIR) {
        if u.is_lexical() {
            self.emit_lexical_union(out, u, ir);
            return;
        }
        let choice_name = type_ident(&u.qname);
        if let Some(ref doc) = u.documentation {
            for line in doc.lines() {
                writeln!(out, "// {}", line).unwrap();
            }
        }

        if self.options.backend == GoBackend::EasyJson {
            writeln!(out, "//easyjson:json").unwrap();
        }

        // Choice container struct
        writeln!(out, "type {} struct {{", choice_name).unwrap();
        let mut seen = HashSet::new();
        let branch_field_names: Vec<String> = u
            .branches
            .iter()
            .map(|b| self.unique_field_name(&b.variant_name, &mut seen))
            .collect();

        for (branch, field_name) in u.branches.iter().zip(&branch_field_names) {
            if let Some(ref doc) = branch.documentation {
                writeln!(out, "    // {}", doc).unwrap();
            }
            let mapped_type = self.context.map_type_ref(&branch.type_ref);
            let mut tag_parts = Vec::new();
            if self.options.emit_xml_tags {
                let branch_xml = match &branch.namespace {
                    Some(ns) if !ns.is_empty() => format!("{} {}", ns, branch.xml_name),
                    _ => branch.xml_name.clone(),
                };
                tag_parts.push(format!("xml:\"{},omitempty\"", branch_xml));
            }
            if self.options.emit_json_tags {
                tag_parts.push(format!("json:\"{},omitempty\"", branch.xml_name));
            }
            if self.options.backend == GoBackend::Sonic {
                tag_parts.push(format!("sonic:\"{},omitempty\"", branch.xml_name));
            }
            let tag = if tag_parts.is_empty() {
                String::new()
            } else {
                format!(" `{}`", tag_parts.join(" "))
            };
            writeln!(out, "    {} *{}{}", field_name, mapped_type, tag).unwrap();
        }
        writeln!(out, "}}\n").unwrap();

        // Selected() string helper
        writeln!(out, "func (c {}) Selected() string {{", choice_name).unwrap();
        for (branch, field_name) in u.branches.iter().zip(&branch_field_names) {
            writeln!(out, "    if c.{} != nil {{", field_name).unwrap();
            writeln!(out, "        return \"{}\"", branch.xml_name).unwrap();
            writeln!(out, "    }}").unwrap();
        }
        writeln!(out, "    return \"\"").unwrap();
        writeln!(out, "}}\n").unwrap();

        // Validate() error method
        writeln!(out, "func (c {}) Validate() error {{", choice_name).unwrap();
        writeln!(out, "    count := 0").unwrap();
        for field_name in &branch_field_names {
            writeln!(out, "    if c.{} != nil {{ count++ }}", field_name).unwrap();
        }
        writeln!(out, "    if count > 1 {{").unwrap();
        writeln!(
            out,
            "        return fmt.Errorf(\"choice {} mutual exclusivity violation: multiple branches populated (%d)\", count)",
            choice_name
        )
        .unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "    return nil").unwrap();
        writeln!(out, "}}\n").unwrap();

        if self.options.validate_choice_exclusivity {
            // UnmarshalXML receiver method
            writeln!(
                out,
                "func (c *{}) UnmarshalXML(d *xml.Decoder, start xml.StartElement) error {{",
                choice_name
            )
            .unwrap();
            writeln!(out, "    switch start.Name.Local {{").unwrap();
            for (branch, field_name) in u.branches.iter().zip(&branch_field_names) {
                let mapped_type = self.context.map_type_ref(&branch.type_ref);
                writeln!(out, "    case \"{}\":", branch.xml_name).unwrap();
                writeln!(out, "        var v {}", mapped_type).unwrap();
                writeln!(
                    out,
                    "        if err := d.DecodeElement(&v, &start); err != nil {{ return err }}"
                )
                .unwrap();
                writeln!(out, "        c.{} = &v", field_name).unwrap();
                writeln!(out, "        return nil").unwrap();
            }
            writeln!(out, "    }}\n").unwrap();
            writeln!(out, "    var raw {}", choice_name).unwrap();
            writeln!(out, "    count := 0\n").unwrap();
            writeln!(out, "    for {{").unwrap();
            writeln!(out, "        tok, err := d.Token()").unwrap();
            writeln!(out, "        if err != nil {{").unwrap();
            writeln!(out, "            if err == io.EOF {{").unwrap();
            writeln!(out, "                break").unwrap();
            writeln!(out, "            }}").unwrap();
            writeln!(out, "            return err").unwrap();
            writeln!(out, "        }}").unwrap();
            writeln!(out, "        switch t := tok.(type) {{").unwrap();
            writeln!(out, "        case xml.StartElement:").unwrap();
            writeln!(out, "            switch t.Name.Local {{").unwrap();

            for (branch, field_name) in u.branches.iter().zip(&branch_field_names) {
                let mapped_type = self.context.map_type_ref(&branch.type_ref);
                writeln!(out, "            case \"{}\":", branch.xml_name).unwrap();
                writeln!(out, "                var v {}", mapped_type).unwrap();
                writeln!(
                    out,
                    "                if err := d.DecodeElement(&v, &t); err != nil {{"
                )
                .unwrap();
                writeln!(out, "                    return err").unwrap();
                writeln!(out, "                }}").unwrap();
                writeln!(out, "                raw.{} = &v", field_name).unwrap();
                writeln!(out, "                count++").unwrap();
            }

            writeln!(out, "            default:").unwrap();
            writeln!(out, "                if err := d.Skip(); err != nil {{").unwrap();
            writeln!(out, "                    return err").unwrap();
            writeln!(out, "                }}").unwrap();
            writeln!(out, "            }}").unwrap();
            writeln!(out, "        case xml.EndElement:").unwrap();
            writeln!(out, "            if t == start.End() {{").unwrap();
            writeln!(out, "                goto validation").unwrap();
            writeln!(out, "            }}").unwrap();
            writeln!(out, "        }}").unwrap();
            writeln!(out, "    }}\n").unwrap();
            writeln!(out, "validation:").unwrap();
            writeln!(out, "    if count > 1 {{").unwrap();
            writeln!(
                out,
                "        return fmt.Errorf(\"choice {} mutual exclusivity violation: multiple branches populated (%d)\", count)",
                choice_name
            )
            .unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out, "    *c = raw").unwrap();
            writeln!(out, "    return nil").unwrap();
            writeln!(out, "}}\n").unwrap();

            // MarshalXML receiver method
            writeln!(
                out,
                "func (c {}) MarshalXML(e *xml.Encoder, start xml.StartElement) error {{",
                choice_name
            )
            .unwrap();
            writeln!(out, "    if err := c.Validate(); err != nil {{").unwrap();
            writeln!(out, "        return err").unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(
                out,
                "    if start.Name.Local == \"\" || start.Name.Local == \"items\" || start.Name.Local == \"Items\" {{"
            )
            .unwrap();
            for (branch, field_name) in u.branches.iter().zip(&branch_field_names) {
                writeln!(out, "        if c.{} != nil {{", field_name).unwrap();
                writeln!(
                    out,
                    "            return e.EncodeElement(c.{}, xml.StartElement{{Name: xml.Name{{Local: \"{}\"}} }})",
                    field_name, branch.xml_name
                )
                .unwrap();
                writeln!(out, "        }}").unwrap();
            }
            writeln!(out, "        return nil").unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out, "    type Alias {}", choice_name).unwrap();
            writeln!(out, "    return e.EncodeElement(Alias(c), start)").unwrap();
            writeln!(out, "}}\n").unwrap();
        }
    }

    fn emit_lexical_union(&self, out: &mut String, u: &UnionDef, ir: &SchemaIR) {
        let name = type_ident(&u.qname);
        let mut seen = HashSet::new();
        let fields = u
            .branches
            .iter()
            .map(|b| self.unique_field_name(&b.variant_name, &mut seen))
            .collect::<Vec<_>>();
        let _ = writeln!(out, "type {} struct {{", name);
        for (branch, field) in u.branches.iter().zip(&fields) {
            let _ = writeln!(
                out,
                "    {} *{} `json:{:?}`",
                field,
                self.context.map_type_ref(&branch.type_ref),
                format!("{},omitempty", field)
            );
        }
        out.push_str("}\n\n");
        let _ = writeln!(out, "func (c {}) Selected() string {{", name);
        for field in &fields {
            let _ = writeln!(out, "    if c.{} != nil {{ return {:?} }}", field, field);
        }
        out.push_str("    return \"\"\n}\n\n");
        let _ = writeln!(
            out,
            "func (c *{}) UnmarshalXML(d *xml.Decoder, start xml.StartElement) error {{",
            name
        );
        out.push_str("    var raw string\n    if err := d.DecodeElement(&raw, &start); err != nil { return err }\n    return c.UnmarshalText([]byte(raw))\n}\n\n");
        let _ = writeln!(
            out,
            "func (c *{}) UnmarshalText(text []byte) error {{",
            name
        );
        out.push_str("    value := strings.TrimSpace(string(text))\n    *c = ");
        let _ = writeln!(out, "{}{{}}", name);
        for (branch, field) in u.branches.iter().zip(&fields) {
            let mapped = self.context.map_type_ref(&branch.type_ref);
            if let TypeRef::Named(qname) = &branch.type_ref {
                if matches!(ir.types.get(qname), Some(TypeDef::Union(inner)) if inner.is_lexical())
                {
                    let _ = writeln!(out, "    {{ var escaped strings.Builder; if err := xml.EscapeText(&escaped, []byte(value)); err != nil {{ return err }}; var v {mapped}; if err := xml.Unmarshal([]byte(\"<v>\"+escaped.String()+\"</v>\"), &v); err == nil {{ c.{field} = &v; return nil }} }}");
                    continue;
                }
                if let Some(TypeDef::Enum(def)) = ir.types.get(qname) {
                    let comparisons = def
                        .variants
                        .iter()
                        .map(|v| format!("value == {:?}", v.value))
                        .collect::<Vec<_>>()
                        .join(" || ");
                    let _ = writeln!(
                        out,
                        "    if {} {{ v := {}(value); c.{} = &v; return nil }}",
                        comparisons, mapped, field
                    );
                    continue;
                }
            }
            let base = super::primitive_base(&branch.type_ref, ir);
            let numeric = match base {
                TypeRef::Primitive(
                    PrimitiveType::Int
                    | PrimitiveType::Integer
                    | PrimitiveType::Long
                    | PrimitiveType::Short
                    | PrimitiveType::Byte,
                ) => Some("signed"),
                TypeRef::Primitive(
                    PrimitiveType::UnsignedInt
                    | PrimitiveType::UnsignedLong
                    | PrimitiveType::UnsignedShort
                    | PrimitiveType::UnsignedByte
                    | PrimitiveType::PositiveInteger
                    | PrimitiveType::NonNegativeInteger,
                ) => Some("unsigned"),
                TypeRef::Primitive(
                    PrimitiveType::Float | PrimitiveType::Double | PrimitiveType::Decimal,
                ) => Some("float"),
                _ => None,
            };
            if matches!(base, TypeRef::Primitive(PrimitiveType::Date)) {
                let _ = writeln!(out, "    if parsed, err := time.Parse(\"2006-01-02\", value); err == nil {{ v := {}{{Time: parsed, lexical: value}}; c.{} = &v; return nil }}", mapped, field);
            } else if matches!(base, TypeRef::Primitive(PrimitiveType::DateTime)) {
                let _ = writeln!(out, "    if parsed, err := time.Parse(time.RFC3339, value); err == nil {{ v := {}{{Time: parsed, lexical: value}}; c.{} = &v; return nil }}", mapped, field);
                let _ = writeln!(out, "    if parsed, err := time.Parse(\"2006-01-02T15:04:05\", value); err == nil {{ v := {}{{Time: parsed, lexical: value}}; c.{} = &v; return nil }}", mapped, field);
            } else if matches!(base, TypeRef::Primitive(PrimitiveType::Time)) {
                let _ = writeln!(out, "    if parsed, err := time.Parse(\"15:04:05Z07:00\", value); err == nil {{ v := {}{{Time: parsed, lexical: value}}; c.{} = &v; return nil }}", mapped, field);
                let _ = writeln!(out, "    if parsed, err := time.Parse(\"15:04:05\", value); err == nil {{ v := {}{{Time: parsed, lexical: value}}; c.{} = &v; return nil }}", mapped, field);
            } else if let Some(kind) = numeric {
                let parse = match kind {
                    "signed" => "strconv.ParseInt(value, 10, 64)",
                    "unsigned" => "strconv.ParseUint(value, 10, 64)",
                    _ => "strconv.ParseFloat(value, 64)",
                };
                let _ = writeln!(out, "    if parsed, err := {}; err == nil {{ v := {}(parsed); c.{} = &v; return nil }}", parse, mapped, field);
            } else if matches!(base, TypeRef::Primitive(PrimitiveType::Boolean)) {
                let _ = writeln!(out, "    if parsed, err := strconv.ParseBool(value); err == nil {{ v := {}(parsed); c.{} = &v; return nil }}", mapped, field);
            } else {
                let simple = match &branch.type_ref {
                    TypeRef::Named(qname) => ir.types.get(qname).and_then(|def| match def {
                        TypeDef::Simple(s) => Some(s.as_ref()),
                        _ => None,
                    }),
                    _ => None,
                };
                if let Some(simple) = simple.filter(|s| !s.facets.patterns.is_empty()) {
                    let checks = simple
                        .facets
                        .patterns
                        .iter()
                        .map(|pattern| {
                            format!(
                                "regexp.MustCompile({:?}).MatchString(value)",
                                format!("^(?:{pattern})$")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" && ");
                    let _ = writeln!(
                        out,
                        "    if {} {{ v := {}(value); c.{} = &v; return nil }}",
                        checks, mapped, field
                    );
                } else {
                    let _ = writeln!(
                        out,
                        "    {{ v := {}(value); c.{} = &v; return nil }}",
                        mapped, field
                    );
                }
            }
        }
        let _ = writeln!(
            out,
            "    return fmt.Errorf(\"invalid lexical value for {}: %q\", value)",
            name
        );
        out.push_str("}\n\n");
        let _ = writeln!(
            out,
            "func (c {}) MarshalXML(e *xml.Encoder, start xml.StartElement) error {{",
            name
        );
        out.push_str("    text, err := c.MarshalText()\n    if err != nil { return err }\n    return e.EncodeElement(string(text), start)\n}\n\n");
        let _ = writeln!(out, "func (c {}) MarshalText() ([]byte, error) {{", name);
        out.push_str("    count := 0\n    var value string\n");
        for (branch, field) in u.branches.iter().zip(&fields) {
            if matches!(&branch.type_ref, TypeRef::Named(qname) if matches!(ir.types.get(qname), Some(TypeDef::Union(inner)) if inner.is_lexical()))
            {
                let _ = writeln!(out, "    if c.{field} != nil {{ data, err := xml.Marshal(c.{field}); if err != nil {{ return nil, err }}; if err := xml.Unmarshal(data, &value); err != nil {{ return nil, err }}; count++ }}");
                continue;
            }
            if matches!(
                super::primitive_base(&branch.type_ref, ir),
                TypeRef::Primitive(
                    PrimitiveType::Date | PrimitiveType::Time | PrimitiveType::DateTime
                )
            ) {
                let _ = writeln!(out, "    if c.{field} != nil {{ text, err := c.{field}.MarshalText(); if err != nil {{ return nil, err }}; count++; value = string(text) }}");
                continue;
            }
            let expr = if matches!(
                super::primitive_base(&branch.type_ref, ir),
                TypeRef::Primitive(PrimitiveType::Date)
            ) {
                format!("c.{}.Format(\"2006-01-02\")", field)
            } else if matches!(
                super::primitive_base(&branch.type_ref, ir),
                TypeRef::Primitive(PrimitiveType::DateTime)
            ) {
                format!("c.{}.Format(time.RFC3339)", field)
            } else if matches!(
                super::primitive_base(&branch.type_ref, ir),
                TypeRef::Primitive(PrimitiveType::Time)
            ) {
                format!("c.{}.Format(\"15:04:05Z07:00\")", field)
            } else {
                format!("fmt.Sprint(*c.{})", field)
            };
            let _ = writeln!(
                out,
                "    if c.{} != nil {{ count++; value = {} }}",
                field, expr
            );
        }
        out.push_str("    if count != 1 { return nil, fmt.Errorf(\"lexical union requires exactly one member\") }\n    return []byte(value), nil\n}\n\n");
    }

    fn emit_struct(&self, out: &mut String, s: &StructDef, ir: &SchemaIR) {
        if s.is_abstract {
            self.emit_abstract_struct(out, s, ir);
            return;
        }

        let struct_name = type_ident(&s.qname);
        if let Some(ref doc) = s.documentation {
            for line in doc.lines() {
                writeln!(out, "// {}", line).unwrap();
            }
        }

        if self.options.backend == GoBackend::EasyJson {
            writeln!(out, "//easyjson:json").unwrap();
        }

        writeln!(out, "type {} struct {{", struct_name).unwrap();

        // Emit XMLName if xml tags enabled
        if self.options.emit_xml_tags {
            let mut xml_tags = Vec::new();
            if self.options.emit_json_tags {
                xml_tags.push("json:\"-\"");
            }
            if self.options.backend == GoBackend::Sonic {
                xml_tags.push("sonic:\"-\"");
            }
            if xml_tags.is_empty() {
                writeln!(out, "    XMLName xml.Name").unwrap();
            } else {
                writeln!(out, "    XMLName xml.Name `{}`", xml_tags.join(" ")).unwrap();
            }
        }

        // Struct composition / inheritance if base struct exists
        let mut simple_content_base: Option<String> = None;
        if let Some(ref base_qname) = s.base_type {
            if let Some(TypeDef::Struct(base_s)) = ir.types.get(base_qname) {
                let base_name = type_ident(base_qname);
                if base_s.is_abstract {
                    writeln!(out, "    {}Base", base_name).unwrap();
                } else {
                    writeln!(out, "    {}", base_name).unwrap();
                }
            } else if let Some(prim) = PrimitiveType::from_xsd_name(&base_qname.local) {
                simple_content_base = Some(self.context.map_primitive(prim).to_string());
            } else if let Some(TypeDef::Simple(st)) = ir.types.get(base_qname) {
                simple_content_base = Some(type_ident(&st.qname));
            }
        }

        if let Some(ref base_type_str) = simple_content_base {
            let has_value_field = s
                .fields
                .iter()
                .any(|f| f.name == "value" || f.kind == FieldKind::Text);
            if !has_value_field {
                let mut parts = Vec::new();
                if self.options.emit_xml_tags {
                    parts.push("xml:\",chardata\"".to_string());
                }
                if self.options.emit_json_tags {
                    parts.push("json:\"value,omitempty\"".to_string());
                }
                if self.options.backend == GoBackend::Sonic {
                    parts.push("sonic:\"value,omitempty\"".to_string());
                }
                let tag = if parts.is_empty() {
                    String::new()
                } else {
                    format!(" `{}`", parts.join(" "))
                };
                writeln!(out, "    Value {}{}", base_type_str, tag).unwrap();
            }
        }

        let mut seen = HashSet::new();
        if self.options.emit_xml_tags {
            seen.insert("XMLName".to_string());
        }
        if simple_content_base.is_some() {
            let has_value_field = s
                .fields
                .iter()
                .any(|f| f.name == "value" || f.kind == FieldKind::Text);
            if !has_value_field {
                seen.insert("Value".to_string());
            }
        }

        let field_names: Vec<String> = s
            .fields
            .iter()
            .map(|f| self.unique_field_name(&f.name, &mut seen))
            .collect();

        let mut seen_json = HashSet::new();
        if simple_content_base.is_some() {
            let has_value_field = s
                .fields
                .iter()
                .any(|f| f.name == "value" || f.kind == FieldKind::Text);
            if !has_value_field {
                seen_json.insert("value".to_string());
            }
        }

        // Fields
        for (f, field_name) in s.fields.iter().zip(&field_names) {
            if let Some(ref doc) = f.documentation {
                writeln!(out, "    // {}", doc).unwrap();
            }

            let field_type = self.resolve_field_type(f);
            let tag = self.build_field_struct_tags(f, s, ir, &mut seen_json);
            writeln!(out, "    {} {}{}", field_name, field_type, tag).unwrap();
        }

        writeln!(out, "}}\n").unwrap();

        if ir.has_ordered_content(s) && self.options.emit_xml_tags {
            self.emit_mixed_struct_xml(out, s, ir, &field_names);
        }

        if self.options.validate_facets {
            self.emit_struct_validator(out, s, ir, &field_names);
        }
    }

    fn emit_mixed_struct_xml(
        &self,
        out: &mut String,
        s: &StructDef,
        ir: &SchemaIR,
        field_names: &[String],
    ) {
        let Some((item_index, union)) =
            s.fields
                .iter()
                .enumerate()
                .find_map(|(index, field)| match &field.type_ref {
                    TypeRef::Named(qname) => match ir.types.get(qname) {
                        Some(TypeDef::Union(union)) if union.is_mixed_content() => {
                            Some((index, union))
                        }
                        _ => None,
                    },
                    _ => None,
                })
        else {
            return;
        };
        let struct_name = type_ident(&s.qname);
        let item_field = &field_names[item_index];
        let union_name = type_ident(&union.qname);
        let mut seen = HashSet::new();
        let branch_fields = union
            .branches
            .iter()
            .map(|branch| self.unique_field_name(&branch.variant_name, &mut seen))
            .collect::<Vec<_>>();

        let _ = writeln!(
            out,
            "func (v *{struct_name}) UnmarshalXML(d *xml.Decoder, start xml.StartElement) error {{"
        );
        let _ = writeln!(out, "    type Alias {struct_name}");
        out.push_str("    var attrBytes bytes.Buffer\n    attrEncoder := xml.NewEncoder(&attrBytes)\n    if err := attrEncoder.EncodeToken(start); err != nil { return err }\n    if err := attrEncoder.EncodeToken(start.End()); err != nil { return err }\n    if err := attrEncoder.Flush(); err != nil { return err }\n    var attrs Alias\n    if err := xml.Unmarshal(attrBytes.Bytes(), &attrs); err != nil { return err }\n");
        let _ = writeln!(out, "    *v = {struct_name}(attrs)");
        let _ = writeln!(out, "    v.{item_field} = nil");
        out.push_str("    for {\n        token, err := d.Token()\n        if err != nil { if err == io.EOF { return io.ErrUnexpectedEOF }; return err }\n        switch element := token.(type) {\n        case xml.CharData:\n            if len(element) > 0 {\n                value := string(element)\n");
        let _ = writeln!(out, "                v.{item_field} = append(v.{item_field}, {union_name}{{{text_field}: &value}})", text_field=branch_fields[0]);
        out.push_str("            }\n        case xml.StartElement:\n            switch element.Name.Local {\n");
        for (branch, branch_field) in union.branches.iter().zip(&branch_fields) {
            if branch.xml_name == "#text" {
                continue;
            }
            let mapped = self.context.map_type_ref(&branch.type_ref);
            let _ = writeln!(out, "            case {:?}:\n                var value {mapped}\n                if err := d.DecodeElement(&value, &element); err != nil {{ return err }}\n                v.{item_field} = append(v.{item_field}, {union_name}{{{branch_field}: &value}})", branch.xml_name);
        }
        out.push_str("            default:\n                if err := d.Skip(); err != nil { return err }\n            }\n        case xml.EndElement:\n            if element.Name == start.Name { return nil }\n        }\n    }\n}\n\n");

        let _ = writeln!(
            out,
            "func (v {struct_name}) MarshalXML(e *xml.Encoder, start xml.StartElement) error {{"
        );
        let _ = writeln!(
            out,
            "    type Alias {struct_name}\n    attrs := Alias(v)\n    attrs.{item_field} = nil"
        );
        out.push_str("    raw, err := xml.Marshal(attrs)\n    if err != nil { return err }\n    decoder := xml.NewDecoder(bytes.NewReader(raw))\n    token, err := decoder.Token()\n    if err != nil { return err }\n    start.Attr = token.(xml.StartElement).Attr\n    if err := e.EncodeToken(start); err != nil { return err }\n");
        let _ = writeln!(out, "    for _, item := range v.{item_field} {{");
        for (branch, branch_field) in union.branches.iter().zip(&branch_fields) {
            let _ = writeln!(out, "        if item.{branch_field} != nil {{");
            if branch.xml_name == "#text" {
                let _ = writeln!(out, "            if err := e.EncodeToken(xml.CharData(*item.{branch_field})); err != nil {{ return err }}");
            } else {
                let _ = writeln!(out, "            if err := e.EncodeElement(item.{branch_field}, xml.StartElement{{Name: xml.Name{{Local: {:?}}}}}); err != nil {{ return err }}", branch.xml_name);
            }
            out.push_str("            continue\n        }\n");
        }
        out.push_str("        return fmt.Errorf(\"mixed content item has no selected branch\")\n    }\n    return e.EncodeToken(start.End())\n}\n\n");
    }

    fn resolve_field_type(&self, f: &FieldDef) -> String {
        if f.kind == FieldKind::AnyAttribute {
            return "[]xml.Attr".to_string();
        }
        if f.kind == FieldKind::Any {
            if f.cardinality.is_list() {
                return "[]AnyElement".to_string();
            } else if f.is_cycle_cut || f.cardinality.is_optional() || f.nillable {
                return "*AnyElement".to_string();
            } else {
                return "AnyElement".to_string();
            }
        }
        let base_type = self.context.map_type_ref(&f.type_ref);

        if f.cardinality.is_list() {
            if f.is_cycle_cut {
                format!("[]*{}", base_type)
            } else {
                format!("[]{}", base_type)
            }
        } else if f.is_cycle_cut || f.cardinality.is_optional() || f.nillable {
            format!("*{}", base_type)
        } else {
            base_type
        }
    }

    fn build_field_struct_tags(
        &self,
        f: &FieldDef,
        s: &StructDef,
        ir: &SchemaIR,
        seen_json: &mut HashSet<String>,
    ) -> String {
        let mut parts = Vec::new();

        if self.options.emit_xml_tags {
            let is_opt = f.cardinality.is_optional() || f.nillable;
            let xml_val = match f.kind {
                FieldKind::Attribute => {
                    let attr_name = match &f.namespace {
                        Some(ns) if !ns.is_empty() => {
                            format!("{} {}", ns, f.xml_name)
                        }
                        _ => f.xml_name.clone(),
                    };
                    if is_opt {
                        format!("{},attr,omitempty", attr_name)
                    } else {
                        format!("{},attr", attr_name)
                    }
                }
                FieldKind::Text => ",chardata".to_string(),
                FieldKind::Any => ",any".to_string(),
                FieldKind::AnyAttribute => ",any,attr".to_string(),
                FieldKind::Element => {
                    let elem_name = match &f.namespace {
                        Some(ns) if !ns.is_empty() && is_namespaced_ref(f, s, ir) => {
                            format!("{} {}", ns, f.xml_name)
                        }
                        _ => f.xml_name.clone(),
                    };
                    if f.xml_name.is_empty() {
                        ",any".to_string()
                    } else if is_opt {
                        format!("{},omitempty", elem_name)
                    } else {
                        elem_name
                    }
                }
            };
            parts.push(format!("xml:\"{}\"", xml_val));
        }

        if self.options.emit_json_tags || self.options.backend == GoBackend::Sonic {
            if f.kind == FieldKind::AnyAttribute {
                if self.options.emit_json_tags {
                    parts.push("json:\"-\"".to_string());
                }
                if self.options.backend == GoBackend::Sonic {
                    parts.push("sonic:\"-\"".to_string());
                }
            } else if f.kind == FieldKind::Any {
                let is_opt = f.cardinality.is_optional() || f.nillable;
                let tag = if is_opt { "any,omitempty" } else { "any" };
                if self.options.emit_json_tags {
                    parts.push(format!("json:\"{}\"", tag));
                }
                if self.options.backend == GoBackend::Sonic {
                    parts.push(format!("sonic:\"{}\"", tag));
                }
            } else {
                let is_opt = f.cardinality.is_optional() || f.nillable;
                let raw_json_name = if f.kind == FieldKind::Text {
                    "value".to_string()
                } else if f.xml_name.is_empty() {
                    f.name.clone()
                } else {
                    f.xml_name.clone()
                };
                let mut json_name = raw_json_name.clone();
                let mut counter = 2;
                while seen_json.contains(&json_name) {
                    json_name = format!("{}_{}", raw_json_name, counter);
                    counter += 1;
                }
                seen_json.insert(json_name.clone());

                let json_val = if is_opt {
                    format!("{},omitempty", json_name)
                } else {
                    json_name
                };
                if self.options.emit_json_tags {
                    parts.push(format!("json:\"{}\"", json_val));
                }
                if self.options.backend == GoBackend::Sonic {
                    parts.push(format!("sonic:\"{}\"", json_val));
                }
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!(" `{}`", parts.join(" "))
        }
    }

    fn emit_struct_validator(
        &self,
        out: &mut String,
        s: &StructDef,
        ir: &SchemaIR,
        field_names: &[String],
    ) {
        let struct_name = type_ident(&s.qname);
        writeln!(out, "func (s {}) Validate() error {{", struct_name).unwrap();

        let mut has_checks = false;
        for (f, field_name) in s.fields.iter().zip(field_names) {
            let is_opt = f.cardinality.is_optional() || f.nillable;

            if super::patterned_simple(&f.type_ref, ir).is_some() {
                if f.cardinality.is_list() || f.type_ref.is_list() {
                    writeln!(out, "    for _, value := range s.{} {{ if err := value.Validate(); err != nil {{ return err }} }}", field_name).unwrap();
                } else if is_opt {
                    writeln!(out, "    if s.{} != nil {{ if err := s.{}.Validate(); err != nil {{ return err }} }}", field_name, field_name).unwrap();
                } else {
                    writeln!(
                        out,
                        "    if err := s.{}.Validate(); err != nil {{ return err }}",
                        field_name
                    )
                    .unwrap();
                }
            }
            if let Some(ref facets) = f.facets {
                if is_opt {
                    writeln!(out, "    if s.{} != nil {{", field_name).unwrap();
                    self.emit_facet_checks(
                        out,
                        facets,
                        &format!("(*s.{})", field_name),
                        "        ",
                    );
                    writeln!(out, "    }}").unwrap();
                } else {
                    self.emit_facet_checks(out, facets, &format!("s.{}", field_name), "    ");
                }
                has_checks = true;
            }
        }

        let _ = has_checks;
        writeln!(out, "    return nil").unwrap();
        writeln!(out, "}}\n").unwrap();
    }

    fn emit_abstract_struct(&self, out: &mut String, s: &StructDef, ir: &SchemaIR) {
        let struct_name = type_ident(&s.qname);
        let base_struct_name = format!("{}Base", struct_name);

        if let Some(ref doc) = s.documentation {
            for line in doc.lines() {
                writeln!(out, "// {}", line).unwrap();
            }
        }

        // 1. Base struct definition holding declared fields of the abstract type
        writeln!(out, "type {} struct {{", base_struct_name).unwrap();
        if let Some(ref base_qname) = s.base_type {
            if let Some(TypeDef::Struct(base_s)) = ir.types.get(base_qname) {
                let base_name = type_ident(base_qname);
                if base_s.is_abstract {
                    writeln!(out, "    {}Base", base_name).unwrap();
                } else {
                    writeln!(out, "    {}", base_name).unwrap();
                }
            }
        }

        let mut seen_fields = HashSet::new();
        let field_names: Vec<String> = s
            .fields
            .iter()
            .map(|f| self.unique_field_name(&f.name, &mut seen_fields))
            .collect();

        let mut seen_json = HashSet::new();
        for (f, field_name) in s.fields.iter().zip(&field_names) {
            let field_type = self.resolve_field_type(f);
            let tag = self.build_field_struct_tags(f, s, ir, &mut seen_json);
            writeln!(out, "    {} {}{}", field_name, field_type, tag).unwrap();
        }
        writeln!(out, "}}\n").unwrap();

        writeln!(
            out,
            "func (s {}) Validate() error {{\n    return nil\n}}\n",
            base_struct_name
        )
        .unwrap();

        // 2. Discover concrete derivations
        let derivations = concrete_derivations(s, ir);

        // 3. Dispatch wrapper struct
        writeln!(out, "type {} struct {{", struct_name).unwrap();
        if self.options.emit_xml_tags {
            let mut xml_tags = Vec::new();
            if self.options.emit_json_tags {
                xml_tags.push("json:\"-\"");
            }
            if self.options.backend == GoBackend::Sonic {
                xml_tags.push("sonic:\"-\"");
            }
            if xml_tags.is_empty() {
                writeln!(out, "    XMLName xml.Name").unwrap();
            } else {
                writeln!(out, "    XMLName xml.Name `{}`", xml_tags.join(" ")).unwrap();
            }
        }
        writeln!(out, "    {}", base_struct_name).unwrap();

        let mut seen_derives = HashSet::new();
        let derive_fields: Vec<(String, String, String)> = derivations
            .iter()
            .map(|d| {
                let type_name = type_ident(&d.qname);
                let field_name = self.unique_field_name(&type_name, &mut seen_derives);
                (d.qname.local.clone(), field_name, type_name)
            })
            .collect();

        for (_local, field_name, type_name) in &derive_fields {
            let json_name = AsLowerCamelCase(field_name).to_string();
            writeln!(
                out,
                "    {} *{} `xml:\"-\" json:\"{},omitempty\"`",
                field_name, type_name, json_name
            )
            .unwrap();
        }
        writeln!(out, "}}\n").unwrap();

        // 4. Methods on dispatch wrapper
        writeln!(out, "func (s {}) Selected() string {{", struct_name).unwrap();
        for (local, field_name, _) in &derive_fields {
            writeln!(
                out,
                "    if s.{} != nil {{ return {:?} }}",
                field_name, local
            )
            .unwrap();
        }
        writeln!(out, "    return \"\"\n}}\n").unwrap();

        writeln!(out, "func (s {}) Value() any {{", struct_name).unwrap();
        for (_, field_name, _) in &derive_fields {
            writeln!(
                out,
                "    if s.{} != nil {{ return s.{} }}",
                field_name, field_name
            )
            .unwrap();
        }
        writeln!(out, "    return nil\n}}\n").unwrap();

        // UnmarshalXML
        writeln!(
            out,
            "func (s *{}) UnmarshalXML(d *xml.Decoder, start xml.StartElement) error {{",
            struct_name
        )
        .unwrap();
        writeln!(out, "    s.XMLName = start.Name").unwrap();
        writeln!(out, "    var typeAttr *xml.Attr").unwrap();
        writeln!(out, "    for _, attr := range start.Attr {{").unwrap();
        writeln!(out, "        if (attr.Name.Local == \"type\" && (attr.Name.Space == \"http://www.w3.org/2001/XMLSchema-instance\" || attr.Name.Space == \"xsi\")) || attr.Name.Local == \"xsi:type\" {{").unwrap();
        writeln!(out, "            typeAttr = &attr").unwrap();
        writeln!(out, "            break").unwrap();
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "    if typeAttr == nil {{").unwrap();
        writeln!(
            out,
            "        return fmt.Errorf(\"abstract type '{}' requires xsi:type naming a concrete derivation\")",
            struct_name
        )
        .unwrap();
        writeln!(out, "    }}").unwrap();

        if derive_fields.is_empty() {
            writeln!(
                out,
                "    return fmt.Errorf(\"abstract type '{}' has no registered derivations; deserialize the concrete type directly (escape hatch)\")",
                struct_name
            )
            .unwrap();
        } else {
            writeln!(out, "    localType := typeAttr.Value").unwrap();
            writeln!(
                out,
                "    if idx := strings.Index(localType, \":\"); idx != -1 {{"
            )
            .unwrap();
            writeln!(out, "        localType = localType[idx+1:]").unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out, "    switch localType {{").unwrap();
            for (local, field_name, type_name) in &derive_fields {
                writeln!(out, "    case {:?}:", local).unwrap();
                writeln!(out, "        var concrete {}", type_name).unwrap();
                writeln!(
                    out,
                    "        if err := d.DecodeElement(&concrete, &start); err != nil {{ return err }}"
                )
                .unwrap();
                writeln!(out, "        s.{} = &concrete", field_name).unwrap();
                writeln!(
                    out,
                    "        s.{} = concrete.{}",
                    base_struct_name, base_struct_name
                )
                .unwrap();
                writeln!(out, "        return nil").unwrap();
            }
            let known = derive_fields
                .iter()
                .map(|(local, _, _)| local.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(out, "    default:").unwrap();
            writeln!(
                    out,
                    "        return fmt.Errorf(\"xsi:type=%q does not match any known derivation of '{}' (known: {})\", typeAttr.Value)",
                    struct_name, known
                )
                .unwrap();
            writeln!(out, "    }}").unwrap();
        }
        writeln!(out, "}}\n").unwrap();

        // MarshalXML
        writeln!(
            out,
            "func (s {}) MarshalXML(e *xml.Encoder, start xml.StartElement) error {{",
            struct_name
        )
        .unwrap();
        writeln!(
            out,
            "    if s.XMLName.Local != \"\" {{ start.Name = s.XMLName }}"
        )
        .unwrap();
        for (local, field_name, _) in &derive_fields {
            writeln!(out, "    if s.{} != nil {{", field_name).unwrap();
            writeln!(out, "        hasXsiNs := false\n        hasType := false").unwrap();
            writeln!(out, "        for _, attr := range start.Attr {{").unwrap();
            writeln!(out, "            if attr.Name.Local == \"xmlns:xsi\" || (attr.Name.Local == \"xsi\" && attr.Name.Space == \"xmlns\") {{ hasXsiNs = true }}").unwrap();
            writeln!(out, "            if attr.Name.Local == \"xsi:type\" || (attr.Name.Local == \"type\" && attr.Name.Space == \"http://www.w3.org/2001/XMLSchema-instance\") {{ hasType = true }}").unwrap();
            writeln!(out, "        }}").unwrap();
            writeln!(out, "        if !hasXsiNs {{ start.Attr = append(start.Attr, xml.Attr{{Name: xml.Name{{Local: \"xmlns:xsi\"}}, Value: \"http://www.w3.org/2001/XMLSchema-instance\"}}) }}").unwrap();
            writeln!(out, "        if !hasType {{ start.Attr = append(start.Attr, xml.Attr{{Name: xml.Name{{Local: \"xsi:type\"}}, Value: {:?}}}) }}", local).unwrap();
            writeln!(
                out,
                "        return e.EncodeElement(s.{}, start)",
                field_name
            )
            .unwrap();
            writeln!(out, "    }}").unwrap();
        }
        if derive_fields.is_empty() {
            writeln!(
                out,
                "    return fmt.Errorf(\"abstract type '{}' has no registered derivations\")",
                struct_name
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "    return fmt.Errorf(\"cannot marshal abstract type '{}' without a concrete derivation\")",
                struct_name
            )
            .unwrap();
        }
        writeln!(out, "}}\n").unwrap();

        // Validate()
        writeln!(out, "func (s {}) Validate() error {{", struct_name).unwrap();
        for (_, field_name, _) in &derive_fields {
            writeln!(
                out,
                "    if s.{} != nil {{ return s.{}.Validate() }}",
                field_name, field_name
            )
            .unwrap();
        }
        if derive_fields.is_empty() {
            writeln!(
                out,
                "    return fmt.Errorf(\"abstract type '{}' has no registered derivations\")",
                struct_name
            )
            .unwrap();
        } else {
            writeln!(
                out,
                "    return fmt.Errorf(\"abstract type '{}' requires a concrete derivation\")",
                struct_name
            )
            .unwrap();
        }
        writeln!(out, "}}\n").unwrap();
    }

    fn emit_facet_checks(
        &self,
        out: &mut String,
        facets: &RestrictionFacets,
        target: &str,
        indent: &str,
    ) {
        for pattern in &facets.patterns {
            writeln!(out, "{}if matched, err := regexp.MatchString({:?}, fmt.Sprint({})); err != nil || !matched {{ return fmt.Errorf(\"pattern constraint failed\") }}", indent, format!("^(?:{pattern})$"), target).unwrap();
        }
        if let Some(min_len) = facets.min_length {
            writeln!(
                out,
                "{}if len({}) < {} {{ return fmt.Errorf(\"field violates minLength constraint ({})\") }}",
                indent, target, min_len, min_len
            )
            .unwrap();
        }
        if let Some(max_len) = facets.max_length {
            writeln!(
                out,
                "{}if len({}) > {} {{ return fmt.Errorf(\"field violates maxLength constraint ({})\") }}",
                indent, target, max_len, max_len
            )
            .unwrap();
        }
        if let Some(len) = facets.length {
            writeln!(
                out,
                "{}if len({}) != {} {{ return fmt.Errorf(\"field violates length constraint ({})\") }}",
                indent, target, len, len
            )
            .unwrap();
        }
        if let Some(ref min_inc) = facets.min_inclusive {
            writeln!(
                out,
                "{}if {} < {} {{ return fmt.Errorf(\"field violates minInclusive constraint ({})\") }}",
                indent, target, min_inc, min_inc
            )
            .unwrap();
        }
        if let Some(ref max_inc) = facets.max_inclusive {
            writeln!(
                out,
                "{}if {} > {} {{ return fmt.Errorf(\"field violates maxInclusive constraint ({})\") }}",
                indent, target, max_inc, max_inc
            )
            .unwrap();
        }
    }

    fn emit_root_aliases(&self, out: &mut String, ir: &SchemaIR) {
        if !self.options.emit_root_aliases || ir.elements.is_empty() {
            return;
        }

        writeln!(out, "// Root XML Element Type Aliases").unwrap();
        let mut declared: HashSet<String> = ir
            .types
            .values()
            .map(|def| type_ident(def.qname()))
            .collect();
        for elem in ir.elements.values() {
            let elem_alias = to_go_type_name(&elem.qname.local);
            let target_type = self.context.map_type_ref(&elem.type_ref);
            if elem_alias != target_type && declared.insert(elem_alias.clone()) {
                if let Some(ref doc) = elem.documentation {
                    for line in doc.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            writeln!(out, "// {}", trimmed).unwrap();
                        }
                    }
                }
                if self.options.emit_xml_tags {
                    let namespace = elem.qname.namespace.as_deref().unwrap_or("");
                    writeln!(out, "type {} {}\n", elem_alias, target_type).unwrap();
                    if matches!(&elem.type_ref, TypeRef::Named(q) if matches!(ir.types.get(q), Some(TypeDef::Struct(s)) if s.is_abstract))
                    {
                        writeln!(out, "func (r {elem_alias}) Selected() string {{ return {target_type}(r).Selected() }}\n").unwrap();
                    }
                    writeln!(out, "func (r *{elem_alias}) UnmarshalXML(d *xml.Decoder, start xml.StartElement) error {{\n    if start.Name.Local != {:?} || start.Name.Space != {:?} {{ return fmt.Errorf(\"unexpected root element: %v\", start.Name) }}\n    return d.DecodeElement((*{target_type})(r), &start)\n}}\n", elem.qname.local, namespace).unwrap();
                    let bind_name = if matches!(&elem.type_ref, TypeRef::Named(q) if matches!(ir.types.get(q), Some(TypeDef::Struct(_))))
                    {
                        "    value.XMLName = start.Name\n"
                    } else {
                        ""
                    };
                    writeln!(out, "func (r {elem_alias}) MarshalXML(e *xml.Encoder, start xml.StartElement) error {{\n    start.Name = xml.Name{{Local: {:?}, Space: {:?}}}\n    value := {target_type}(r)\n{bind_name}    return e.EncodeElement(value, start)\n}}\n", elem.qname.local, namespace).unwrap();
                } else {
                    writeln!(out, "type {} = {}\n", elem_alias, target_type).unwrap();
                }
            }
        }
    }
}

fn derives_from(ir: &SchemaIR, d: &StructDef, base: &QName) -> bool {
    let mut seen: HashSet<QName> = HashSet::new();
    let mut cur = d.base_type.as_ref();
    while let Some(q) = cur {
        if q == base {
            return true;
        }
        if !seen.insert(q.clone()) {
            break;
        }
        cur = match ir.types.get(q) {
            Some(TypeDef::Struct(b)) => b.base_type.as_ref(),
            _ => None,
        };
    }
    false
}

fn concrete_derivations<'a>(s: &'a StructDef, ir: &'a SchemaIR) -> Vec<&'a StructDef> {
    let mut result = Vec::new();
    for type_def in ir.types.values() {
        if let TypeDef::Struct(d) = type_def {
            if !d.is_abstract && d.qname != s.qname && derives_from(ir, d, &s.qname) {
                result.push(d);
            }
        }
    }
    result.sort_by(|a, b| a.qname.local.cmp(&b.qname.local));
    result
}

fn is_namespaced_ref(f: &FieldDef, s: &StructDef, ir: &SchemaIR) -> bool {
    let Some(ref ns) = f.namespace else {
        return false;
    };
    if ns.is_empty() {
        return false;
    }
    // If field namespace is different from struct namespace, it's definitely an external ref
    if s.qname.namespace.as_deref() != Some(ns.as_str()) {
        return true;
    }
    // If it matches a global element definition, it's an element ref
    let q = QName::new(Some(ns.as_str()), &f.xml_name);
    ir.elements.contains_key(&q)
}
