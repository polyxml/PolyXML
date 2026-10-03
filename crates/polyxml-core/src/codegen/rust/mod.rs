use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write as FmtWrite;

use heck::{AsPascalCase, AsSnakeCase};
use serde::{Deserialize, Serialize};

use crate::codegen::{
    build_type_name_map, flatten_fields, lookup_type_name, primitive_base, sanitize_keyword,
    set_type_name_map, write_documentation_lines, LanguageContext,
};
use crate::ir::{
    EnumDef, FieldDef, FieldKind, PrimitiveType, QName, SchemaIR, SimpleTypeDef, StructDef,
    TypeDef, TypeRef, UnionDef,
};

/// Options configuring Rust 2021/2024 code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustOptions {
    /// Use zero-copy Cow<'a, str> for XML lexical values (default: true).
    pub zero_copy: bool,
    /// Derive serde::{Serialize, Deserialize} (default: true).
    pub derive_serde: bool,
    /// Derive Default when applicable (default: true).
    pub derive_default: bool,
    /// Emit #[polyxml(...)] field attributes (default: true).
    pub emit_polyxml_attrs: bool,
    /// Emit root element type aliases (default: true).
    pub emit_root_aliases: bool,
    /// Emit streaming XML codecs (from_xml, to_xml, decode_xml, encode_xml) (default: true).
    pub emit_codecs: bool,
    /// Derive rkyv::{Archive, Serialize, Deserialize} zero-copy wire format serialization (default: false).
    pub emit_rkyv: bool,
    /// Emit compile-time perfect-hash (phf) element-tag dispatch instead of a
    /// string `match` (default: false). Generated crates must depend on `phf`.
    #[serde(default)]
    pub phf: bool,
    /// Emit PyO3 #[pyclass], #[pymethods], and #[pymodule] bindings for Python AOT extension modules.
    #[serde(default)]
    pub pyo3: bool,
    /// PyO3 module name when `pyo3` is true. Defaults to "models" if None.
    #[serde(default)]
    pub pyo3_module_name: Option<String>,
    /// Custom header text to prepend to generated files (default: None).
    pub custom_header: Option<String>,
    /// Split oversized modules into bounded topological chunks (optional setting, default: false).
    #[serde(default)]
    pub split_units: Option<bool>,
    /// Target maximum types per compilation unit chunk (default: 250).
    #[serde(default)]
    pub chunk_size: Option<usize>,
}

impl Default for RustOptions {
    fn default() -> Self {
        Self {
            zero_copy: true,
            derive_serde: true,
            derive_default: true,
            emit_polyxml_attrs: false,
            emit_root_aliases: true,
            emit_codecs: true,
            emit_rkyv: false,
            phf: false,
            pyo3: false,
            pyo3_module_name: None,
            custom_header: None,
            split_units: None,
            chunk_size: None,
        }
    }
}

/// Language context adapter for Rust 2021/2024.
pub struct RustLanguageContext {
    zero_copy: bool,
}

impl RustLanguageContext {
    pub fn new(zero_copy: bool) -> Self {
        Self { zero_copy }
    }
}

impl LanguageContext for RustLanguageContext {
    fn target_language(&self) -> &'static str {
        "rust"
    }

    fn map_primitive(&self, prim: PrimitiveType) -> &'static str {
        match prim {
            PrimitiveType::Integer
            | PrimitiveType::PositiveInteger
            | PrimitiveType::NegativeInteger
            | PrimitiveType::NonPositiveInteger
            | PrimitiveType::NonNegativeInteger => {
                if self.zero_copy {
                    "Cow<'a, str>"
                } else {
                    "String"
                }
            }
            PrimitiveType::String
            | PrimitiveType::NormalizedString
            | PrimitiveType::Token
            | PrimitiveType::Language
            | PrimitiveType::NMTOKEN
            | PrimitiveType::NMTOKENS
            | PrimitiveType::Name
            | PrimitiveType::NCName
            | PrimitiveType::Id
            | PrimitiveType::IdRef
            | PrimitiveType::IdRefs
            | PrimitiveType::Entity
            | PrimitiveType::Entities
            | PrimitiveType::DateTime
            | PrimitiveType::Date
            | PrimitiveType::Time
            | PrimitiveType::Duration
            | PrimitiveType::GYearMonth
            | PrimitiveType::GYear
            | PrimitiveType::GMonthDay
            | PrimitiveType::GDay
            | PrimitiveType::GMonth
            | PrimitiveType::AnyUri
            | PrimitiveType::QName
            | PrimitiveType::AnyType
            | PrimitiveType::AnySimpleType => {
                if self.zero_copy {
                    "Cow<'a, str>"
                } else {
                    "String"
                }
            }

            PrimitiveType::Boolean => "bool",

            PrimitiveType::Decimal | PrimitiveType::Double => "f64",
            PrimitiveType::Float => "f32",

            PrimitiveType::Long => "i64",
            PrimitiveType::Int => "i32",
            PrimitiveType::Short => "i16",
            PrimitiveType::Byte => "i8",

            PrimitiveType::UnsignedLong => "u64",
            PrimitiveType::UnsignedInt => "u32",
            PrimitiveType::UnsignedShort => "u16",
            PrimitiveType::UnsignedByte => "u8",

            // XML carries hex/base64 lexical text, just as the runtime schema does.
            // Preserve that spelling; callers can explicitly decode binary payloads.
            PrimitiveType::HexBinary | PrimitiveType::Base64Binary => {
                if self.zero_copy {
                    "Cow<'a, str>"
                } else {
                    "String"
                }
            }
        }
    }

    fn map_type_ref(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(prim) => self.map_primitive(*prim).to_string(),
            TypeRef::Named(qname) => type_ident(qname),
            TypeRef::Boxed(inner) => format!("Box<{}>", self.map_type_ref(inner)),
            TypeRef::List(inner) => format!("Vec<{}>", self.map_type_ref(inner)),
        }
    }
}

/// Converts an arbitrary string into a safe, valid Rust enum variant identifier (PascalCase).
pub fn to_rust_variant_identifier(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return "Empty".to_string();
    }

    let cleaned: String = trimmed
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();

    let pascal = AsPascalCase(&cleaned).to_string();
    let identifier = if pascal.is_empty() {
        "Empty".to_string()
    } else {
        pascal
    };

    if identifier
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("Value{}", identifier)
    } else {
        identifier
    }
}

/// Sanitizes a Rust field name to snake_case and raw identifier if reserved.
pub fn to_rust_field_identifier(name: &str) -> String {
    let snake = AsSnakeCase(name).to_string();
    let safe_name = if snake
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("_{}", snake)
    } else if snake.is_empty() {
        "value".to_string()
    } else {
        snake
    };

    sanitize_keyword(&safe_name, "rust")
}

/// Sanitizes a Rust module identifier (derived from a schema file stem or module name).
pub fn sanitize_rust_module_name(name: &str) -> String {
    let mut safe = String::new();
    let mut chars = name.chars().peekable();
    if let Some(&first) = chars.peek() {
        if first.is_ascii_digit() {
            safe.push('_');
        }
    }
    for c in chars {
        if c.is_alphanumeric() || c == '_' {
            safe.push(c);
        } else {
            safe.push('_');
        }
    }
    if safe.is_empty() {
        safe = "_module".to_string();
    }
    sanitize_keyword(&safe, "rust")
}

/// Pure-Rust code generator emitting zero-copy / low-allocation Rust 2021/2024 data structures.
pub struct RustCodegen {
    options: RustOptions,
    context: RustLanguageContext,
}

/// Emitted Rust identifier for a named type, disambiguated across
/// namespaces for the IR currently being generated.
fn type_ident(q: &QName) -> String {
    lookup_type_name(q, || AsPascalCase(&q.local).to_string())
}

impl RustCodegen {
    pub fn new(options: RustOptions) -> Self {
        let zero_copy = options.zero_copy;
        Self {
            options,
            context: RustLanguageContext::new(zero_copy),
        }
    }

    pub fn generate_module(&self, ir: &SchemaIR) -> String {
        set_type_name_map(build_type_name_map(ir, |local| {
            AsPascalCase(local).to_string()
        }));
        let mut out = String::new();

        if let Some(ref header) = self.options.custom_header {
            let trimmed = header.trim();
            if !trimmed.is_empty() {
                out.push_str(trimmed);
                out.push_str("\n\n");
            }
        }

        out.push_str("// @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)\n");
        out.push_str(
            "#![allow(dead_code, unused_imports, unused_mut, unused_variables, unused_assignments, non_camel_case_types, non_snake_case)]\n\n",
        );

        let types_with_lifetime = if self.options.zero_copy {
            self.compute_types_with_lifetime(ir)
        } else {
            HashSet::new()
        };

        self.emit_imports(&mut out, !types_with_lifetime.is_empty());

        // Sort types topologically (base types before derived types)
        let sorted_types = self.order_types(ir);

        if self.options.emit_codecs {
            self.emit_codec_helpers(&mut out);
        }

        for type_def in &sorted_types {
            if ir.is_external_type(type_def.qname()) {
                continue;
            }
            out.push('\n');
            self.emit_single_type(&mut out, type_def, &types_with_lifetime, ir);
        }

        if self.options.emit_root_aliases {
            self.emit_root_aliases(&mut out, ir, &types_with_lifetime);
        }

        if self.options.pyo3 {
            self.emit_pymodule(&mut out, ir, &sorted_types);
        }

        // A field may use a raw identifier such as `r#type`. Local codec
        // variables have a `var_` prefix, so the raw marker is neither needed
        // nor valid there (`var_r#type` is not a Rust identifier).
        out.replace("var_r#", "var_")
    }

    fn emit_single_type(
        &self,
        out: &mut String,
        type_def: &TypeDef,
        types_with_lifetime: &HashSet<QName>,
        ir: &SchemaIR,
    ) {
        match type_def {
            TypeDef::Simple(s) => self.emit_simple_type(out, s, types_with_lifetime),
            TypeDef::Enum(e) => self.emit_enum(out, e),
            TypeDef::Union(u) => self.emit_union(out, u, types_with_lifetime, ir),
            TypeDef::Struct(s) => self.emit_struct(out, s, types_with_lifetime, ir),
        }
    }

    /// Generate a multi-file or single-file module representation.
    /// When `split_units` is requested, emits topologically sorted, bounded chunk modules
    /// (`chunk_00.rs`, `chunk_01.rs`, ...) and a parent `mod.rs` that re-exports all chunks.
    pub fn generate_files(&self, ir: &SchemaIR, base_name: &str) -> Vec<(String, String)> {
        set_type_name_map(build_type_name_map(ir, |local| {
            AsPascalCase(local).to_string()
        }));

        let should_split = self.options.split_units.unwrap_or(false);
        let chunk_size = self.options.chunk_size.unwrap_or(250);

        let safe_mod = sanitize_rust_module_name(base_name);
        let mut mod_export = if safe_mod == base_name {
            format!("pub mod {base_name};\npub use {base_name}::*;\n")
        } else {
            format!("#[path = \"{base_name}.rs\"]\npub mod {safe_mod};\npub use {safe_mod}::*;\n")
        };

        if !should_split {
            let code = self.generate_module(ir);
            if !code.lines().any(|line| line.starts_with("pub ")) {
                mod_export = mod_export
                    .lines()
                    .filter(|line| !line.starts_with("pub use "))
                    .map(|line| format!("{line}\n"))
                    .collect();
            }
            if base_name == "mod" {
                return vec![("mod.rs".to_string(), code)];
            }
            return vec![
                (format!("{}.rs", base_name), code),
                ("mod.rs".to_string(), mod_export),
            ];
        }

        let plan = ir.partition_topological_chunks(chunk_size);
        if plan.chunks.len() <= 1 {
            let code = self.generate_module(ir);
            if !code.lines().any(|line| line.starts_with("pub ")) {
                mod_export = mod_export
                    .lines()
                    .filter(|line| !line.starts_with("pub use "))
                    .map(|line| format!("{line}\n"))
                    .collect();
            }
            if base_name == "mod" {
                return vec![("mod.rs".to_string(), code)];
            }
            return vec![
                (format!("{}.rs", base_name), code),
                ("mod.rs".to_string(), mod_export),
            ];
        }

        let types_with_lifetime = if self.options.zero_copy {
            self.compute_types_with_lifetime(ir)
        } else {
            HashSet::new()
        };

        let mut files = Vec::new();

        // 1. Chunks
        for (chunk_idx, chunk) in plan.chunks.iter().enumerate() {
            let mut out = String::new();
            if let Some(ref header) = self.options.custom_header {
                let trimmed = header.trim();
                if !trimmed.is_empty() {
                    out.push_str(trimmed);
                    out.push_str("\n\n");
                }
            }
            out.push_str(
                "// @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)\n",
            );
            out.push_str(
                "#![allow(dead_code, unused_imports, unused_mut, unused_variables, unused_assignments, non_camel_case_types, non_snake_case)]\n\n",
            );

            self.emit_imports(&mut out, !types_with_lifetime.is_empty());
            out.push('\n');
            if self.options.emit_codecs {
                out.push_str("use super::{read_element_text, skip_xml_element};\n");
            }
            for prev in &plan.chunks[..chunk_idx] {
                let _ = writeln!(out, "use super::{}::*;", prev.name);
            }

            for qname in &chunk.types {
                if let Some(type_def) = ir.types.get(qname) {
                    out.push('\n');
                    self.emit_single_type(&mut out, type_def, &types_with_lifetime, ir);
                }
            }

            let code = out.replace("var_r#", "var_");
            files.push((format!("{}.rs", chunk.name), code));
        }

        // 2. mod.rs
        let mut mod_out = String::new();
        if let Some(ref header) = self.options.custom_header {
            let trimmed = header.trim();
            if !trimmed.is_empty() {
                mod_out.push_str(trimmed);
                mod_out.push_str("\n\n");
            }
        }
        mod_out
            .push_str("// @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)\n");
        mod_out.push_str(
            "#![allow(dead_code, unused_imports, unused_mut, unused_variables, unused_assignments, non_camel_case_types, non_snake_case)]\n\n",
        );

        self.emit_imports(&mut mod_out, !types_with_lifetime.is_empty());
        mod_out.push('\n');

        for chunk in &plan.chunks {
            let _ = writeln!(mod_out, "pub mod {};", chunk.name);
        }
        mod_out.push('\n');
        for chunk in &plan.chunks {
            let _ = writeln!(mod_out, "pub use {}::*;", chunk.name);
        }

        if self.options.emit_codecs {
            self.emit_codec_helpers(&mut mod_out);
        }

        if self.options.emit_root_aliases {
            self.emit_root_aliases(&mut mod_out, ir, &types_with_lifetime);
        }

        if self.options.pyo3 {
            let sorted_types = self.order_types(ir);
            self.emit_pymodule(&mut mod_out, ir, &sorted_types);
        }

        let mod_code = mod_out.replace("var_r#", "var_");
        files.push(("mod.rs".to_string(), mod_code));

        files
    }

