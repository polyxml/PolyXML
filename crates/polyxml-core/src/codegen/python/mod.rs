use std::collections::{BTreeSet, HashSet};
use std::fmt::Write as FmtWrite;

use heck::{AsPascalCase, AsShoutySnakeCase, AsSnakeCase};
use serde::{Deserialize, Serialize};

use crate::codegen::{
    build_type_name_map, lookup_type_name, normalize_symbol_name, sanitize_keyword,
    set_type_name_map, LanguageContext,
};
use crate::ir::{
    EnumDef, FieldDef, FieldKind, PrimitiveType, QName, RestrictionFacets, SchemaIR, SimpleTypeDef,
    StructDef, TypeDef, TypeRef, UnionDef,
};

/// Module-level helper emitted when a simple type carries multiple pattern
/// facets: pydantic's `Field(pattern=...)` keeps only a single regex, so
/// AND-combined patterns are enforced through an `AfterValidator` wrapping
/// this factory instead.
const PATTERN_VALIDATOR_HELPER: &str = r#"def _polyxml_patterns(*patterns: str):
    _compiled = tuple(re.compile(p) for p in patterns)

    def _validate(value: str) -> str:
        for regex in _compiled:
            if regex.fullmatch(value) is None:
                raise ValueError(f"string_pattern_mismatch: {regex.pattern!r}")
        return value

    return _validate
"#;

pub mod aot;
pub use aot::{PythonAotCodegen, PythonAotCrate, PythonAotOptions};

/// Target backend for Python code emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PythonBackend {
    #[default]
    Dataclass,
    Pydantic,
    Aot,
}

impl PythonBackend {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "dataclass" | "dataclasses" | "std" | "stdlib" => Some(Self::Dataclass),
            "pydantic" | "pydantic_v2" | "pydantic-v2" | "pydantic2" => Some(Self::Pydantic),
            "aot" | "native" | "pyo3" => Some(Self::Aot),
            _ => None,
        }
    }
}

/// Options configuring Python 3.12+ code generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonOptions {
    pub backend: PythonBackend,
    pub slots: bool,
    pub kw_only: bool,
    pub pep695_aliases: bool,
    pub emit_meta: bool,
    pub emit_root_aliases: bool,
    pub emit_codecs: bool,
    pub emit_json_metadata: bool,
    pub custom_header: Option<String>,
}

impl Default for PythonOptions {
    fn default() -> Self {
        Self {
            backend: PythonBackend::Dataclass,
            slots: true,
            kw_only: true,
            pep695_aliases: true,
            emit_meta: true,
            emit_root_aliases: true,
            emit_codecs: true,
            emit_json_metadata: true,
            custom_header: None,
        }
    }
}

/// Language context adapter for Python 3.12+.
pub struct PythonLanguageContext;

impl LanguageContext for PythonLanguageContext {
    fn target_language(&self) -> &'static str {
        "python"
    }

    fn map_primitive(&self, prim: PrimitiveType) -> &'static str {
        match prim {
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
            | PrimitiveType::QName => "str",

            PrimitiveType::Boolean => "bool",

            PrimitiveType::Decimal => "Decimal",

            PrimitiveType::Float | PrimitiveType::Double => "float",

            PrimitiveType::Integer
            | PrimitiveType::NegativeInteger
            | PrimitiveType::NonNegativeInteger
            | PrimitiveType::PositiveInteger
            | PrimitiveType::NonPositiveInteger
            | PrimitiveType::Long
            | PrimitiveType::Int
            | PrimitiveType::Short
            | PrimitiveType::Byte
            | PrimitiveType::UnsignedLong
            | PrimitiveType::UnsignedInt
            | PrimitiveType::UnsignedShort
            | PrimitiveType::UnsignedByte => "int",

            PrimitiveType::HexBinary | PrimitiveType::Base64Binary => "bytes",

            PrimitiveType::AnyType | PrimitiveType::AnySimpleType => "object",
        }
    }

    fn map_type_ref(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(prim) => self.map_primitive(*prim).to_string(),
            TypeRef::Named(qname) => type_ident(qname),
            TypeRef::Boxed(inner) => self.map_type_ref(inner),
            TypeRef::List(inner) => format!("list[{}]", self.map_type_ref(inner)),
        }
    }
}