    /// Fixed-point analysis determining which types in SchemaIR require a lifetime parameter `<'a>`.
    fn compute_types_with_lifetime(&self, ir: &SchemaIR) -> HashSet<QName> {
        let mut requires_lifetime = HashSet::new();

        fn primitive_has_lifetime(prim: PrimitiveType) -> bool {
            matches!(
                prim,
                PrimitiveType::Integer
                    | PrimitiveType::PositiveInteger
                    | PrimitiveType::NegativeInteger
                    | PrimitiveType::NonPositiveInteger
                    | PrimitiveType::NonNegativeInteger
                    | PrimitiveType::String
                    | PrimitiveType::NormalizedString
                    | PrimitiveType::Token
                    | PrimitiveType::Language
                    | PrimitiveType::NMTOKEN
                    | PrimitiveType::NMTOKENS
                    | PrimitiveType::Name
                    | PrimitiveType::NCName
                    | PrimitiveType::Id
                    | PrimitiveType::IdRef
                    | PrimitiveType::IdRefs
                    | PrimitiveType::Entity
                    | PrimitiveType::Entities
                    | PrimitiveType::DateTime
                    | PrimitiveType::Date
                    | PrimitiveType::Time
                    | PrimitiveType::Duration
                    | PrimitiveType::GYearMonth
                    | PrimitiveType::GYear
                    | PrimitiveType::GMonthDay
                    | PrimitiveType::GDay
                    | PrimitiveType::GMonth
                    | PrimitiveType::AnyUri
                    | PrimitiveType::QName
                    | PrimitiveType::HexBinary
                    | PrimitiveType::Base64Binary
                    | PrimitiveType::AnyType
                    | PrimitiveType::AnySimpleType
            )
        }

        fn typeref_has_lifetime(type_ref: &TypeRef, current_set: &HashSet<QName>) -> bool {
            match type_ref {
                TypeRef::Primitive(prim) => primitive_has_lifetime(*prim),
                TypeRef::Named(qname) => current_set.contains(qname),
                TypeRef::Boxed(inner) | TypeRef::List(inner) => {
                    typeref_has_lifetime(inner, current_set)
                }
            }
        }

        // Fixed-point iteration
        loop {
            let mut changed = false;

            for (qname, type_def) in &ir.types {
                if requires_lifetime.contains(qname) {
                    continue;
                }

                let needs = match type_def {
                    TypeDef::Simple(s) => typeref_has_lifetime(&s.base_type, &requires_lifetime),
                    TypeDef::Enum(_) => false, // unit variants do not borrow
                    TypeDef::Union(u) => u
                        .branches
                        .iter()
                        .any(|b| typeref_has_lifetime(&b.type_ref, &requires_lifetime)),
                    TypeDef::Struct(s) => flatten_fields(s, ir)
                        .into_iter()
                        .any(|f| typeref_has_lifetime(&f.type_ref, &requires_lifetime)),
                };

                if needs {
                    requires_lifetime.insert(qname.clone());
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        requires_lifetime
    }

    fn emit_imports(&self, out: &mut String, has_borrowed_types: bool) {
        if self.options.pyo3 {
            out.push_str("use pyo3::prelude::*;\n");
            out.push_str("use pyo3::types::PyModule;\n");
            out.push_str("use serde::{Deserialize, Serialize};\n");
            out.push_str("use std::str::FromStr;\n");
            out.push_str("use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};\n");
            out.push_str("use quick_xml::{Reader, Writer};\n\n");
            self.emit_pyo3_error_type(out);
            return;
        }
        if (self.options.zero_copy && has_borrowed_types) || self.options.emit_codecs {
            out.push_str("use std::borrow::Cow;\n");
        }
        if self.options.derive_serde {
            out.push_str("use serde::{Deserialize, Serialize};\n");
        }
        if self.options.emit_codecs {
            out.push_str("use std::str::FromStr;\n");
            out.push_str("use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};\n");
            out.push_str("use quick_xml::{Reader, Writer};\n");
            out.push_str("use polyxml::{PolyXmlError, Result};\n");
        }
    }

    fn order_types<'a>(&self, ir: &'a SchemaIR) -> Vec<&'a TypeDef> {
        let mut simples = Vec::new();
        let mut enums = Vec::new();
        let mut unions = Vec::new();
        let mut structs: Vec<&'a StructDef> = Vec::new();

        for type_def in ir.types.values() {
            match type_def {
                TypeDef::Simple(_) => simples.push(type_def),
                TypeDef::Enum(_) => enums.push(type_def),
                TypeDef::Union(_) => unions.push(type_def),
                TypeDef::Struct(s) => structs.push(s),
            }
        }
        // Topologically sort structs based on inheritance
        let mut ordered_structs: Vec<&'a TypeDef> = Vec::new();
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();

        fn visit<'a>(
            s: &'a StructDef,
            ir: &'a SchemaIR,
            visiting: &mut HashSet<QName>,
            visited: &mut HashSet<QName>,
            ordered: &mut Vec<&'a TypeDef>,
        ) {
            if visited.contains(&s.qname) || visiting.contains(&s.qname) {
                return;
            }
            visiting.insert(s.qname.clone());
            if let Some(ref base_qname) = s.base_type {
                if base_qname != &s.qname {
                    if let Some(TypeDef::Struct(parent)) = ir.find_type(base_qname) {
                        visit(parent, ir, visiting, visited, ordered);
                    }
                }
            }
            visiting.remove(&s.qname);
            visited.insert(s.qname.clone());
            if let Some(td) = ir.find_type(&s.qname) {
                ordered.push(td);
            }
        }

        for s in &structs {
            visit(s, ir, &mut visiting, &mut visited, &mut ordered_structs);
        }

        let mut result = Vec::new();
        result.extend(simples);
        result.extend(enums);
        result.extend(unions);
        result.extend(ordered_structs);
        result
    }

    fn emit_simple_type(
        &self,
        out: &mut String,
        s: &SimpleTypeDef,
        types_with_lifetime: &HashSet<QName>,
    ) {
        let type_name = type_ident(&s.qname);
        let needs_lifetime = types_with_lifetime.contains(&s.qname);

        if let Some(ref doc) = s.documentation {
            write_documentation_lines(out, "/// ", doc);
        }

        let base_type = self.format_rust_type_ref(&s.base_type, types_with_lifetime);
        if needs_lifetime {
            let _ = writeln!(out, "pub type {}<'a> = {};", type_name, base_type);
        } else {
            let _ = writeln!(out, "pub type {} = {};", type_name, base_type);
        }

        if !s.facets.patterns.is_empty() {
            let _ = writeln!(out, "pub fn validate_{}_patterns(value: &str) -> std::result::Result<(), &'static str> {{", type_name);
            for (i, pattern) in s.facets.patterns.iter().enumerate() {
                let _ = writeln!(out, "    static PATTERN_{}: std::sync::OnceLock<std::result::Result<regex::Regex, regex::Error>> = std::sync::OnceLock::new();", i);
                let full_pattern = format!(r"\A(?:{pattern})\z");
                let _ = writeln!(out, "    let pattern = PATTERN_{}.get_or_init(|| regex::Regex::new({:?})).as_ref().map_err(|_| \"unsupported pattern syntax\")?;", i, full_pattern);
                out.push_str("    if !pattern.is_match(value) { return Err(\"pattern constraint failed\"); }\n");
            }
            out.push_str("    Ok(())\n}\n");
        }
    }

    fn emit_enum(&self, out: &mut String, e: &EnumDef) {
        let enum_name = type_ident(&e.qname);

        if let Some(ref doc) = e.documentation {
            write_documentation_lines(out, "/// ", doc);
        }

        let mut derives = vec!["Debug", "Clone", "Copy", "PartialEq", "Eq", "Hash"];
        if self.options.derive_default {
            derives.push("Default");
        }
        if self.options.derive_serde {
            derives.push("Serialize");
            derives.push("Deserialize");
        }

        let _ = writeln!(out, "#[derive({})]", derives.join(", "));
        if self.options.emit_rkyv {
            let _ = writeln!(
                out,
                "#[cfg_attr(feature = \"rkyv\", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]"
            );
            let _ = writeln!(out, "#[cfg_attr(feature = \"rkyv\", rkyv(check_bytes))]");
        }
        if self.options.pyo3 {
            let _ = writeln!(out, "#[pyclass(eq, eq_int, from_py_object)]");
        }
        let _ = writeln!(out, "pub enum {} {{", enum_name);

        let mut seen_variants = HashSet::new();
        let mut variant_map = Vec::new();

        for (idx, variant) in e.variants.iter().enumerate() {
            let mut var_id = to_rust_variant_identifier(&variant.name);
            let mut counter = 1;
            while seen_variants.contains(&var_id) {
                counter += 1;
                var_id = format!("{}{}", to_rust_variant_identifier(&variant.name), counter);
            }
            seen_variants.insert(var_id.clone());

            if let Some(ref doc) = variant.documentation {
                write_documentation_lines(out, "    /// ", doc);
            }
            if self.options.derive_serde {
                let _ = writeln!(out, "    #[serde(rename = \"{}\")]", variant.value);
            }
            if self.options.derive_default && idx == 0 {
                let _ = writeln!(out, "    #[default]");
            }
            let _ = writeln!(out, "    {},", var_id);
            variant_map.push((var_id, variant.value.clone()));
        }

        out.push_str("}\n\n");

        // Implement helper methods: as_str(), FromStr, Display
        let _ = writeln!(out, "impl {} {{", enum_name);
        out.push_str("    pub fn as_str(&self) -> &'static str {\n");
        out.push_str("        match self {\n");
        for (var_id, val) in &variant_map {
            let _ = writeln!(out, "            Self::{} => \"{}\",", var_id, val);
        }
        if variant_map.is_empty() {
            out.push_str("            _ => \"\",\n");
        }
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");

        // std::str::FromStr
        let _ = writeln!(out, "impl std::str::FromStr for {} {{", enum_name);
        out.push_str("    type Err = String;\n\n");
        out.push_str("    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {\n");
        out.push_str("        match s {\n");
        for (var_id, val) in &variant_map {
            let _ = writeln!(out, "            \"{}\" => Ok(Self::{}),", val, var_id);
        }
        let _ = writeln!(
            out,
            "            _ => Err(format!(\"Unknown {} variant: {{}}\", s)),",
            enum_name
        );
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n\n");

        // std::fmt::Display
        let _ = writeln!(out, "impl std::fmt::Display for {} {{", enum_name);
        out.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
        out.push_str("        write!(f, \"{}\", self.as_str())\n");
        out.push_str("    }\n");
        out.push_str("}\n");

        if self.options.pyo3 {
            let _ = writeln!(out, "\n#[pymethods]\nimpl {} {{", enum_name);
            out.push_str("    fn __repr__(&self) -> String {\n");
            out.push_str("        format!(\"{:?}\", self)\n");
            out.push_str("    }\n");
            out.push_str("    fn __str__(&self) -> &'static str {\n");
            out.push_str("        self.as_str()\n");
            out.push_str("    }\n");
            out.push_str("}\n");
        }
    }

    fn emit_union(
        &self,
        out: &mut String,
        u: &UnionDef,
        types_with_lifetime: &HashSet<QName>,
        ir: &SchemaIR,
    ) {
        let union_name = type_ident(&u.qname);
        let needs_lifetime = types_with_lifetime.contains(&u.qname);

        if let Some(ref doc) = u.documentation {
            write_documentation_lines(out, "/// ", doc);
        }

        let mut derives = vec!["Debug", "Clone", "PartialEq"];
        if self.options.derive_serde {
            derives.push("Serialize");
            derives.push("Deserialize");
        }

        let _ = writeln!(out, "#[derive({})]", derives.join(", "));
        if u.is_lexical() && self.options.derive_serde {
            out.push_str("#[serde(untagged)]\n");
        }
        if self.options.emit_rkyv {
            let _ = writeln!(
                out,
                "#[cfg_attr(feature = \"rkyv\", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]"
            );
            let _ = writeln!(out, "#[cfg_attr(feature = \"rkyv\", rkyv(check_bytes))]");
        }
        let type_signature = if needs_lifetime {
            format!("{}<'a>", union_name)
        } else {
            union_name.clone()
        };

        let _ = writeln!(out, "pub enum {} {{", type_signature);

        let mut seen_variants = HashSet::new();
        for branch in &u.branches {
            let mut var_id = to_rust_variant_identifier(&branch.variant_name);
            let mut counter = 1;
            while seen_variants.contains(&var_id) {
                counter += 1;
                var_id = format!(
                    "{}{}",
                    to_rust_variant_identifier(&branch.variant_name),
                    counter
                );
            }
            seen_variants.insert(var_id.clone());

            let branch_type = self.format_rust_type_ref(&branch.type_ref, types_with_lifetime);

            if let Some(ref doc) = branch.documentation {
                write_documentation_lines(out, "    /// ", doc);
            }
            if self.options.derive_serde && !u.is_lexical() {
                let _ = writeln!(out, "    #[serde(rename = \"{}\")]", branch.xml_name);
            }
            if self.options.emit_polyxml_attrs && !u.is_lexical() {
                let _ = writeln!(out, "    #[polyxml(element = \"{}\")]", branch.xml_name);
            }
            let _ = writeln!(out, "    {}({}),", var_id, branch_type);
        }

        out.push_str("}\n");

        if self.options.derive_default {
            if let Some(first) = u.branches.first() {
                let variant = to_rust_variant_identifier(&first.variant_name);
                let impl_header = if needs_lifetime {
                    format!("impl<'a> Default for {}<'a>", union_name)
                } else {
                    format!("impl Default for {}", union_name)
                };
                let _ = writeln!(
                    out,
                    "{} {{ fn default() -> Self {{ Self::{}(Default::default()) }} }}",
                    impl_header, variant
                );
            }
        }

        if self.options.emit_codecs {
            self.emit_union_codecs(out, u, types_with_lifetime, ir);
        }
    }