/// Converts an arbitrary string into a safe, valid Python enum identifier.
pub fn to_enum_identifier(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.is_empty() {
        return "EMPTY".to_string();
    }

    let normalized = normalize_symbol_name(trimmed);
    let cleaned: String = normalized
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();

    let shouty = AsShoutySnakeCase(&cleaned).to_string();
    let identifier = if shouty.is_empty() {
        "EMPTY".to_string()
    } else {
        shouty
    };

    let with_prefix = if identifier
        .chars()
        .next()
        .map(|c| !c.is_ascii_alphabetic() && c != '_')
        .unwrap_or(true)
    {
        format!("VALUE_{}", identifier)
    } else {
        identifier
    };

    match with_prefix.as_str() {
        "NONE" | "TRUE" | "FALSE" => with_prefix,
        _ => sanitize_keyword(&with_prefix, "python"),
    }
}

/// Sanitizes a field identifier, ensuring it does not start with a digit and is not a keyword.
pub fn to_field_identifier(name: &str) -> String {
    let snake = AsSnakeCase(name).to_string();
    let safe_name = if snake
        .chars()
        .next()
        .map(|c| !c.is_ascii_alphabetic() && c != '_')
        .unwrap_or(true)
    {
        format!("_{}", snake)
    } else if snake.is_empty() {
        "value".to_string()
    } else {
        snake
    };

    sanitize_keyword(&safe_name, "python")
}

/// Pure-Rust Python 3.12+ code generator emitting dataclasses or Pydantic v2 models.
pub struct PythonCodegen {
    options: PythonOptions,
    context: PythonLanguageContext,
}

/// Emitted Python identifier for a named type, disambiguated across
/// namespaces for the IR currently being generated.
fn type_ident(q: &QName) -> String {
    lookup_type_name(q, || AsPascalCase(&q.local).to_string())
}

impl PythonCodegen {
    pub fn new(options: PythonOptions) -> Self {
        Self {
            options,
            context: PythonLanguageContext,
        }
    }

    pub fn generate_module(&self, ir: &SchemaIR) -> String {
        if self.options.backend == PythonBackend::Aot {
            let aot = PythonAotCodegen::new(PythonAotOptions {
                module_name: "models".to_string(),
                custom_header: self.options.custom_header.clone(),
            });
            return aot.generate_crate(ir).lib_rs;
        }

        set_type_name_map(build_type_name_map(ir, |local| {
            AsPascalCase(local).to_string()
        }));
        let mut out = String::new();

        if let Some(ref header) = self.options.custom_header {
            let trimmed = header.trim();
            if !trimmed.is_empty() {
                for line in trimmed.lines() {
                    let line_trimmed = line.trim_start();
                    if line_trimmed.starts_with("//") {
                        let content = line_trimmed.strip_prefix("//").unwrap_or("");
                        out.push('#');
                        out.push_str(content);
                        out.push('\n');
                    } else {
                        out.push_str(line);
                        out.push('\n');
                    }
                }
                out.push('\n');
            }
        }

        out.push_str("# @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)\n");
        out.push_str("from __future__ import annotations\n\n");

        self.emit_imports(&mut out, ir);

        if self.needs_pattern_validator(ir) {
            out.push('\n');
            out.push_str(PATTERN_VALIDATOR_HELPER);
        }

        // Sort types topologically (base types before derived types)
        let sorted_types = self.order_types(ir);

        for type_def in sorted_types {
            out.push('\n');
            match type_def {
                TypeDef::Simple(s) => self.emit_simple_type(&mut out, s),
                TypeDef::Enum(e) => self.emit_enum(&mut out, e),
                TypeDef::Union(u) => self.emit_union(&mut out, u),
                TypeDef::Struct(s) => self.emit_struct(&mut out, s, ir),
            }
        }

        if self.options.emit_root_aliases {
            self.emit_root_aliases(&mut out, ir);
        }

        out
    }