    fn emit_struct(
        &self,
        out: &mut String,
        s: &StructDef,
        types_with_lifetime: &HashSet<QName>,
        ir: &SchemaIR,
    ) {
        let struct_name = type_ident(&s.qname);
        let needs_lifetime = types_with_lifetime.contains(&s.qname);

        if let Some(ref doc) = s.documentation {
            write_documentation_lines(out, "/// ", doc);
        }

        let mut derives = vec!["Debug", "Clone", "PartialEq"];
        if self.options.derive_default {
            derives.push("Default");
        }
        if self.options.derive_serde {
            derives.push("Serialize");
            derives.push("Deserialize");
        }

        let _ = writeln!(out, "#[derive({})]", derives.join(", "));
        if self.options.emit_rkyv {
            let _ = writeln!(
                out,
                "#[cfg_attr(feature = \"rkyv\", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]"
            );
            let _ = writeln!(out, "#[cfg_attr(feature = \"rkyv\", rkyv(check_bytes))]");
        }

        let has_boxed = flatten_fields(s, ir)
            .into_iter()
            .any(|f| f.is_cycle_cut || f.type_ref.is_boxed());

        if self.options.pyo3 {
            if has_boxed {
                let _ = writeln!(out, "#[pyclass(from_py_object)]");
            } else {
                let _ = writeln!(out, "#[pyclass(get_all, set_all, from_py_object)]");
            }
        }

        let struct_decl = if needs_lifetime {
            format!("pub struct {}<'a>", struct_name)
        } else {
            format!("pub struct {}", struct_name)
        };

        let _ = writeln!(out, "{} {{", struct_decl);

        let mut seen_fields = HashSet::new();
        let mut seen_json_names = HashSet::new();
        let fields = flatten_fields(s, ir);
        let wire_names: HashSet<_> = fields.iter().map(|field| field.xml_name.as_str()).collect();
        for field in fields {
            let rust_name = self.unique_rust_field_name(&field.name, &mut seen_fields);
            let mut json_name = field.xml_name.clone();
            if !seen_json_names.insert(json_name.clone()) {
                let base = rust_name.strip_prefix("r#").unwrap_or(&rust_name);
                json_name = base.to_string();
                let mut suffix = 2;
                while wire_names.contains(json_name.as_str())
                    || !seen_json_names.insert(json_name.clone())
                {
                    json_name = format!("{base}_{suffix}");
                    suffix += 1;
                }
            }
            self.emit_struct_field(
                out,
                field,
                &rust_name,
                &json_name,
                types_with_lifetime,
                has_boxed,
            );
        }

        out.push_str("}\n");

        if self.options.emit_codecs {
            self.emit_struct_codecs(out, s, types_with_lifetime, ir);
        }

        if self.options.pyo3 {
            self.emit_struct_pymethods(out, s, has_boxed, ir);
        }
    }

    fn unique_rust_field_name(&self, name: &str, seen: &mut HashSet<String>) -> String {
        let mut candidate = to_rust_field_identifier(name);
        let mut counter = 1;
        while seen.contains(&candidate) {
            counter += 1;
            candidate = format!("{}_{}", to_rust_field_identifier(name), counter);
        }
        seen.insert(candidate.clone());
        candidate
    }

    fn emit_struct_field(
        &self,
        out: &mut String,
        field: &FieldDef,
        rust_name: &str,
        json_name: &str,
        types_with_lifetime: &HashSet<QName>,
        has_boxed: bool,
    ) {
        if let Some(ref doc) = field.documentation {
            write_documentation_lines(out, "    /// ", doc);
        }

        let is_list = field.cardinality.is_list() || field.type_ref.is_list();
        let is_optional = field.cardinality.is_optional() || field.nillable;

        // PolyXML attribute
        if self.options.emit_polyxml_attrs {
            let kind_attr = match field.kind {
                FieldKind::Element => format!("element = \"{}\"", field.xml_name),
                FieldKind::Attribute => format!("attribute = \"{}\"", field.xml_name),
                FieldKind::Text => "text".to_string(),
                FieldKind::Any | FieldKind::AnyAttribute => "wildcard".to_string(),
            };
            let _ = writeln!(out, "    #[polyxml({})]", kind_attr);
        }

        // Serde attribute
        if self.options.derive_serde {
            let mut serde_parts = Vec::new();
            if json_name != rust_name.strip_prefix("r#").unwrap_or(rust_name) {
                serde_parts.push(format!("rename = \"{}\"", json_name));
            }
            if is_optional {
                serde_parts.push("default".to_string());
                serde_parts.push("skip_serializing_if = \"Option::is_none\"".to_string());
            } else if is_list {
                serde_parts.push("default".to_string());
                serde_parts.push("skip_serializing_if = \"Vec::is_empty\"".to_string());
            }

            if !serde_parts.is_empty() {
                let _ = writeln!(out, "    #[serde({})]", serde_parts.join(", "));
            }
        }

        // Compute Rust type
        let inner_ref = match &field.type_ref {
            TypeRef::List(inner) => inner.as_ref(),
            other => other,
        };

        let is_boxed = field.is_cycle_cut || field.type_ref.is_boxed();
        if self.options.pyo3 && has_boxed && !is_boxed {
            let _ = writeln!(out, "    #[pyo3(get, set)]");
        }
        let formatted_inner = self.format_rust_type_ref(inner_ref, types_with_lifetime);

        let final_type = if is_list {
            format!("Vec<{}>", formatted_inner)
        } else if is_optional {
            if is_boxed {
                format!("Option<Box<{}>>", formatted_inner)
            } else {
                format!("Option<{}>", formatted_inner)
            }
        } else if is_boxed {
            format!("Box<{}>", formatted_inner)
        } else {
            formatted_inner
        };

        let _ = writeln!(out, "    pub {}: {},", rust_name, final_type);
    }

    fn format_rust_type_ref(
        &self,
        type_ref: &TypeRef,
        types_with_lifetime: &HashSet<QName>,
    ) -> String {
        match type_ref {
            TypeRef::Primitive(prim) => self.context.map_primitive(*prim).to_string(),
            TypeRef::Named(qname) => {
                let name = type_ident(qname);
                if self.options.zero_copy && types_with_lifetime.contains(qname) {
                    format!("{}<'a>", name)
                } else {
                    name
                }
            }
            TypeRef::Boxed(inner) => {
                format!(
                    "Box<{}>",
                    self.format_rust_type_ref(inner, types_with_lifetime)
                )
            }
            TypeRef::List(inner) => {
                format!(
                    "Vec<{}>",
                    self.format_rust_type_ref(inner, types_with_lifetime)
                )
            }
        }
    }

    fn emit_root_aliases(
        &self,
        out: &mut String,
        ir: &SchemaIR,
        types_with_lifetime: &HashSet<QName>,
    ) {
        let mut declared_names = BTreeSet::new();
        for td in ir.types.values() {
            declared_names.insert(type_ident(td.qname()));
        }

        for element in ir.elements.values() {
            let el_name = AsPascalCase(&element.qname.local).to_string();
            if !declared_names.contains(&el_name) {
                let target_type = self.format_rust_type_ref(&element.type_ref, types_with_lifetime);
                let target_clean = target_type
                    .split('<')
                    .next()
                    .unwrap_or(&target_type)
                    .to_string();

                if el_name != target_clean {
                    if target_type.contains("'a") {
                        let _ = writeln!(out, "\npub type {}<'a> = {};", el_name, target_type);
                    } else {
                        let _ = writeln!(out, "\npub type {} = {};", el_name, target_type);
                    }
                    declared_names.insert(el_name);
                }
            }
        }
    }

    fn emit_codec_helpers(&self, out: &mut String) {
        out.push_str("\n#[allow(dead_code)]\n");
        out.push_str(
            "pub(crate) fn skip_xml_element(reader: &mut Reader<&[u8]>) -> Result<()> {\n",
        );
        out.push_str("    let mut depth = 1;\n");
        out.push_str("    loop {\n");
        out.push_str("        match reader.read_event()? {\n");
        out.push_str("            Event::Start(_) => depth += 1,\n");
        out.push_str("            Event::End(_) => {\n");
        out.push_str("                depth -= 1;\n");
        out.push_str("                if depth == 0 {\n");
        out.push_str("                    break;\n");
        out.push_str("                }\n");
        out.push_str("            }\n");
        out.push_str("            Event::Eof => break,\n");
        out.push_str("            _ => {}\n");
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("    Ok(())\n");
        out.push_str("}\n\n");

        if self.options.zero_copy {
            out.push_str("#[allow(dead_code)]\n");
            out.push_str("pub(crate) fn read_element_text<'a>(reader: &mut Reader<&'a [u8]>, tag_name: &str) -> Result<Cow<'a, str>> {\n");
            out.push_str("    let mut text = Cow::Borrowed(\"\");\n");
            out.push_str("    loop {\n");
            out.push_str("        match reader.read_event()? {\n");
            out.push_str("            Event::Text(t) => {\n");
            out.push_str("                let raw = match t.into_inner() {\n");
            out.push_str(
                "                    Cow::Borrowed(b) => match quick_xml::escape::unescape(b)? {\n",
            );
            out.push_str("                        Cow::Borrowed(s) => Cow::Borrowed(s),\n");
            out.push_str("                        Cow::Owned(s) => Cow::Owned(s),\n");
            out.push_str("                    },\n");
            out.push_str("                    Cow::Owned(s) => Cow::Owned(quick_xml::escape::unescape(&s)?.into_owned()),\n");
            out.push_str("                };\n");
            out.push_str("                if text.is_empty() {\n");
            out.push_str("                    text = raw;\n");
            out.push_str("                } else {\n");
            out.push_str("                    text.to_mut().push_str(&raw);\n");
            out.push_str("                }\n");
            out.push_str("            }\n");
            out.push_str("            Event::CData(c) => {\n");
            out.push_str("                text.to_mut().push_str(c.as_ref());\n");
            out.push_str("            }\n");
            out.push_str("            Event::GeneralRef(r) => {\n");
            out.push_str("                if r.is_char_ref() {\n");
            out.push_str("                    if let Some(ch) = r.resolve_char_ref()? {\n");
            out.push_str("                        text.to_mut().push(ch);\n");
            out.push_str("                    }\n");
            out.push_str(
                "                } else if let Some(val) = quick_xml::escape::resolve_xml_entity(r.as_ref()) {\n",
            );
            out.push_str("                    text.to_mut().push_str(val);\n");
            out.push_str("                } else {\n");
            out.push_str("                    text.to_mut().push_str(r.as_ref());\n");
            out.push_str("                }\n");
            out.push_str("            }\n");
            out.push_str(
                "            Event::End(e) if e.local_name().as_ref() == tag_name => break,\n",
            );
            out.push_str("            Event::Eof => break,\n");
            out.push_str("            _ => {}\n");
            out.push_str("        }\n");
            out.push_str("    }\n");
            out.push_str("    Ok(text)\n");
            out.push_str("}\n");
        } else {
            out.push_str("#[allow(dead_code)]\n");
            out.push_str("pub(crate) fn read_element_text(reader: &mut Reader<&[u8]>, tag_name: &str) -> Result<String> {\n");
            out.push_str("    let mut text = String::new();\n");
            out.push_str("    loop {\n");
            out.push_str("        match reader.read_event()? {\n");
            out.push_str("            Event::Text(t) => {\n");
            out.push_str(
                "                let unescaped = quick_xml::escape::unescape(t.as_ref())?;\n",
            );
            out.push_str("                text.push_str(&unescaped);\n");
            out.push_str("            }\n");
            out.push_str("            Event::CData(c) => {\n");
            out.push_str("                text.push_str(c.as_ref());\n");
            out.push_str("            }\n");
            out.push_str("            Event::GeneralRef(r) => {\n");
            out.push_str("                if r.is_char_ref() {\n");
            out.push_str("                    if let Some(ch) = r.resolve_char_ref()? {\n");
            out.push_str("                        text.push(ch);\n");
            out.push_str("                    }\n");
            out.push_str(
                "                } else if let Some(val) = quick_xml::escape::resolve_xml_entity(r.as_ref()) {\n",
            );
            out.push_str("                    text.push_str(val);\n");
            out.push_str("                } else {\n");
            out.push_str("                    text.push_str(r.as_ref());\n");
            out.push_str("                }\n");
            out.push_str("            }\n");
            out.push_str(
                "            Event::End(e) if e.local_name().as_ref() == tag_name => break,\n",
            );
            out.push_str("            Event::Eof => break,\n");
            out.push_str("            _ => {}\n");
            out.push_str("        }\n");
            out.push_str("    }\n");
            out.push_str("    Ok(text)\n");
            out.push_str("}\n");
        }
    }

    fn unwrap_type_ref(mut ty: &TypeRef) -> &TypeRef {
        while let TypeRef::Boxed(inner) | TypeRef::List(inner) = ty {
            ty = inner;
        }
        ty
    }

    fn field_is_string(&self, type_ref: &TypeRef, ir: &SchemaIR) -> bool {
        if super::unbounded_integer(type_ref, ir).is_some() {
            return true;
        }
        let ty = Self::unwrap_type_ref(type_ref);
        let base = primitive_base(ty, ir);
        match base {
            TypeRef::Primitive(prim) => matches!(
                prim,
                PrimitiveType::String
                    | PrimitiveType::NormalizedString
                    | PrimitiveType::Token
                    | PrimitiveType::Language
                    | PrimitiveType::NMTOKEN
                    | PrimitiveType::NMTOKENS
                    | PrimitiveType::Name
                    | PrimitiveType::NCName
                    | PrimitiveType::Id
                    | PrimitiveType::IdRef
                    | PrimitiveType::IdRefs
                    | PrimitiveType::Entity
                    | PrimitiveType::Entities
                    | PrimitiveType::AnyUri
                    | PrimitiveType::QName
                    | PrimitiveType::DateTime
                    | PrimitiveType::Date
                    | PrimitiveType::Time
                    | PrimitiveType::Duration
                    | PrimitiveType::GYear
                    | PrimitiveType::GYearMonth
                    | PrimitiveType::GMonth
                    | PrimitiveType::GMonthDay
                    | PrimitiveType::GDay
                    | PrimitiveType::Base64Binary
                    | PrimitiveType::HexBinary
                    | PrimitiveType::AnySimpleType
            ),
            _ => false,
        }
    }

    fn field_numeric_type(&self, type_ref: &TypeRef, ir: &SchemaIR) -> Option<&'static str> {
        let ty = Self::unwrap_type_ref(type_ref);
        let base = primitive_base(ty, ir);
        match base {
            TypeRef::Primitive(prim) => match prim {
                PrimitiveType::Int => Some("i32"),
                PrimitiveType::Long => Some("i64"),
                PrimitiveType::Short => Some("i16"),
                PrimitiveType::Byte => Some("i8"),
                PrimitiveType::UnsignedLong => Some("u64"),
                PrimitiveType::UnsignedInt => Some("u32"),
                PrimitiveType::UnsignedShort => Some("u16"),
                PrimitiveType::UnsignedByte => Some("u8"),
                PrimitiveType::Decimal | PrimitiveType::Double => Some("f64"),
                PrimitiveType::Float => Some("f32"),
                _ => None,
            },
            _ => None,
        }
    }

    fn field_is_bool(&self, type_ref: &TypeRef, ir: &SchemaIR) -> bool {
        let ty = Self::unwrap_type_ref(type_ref);
        let base = primitive_base(ty, ir);
        matches!(base, TypeRef::Primitive(PrimitiveType::Boolean))
    }

    fn field_is_enum(&self, type_ref: &TypeRef, ir: &SchemaIR) -> Option<String> {
        match type_ref {
            TypeRef::Named(qname) => {
                if let Some(TypeDef::Enum(_)) = ir.types.get(qname) {
                    Some(type_ident(qname))
                } else {
                    None
                }
            }
            TypeRef::Boxed(inner) | TypeRef::List(inner) => self.field_is_enum(inner, ir),
            _ => None,
        }
    }

    fn resolve_union_def<'a>(
        &self,
        type_ref: &'a TypeRef,
        ir: &'a SchemaIR,
    ) -> Option<&'a UnionDef> {
        match type_ref {
            TypeRef::Named(qname) => {
                if let Some(TypeDef::Union(u)) = ir.types.get(qname) {
                    (!u.is_lexical()).then_some(u)
                } else {
                    None
                }
            }
            TypeRef::Boxed(inner) | TypeRef::List(inner) => self.resolve_union_def(inner, ir),
            _ => None,
        }
    }

    fn emit_struct_codecs(
        &self,
        out: &mut String,
        s: &StructDef,
        types_with_lifetime: &HashSet<QName>,
        ir: &SchemaIR,
    ) {
        let has_patterns = flatten_fields(s, ir)
            .into_iter()
            .any(|f| super::patterned_simple(&f.type_ref, ir).is_some() || matches!(primitive_base(Self::unwrap_type_ref(&f.type_ref), ir), TypeRef::Primitive(p) if p.is_unbounded_integer()));
        let struct_name = type_ident(&s.qname);
        let needs_lifetime = types_with_lifetime.contains(&s.qname);

        let impl_header = if needs_lifetime {
            format!("impl<'a> {}<'a>", struct_name)
        } else {
            format!("impl {}", struct_name)
        };

        // --feature phf: emit a module-level perfect-hash dispatch table (enum
        // + static, built with phf_codegen) ahead of the impl and route both
        // child-event match sites through it. Fully gated so default output
        // stays byte-identical.
        let phf_enum_name = format!("__{}ElementId", struct_name);
        let phf_static_name = format!("__{}_ELEMENT_DISPATCH", struct_name.to_uppercase());
        let mut phf_arms: Vec<(String, String, Vec<String>)> = Vec::new();
        if self.options.phf {
            let mut seen_fields = HashSet::new();
            let mut seen_variants = HashSet::new();
            for field in flatten_fields(s, ir) {
                // Uniquify over EVERY field (attrs/text included) so names stay
                // in lockstep with field_metas and struct-field emission; a
                // non-element sharing a name with a later element must still
                // consume its slot or dispatch lookups miss and fall back to a
                // bare-string arm inside the Option match (won't compile).
                let rust_name = self.unique_rust_field_name(&field.name, &mut seen_fields);
                if field.kind != FieldKind::Element {
                    continue;
                }
                let tags: Vec<String> = match self
                    .resolve_union_def(&field.type_ref, ir)
                    .filter(|_| field.xml_name.is_empty())
                {
                    Some(u) => u
                        .branches
                        .iter()
                        .filter(|b| b.xml_name != "#text")
                        .map(|b| b.xml_name.clone())
                        .collect(),
                    None => vec![field.xml_name.clone()],
                };
                let base_variant = to_rust_variant_identifier(rust_name.trim_start_matches("r#"));
                let mut variant = base_variant.clone();
                let mut n = 2;
                while !seen_variants.insert(variant.clone()) {
                    variant = format!("{}{}", base_variant, n);
                    n += 1;
                }
                phf_arms.push((rust_name, variant, tags));
            }
            if !phf_arms.is_empty() {
                out.push('\n');
                let _ = writeln!(
                    out,
                    "/// Perfect-hash element dispatch id for `{struct_name}` (`--feature phf`)."
                );
                out.push_str("#[derive(Clone, Copy)]\n");
                let _ = writeln!(out, "enum {} {{", phf_enum_name);
                for (_, variant, _) in &phf_arms {
                    let _ = writeln!(out, "    {},", variant);
                }
                out.push_str("}\n\n");
                let mut map = phf_codegen::Map::new();
                for (_, variant, tags) in &phf_arms {
                    for tag in tags {
                        map.entry(tag.as_str(), format!("{}::{}", phf_enum_name, variant));
                    }
                }
                let _ = writeln!(
                    out,
                    "static {}: ::phf::Map<&'static str, {}> = {};",
                    phf_static_name,
                    phf_enum_name,
                    map.build()
                );
                out.push('\n');
            }
        }
        let phf_variant_of: HashMap<&str, &str> = phf_arms
            .iter()
            .map(|(rust_name, variant, _)| (rust_name.as_str(), variant.as_str()))
            .collect();

        out.push('\n');
        let _ = writeln!(out, "{} {{", impl_header);

        let from_xml_sig = if needs_lifetime {
            "pub fn from_xml(xml: &'a str) -> Result<Self>"
        } else {
            "pub fn from_xml(xml: &str) -> Result<Self>"
        };
        let _ = writeln!(out, "    {}", from_xml_sig);
        out.push_str("    {\n");
        out.push_str("        let mut reader = Reader::from_str(xml);\n");
        out.push_str("        loop {\n");
        out.push_str("            match reader.read_event()? {\n");
        out.push_str(
            "                Event::Start(e) => return Self::decode_xml(&mut reader, &e),\n",
        );
        out.push_str("                Event::Empty(e) => return Self::decode_xml_empty(&e),\n");
        out.push_str("                Event::Eof => break,\n");
        out.push_str("                _ => {}\n");
        out.push_str("            }\n");
        out.push_str("        }\n");
        let _ = writeln!(
            out,
            "        Err(PolyXmlError::SchemaError(\"Unexpected EOF while parsing {}\".into()))",
            struct_name
        );
        out.push_str("    }\n\n");

        let from_bytes_sig = if needs_lifetime {
            "pub fn from_xml_bytes(xml_bytes: &'a [u8]) -> Result<Self>"
        } else {
            "pub fn from_xml_bytes(xml_bytes: &[u8]) -> Result<Self>"
        };
        let _ = writeln!(out, "    {}", from_bytes_sig);
        out.push_str("    {\n");
        out.push_str("        let s = std::str::from_utf8(xml_bytes)?;\n");
        out.push_str("        Self::from_xml(s)\n");
        out.push_str("    }\n\n");

        let decode_sig = if needs_lifetime {
            "pub fn decode_xml(reader: &mut Reader<&'a [u8]>, start: &BytesStart<'_>) -> Result<Self>"
        } else {
            "pub fn decode_xml(reader: &mut Reader<&'_ [u8]>, start: &BytesStart<'_>) -> Result<Self>"
        };
        let _ = writeln!(out, "    {}", decode_sig);
        out.push_str("    {\n");

        let mut seen_fields = HashSet::new();
        struct FieldMeta {
            field: FieldDef,
            rust_name: String,
        }
        let mut field_metas = Vec::new();
        for field in flatten_fields(s, ir) {
            let rust_name = self.unique_rust_field_name(&field.name, &mut seen_fields);
            field_metas.push(FieldMeta {
                field: field.clone(),
                rust_name,
            });
        }

        for meta in &field_metas {
            let is_list = meta.field.cardinality.is_list() || meta.field.type_ref.is_list();
            if is_list {
                let _ = writeln!(out, "        let mut var_{} = Vec::new();", meta.rust_name);
            } else {
                let _ = writeln!(out, "        let mut var_{} = None;", meta.rust_name);
            }
        }

        let text_meta = field_metas.iter().find(|m| m.field.kind == FieldKind::Text);

        let attr_fields: Vec<_> = field_metas
            .iter()
            .filter(|m| m.field.kind == FieldKind::Attribute)
            .collect();
        if !attr_fields.is_empty() {
            out.push_str("\n        for attr in start.attributes() {\n");
            out.push_str("            let attr = attr?;\n");
            out.push_str("            match attr.key.local_name().as_ref() {\n");
            for meta in &attr_fields {
                let _ = writeln!(out, "                \"{}\" => {{", meta.field.xml_name);
                self.emit_attr_parse(out, &meta.field, &meta.rust_name, ir);
                out.push_str("                }\n");
            }
            out.push_str("                _ => {}\n");
            out.push_str("            }\n");
            out.push_str("        }\n");
        }

        let elem_fields: Vec<_> = field_metas
            .iter()
            .filter(|m| m.field.kind == FieldKind::Element)
            .collect();

        if !elem_fields.is_empty() {
            out.push_str("\n        loop {\n");
            out.push_str("            match reader.read_event()? {\n");
            if phf_variant_of.is_empty() {
                out.push_str(
                    "                Event::Start(e) => match e.local_name().as_ref() {\n",
                );
            } else {
                let _ = writeln!(
                    out,
                    "                Event::Start(e) => match {}.get(e.local_name().as_ref()) {{",
                    phf_static_name
                );
            }

            for meta in &elem_fields {
                if let Some(union_def) = self
                    .resolve_union_def(&meta.field.type_ref, ir)
                    .filter(|_| meta.field.xml_name.is_empty())
                {
                    let branch_tags: Vec<_> = union_def
                        .branches
                        .iter()
                        .filter(|b| b.xml_name != "#text")
                        .map(|b| format!("\"{}\"", b.xml_name))
                        .collect();
                    if branch_tags.is_empty() {
                        continue;
                    }
                    if let Some(variant) = phf_variant_of.get(meta.rust_name.as_str()) {
                        let _ = writeln!(
                            out,
                            "                    Some(&{}::{}) => {{",
                            phf_enum_name, variant
                        );
                    } else {
                        let _ =
                            writeln!(out, "                    {} => {{", branch_tags.join(" | "));
                    }
                    let union_name = type_ident(&union_def.qname);
                    let _ = writeln!(
                        out,
                        "                        let val = {}::decode_xml(reader, &e)?;",
                        union_name
                    );
                    let is_list = meta.field.cardinality.is_list() || meta.field.type_ref.is_list();
                    if is_list {
                        let _ = writeln!(
                            out,
                            "                        var_{}.push(val);",
                            meta.rust_name
                        );
                    } else {
                        let _ = writeln!(
                            out,
                            "                        var_{} = Some(val);",
                            meta.rust_name
                        );
                    }
                    out.push_str("                    }\n");
                } else {
                    if let Some(variant) = phf_variant_of.get(meta.rust_name.as_str()) {
                        let _ = writeln!(
                            out,
                            "                    Some(&{}::{}) => {{",
                            phf_enum_name, variant
                        );
                    } else {
                        let _ =
                            writeln!(out, "                    \"{}\" => {{", meta.field.xml_name);
                    }
                    self.emit_element_parse(
                        out,
                        &meta.field,
                        &meta.rust_name,
                        ir,
                        types_with_lifetime,
                    );
                    out.push_str("                    }\n");
                }
            }

            out.push_str("                    _ => {\n");
            out.push_str("                        skip_xml_element(reader)?;\n");
            out.push_str("                    }\n");
            out.push_str("                },\n");

            if phf_variant_of.is_empty() {
                out.push_str(
                    "                Event::Empty(e) => match e.local_name().as_ref() {\n",
                );
            } else {
                let _ = writeln!(
                    out,
                    "                Event::Empty(e) => match {}.get(e.local_name().as_ref()) {{",
                    phf_static_name
                );
            }
            for meta in &elem_fields {
                if let Some(union_def) = self
                    .resolve_union_def(&meta.field.type_ref, ir)
                    .filter(|_| meta.field.xml_name.is_empty())
                {
                    let branch_tags: Vec<_> = union_def
                        .branches
                        .iter()
                        .filter(|b| b.xml_name != "#text")
                        .map(|b| format!("\"{}\"", b.xml_name))
                        .collect();
                    if branch_tags.is_empty() {
                        continue;
                    }
                    if let Some(variant) = phf_variant_of.get(meta.rust_name.as_str()) {
                        let _ = writeln!(
                            out,
                            "                    Some(&{}::{}) => {{",
                            phf_enum_name, variant
                        );
                    } else {
                        let _ =
                            writeln!(out, "                    {} => {{", branch_tags.join(" | "));
                    }
                    let union_name = type_ident(&union_def.qname);
                    let _ = writeln!(
                        out,
                        "                        let val = {}::decode_xml(reader, &e)?;",
                        union_name
                    );
                    let is_list = meta.field.cardinality.is_list() || meta.field.type_ref.is_list();
                    if is_list {
                        let _ = writeln!(
                            out,
                            "                        var_{}.push(val);",
                            meta.rust_name
                        );
                    } else {
                        let _ = writeln!(
                            out,
                            "                        var_{} = Some(val);",
                            meta.rust_name
                        );
                    }
                    out.push_str("                    }\n");
                } else {
                    if let Some(variant) = phf_variant_of.get(meta.rust_name.as_str()) {
                        let _ = writeln!(
                            out,
                            "                    Some(&{}::{}) => {{",
                            phf_enum_name, variant
                        );
                    } else {
                        let _ =
                            writeln!(out, "                    \"{}\" => {{", meta.field.xml_name);
                    }
                    self.emit_empty_element_parse(
                        out,
                        &meta.field,
                        &meta.rust_name,
                        ir,
                        types_with_lifetime,
                    );
                    out.push_str("                    }\n");
                }
            }
            out.push_str("                    _ => {}\n");
            out.push_str("                },\n");

            out.push_str("                Event::End(e) if e.local_name().as_ref() == start.local_name().as_ref() => break,\n");
            if ir.has_ordered_content(s) {
                let item_union = field_metas.iter().find_map(|meta| {
                    self.resolve_union_def(&meta.field.type_ref, ir)
                        .filter(|u| u.is_mixed_content())
                        .map(|u| (meta, u))
                });
                if let Some((meta, union)) = item_union {
                    let union_name = type_ident(&union.qname);
                    let wrap = if self.options.zero_copy {
                        "Cow::Owned"
                    } else {
                        "String::from"
                    };
                    let _ = writeln!(out, "                Event::Text(e) => var_{}.push({}::Text({wrap}(quick_xml::escape::unescape(e.as_ref())?.into_owned()))),", meta.rust_name, union_name);
                    let _ = writeln!(out, "                Event::CData(e) => var_{}.push({}::Text({wrap}(e.as_ref().to_string()))),", meta.rust_name, union_name);
                    let _ = writeln!(out, "                Event::GeneralRef(e) => {{ let text = if e.is_char_ref() {{ e.resolve_char_ref()?.map(|ch| ch.to_string()).unwrap_or_default() }} else if let Some(value) = quick_xml::escape::resolve_xml_entity(e.as_ref()) {{ value.to_string() }} else {{ e.as_ref().to_string() }}; var_{}.push({}::Text({wrap}(text))); }},", meta.rust_name, union_name);
                }
            }
            out.push_str("                Event::Eof => break,\n");
            out.push_str("                _ => {}\n");
            out.push_str("            }\n");
            out.push_str("        }\n");
        }

        // simpleContent value field: consume the element's own text content.
        // Only reachable when there are no child elements — the element loop
        // above already reads through End(start).
        if elem_fields.is_empty() {
            if let Some(meta) = &text_meta {
                self.emit_text_content_parse(out, &meta.field, &meta.rust_name, ir);
            }
        }

        if has_patterns {
            out.push_str("\n        let value = Self {\n");
        } else {
            out.push_str("\n        Ok(Self {\n");
        }
        for meta in &field_metas {
            let is_list = meta.field.cardinality.is_list() || meta.field.type_ref.is_list();
            let is_optional = meta.field.cardinality.is_optional() || meta.field.nillable;

            if is_list || is_optional {
                let _ = writeln!(
                    out,
                    "            {}: var_{},",
                    meta.rust_name, meta.rust_name
                );
            } else {
                let _ = writeln!(
                    out,
                    "            {}: var_{}.ok_or_else(|| PolyXmlError::SchemaError(\"Missing required field '{}'\".into()))?,",
                    meta.rust_name, meta.rust_name, meta.field.xml_name
                );
            }
        }
        if has_patterns {
            out.push_str("        };\n        value.validate_patterns()?;\n        Ok(value)\n");
        } else {
            out.push_str("        })\n");
        }
        out.push_str("    }\n\n");

        out.push_str("    pub fn decode_xml_empty(start: &BytesStart<'_>) -> Result<Self> {\n");
        for meta in &field_metas {
            let is_list = meta.field.cardinality.is_list() || meta.field.type_ref.is_list();
            if is_list {
                let _ = writeln!(out, "        let var_{} = Vec::new();", meta.rust_name);
            } else {
                let _ = writeln!(out, "        let mut var_{} = None;", meta.rust_name);
            }
        }

        if !attr_fields.is_empty() {
            out.push_str("\n        for attr in start.attributes() {\n");
            out.push_str("            let attr = attr?;\n");
            out.push_str("            match attr.key.local_name().as_ref() {\n");
            for meta in &attr_fields {
                let _ = writeln!(out, "                \"{}\" => {{", meta.field.xml_name);
                self.emit_attr_parse(out, &meta.field, &meta.rust_name, ir);
                out.push_str("                }\n");
            }
            out.push_str("                _ => {}\n");
            out.push_str("            }\n");
            out.push_str("        }\n");
        }

        if has_patterns {
            out.push_str("\n        let value = Self {\n");
        } else {
            out.push_str("\n        Ok(Self {\n");
        }
        for meta in &field_metas {
            let is_list = meta.field.cardinality.is_list() || meta.field.type_ref.is_list();
            let is_optional = meta.field.cardinality.is_optional() || meta.field.nillable;

            if is_list || is_optional {
                let _ = writeln!(
                    out,
                    "            {}: var_{},",
                    meta.rust_name, meta.rust_name
                );
            } else if meta.field.kind == FieldKind::Attribute {
                let _ = writeln!(
                    out,
                    "            {}: var_{}.ok_or_else(|| PolyXmlError::SchemaError(\"Missing required attribute '{}'\".into()))?,",
                    meta.rust_name, meta.rust_name, meta.field.xml_name
                );
            } else if self.field_is_string(&meta.field.type_ref, ir) {
                if self.options.zero_copy {
                    let _ = writeln!(
                        out,
                        "            {}: var_{}.unwrap_or(Cow::Borrowed(\"\")),",
                        meta.rust_name, meta.rust_name
                    );
                } else {
                    let _ = writeln!(
                        out,
                        "            {}: var_{}.unwrap_or_default(),",
                        meta.rust_name, meta.rust_name
                    );
                }
            } else if self.options.derive_default {
                let _ = writeln!(
                    out,
                    "            {}: var_{}.unwrap_or_default(),",
                    meta.rust_name, meta.rust_name
                );
            } else {
                let _ = writeln!(
                    out,
                    "            {}: var_{}.ok_or_else(|| PolyXmlError::SchemaError(\"Missing required field '{}'\".into()))?,",
                    meta.rust_name, meta.rust_name, meta.field.xml_name
                );
            }
        }
        if has_patterns {
            out.push_str("        };\n        value.validate_patterns()?;\n        Ok(value)\n");
        } else {
            out.push_str("        })\n");
        }
        out.push_str("    }\n\n");

        out.push_str("    pub fn to_xml(&self) -> Result<Vec<u8>> {\n");
        out.push_str("        let mut buf = Vec::new();\n");
        out.push_str("        let mut writer = Writer::new(std::io::Cursor::new(&mut buf));\n");
        out.push_str("        self.encode_xml(&mut writer, None)?;\n");
        out.push_str("        Ok(buf)\n");
        out.push_str("    }\n\n");

        out.push_str("    pub fn to_xml_string(&self) -> Result<String> {\n");
        out.push_str("        let bytes = self.to_xml()?;\n");
        out.push_str("        String::from_utf8(bytes).map_err(|e| PolyXmlError::Utf8Error(e.utf8_error()))\n");
        out.push_str("    }\n\n");

        if self.options.derive_serde {
            let from_json_sig = if needs_lifetime {
                "pub fn from_json_str(json_str: &'a str) -> std::result::Result<Self, serde_json::Error>"
            } else {
                "pub fn from_json_str(json_str: &str) -> std::result::Result<Self, serde_json::Error>"
            };
            let _ = writeln!(out, "    {}", from_json_sig);
            out.push_str("    {\n");
            out.push_str("        serde_json::from_str(json_str)\n");
            out.push_str("    }\n\n");

            let from_json_slice_sig = if needs_lifetime {
                "pub fn from_json_slice(bytes: &'a [u8]) -> std::result::Result<Self, serde_json::Error>"
            } else {
                "pub fn from_json_slice(bytes: &[u8]) -> std::result::Result<Self, serde_json::Error>"
            };
            let _ = writeln!(out, "    {}", from_json_slice_sig);
            out.push_str("    {\n");
            out.push_str("        serde_json::from_slice(bytes)\n");
            out.push_str("    }\n\n");

            out.push_str("    pub fn to_json_string(&self) -> std::result::Result<String, serde_json::Error> {\n");
            out.push_str("        serde_json::to_string(self)\n");
            out.push_str("    }\n\n");

            out.push_str("    pub fn to_json_vec(&self) -> std::result::Result<Vec<u8>, serde_json::Error> {\n");
            out.push_str("        serde_json::to_vec(self)\n");
            out.push_str("    }\n\n");
        }

        if has_patterns {
            out.push_str("    pub fn validate_patterns(&self) -> Result<()> {\n");
            for meta in &field_metas {
                let f = &meta.field;
                if super::patterned_simple(&f.type_ref, ir).is_none()
                    && !matches!(primitive_base(Self::unwrap_type_ref(&f.type_ref), ir), TypeRef::Primitive(p) if p.is_unbounded_integer())
                {
                    continue;
                }
                if f.cardinality.is_list() || f.type_ref.is_list() {
                    let _ = writeln!(out, "        for value in &self.{} {{", meta.rust_name);
                    self.emit_pattern_check(out, f, "&value.to_string()", ir);
                    out.push_str("        }\n");
                } else if f.cardinality.is_optional() || f.nillable {
                    let _ = writeln!(
                        out,
                        "        if let Some(value) = &self.{} {{",
                        meta.rust_name
                    );
                    self.emit_pattern_check(out, f, "&value.to_string()", ir);
                    out.push_str("        }\n");
                } else {
                    self.emit_pattern_check(
                        out,
                        f,
                        &format!("&self.{}.to_string()", meta.rust_name),
                        ir,
                    );
                }
            }

            out.push_str("        Ok(())\n    }\n");
        }
        out.push_str("    pub fn encode_xml<W: std::io::Write>(&self, writer: &mut Writer<W>, tag_name: Option<&str>) -> Result<()> {\n");
        if has_patterns {
            out.push_str("        self.validate_patterns()?;\n");
        }
        let _ = writeln!(
            out,
            "        let tag = tag_name.unwrap_or(\"{}\");",
            s.qname.local
        );
        out.push_str("        let mut start = BytesStart::new(tag);\n");

        for meta in &attr_fields {
            self.emit_attr_serialize(out, &meta.field, &meta.rust_name, ir);
        }

        out.push_str("        writer.write_event(Event::Start(start))?;\n");

        // simpleContent value: write the element's own text content between
        // Start and End. Text and child content are mutually exclusive in the
        // IR, so this never overlaps the element loop below.
        if elem_fields.is_empty() {
            if let Some(meta) = &text_meta {
                self.emit_text_content_serialize(out, &meta.field, &meta.rust_name, ir);
            }
        }

        for meta in &elem_fields {
            self.emit_element_serialize(out, &meta.field, &meta.rust_name, ir);
        }

        out.push_str("        writer.write_event(Event::End(BytesEnd::new(tag)))?;\n");
        out.push_str("        Ok(())\n");
        out.push_str("    }\n");

        out.push_str("}\n");
    }

    fn emit_integer_bounds(
        &self,
        out: &mut String,
        facets: &crate::ir::RestrictionFacets,
        value: &str,
        name: &str,
    ) {
        let option = |bound: &Option<String>| {
            bound
                .as_ref()
                .map(|s| format!("Some({s:?})"))
                .unwrap_or("None".into())
        };
        writeln!(out,"        if !polyxml::integer::within_bounds({value}, {}, {}, {}, {}) {{return Err(PolyXmlError::FacetViolation {{field:{name:?}.into(),expected:\"integer bounds\".into(),actual:({value}).to_string()}});}}",option(&facets.min_inclusive),option(&facets.max_inclusive),option(&facets.min_exclusive),option(&facets.max_exclusive)).unwrap();
    }

    fn emit_pattern_check(&self, out: &mut String, field: &FieldDef, value: &str, ir: &SchemaIR) {
        if let TypeRef::Primitive(p) = primitive_base(Self::unwrap_type_ref(&field.type_ref), ir) {
            if p.is_unbounded_integer() {
                let _ = writeln!(out, "        if !polyxml::integer::validate({value},polyxml::ir::PrimitiveType::{p:?}) {{return Err(PolyXmlError::ScalarParseError {{field:{:?}.into(),expected:\"XML Schema integer\",value:({value}).to_string()}});}}", field.name);
            }
        }

        if super::unbounded_integer(&field.type_ref, ir).is_some() {
            if let Some(facets) = &field.facets {
                self.emit_integer_bounds(out, facets, value, &field.name);
            }
            if let TypeRef::Named(name) = Self::unwrap_type_ref(&field.type_ref) {
                if let Some(TypeDef::Simple(simple)) = ir.types.get(name) {
                    self.emit_integer_bounds(out, &simple.facets, value, &field.name);
                }
            }
        }
        if let Some(simple) = super::patterned_simple(&field.type_ref, ir) {
            let _ = writeln!(out, "        validate_{}_patterns({}).map_err(|message| PolyXmlError::FacetViolation {{ field: {:?}.into(), expected: message.into(), actual: ({}).to_string() }})?;", type_ident(&simple.qname), value, field.name, value);
        }
    }

    fn emit_attr_parse(&self, out: &mut String, field: &FieldDef, rust_name: &str, ir: &SchemaIR) {
        if self.field_is_string(&field.type_ref, ir) {
            out.push_str("                    let s = attr.value.as_ref();\n");
            out.push_str("                    let val: Cow<'_, str> = match quick_xml::escape::unescape(s)? {\n");
            out.push_str(
                "                        Cow::Borrowed(s) => Cow::Owned(s.to_string()),\n",
            );
            out.push_str("                        Cow::Owned(s) => Cow::Owned(s),\n");
            out.push_str("                    };\n");
            self.emit_facet_checks(out, field, "val", ir);
            self.emit_pattern_check(out, field, "&val", ir);
            if self.options.zero_copy {
                let _ = writeln!(out, "                    var_{} = Some(val);", rust_name);
            } else {
                let _ = writeln!(
                    out,
                    "                    var_{} = Some(val.into_owned());",
                    rust_name
                );
            }
        } else if let Some(num) = self.field_numeric_type(&field.type_ref, ir) {
            out.push_str("                    let s = attr.value.as_ref().trim();\n");
            let _ = writeln!(
                out,
                "                    let val = s.parse::<{}>().map_err(|_| PolyXmlError::ScalarParseError {{ field: \"{}\".into(), expected: \"{}\", value: s.into() }})?;",
                num, field.name, num
            );
            self.emit_facet_checks(out, field, "val", ir);
            let _ = writeln!(out, "                    var_{} = Some(val);", rust_name);
        } else if self.field_is_bool(&field.type_ref, ir) {
            out.push_str("                    let s = attr.value.as_ref().trim();\n");
            out.push_str("                    let val = s == \"true\" || s == \"1\";\n");
            let _ = writeln!(out, "                    var_{} = Some(val);", rust_name);
        } else if let Some(enum_name) = self.field_is_enum(&field.type_ref, ir) {
            out.push_str("                    let s = attr.value.as_ref().trim();\n");
            let _ = writeln!(
                out,
                "                    let val = {}::from_str(s).map_err(|_| PolyXmlError::ScalarParseError {{ field: \"{}\".into(), expected: \"{}\", value: s.into() }})?;",
                enum_name, field.name, enum_name
            );
            let _ = writeln!(out, "                    var_{} = Some(val);", rust_name);
        }
    }

    fn emit_element_parse(
        &self,
        out: &mut String,
        field: &FieldDef,
        rust_name: &str,
        ir: &SchemaIR,
        types_with_lifetime: &HashSet<QName>,
    ) {
        let is_list = field.cardinality.is_list() || field.type_ref.is_list();
        let is_boxed = field.is_cycle_cut || field.type_ref.is_boxed();

        if self.field_is_string(&field.type_ref, ir) {
            let _ = writeln!(
                out,
                "                        let text = read_element_text(reader, \"{}\")?;",
                field.xml_name
            );
            self.emit_facet_checks(out, field, "text", ir);
            self.emit_pattern_check(out, field, "&text", ir);
            if is_list {
                let _ = writeln!(out, "                        var_{}.push(text);", rust_name);
            } else {
                let _ = writeln!(
                    out,
                    "                        var_{} = Some(text);",
                    rust_name
                );
            }
        } else if let Some(num) = self.field_numeric_type(&field.type_ref, ir) {
            let _ = writeln!(
                out,
                "                        let text = read_element_text(reader, \"{}\")?;",
                field.xml_name
            );
            out.push_str("                        let s = text.trim();\n");
            let _ = writeln!(
                out,
                "                        let val = s.parse::<{}>().map_err(|_| PolyXmlError::ScalarParseError {{ field: \"{}\".into(), expected: \"{}\", value: s.into() }})?;",
                num, field.name, num
            );
            self.emit_facet_checks(out, field, "val", ir);
            if is_list {
                let _ = writeln!(out, "                        var_{}.push(val);", rust_name);
            } else {
                let _ = writeln!(
                    out,
                    "                        var_{} = Some(val);",
                    rust_name
                );
            }
        } else if self.field_is_bool(&field.type_ref, ir) {
            let _ = writeln!(
                out,
                "                        let text = read_element_text(reader, \"{}\")?;",
                field.xml_name
            );
            out.push_str("                        let s = text.trim();\n");
            out.push_str("                        let val = s == \"true\" || s == \"1\";\n");
            if is_list {
                let _ = writeln!(out, "                        var_{}.push(val);", rust_name);
            } else {
                let _ = writeln!(
                    out,
                    "                        var_{} = Some(val);",
                    rust_name
                );
            }
        } else if let Some(enum_name) = self.field_is_enum(&field.type_ref, ir) {
            let _ = writeln!(
                out,
                "                        let text = read_element_text(reader, \"{}\")?;",
                field.xml_name
            );
            out.push_str("                        let s = text.trim();\n");
            let _ = writeln!(
                out,
                "                        let val = {}::from_str(s).map_err(|_| PolyXmlError::ScalarParseError {{ field: \"{}\".into(), expected: \"{}\", value: s.into() }})?;",
                enum_name, field.name, enum_name
            );
            if is_list {
                let _ = writeln!(out, "                        var_{}.push(val);", rust_name);
            } else {
                let _ = writeln!(
                    out,
                    "                        var_{} = Some(val);",
                    rust_name
                );
            }
        } else {
            let target_type = self.format_rust_type_ref(&field.type_ref, types_with_lifetime);
            let target_clean = target_type.split('<').next().unwrap_or(&target_type);
            let decode = if self.resolve_union_def(&field.type_ref, ir).is_some() {
                "decode_xml_wrapped"
            } else {
                "decode_xml"
            };
            let _ = writeln!(
                out,
                "                        let val = {target_clean}::{decode}(reader, &e)?;"
            );
            let final_val = if is_boxed { "Box::new(val)" } else { "val" };
            if is_list {
                let _ = writeln!(
                    out,
                    "                        var_{}.push({});",
                    rust_name, final_val
                );
            } else {
                let _ = writeln!(
                    out,
                    "                        var_{} = Some({});",
                    rust_name, final_val
                );
            }
        }
    }

    fn emit_empty_element_parse(
        &self,
        out: &mut String,
        field: &FieldDef,
        rust_name: &str,
        ir: &SchemaIR,
        types_with_lifetime: &HashSet<QName>,
    ) {
        let is_list = field.cardinality.is_list() || field.type_ref.is_list();
        let is_boxed = field.is_cycle_cut || field.type_ref.is_boxed();

        if self.resolve_union_def(&field.type_ref, ir).is_some() {
            let _ = writeln!(out, "                        return Err(PolyXmlError::SchemaError(\"Missing choice in {}\".into()));", field.xml_name);
            return;
        }
        if self.field_is_string(&field.type_ref, ir) {
            self.emit_pattern_check(out, field, "\"\"", ir);
            if self.options.zero_copy {
                if is_list {
                    let _ = writeln!(
                        out,
                        "                        var_{}.push(Cow::Borrowed(\"\"));",
                        rust_name
                    );
                } else {
                    let _ = writeln!(
                        out,
                        "                        var_{} = Some(Cow::Borrowed(\"\"));",
                        rust_name
                    );
                }
            } else {
                if is_list {
                    let _ = writeln!(
                        out,
                        "                        var_{}.push(String::new());",
                        rust_name
                    );
                } else {
                    let _ = writeln!(
                        out,
                        "                        var_{} = Some(String::new());",
                        rust_name
                    );
                }
            }
        } else if self.field_numeric_type(&field.type_ref, ir).is_none()
            && !self.field_is_bool(&field.type_ref, ir)
            && self.field_is_enum(&field.type_ref, ir).is_none()
        {
            let target_type = self.format_rust_type_ref(&field.type_ref, types_with_lifetime);
            let target_clean = target_type.split('<').next().unwrap_or(&target_type);
            let _ = writeln!(
                out,
                "                        let val = {}::decode_xml_empty(&e)?;",
                target_clean
            );
            let final_val = if is_boxed { "Box::new(val)" } else { "val" };
            if is_list {
                let _ = writeln!(
                    out,
                    "                        var_{}.push({});",
                    rust_name, final_val
                );
            } else {
                let _ = writeln!(
                    out,
                    "                        var_{} = Some({});",
                    rust_name, final_val
                );
            }
        }
    }

    /// Decode the simpleContent value field from the element's own text
    /// content. Emitted only when the struct has no child elements (the
    /// element loop already consumes through `End(start)`), and only for
    /// scalar-backed text: a simpleContent chain whose value is typed as its
    /// base struct has no scalar form here and keeps its previous behavior.
    fn emit_text_content_parse(
        &self,
        out: &mut String,
        field: &FieldDef,
        rust_name: &str,
        ir: &SchemaIR,
    ) {
        let read = "        let text = read_element_text(reader, start.local_name().as_ref())?;\n";
        if self.field_is_string(&field.type_ref, ir) {
            out.push_str(read);
            self.emit_facet_checks(out, field, "text", ir);
            self.emit_pattern_check(out, field, "&text", ir);
            let _ = writeln!(out, "        var_{} = Some(text);", rust_name);
        } else if let Some(num) = self.field_numeric_type(&field.type_ref, ir) {
            out.push_str(read);
            out.push_str("        let s = text.trim();\n");
            let _ = writeln!(
                out,
                "        let val = s.parse::<{}>().map_err(|_| PolyXmlError::ScalarParseError {{ field: \"{}\".into(), expected: \"{}\", value: s.into() }})?;",
                num, field.name, num
            );
            self.emit_facet_checks(out, field, "val", ir);
            let _ = writeln!(out, "        var_{} = Some(val);", rust_name);
        } else if self.field_is_bool(&field.type_ref, ir) {
            out.push_str(read);
            out.push_str("        let s = text.trim();\n");
            let _ = writeln!(out, "        let val = match s {{ \"true\" | \"1\" => true, \"false\" | \"0\" => false, _ => return Err(PolyXmlError::ScalarParseError {{ field: {:?}.into(), expected: \"bool\", value: s.into() }}) }};", field.name);
            let _ = writeln!(out, "        var_{} = Some(val);", rust_name);
        } else if let Some(enum_name) = self.field_is_enum(&field.type_ref, ir) {
            out.push_str(read);
            out.push_str("        let s = text.trim();\n");
            let _ = writeln!(
                out,
                "        let val = {}::from_str(s).map_err(|_| PolyXmlError::ScalarParseError {{ field: \"{}\".into(), expected: \"{}\", value: s.into() }})?;",
                enum_name, field.name, enum_name
            );
            let _ = writeln!(out, "        var_{} = Some(val);", rust_name);
        }
    }

    /// Serialize the simpleContent value field as the element's own text
    /// content; mirrors [`Self::emit_text_content_parse`] and skips the same
    /// struct-typed values.
    fn emit_text_content_serialize(
        &self,
        out: &mut String,
        field: &FieldDef,
        rust_name: &str,
        ir: &SchemaIR,
    ) {
        let is_optional = field.cardinality.is_optional() || field.nillable;
        if self.field_is_string(&field.type_ref, ir) {
            if is_optional {
                let _ = writeln!(out, "        if let Some(ref val) = self.{} {{", rust_name);
                out.push_str(
                    "            writer.write_event(Event::Text(BytesText::new(val.as_ref())))?;\n        }\n",
                );
            } else {
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::Text(BytesText::new(self.{}.as_ref())))?;",
                    rust_name
                );
            }
        } else if self.field_numeric_type(&field.type_ref, ir).is_some()
            || self.field_is_bool(&field.type_ref, ir)
        {
            if is_optional {
                let _ = writeln!(out, "        if let Some(ref val) = self.{} {{", rust_name);
                out.push_str(
                    "            writer.write_event(Event::Text(BytesText::new(&val.to_string())))?;\n        }\n",
                );
            } else {
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::Text(BytesText::new(&self.{}.to_string())))?;",
                    rust_name
                );
            }
        } else if self.field_is_enum(&field.type_ref, ir).is_some() {
            if is_optional {
                let _ = writeln!(out, "        if let Some(ref val) = self.{} {{", rust_name);
                out.push_str(
                    "            writer.write_event(Event::Text(BytesText::new(val.as_str())))?;\n        }\n",
                );
            } else {
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::Text(BytesText::new(self.{}.as_str())))?;",
                    rust_name
                );
            }
        }
    }

    fn emit_facet_checks(&self, out: &mut String, field: &FieldDef, val_var: &str, ir: &SchemaIR) {
        if let Some(ref facets) = field.facets {
            if super::unbounded_integer(&field.type_ref, ir).is_some() {
                self.emit_integer_bounds(out, facets, val_var, &field.name);
                return;
            }

            if let Some(min_len) = facets.min_length {
                let _ = writeln!(
                    out,
                    "                    if {}.len() < {} {{ return Err(PolyXmlError::FacetViolation {{ field: \"{}\".into(), expected: \"minLength {}\".into(), actual: format!(\"length {{}}\", {}.len()) }}); }}",
                    val_var, min_len, field.name, min_len, val_var
                );
            }
            if let Some(max_len) = facets.max_length {
                let _ = writeln!(
                    out,
                    "                    if {}.len() > {} {{ return Err(PolyXmlError::FacetViolation {{ field: \"{}\".into(), expected: \"maxLength {}\".into(), actual: format!(\"length {{}}\", {}.len()) }}); }}",
                    val_var, max_len, field.name, max_len, val_var
                );
            }
            if let Some(ref min_inc) = facets.min_inclusive {
                let _ = writeln!(
                    out,
                    "                    if {} < {} {{ return Err(PolyXmlError::FacetViolation {{ field: \"{}\".into(), expected: \"minInclusive {}\".into(), actual: {}.to_string() }}); }}",
                    val_var, min_inc, field.name, min_inc, val_var
                );
            }
            if let Some(ref max_inc) = facets.max_inclusive {
                let _ = writeln!(
                    out,
                    "                    if {} > {} {{ return Err(PolyXmlError::FacetViolation {{ field: \"{}\".into(), expected: \"maxInclusive {}\".into(), actual: {}.to_string() }}); }}",
                    val_var, max_inc, field.name, max_inc, val_var
                );
            }
        }
    }

    fn emit_attr_serialize(
        &self,
        out: &mut String,
        field: &FieldDef,
        rust_name: &str,
        ir: &SchemaIR,
    ) {
        let is_optional = field.cardinality.is_optional() || field.nillable;
        if self.field_is_string(&field.type_ref, ir) {
            if is_optional {
                let _ = writeln!(out, "        if let Some(ref val) = self.{} {{", rust_name);
                let _ = writeln!(
                    out,
                    "            start.push_attribute((\"{}\", val.as_ref()));",
                    field.xml_name
                );
                out.push_str("        }\n");
            } else {
                let _ = writeln!(
                    out,
                    "        start.push_attribute((\"{}\", self.{}.as_ref()));",
                    field.xml_name, rust_name
                );
            }
        } else if is_optional {
            let _ = writeln!(out, "        if let Some(ref val) = self.{} {{", rust_name);
            let _ = writeln!(out, "            let val_s = val.to_string();");
            let _ = writeln!(
                out,
                "            start.push_attribute((\"{}\", val_s.as_str()));",
                field.xml_name
            );
            out.push_str("        }\n");
        } else {
            let _ = writeln!(
                out,
                "        let val_s_{} = self.{}.to_string();",
                rust_name, rust_name
            );
            let _ = writeln!(
                out,
                "        start.push_attribute((\"{}\", val_s_{}.as_str()));",
                field.xml_name, rust_name
            );
        }
    }

    fn emit_element_serialize(
        &self,
        out: &mut String,
        field: &FieldDef,
        rust_name: &str,
        ir: &SchemaIR,
    ) {
        let is_list = field.cardinality.is_list() || field.type_ref.is_list();
        let is_optional = field.cardinality.is_optional() || field.nillable;
        let is_string = self.field_is_string(&field.type_ref, ir);
        let is_scalar = self.field_numeric_type(&field.type_ref, ir).is_some()
            || self.field_is_bool(&field.type_ref, ir);
        let is_enum = self.field_is_enum(&field.type_ref, ir).is_some();

        if is_list {
            let _ = writeln!(out, "        for item in &self.{} {{", rust_name);
            if is_string {
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                out.push_str("            writer.write_event(Event::Text(BytesText::new(item.as_ref())))?;\n");
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if is_scalar {
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                out.push_str("            writer.write_event(Event::Text(BytesText::new(&item.to_string())))?;\n");
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if is_enum {
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                out.push_str("            writer.write_event(Event::Text(BytesText::new(item.as_str())))?;\n");
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if self.resolve_union_def(&field.type_ref, ir).is_some() {
                if field.xml_name.is_empty() {
                    let _ = writeln!(out, "            item.encode_xml(writer, None)?;");
                } else {
                    let _ = writeln!(
                        out,
                        "            item.encode_xml_wrapped(writer, {:?})?;",
                        field.xml_name
                    );
                }
            } else {
                let _ = writeln!(
                    out,
                    "            item.encode_xml(writer, Some(\"{}\"))?;",
                    field.xml_name
                );
            }
            out.push_str("        }\n");
        } else if is_optional {
            let _ = writeln!(out, "        if let Some(ref val) = self.{} {{", rust_name);
            if is_string {
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                out.push_str(
                    "            writer.write_event(Event::Text(BytesText::new(val.as_ref())))?;\n",
                );
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if is_scalar {
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                out.push_str("            writer.write_event(Event::Text(BytesText::new(&val.to_string())))?;\n");
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if is_enum {
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                out.push_str(
                    "            writer.write_event(Event::Text(BytesText::new(val.as_str())))?;\n",
                );
                let _ = writeln!(
                    out,
                    "            writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if self.resolve_union_def(&field.type_ref, ir).is_some() {
                if field.xml_name.is_empty() {
                    let _ = writeln!(out, "            val.encode_xml(writer, None)?;");
                } else {
                    let _ = writeln!(
                        out,
                        "            val.encode_xml_wrapped(writer, {:?})?;",
                        field.xml_name
                    );
                }
            } else {
                let _ = writeln!(
                    out,
                    "            val.encode_xml(writer, Some(\"{}\"))?;",
                    field.xml_name
                );
            }
            out.push_str("        }\n");
        } else {
            if is_string {
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::Text(BytesText::new(self.{}.as_ref())))?;",
                    rust_name
                );
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if is_scalar {
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                let _ = writeln!(out, "        writer.write_event(Event::Text(BytesText::new(&self.{}.to_string())))?;", rust_name);
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if is_enum {
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::Start(BytesStart::new(\"{}\")))?;",
                    field.xml_name
                );
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::Text(BytesText::new(self.{}.as_str())))?;",
                    rust_name
                );
                let _ = writeln!(
                    out,
                    "        writer.write_event(Event::End(BytesEnd::new(\"{}\")))?;",
                    field.xml_name
                );
            } else if self.resolve_union_def(&field.type_ref, ir).is_some() {
                if field.xml_name.is_empty() {
                    let _ = writeln!(out, "        self.{}.encode_xml(writer, None)?;", rust_name);
                } else {
                    let _ = writeln!(
                        out,
                        "        self.{}.encode_xml_wrapped(writer, {:?})?;",
                        rust_name, field.xml_name
                    );
                }
            } else {
                let _ = writeln!(
                    out,
                    "        self.{}.encode_xml(writer, Some(\"{}\"))?;",
                    rust_name, field.xml_name
                );
            }
        }
    }

    fn emit_union_codecs(
        &self,
        out: &mut String,
        u: &UnionDef,
        types_with_lifetime: &HashSet<QName>,
        ir: &SchemaIR,
    ) {
        if u.is_lexical() {
            self.emit_lexical_union_codecs(out, u, types_with_lifetime, ir);
            return;
        }
        let union_name = type_ident(&u.qname);
        let needs_lifetime = types_with_lifetime.contains(&u.qname);

        let impl_header = if needs_lifetime {
            format!("impl<'a> {}<'a>", union_name)
        } else {
            format!("impl {}", union_name)
        };

        out.push('\n');
        let _ = writeln!(out, "{} {{", impl_header);

        let reader_lifetime = if needs_lifetime { "'a" } else { "'_" };
        let _ = writeln!(out, "    pub fn decode_xml_wrapped(reader: &mut Reader<&{reader_lifetime} [u8]>, start: &BytesStart<'_>) -> Result<Self> {{");
        out.push_str(r#"        let mut value = None;
        loop {
            match reader.read_event()? {
                Event::Start(e) => {
                    if value.is_some() { return Err(PolyXmlError::SchemaError("Multiple choice alternatives".into())); }
                    value = Some(Self::decode_xml(reader, &e)?);
                }
                Event::Empty(e) => {
                    if value.is_some() { return Err(PolyXmlError::SchemaError("Multiple choice alternatives".into())); }
                    let mut empty_reader = Reader::from_str("");
                    value = Some(Self::decode_xml(&mut empty_reader, &e)?);
                }
                Event::End(e) if e.name() == start.name() => break,
                Event::Eof => return Err(PolyXmlError::SchemaError("Unclosed choice wrapper".into())),
                _ => {}
            }
        }
        value.ok_or_else(|| PolyXmlError::SchemaError("Missing choice alternative".into()))
    }
    pub fn encode_xml_wrapped<W: std::io::Write>(&self, writer: &mut Writer<W>, tag: &str) -> Result<()> {
        writer.write_event(Event::Start(BytesStart::new(tag)))?;
        self.encode_xml(writer, None)?;
        writer.write_event(Event::End(BytesEnd::new(tag)))?;
        Ok(())
    }
"#);

        let from_xml_sig = if needs_lifetime {
            "pub fn from_xml(xml: &'a str) -> Result<Self>"
        } else {
            "pub fn from_xml(xml: &str) -> Result<Self>"
        };
        let _ = writeln!(out, "    {}", from_xml_sig);
        out.push_str("    {\n");
        out.push_str("        let mut reader = Reader::from_str(xml);\n");
        out.push_str("        loop {\n");
        out.push_str("            match reader.read_event()? {\n");
        out.push_str(
            "                Event::Start(e) => return Self::decode_xml(&mut reader, &e),\n",
        );
        out.push_str("                Event::Eof => break,\n");
        out.push_str("                _ => {}\n");
        out.push_str("            }\n");
        out.push_str("        }\n");
        let _ = writeln!(
            out,
            "        Err(PolyXmlError::SchemaError(\"Unexpected EOF while parsing {}\".into()))",
            union_name
        );
        out.push_str("    }\n\n");

        let from_bytes_sig = if needs_lifetime {
            "pub fn from_xml_bytes(xml_bytes: &'a [u8]) -> Result<Self>"
        } else {
            "pub fn from_xml_bytes(xml_bytes: &[u8]) -> Result<Self>"
        };
        let _ = writeln!(out, "    {}", from_bytes_sig);
        out.push_str("    {\n");
        out.push_str("        let s = std::str::from_utf8(xml_bytes)?;\n");
        out.push_str("        Self::from_xml(s)\n");
        out.push_str("    }\n\n");

        let decode_sig = if needs_lifetime {
            "pub fn decode_xml(reader: &mut Reader<&'a [u8]>, start: &BytesStart<'_>) -> Result<Self>"
        } else {
            "pub fn decode_xml(reader: &mut Reader<&'_ [u8]>, start: &BytesStart<'_>) -> Result<Self>"
        };
        let _ = writeln!(out, "    {}", decode_sig);
        out.push_str("    {\n");
        out.push_str("        match start.local_name().as_ref() {\n");

        for branch in &u.branches {
            let var_id = to_rust_variant_identifier(&branch.variant_name);
            if u.is_mixed_content() && branch.xml_name == "#text" {
                continue;
            }
            let _ = writeln!(out, "            \"{}\" => {{", branch.xml_name);
            if self.field_is_string(&branch.type_ref, ir) {
                let _ = writeln!(
                    out,
                    "                let text = read_element_text(reader, \"{}\")?;",
                    branch.xml_name
                );
                let _ = writeln!(out, "                Ok({}::{}(text))", union_name, var_id);
            } else if let Some(num) = self.field_numeric_type(&branch.type_ref, ir) {
                let _ = writeln!(
                    out,
                    "                let text = read_element_text(reader, \"{}\")?;",
                    branch.xml_name
                );
                out.push_str("                let s = text.trim();\n");
                let _ = writeln!(out, "                let val = s.parse::<{}>().map_err(|_| PolyXmlError::ScalarParseError {{ field: \"{}\".into(), expected: \"{}\", value: s.into() }})?;", num, branch.xml_name, num);
                let _ = writeln!(out, "                Ok({}::{}(val))", union_name, var_id);
            } else if self.field_is_bool(&branch.type_ref, ir) {
                let _ = writeln!(
                    out,
                    "                let text = read_element_text(reader, \"{}\")?;",
                    branch.xml_name
                );
                out.push_str("                let s = text.trim();\n");
                out.push_str("                let val = s == \"true\" || s == \"1\";\n");
                let _ = writeln!(out, "                Ok({}::{}(val))", union_name, var_id);
            } else if let Some(enum_name) = self.field_is_enum(&branch.type_ref, ir) {
                let _ = writeln!(
                    out,
                    "                let text = read_element_text(reader, \"{}\")?;",
                    branch.xml_name
                );
                out.push_str("                let s = text.trim();\n");
                let _ = writeln!(out, "                let val = {}::from_str(s).map_err(|_| PolyXmlError::ScalarParseError {{ field: \"{}\".into(), expected: \"{}\", value: s.into() }})?;", enum_name, branch.xml_name, enum_name);
                let _ = writeln!(out, "                Ok({}::{}(val))", union_name, var_id);
            } else {
                let target_type = self.format_rust_type_ref(&branch.type_ref, types_with_lifetime);
                let target_clean = target_type.split('<').next().unwrap_or(&target_type);
                let _ = writeln!(
                    out,
                    "                let val = {}::decode_xml(reader, start)?;",
                    target_clean
                );
                let _ = writeln!(out, "                Ok({}::{}(val))", union_name, var_id);
            }
            out.push_str("            }\n");
        }

        let branch_names: Vec<_> = u.branches.iter().map(|b| b.xml_name.as_str()).collect();
        let _ = writeln!(out, "            other => Err(PolyXmlError::UnexpectedRootElement {{ expected: \"{}\".into(), actual: other.into() }}),", branch_names.join(" or "));
        out.push_str("        }\n");
        out.push_str("    }\n\n");

        out.push_str("    pub fn to_xml(&self) -> Result<Vec<u8>> {\n");
        out.push_str("        let mut buf = Vec::new();\n");
        out.push_str("        let mut writer = Writer::new(std::io::Cursor::new(&mut buf));\n");
        out.push_str("        self.encode_xml(&mut writer, None)?;\n");
        out.push_str("        Ok(buf)\n");
        out.push_str("    }\n\n");

        out.push_str("    pub fn to_xml_string(&self) -> Result<String> {\n");
        out.push_str("        let bytes = self.to_xml()?;\n");
        out.push_str("        String::from_utf8(bytes).map_err(|e| PolyXmlError::Utf8Error(e.utf8_error()))\n");
        out.push_str("    }\n\n");

        if self.options.derive_serde {
            let from_json_sig = if needs_lifetime {
                "pub fn from_json_str(json_str: &'a str) -> std::result::Result<Self, serde_json::Error>"
            } else {
                "pub fn from_json_str(json_str: &str) -> std::result::Result<Self, serde_json::Error>"
            };
            let _ = writeln!(out, "    {}", from_json_sig);
            out.push_str("    {\n");
            out.push_str("        serde_json::from_str(json_str)\n");
            out.push_str("    }\n\n");

            let from_json_slice_sig = if needs_lifetime {
                "pub fn from_json_slice(bytes: &'a [u8]) -> std::result::Result<Self, serde_json::Error>"
            } else {
                "pub fn from_json_slice(bytes: &[u8]) -> std::result::Result<Self, serde_json::Error>"
            };
            let _ = writeln!(out, "    {}", from_json_slice_sig);
            out.push_str("    {\n");
            out.push_str("        serde_json::from_slice(bytes)\n");
            out.push_str("    }\n\n");

            out.push_str("    pub fn to_json_string(&self) -> std::result::Result<String, serde_json::Error> {\n");
            out.push_str("        serde_json::to_string(self)\n");
            out.push_str("    }\n\n");

            out.push_str("    pub fn to_json_vec(&self) -> std::result::Result<Vec<u8>, serde_json::Error> {\n");
            out.push_str("        serde_json::to_vec(self)\n");
            out.push_str("    }\n\n");
        }

        out.push_str("    pub fn encode_xml<W: std::io::Write>(&self, writer: &mut Writer<W>, tag_name: Option<&str>) -> Result<()> {\n");
        out.push_str("        match self {\n");
        for branch in &u.branches {
            let var_id = to_rust_variant_identifier(&branch.variant_name);
            if u.is_mixed_content() && branch.xml_name == "#text" {
                let _ = writeln!(out, "            {}::{}(ref val) => {{", union_name, var_id);
                out.push_str("                writer.write_event(Event::Text(BytesText::new(val.as_ref())))?;\n                Ok(())\n            }\n");
                continue;
            }
            if self.field_is_string(&branch.type_ref, ir) {
                let _ = writeln!(out, "            {}::{}(ref val) => {{", union_name, var_id);
                let _ = writeln!(
                    out,
                    "                let tag = tag_name.unwrap_or(\"{}\");",
                    branch.xml_name
                );
                out.push_str(
                    "                writer.write_event(Event::Start(BytesStart::new(tag)))?;\n",
                );
                out.push_str("                writer.write_event(Event::Text(BytesText::new(val.as_ref())))?;\n");
                out.push_str(
                    "                writer.write_event(Event::End(BytesEnd::new(tag)))?;\n",
                );
                out.push_str("                Ok(())\n");
                out.push_str("            }\n");
            } else if self.field_numeric_type(&branch.type_ref, ir).is_some()
                || self.field_is_bool(&branch.type_ref, ir)
            {
                let _ = writeln!(out, "            {}::{}(ref val) => {{", union_name, var_id);
                let _ = writeln!(
                    out,
                    "                let tag = tag_name.unwrap_or(\"{}\");",
                    branch.xml_name
                );
                out.push_str(
                    "                writer.write_event(Event::Start(BytesStart::new(tag)))?;\n",
                );
                out.push_str("                writer.write_event(Event::Text(BytesText::new(&val.to_string())))?;\n");
                out.push_str(
                    "                writer.write_event(Event::End(BytesEnd::new(tag)))?;\n",
                );
                out.push_str("                Ok(())\n");
                out.push_str("            }\n");
            } else if self.field_is_enum(&branch.type_ref, ir).is_some() {
                let _ = writeln!(out, "            {}::{}(ref val) => {{", union_name, var_id);
                let _ = writeln!(
                    out,
                    "                let tag = tag_name.unwrap_or(\"{}\");",
                    branch.xml_name
                );
                out.push_str(
                    "                writer.write_event(Event::Start(BytesStart::new(tag)))?;\n",
                );
                out.push_str("                writer.write_event(Event::Text(BytesText::new(val.as_str())))?;\n");
                out.push_str(
                    "                writer.write_event(Event::End(BytesEnd::new(tag)))?;\n",
                );
                out.push_str("                Ok(())\n");
                out.push_str("            }\n");
            } else {
                let _ = writeln!(out, "            {}::{}(ref val) => val.encode_xml(writer, tag_name.or(Some(\"{}\"))),", union_name, var_id, branch.xml_name);
            }
        }
        out.push_str("        }\n");
        out.push_str("    }\n");
        out.push_str("}\n");
    }

    fn emit_lexical_union_codecs(
        &self,
        out: &mut String,
        u: &UnionDef,
        types_with_lifetime: &HashSet<QName>,
        ir: &SchemaIR,
    ) {
        let name = type_ident(&u.qname);
        let borrowed = types_with_lifetime.contains(&u.qname);
        let header = if borrowed {
            format!("impl<'a> {}<'a>", name)
        } else {
            format!("impl {}", name)
        };
        let text_type = if borrowed { "Cow<'a, str>" } else { "String" };
        let xml_arg = if borrowed { "&'a str" } else { "&str" };
        let bytes_arg = if borrowed { "&'a [u8]" } else { "&[u8]" };
        let reader_arg = if borrowed {
            "&mut Reader<&'a [u8]>"
        } else {
            "&mut Reader<&[u8]>"
        };

        let _ = writeln!(out, "\n{} {{", header);
        let _ = writeln!(
            out,
            "    fn parse_lexical(text: {}) -> Result<Self> {{",
            text_type
        );
        out.push_str("        let s = text.trim();\n");
        for branch in &u.branches {
            let variant = to_rust_variant_identifier(&branch.variant_name);
            if let Some(enum_name) = self.field_is_enum(&branch.type_ref, ir) {
                let _ = writeln!(
                    out,
                    "        if let Ok(value) = {}::from_str(s) {{ return Ok(Self::{}(value)); }}",
                    enum_name, variant
                );
            } else if let Some(numeric) = self.field_numeric_type(&branch.type_ref, ir) {
                let _ = writeln!(
                    out,
                    "        if let Ok(value) = s.parse::<{}>() {{ return Ok(Self::{}(value)); }}",
                    numeric, variant
                );
            } else if self.field_is_bool(&branch.type_ref, ir) {
                let _ = writeln!(
                    out,
                    "        if s == \"true\" || s == \"1\" {{ return Ok(Self::{}(true)); }}",
                    variant
                );
                let _ = writeln!(
                    out,
                    "        if s == \"false\" || s == \"0\" {{ return Ok(Self::{}(false)); }}",
                    variant
                );
            } else if self.field_is_string(&branch.type_ref, ir) {
                if let Some(p) = super::unbounded_integer(&branch.type_ref, ir) {
                    writeln!(out,"        if polyxml::integer::validate(s,polyxml::ir::PrimitiveType::{p:?}) {{ return Ok(Self::{variant}(text)); }}").unwrap();
                } else if let Some(simple) = super::patterned_simple(&branch.type_ref, ir) {
                    let _ = writeln!(out, "        if validate_{}_patterns(s).is_ok() {{ return Ok(Self::{}(text)); }}", type_ident(&simple.qname), variant);
                } else if matches!(
                    super::primitive_base(&branch.type_ref, ir),
                    TypeRef::Primitive(PrimitiveType::Date)
                ) {
                    let _ = writeln!(out, "        if polyxml::converters::ValueConverter::parse_scalar(&polyxml::ScalarType::XmlDate, s.as_bytes(), {:?}).is_ok() {{ return Ok(Self::{}(text)); }}", name, variant);
                } else {
                    let _ = writeln!(out, "        return Ok(Self::{}(text));", variant);
                }
            }
        }
        let _ = writeln!(out, "        Err(PolyXmlError::ScalarParseError {{ field: {:?}.into(), expected: \"union member\", value: s.into() }})", name);
        out.push_str("    }\n\n");
        let _ = writeln!(
            out,
            "    pub fn from_xml(xml: {}) -> Result<Self> {{",
            xml_arg
        );
        out.push_str("        let mut reader = Reader::from_str(xml);\n        loop {\n            match reader.read_event()? {\n                Event::Start(e) => return Self::decode_xml(&mut reader, &e),\n                Event::Empty(e) => return Self::decode_xml_empty(&e),\n                Event::Eof => return Err(PolyXmlError::SchemaError(\"Unexpected EOF while parsing union\".into())),\n                _ => {}\n            }\n        }\n    }\n\n");
        let _ = writeln!(
            out,
            "    pub fn from_xml_bytes(xml_bytes: {}) -> Result<Self> {{",
            bytes_arg
        );
        out.push_str("        Self::from_xml(std::str::from_utf8(xml_bytes)?)\n    }\n\n");
        let _ = writeln!(
            out,
            "    pub fn decode_xml(reader: {}, start: &BytesStart<'_>) -> Result<Self> {{",
            reader_arg
        );
        out.push_str("        let tag = start.local_name().as_ref().to_owned();\n        Self::parse_lexical(read_element_text(reader, &tag)?)\n    }\n\n");
        out.push_str("    pub fn decode_xml_empty(_start: &BytesStart<'_>) -> Result<Self> {\n        Self::parse_lexical(Default::default())\n    }\n\n");
        out.push_str("    pub fn to_xml(&self) -> Result<Vec<u8>> {\n        let mut buf = Vec::new();\n        let mut writer = Writer::new(std::io::Cursor::new(&mut buf));\n        self.encode_xml(&mut writer, None)?;\n        Ok(buf)\n    }\n\n");
        out.push_str("    pub fn to_xml_string(&self) -> Result<String> {\n        String::from_utf8(self.to_xml()?).map_err(|e| PolyXmlError::Utf8Error(e.utf8_error()))\n    }\n\n");
        if self.options.derive_serde {
            let json_arg = if borrowed { "&'a str" } else { "&str" };
            let _ = writeln!(out, "    pub fn from_json_str(json_str: {}) -> std::result::Result<Self, serde_json::Error> {{ serde_json::from_str(json_str) }}", json_arg);
            let _ = writeln!(out, "    pub fn from_json_slice(bytes: {}) -> std::result::Result<Self, serde_json::Error> {{ serde_json::from_slice(bytes) }}", bytes_arg);
            out.push_str("    pub fn to_json_string(&self) -> std::result::Result<String, serde_json::Error> { serde_json::to_string(self) }\n");
            out.push_str("    pub fn to_json_vec(&self) -> std::result::Result<Vec<u8>, serde_json::Error> { serde_json::to_vec(self) }\n\n");
        }
        out.push_str("    pub fn encode_xml<W: std::io::Write>(&self, writer: &mut Writer<W>, tag_name: Option<&str>) -> Result<()> {\n        let tag = tag_name.unwrap_or(");
        let _ = writeln!(out, "{:?});", name);
        out.push_str("        writer.write_event(Event::Start(BytesStart::new(tag)))?;\n        let text = match self {\n");
        for branch in &u.branches {
            let variant = to_rust_variant_identifier(&branch.variant_name);
            let expr = if self.field_is_enum(&branch.type_ref, ir).is_some() {
                "value.as_str().to_owned()"
            } else {
                "value.to_string()"
            };
            let _ = writeln!(out, "            Self::{}(value) => {},", variant, expr);
        }
        out.push_str("        };\n        writer.write_event(Event::Text(BytesText::new(&text)))?;\n        writer.write_event(Event::End(BytesEnd::new(tag)))?;\n        Ok(())\n    }\n}\n");
    }

    fn emit_pyo3_error_type(&self, out: &mut String) {
        out.push_str(r#"
#[derive(Debug)]
pub enum PolyXmlError {
    XmlSyntaxError { position: u64, message: String },
    Utf8Error(std::str::Utf8Error),
    ScalarParseError { field: String, expected: &'static str, value: String },
    SchemaError(String),
    UnexpectedRootElement { expected: String, actual: String },
    FacetViolation { field: String, expected: String, actual: String },
    SerializationError(String),
    MaxDepthExceeded { max_depth: usize, current: usize },
    XmlError(quick_xml::Error),
    AttrError(quick_xml::events::attributes::AttrError),
    EscapeError(quick_xml::escape::EscapeError),
    IoError(std::io::Error),
    JsonError(serde_json::Error),
}

impl std::fmt::Display for PolyXmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::XmlSyntaxError { position, message } => write!(f, "XML reader error at position {position}: {message}"),
            Self::Utf8Error(e) => write!(f, "Invalid UTF-8: {e}"),
            Self::ScalarParseError { field, expected, value } => write!(f, "Scalar parse error for field '{field}': expected {expected}, got '{value}'"),
            Self::SchemaError(s) => write!(f, "Schema error: {s}"),
            Self::UnexpectedRootElement { expected, actual } => write!(f, "Unexpected root element '{actual}', expected '{expected}'"),
            Self::FacetViolation { field, expected, actual } => write!(f, "Facet violation for field '{field}': expected {expected}, got '{actual}'"),
            Self::SerializationError(s) => write!(f, "Serialization error: {s}"),
            Self::MaxDepthExceeded { max_depth, current } => write!(f, "Maximum XML recursion depth exceeded: {current} >= {max_depth}"),
            Self::XmlError(e) => write!(f, "XML error: {e}"),
            Self::AttrError(e) => write!(f, "XML attribute error: {e}"),
            Self::EscapeError(e) => write!(f, "XML escape error: {e}"),
            Self::IoError(e) => write!(f, "I/O error: {e}"),
            Self::JsonError(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl std::error::Error for PolyXmlError {}

impl From<std::str::Utf8Error> for PolyXmlError {
    fn from(e: std::str::Utf8Error) -> Self { Self::Utf8Error(e) }
}
impl From<quick_xml::Error> for PolyXmlError {
    fn from(e: quick_xml::Error) -> Self { Self::XmlError(e) }
}
impl From<quick_xml::events::attributes::AttrError> for PolyXmlError {
    fn from(e: quick_xml::events::attributes::AttrError) -> Self { Self::AttrError(e) }
}
impl From<quick_xml::escape::EscapeError> for PolyXmlError {
    fn from(e: quick_xml::escape::EscapeError) -> Self { Self::EscapeError(e) }
}
impl From<std::io::Error> for PolyXmlError {
    fn from(e: std::io::Error) -> Self { Self::IoError(e) }
}
impl From<serde_json::Error> for PolyXmlError {
    fn from(e: serde_json::Error) -> Self { Self::JsonError(e) }
}

pub type Result<T> = std::result::Result<T, PolyXmlError>;
"#);
    }

    fn emit_struct_pymethods(
        &self,
        out: &mut String,
        s: &StructDef,
        has_boxed: bool,
        ir: &SchemaIR,
    ) {
        let struct_name = type_ident(&s.qname);
        let fields = flatten_fields(s, ir);

        let mut seen_fields = HashSet::new();
        let mut py_sig_parts = Vec::new();
        let mut py_param_parts = Vec::new();
        let mut py_init_fields = Vec::new();
        let mut boxed_accessors = Vec::new();

        for field in &fields {
            let rust_name = self.unique_rust_field_name(&field.name, &mut seen_fields);
            let raw_name = rust_name.trim_start_matches("r#").to_string();
            py_sig_parts.push(format!("{raw_name} = None"));

            let is_list = field.cardinality.is_list() || field.type_ref.is_list();
            let is_optional = field.cardinality.is_optional() || field.nillable;
            let is_boxed = field.is_cycle_cut || field.type_ref.is_boxed();
            let inner_ref = match &field.type_ref {
                TypeRef::List(inner) => inner.as_ref(),
                other => other,
            };
            let formatted_inner = self.format_rust_type_ref(inner_ref, &HashSet::new());

            if is_list {
                py_param_parts.push(format!("{raw_name}: Option<Vec<{formatted_inner}>>"));
                py_init_fields.push(format!("{rust_name}: {raw_name}.unwrap_or_default()"));
            } else if is_optional {
                py_param_parts.push(format!("{raw_name}: Option<{formatted_inner}>"));
                if is_boxed {
                    py_init_fields.push(format!("{rust_name}: {raw_name}.map(Box::new)"));
                } else {
                    py_init_fields.push(format!("{rust_name}: {raw_name}"));
                }
            } else {
                py_param_parts.push(format!("{raw_name}: Option<{formatted_inner}>"));
                if is_boxed {
                    py_init_fields.push(format!(
                        "{rust_name}: Box::new({raw_name}.unwrap_or_default())"
                    ));
                } else {
                    py_init_fields.push(format!("{rust_name}: {raw_name}.unwrap_or_default()"));
                }
            }

            if has_boxed && is_boxed {
                if is_optional {
                    boxed_accessors.push(format!(
                        "    #[getter]\n    pub fn {raw_name}(&self) -> Option<{formatted_inner}> {{\n        self.{rust_name}.as_ref().map(|b| (**b).clone())\n    }}\n\n    #[setter]\n    pub fn set_{raw_name}(&mut self, val: Option<{formatted_inner}>) {{\n        self.{rust_name} = val.map(Box::new);\n    }}"
                    ));
                } else {
                    boxed_accessors.push(format!(
                        "    #[getter]\n    pub fn {raw_name}(&self) -> {formatted_inner} {{\n        (*self.{rust_name}).clone()\n    }}\n\n    #[setter]\n    pub fn set_{raw_name}(&mut self, val: {formatted_inner}) {{\n        self.{rust_name} = Box::new(val);\n    }}"
                    ));
                }
            }
        }

        let _ = writeln!(out, "\n#[pymethods]\nimpl {} {{", struct_name);

        let sig = py_sig_parts.join(", ");
        let params = py_param_parts.join(", ");
        let inits = py_init_fields.join(",\n            ");

        let _ = writeln!(out, "    #[new]");
        let _ = writeln!(out, "    #[pyo3(signature = ({}))]", sig);
        let _ = writeln!(out, "    pub fn py_new({}) -> Self {{", params);
        if inits.is_empty() {
            let _ = writeln!(out, "        Self {{}}");
        } else {
            let _ = writeln!(out, "        Self {{\n            {}\n        }}", inits);
        }
        let _ = writeln!(out, "    }}\n");

        for accessor in boxed_accessors {
            let _ = writeln!(out, "{accessor}\n");
        }

        out.push_str("    #[pyo3(name = \"to_xml\")]\n");
        out.push_str("    pub fn py_to_xml(&self) -> pyo3::PyResult<String> {\n");
        out.push_str("        self.to_xml_string().map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))\n");
        out.push_str("    }\n\n");

        out.push_str("    #[staticmethod]\n");
        out.push_str("    #[pyo3(name = \"from_xml\")]\n");
        out.push_str("    pub fn py_from_xml(xml: &str) -> pyo3::PyResult<Self> {\n");
        out.push_str("        Self::from_xml(xml).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))\n");
        out.push_str("    }\n\n");

        out.push_str("    #[pyo3(name = \"to_json\")]\n");
        out.push_str("    pub fn py_to_json(&self) -> pyo3::PyResult<String> {\n");
        out.push_str("        self.to_json_string().map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))\n");
        out.push_str("    }\n\n");

        out.push_str("    #[staticmethod]\n");
        out.push_str("    #[pyo3(name = \"from_json\")]\n");
        out.push_str("    pub fn py_from_json(json_str: &str) -> pyo3::PyResult<Self> {\n");
        out.push_str("        Self::from_json_str(json_str).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))\n");
        out.push_str("    }\n\n");

        out.push_str("    fn __repr__(&self) -> String {\n");
        out.push_str("        format!(\"{:?}\", self)\n");
        out.push_str("    }\n");

        out.push_str("}\n");
    }

    fn emit_pymodule(&self, out: &mut String, _ir: &SchemaIR, sorted_types: &[&TypeDef]) {
        let mod_name = self.options.pyo3_module_name.as_deref().unwrap_or("models");
        let _ = writeln!(out, "\n#[pymodule]");
        let _ = writeln!(
            out,
            "fn {}(m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()> {{",
            mod_name
        );
        for td in sorted_types {
            match td {
                TypeDef::Struct(s) => {
                    let struct_name = type_ident(&s.qname);
                    let _ = writeln!(out, "    m.add_class::<{}>()?;", struct_name);
                }
                TypeDef::Enum(e) => {
                    let enum_name = type_ident(&e.qname);
                    let _ = writeln!(out, "    m.add_class::<{}>()?;", enum_name);
                }
                _ => {}
            }
        }
        out.push_str("    Ok(())\n");
        out.push_str("}\n");
    }
}