    fn emit_imports(&self, out: &mut String, ir: &SchemaIR) {
        let mut has_structs = false;
        let mut has_enums = false;
        let mut has_decimal = false;
        let mut has_annotated = false;

        for type_def in ir.types.values() {
            match type_def {
                TypeDef::Struct(_) => has_structs = true,
                TypeDef::Enum(_) => has_enums = true,
                TypeDef::Simple(s) => {
                    if self.options.backend == PythonBackend::Pydantic && !s.facets.is_empty() {
                        has_annotated = true;
                    }
                    if let TypeRef::Primitive(PrimitiveType::Decimal) = s.base_type {
                        has_decimal = true;
                    }
                }
                TypeDef::Union(_) => {}
            }

            if let TypeDef::Struct(s) = type_def {
                for f in &s.fields {
                    if let TypeRef::Primitive(PrimitiveType::Decimal) = &f.type_ref {
                        has_decimal = true;
                    }
                    if self.options.backend == PythonBackend::Pydantic
                        && f.facets
                            .as_ref()
                            .map(|fac| !fac.is_empty())
                            .unwrap_or(false)
                    {
                        has_annotated = true;
                    }
                }
            }
        }

        for elem in ir.elements.values() {
            if let TypeRef::Primitive(PrimitiveType::Decimal) = &elem.type_ref {
                has_decimal = true;
            }
        }

        if self.options.backend == PythonBackend::Dataclass && has_structs {
            out.push_str("from dataclasses import dataclass, field\n");
        }
        if has_decimal {
            out.push_str("from decimal import Decimal\n");
        }
        if has_enums {
            out.push_str("from enum import StrEnum\n");
        }
        if self.needs_pattern_validator(ir) {
            out.push_str("import re\n");
        }
        let mut typing_imports = Vec::new();
        if has_annotated {
            typing_imports.push("Annotated");
        }
        if has_structs && self.options.emit_codecs {
            typing_imports.push("Self");
        }
        if !typing_imports.is_empty() {
            let _ = writeln!(out, "from typing import {}", typing_imports.join(", "));
        }
        let av_import = if self.needs_pattern_validator(ir) {
            "AfterValidator, "
        } else {
            ""
        };
        if self.options.backend == PythonBackend::Pydantic && has_structs {
            let _ = writeln!(
                out,
                "from pydantic import {}BaseModel, ConfigDict, Field",
                av_import
            );
        } else if self.options.backend == PythonBackend::Pydantic && has_annotated && !has_structs {
            let _ = writeln!(out, "from pydantic import {}Field", av_import);
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

    fn emit_simple_type(&self, out: &mut String, s: &SimpleTypeDef) {
        let type_name = type_ident(&s.qname);
        let base_type = self.context.map_type_ref(&s.base_type);

        if let Some(ref doc) = s.documentation {
            let _ = writeln!(out, "# {}", doc.trim());
        }

        if self.options.backend == PythonBackend::Pydantic && !s.facets.is_empty() {
            let facet_args = self.format_pydantic_facets(&s.facets);
            let after = self.pattern_after_validator(&s.facets);
            if !facet_args.is_empty() || after.is_some() {
                let mut annotations = vec![base_type.clone()];
                if !facet_args.is_empty() {
                    annotations.push(format!("Field({facet_args})"));
                }
                if let Some(av) = after {
                    annotations.push(av);
                }
                let _ = writeln!(
                    out,
                    "type {} = Annotated[{}]",
                    type_name,
                    annotations.join(", ")
                );
                return;
            }
        }

        let _ = writeln!(out, "type {} = {}", type_name, base_type);
    }

    fn emit_enum(&self, out: &mut String, e: &EnumDef) {
        let enum_name = type_ident(&e.qname);
        let _ = writeln!(out, "class {}(StrEnum):", enum_name);

        if let Some(ref doc) = e.documentation {
            let _ = writeln!(out, "    \"\"\"{}\"\"\"", doc.trim());
        }

        if e.variants.is_empty() {
            out.push_str("    pass\n");
            return;
        }

        let mut seen_names = HashSet::new();
        for variant in &e.variants {
            let mut var_id = to_enum_identifier(&variant.name);
            let mut counter = 1;
            while seen_names.contains(&var_id) {
                counter += 1;
                var_id = format!("{}_{}", to_enum_identifier(&variant.name), counter);
            }
            seen_names.insert(var_id.clone());

            let _ = writeln!(
                out,
                "    {} = \"{}\"",
                var_id,
                variant.value.replace('"', "\\\"")
            );
        }
    }

    fn emit_union(&self, out: &mut String, u: &UnionDef) {
        let union_name = type_ident(&u.qname);

        if let Some(ref doc) = u.documentation {
            let _ = writeln!(out, "# {}", doc.trim());
        }

        if u.branches.is_empty() {
            let _ = writeln!(out, "type {} = object", union_name);
            return;
        }

        let branch_types: Vec<String> = u
            .branches
            .iter()
            .map(|b| self.context.map_type_ref(&b.type_ref))
            .collect();

        let _ = writeln!(out, "type {} = {}", union_name, branch_types.join(" | "));
    }

    fn emit_struct(&self, out: &mut String, s: &StructDef, ir: &SchemaIR) {
        let class_name = type_ident(&s.qname);

        let struct_base = s
            .base_type
            .as_ref()
            .filter(|b| {
                if let Some(TypeDef::Struct(_)) = ir.find_type(b) {
                    *b != &s.qname
                } else {
                    false
                }
            })
            .map(type_ident);

        match self.options.backend {
            PythonBackend::Dataclass | PythonBackend::Aot => {
                let slots_arg = if self.options.slots {
                    "slots=True"
                } else {
                    "slots=False"
                };
                let kw_arg = if self.options.kw_only {
                    "kw_only=True"
                } else {
                    "kw_only=False"
                };
                let _ = writeln!(out, "@dataclass({}, {})", slots_arg, kw_arg);

                if let Some(ref base_name) = struct_base {
                    let _ = writeln!(out, "class {}({}):", class_name, base_name);
                } else {
                    let _ = writeln!(out, "class {}:", class_name);
                }
            }
            PythonBackend::Pydantic => {
                if let Some(ref base_name) = struct_base {
                    let _ = writeln!(out, "class {}({}):", class_name, base_name);
                } else {
                    let _ = writeln!(out, "class {}(BaseModel):", class_name);
                }
            }
        }

        let mut has_body = false;

        if let Some(ref doc) = s.documentation {
            let _ = writeln!(out, "    \"\"\"{}\"\"\"\n", doc.trim());
            has_body = true;
        }

        if self.options.backend == PythonBackend::Pydantic {
            out.push_str(
                "    model_config = ConfigDict(defer_build=True, populate_by_name=True)\n",
            );
            has_body = true;
        }

        if self.options.emit_meta {
            if has_body {
                out.push('\n');
            }
            out.push_str("    class Meta:\n");
            let _ = writeln!(out, "        name = \"{}\"", s.qname.local);
            if let Some(ref ns) = s.qname.namespace {
                let _ = writeln!(out, "        namespace = \"{}\"", ns);
            }
            // Marks the type as a dispatch base: the runtime raises a clear
            // error when xsi:type targets an abstract type with no registered
            // derivations.
            if s.is_abstract {
                out.push_str("        abstract = True\n");
            }
            has_body = true;
        }

        if !s.fields.is_empty() {
            if has_body {
                out.push('\n');
            }

            let mut seen_fields = HashSet::new();
            for field in &s.fields {
                let py_field_name = self.unique_field_name(&field.name, &mut seen_fields);
                self.emit_field(out, field, &py_field_name);
            }
            has_body = true;
        }

        if self.options.emit_codecs {
            if has_body {
                out.push('\n');
            }
            out.push_str("    @classmethod\n");
            out.push_str("    def from_xml(cls, data: bytes | str) -> Self:\n");
            out.push_str("        \"\"\"Deserialize XML bytes or string into this model.\"\"\"\n");
            out.push_str("        import polyxml\n");
            out.push_str(
                "        raw_bytes = data.encode(\"utf-8\") if isinstance(data, str) else data\n",
            );
            out.push_str("        return polyxml.deserialize(raw_bytes, cls)\n\n");
            out.push_str("    def to_xml(\n");
            out.push_str("        self,\n");
            out.push_str("        *,\n");
            out.push_str("        indent: int | None = None,\n");
            out.push_str("        namespaces: bool | None = None,\n");
            out.push_str("        ns_map: dict[str, str] | None = None,\n");
            out.push_str("    ) -> bytes:\n");
            out.push_str("        \"\"\"Serialize this model instance into XML bytes.\"\"\"\n");
            out.push_str("        import polyxml\n");
            out.push_str("        return polyxml.serialize(self, indent=indent, namespaces=namespaces, ns_map=ns_map)\n\n");
            out.push_str("    @classmethod\n");
            out.push_str("    def from_json(cls, data: bytes | str) -> Self:\n");
            out.push_str("        \"\"\"Deserialize JSON bytes or string into this model.\"\"\"\n");
            out.push_str("        import polyxml\n");
            out.push_str("        return polyxml.deserialize_json(data, cls)\n\n");
            out.push_str("    def to_json(\n");
            out.push_str("        self,\n");
            out.push_str("        *,\n");
            out.push_str("        indent: int | None = None,\n");
            out.push_str("        by_alias: bool = True,\n");
            out.push_str("    ) -> bytes:\n");
            out.push_str("        \"\"\"Serialize this model instance into JSON bytes.\"\"\"\n");
            out.push_str("        import polyxml\n");
            out.push_str(
                "        return polyxml.serialize_json(self, indent=indent, by_alias=by_alias)\n",
            );
            has_body = true;
        }

        if !has_body {
            out.push_str("    pass\n");
        }
    }

    fn unique_field_name(&self, name: &str, seen: &mut HashSet<String>) -> String {
        let mut candidate = to_field_identifier(name);
        let mut counter = 1;
        while seen.contains(&candidate) {
            counter += 1;
            candidate = format!("{}_{}", to_field_identifier(name), counter);
        }
        seen.insert(candidate.clone());
        candidate
    }

    fn emit_field(&self, out: &mut String, field: &FieldDef, py_name: &str) {
        let is_list = field.cardinality.is_list() || field.type_ref.is_list();
        let unwrapped_ref = match &field.type_ref {
            TypeRef::List(inner) => inner.as_ref(),
            other => other,
        };
        let inner_type = self.context.map_type_ref(unwrapped_ref);
        let meta_dict = self.build_field_metadata(field);

        let (field_type, field_call) = match self.options.backend {
            PythonBackend::Dataclass | PythonBackend::Aot => {
                self.format_dataclass_field(field, is_list, &inner_type, &meta_dict)
            }
            PythonBackend::Pydantic => {
                self.format_pydantic_field(field, py_name, is_list, &inner_type, &meta_dict)
            }
        };

        if let Some(ref doc) = field.documentation {
            let _ = writeln!(out, "    # {}", doc.trim());
        }

        let _ = writeln!(out, "    {}: {} = {}", py_name, field_type, field_call);
    }

    fn format_dataclass_field(
        &self,
        field: &FieldDef,
        is_list: bool,
        base_type: &str,
        meta_dict: &str,
    ) -> (String, String) {
        if is_list {
            (
                format!("list[{}]", base_type),
                format!("field(default_factory=list, metadata={})", meta_dict),
            )
        } else if field.cardinality.is_optional() || field.nillable {
            if let Some(ref def) = field.default_value {
                let py_val = self.format_default_value(def, &field.type_ref);
                (
                    format!("{} | None", base_type),
                    format!("field(default={}, metadata={})", py_val, meta_dict),
                )
            } else {
                (
                    format!("{} | None", base_type),
                    format!("field(default=None, metadata={})", meta_dict),
                )
            }
        } else if let Some(ref def) = field.default_value {
            let py_val = self.format_default_value(def, &field.type_ref);
            (
                base_type.to_string(),
                format!("field(default={}, metadata={})", py_val, meta_dict),
            )
        } else {
            (
                base_type.to_string(),
                format!("field(metadata={})", meta_dict),
            )
        }
    }

    fn format_pydantic_field(
        &self,
        field: &FieldDef,
        py_name: &str,
        is_list: bool,
        base_type: &str,
        meta_dict: &str,
    ) -> (String, String) {
        let facet_args = field
            .facets
            .as_ref()
            .map(|f| self.format_pydantic_facets(f))
            .unwrap_or_default();

        let mut clauses = Vec::new();
        if self.options.emit_json_metadata && py_name != field.xml_name {
            clauses.push(format!("alias=\"{}\"", field.xml_name));
            clauses.push(format!("serialization_alias=\"{}\"", field.xml_name));
        }
        clauses.push(format!("json_schema_extra={}", meta_dict));
        if !facet_args.is_empty() {
            clauses.push(facet_args);
        }
        let extra_clause = clauses.join(", ");

        if is_list {
            (
                format!("list[{}]", base_type),
                format!("Field(default_factory=list, {})", extra_clause),
            )
        } else if field.cardinality.is_optional() || field.nillable {
            if let Some(ref def) = field.default_value {
                let py_val = self.format_default_value(def, &field.type_ref);
                (
                    format!("{} | None", base_type),
                    format!("Field(default={}, {})", py_val, extra_clause),
                )
            } else {
                (
                    format!("{} | None", base_type),
                    format!("Field(default=None, {})", extra_clause),
                )
            }
        } else if let Some(ref def) = field.default_value {
            let py_val = self.format_default_value(def, &field.type_ref);
            (
                base_type.to_string(),
                format!("Field(default={}, {})", py_val, extra_clause),
            )
        } else {
            (
                base_type.to_string(),
                format!("Field(..., {})", extra_clause),
            )
        }
    }

    fn build_field_metadata(&self, field: &FieldDef) -> String {
        let kind_str = match field.kind {
            FieldKind::Element => "Element",
            FieldKind::Attribute => "Attribute",
            FieldKind::Text => "Text",
            FieldKind::Any | FieldKind::AnyAttribute => "Wildcard",
        };

        let mut parts = Vec::new();
        parts.push(format!("\"type\": \"{}\"", kind_str));
        parts.push(format!("\"name\": \"{}\"", field.xml_name));
        if self.options.emit_json_metadata {
            parts.push(format!("\"json_name\": \"{}\"", field.xml_name));
        }

        if let Some(ref ns) = field.namespace {
            parts.push(format!("\"namespace\": \"{}\"", ns));
        }

        if field.nillable {
            parts.push("\"nillable\": True".to_string());
        }

        format!("{{{}}}", parts.join(", "))
    }

    fn format_default_value(&self, val: &str, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(PrimitiveType::Boolean) => match val.trim() {
                "true" | "1" => "True".to_string(),
                "false" | "0" => "False".to_string(),
                _ => "False".to_string(),
            },
            TypeRef::Primitive(
                PrimitiveType::Integer
                | PrimitiveType::Int
                | PrimitiveType::Long
                | PrimitiveType::Short
                | PrimitiveType::Byte
                | PrimitiveType::NonNegativeInteger
                | PrimitiveType::PositiveInteger
                | PrimitiveType::NonPositiveInteger
                | PrimitiveType::NegativeInteger
                | PrimitiveType::UnsignedLong
                | PrimitiveType::UnsignedInt
                | PrimitiveType::UnsignedShort
                | PrimitiveType::UnsignedByte,
            ) => {
                if val.trim().parse::<i64>().is_ok() {
                    val.trim().to_string()
                } else {
                    format!("\"{}\"", val.replace('"', "\\\""))
                }
            }
            TypeRef::Primitive(PrimitiveType::Float | PrimitiveType::Double) => {
                if val.trim().parse::<f64>().is_ok() {
                    val.trim().to_string()
                } else {
                    format!("\"{}\"", val.replace('"', "\\\""))
                }
            }
            TypeRef::Primitive(PrimitiveType::Decimal) => {
                format!("Decimal(\"{}\")", val.replace('"', "\\\""))
            }
            _ => format!("\"{}\"", val.replace('"', "\\\"")),
        }
    }

    /// True when any simple type carries multiple pattern facets, requiring
    /// the module-level `_polyxml_patterns` validator helper (pydantic only —
    /// `Field(pattern=...)` can hold a single regex).
    fn needs_pattern_validator(&self, ir: &SchemaIR) -> bool {
        self.options.backend == PythonBackend::Pydantic
            && ir
                .types
                .values()
                .any(|td| matches!(td, TypeDef::Simple(s) if s.facets.patterns.len() > 1))
    }

    /// Pydantic keeps only one `pattern` constraint, so multiple patterns are
    /// AND-combined through an `AfterValidator` over the shared helper.
    fn pattern_after_validator(&self, facets: &RestrictionFacets) -> Option<String> {
        if facets.patterns.len() <= 1 {
            return None;
        }
        let pats: Vec<String> = facets
            .patterns
            .iter()
            .map(|p| format!("r\"{p}\""))
            .collect();
        Some(format!(
            "AfterValidator(_polyxml_patterns({}))",
            pats.join(", ")
        ))
    }

    fn format_pydantic_facets(&self, facets: &RestrictionFacets) -> String {
        let mut clauses = Vec::new();

        if let Some(ref min_inc) = facets.min_inclusive {
            clauses.push(format!("ge={}", min_inc));
        }
        if let Some(ref max_inc) = facets.max_inclusive {
            clauses.push(format!("le={}", max_inc));
        }
        if let Some(ref min_exc) = facets.min_exclusive {
            clauses.push(format!("gt={}", min_exc));
        }
        if let Some(ref max_exc) = facets.max_exclusive {
            clauses.push(format!("lt={}", max_exc));
        }
        if let Some(min_l) = facets.min_length {
            clauses.push(format!("min_length={}", min_l));
        }
        if let Some(max_l) = facets.max_length {
            clauses.push(format!("max_length={}", max_l));
        }
        if let Some(l) = facets.length {
            clauses.push(format!("min_length={}, max_length={}", l, l));
        }
        if let Some(pat) = facets.patterns.first() {
            clauses.push(format!("pattern=r\"\\A(?:{})\\z\"", pat));
        }

        clauses.join(", ")
    }

    fn emit_root_aliases(&self, out: &mut String, ir: &SchemaIR) {
        let mut declared_names = BTreeSet::new();
        for td in ir.types.values() {
            declared_names.insert(type_ident(td.qname()));
        }

        for element in ir.elements.values() {
            let el_name = AsPascalCase(&element.qname.local).to_string();
            if !declared_names.contains(&el_name) {
                let target_type = self.context.map_type_ref(&element.type_ref);
                if el_name != target_type {
                    let _ = writeln!(out, "\ntype {} = {}", el_name, target_type);
                    declared_names.insert(el_name);
                }
            }
        }
    }
}
