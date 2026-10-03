use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as FmtWrite;

use heck::{AsPascalCase, AsSnakeCase};
use serde::{Deserialize, Serialize};

use crate::codegen::{
    build_type_name_map, lookup_type_name, normalize_symbol_name, sanitize_keyword,
    set_type_name_map, LanguageContext,
};
use crate::ir::{
    EnumDef, PrimitiveType, QName, RestrictionFacets, SchemaIR, SimpleTypeDef, StructDef, TypeDef,
    TypeRef, UnionDef,
};

/// Target packaging and compilation mode for C++ codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CppMode {
    /// Standard header-only library (`.hpp`)
    #[default]
    HeaderOnly,
    /// C++20 Module Interface Unit (`.cppm`)
    Module,
}

impl CppMode {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "header" | "headeronly" | "header-only" | "headers" | "hpp" => Some(Self::HeaderOnly),
            "module" | "modules" | "cppm" | "c++20-modules" => Some(Self::Module),
            _ => None,
        }
    }
}

/// Target backend for C++ serialization/reflection metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CppBackend {
    /// Zero-dependency standard C++20 (default)
    #[default]
    Standard,
    /// Glaze compile-time reflectionless serde (`glz::meta`)
    Glaze,
}

impl CppBackend {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().trim() {
            "standard" | "std" | "default" | "none" => Some(Self::Standard),
            "glaze" | "glz" => Some(Self::Glaze),
            _ => None,
        }
    }

    pub fn is_glaze(self) -> bool {
        matches!(self, Self::Glaze)
    }
}

/// Options configuring modern C++20/C++23 code generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppOptions {
    /// C++ namespace for generated types (default: "polyxml::generated")
    pub namespace: String,
    /// Header-only (.hpp) or C++20 Module (.cppm) mode (default: HeaderOnly)
    pub mode: CppMode,
    /// Target serialization and metadata backend (default: Standard)
    pub backend: CppBackend,
    /// Standard language dialect (default: "c++20")
    pub standard: String,
    /// Emit C++20 defaulted equality operators `bool operator==(const T&) const = default;` (default: true)
    pub emit_equality_operators: bool,
    /// Emit string conversion helpers for enums (`to_string`, `from_string`) (default: true)
    pub emit_enum_converters: bool,
    /// Emit constraint validation helper methods enforcing XSD restriction facets (default: true)
    pub validate_facets: bool,
    /// Emit top-level type aliases for root elements (default: true)
    pub emit_root_aliases: bool,
    /// Emit CMake integration files (CMakeLists.txt and PolyXMLConfig.cmake) (default: false)
    pub emit_cmake: bool,
    /// Emit Meson build definition (meson.build) (default: false)
    pub emit_meson: bool,
    /// Custom header text to prepend to generated files (default: None)
    pub custom_header: Option<String>,
}

impl Default for CppOptions {
    fn default() -> Self {
        Self {
            namespace: "polyxml::generated".to_string(),
            mode: CppMode::HeaderOnly,
            backend: CppBackend::Standard,
            standard: "c++20".to_string(),
            emit_equality_operators: true,
            emit_enum_converters: true,
            validate_facets: true,
            emit_root_aliases: true,
            emit_cmake: false,
            emit_meson: false,
            custom_header: None,
        }
    }
}

/// Language context adapter for modern C++20/C++23.
pub struct CppLanguageContext;

impl LanguageContext for CppLanguageContext {
    fn target_language(&self) -> &'static str {
        "cpp"
    }

    fn map_primitive(&self, prim: PrimitiveType) -> &'static str {
        match prim {
            PrimitiveType::Integer
            | PrimitiveType::PositiveInteger
            | PrimitiveType::NegativeInteger
            | PrimitiveType::NonPositiveInteger
            | PrimitiveType::NonNegativeInteger => "std::string",
            PrimitiveType::Boolean => "bool",
            PrimitiveType::Float => "float",
            PrimitiveType::Double | PrimitiveType::Decimal => "double",
            PrimitiveType::Byte => "std::int8_t",
            PrimitiveType::Short => "std::int16_t",
            PrimitiveType::Int => "std::int32_t",
            PrimitiveType::Long => "std::int64_t",
            PrimitiveType::UnsignedByte => "std::uint8_t",
            PrimitiveType::UnsignedShort => "std::uint16_t",
            PrimitiveType::UnsignedInt => "std::uint32_t",
            PrimitiveType::UnsignedLong => "std::uint64_t",
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
            | PrimitiveType::Date
            | PrimitiveType::Time
            | PrimitiveType::DateTime
            | PrimitiveType::Duration => "std::string",
            PrimitiveType::Base64Binary | PrimitiveType::HexBinary => "std::vector<std::uint8_t>",
            PrimitiveType::AnyType | PrimitiveType::AnySimpleType => "std::string",
        }
    }

    fn map_type_ref(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(prim) => self.map_primitive(*prim).to_string(),
            TypeRef::Named(qname) => type_ident(qname),
            TypeRef::Boxed(inner) => format!("std::unique_ptr<{}>", self.map_type_ref(inner)),
            TypeRef::List(inner) => format!("std::vector<{}>", self.map_type_ref(inner)),
        }
    }
}

/// Sanitizes a string into a valid C++ namespace path (`part1::part2`).
pub fn to_cpp_namespace(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "polyxml::generated".to_string();
    }

    // Split on '.', '/', or '::'
    let parts: Vec<String> = trimmed
        .replace(['/', '.'], "::")
        .split("::")
        .filter(|p| !p.is_empty())
        .map(|p| {
            let snake = AsSnakeCase(p).to_string();
            let safe = if snake.starts_with(|c: char| c.is_ascii_digit()) {
                format!("_{}", snake)
            } else {
                snake
            };
            sanitize_keyword(&safe, "cpp")
        })
        .collect();

    if parts.is_empty() {
        "polyxml::generated".to_string()
    } else {
        parts.join("::")
    }
}

/// Converts a raw identifier into a safe PascalCase C++ type name.
pub fn to_cpp_type_name(raw: &str) -> String {
    let pascal = AsPascalCase(raw).to_string();
    let safe = if pascal.is_empty() {
        "Type".to_string()
    } else if pascal.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Type_{}", pascal)
    } else {
        pascal
    };
    sanitize_keyword(&safe, "cpp")
}

/// Converts a raw identifier into a safe snake_case C++ member variable or parameter name.
pub fn to_cpp_field_name(raw: &str) -> String {
    let snake = AsSnakeCase(raw).to_string();
    let safe = if snake.is_empty() {
        "field".to_string()
    } else if snake.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{}", snake)
    } else {
        snake
    };
    sanitize_keyword(&safe, "cpp")
}

/// Converts an enumeration variant raw value into a safe C++ scoped enum identifier.
pub fn to_cpp_enum_variant(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "Unknown".to_string();
    }
    let normalized = normalize_symbol_name(raw);
    let pascal = AsPascalCase(&normalized).to_string();
    let safe = if pascal.is_empty() {
        "Unknown".to_string()
    } else if pascal.starts_with(|c: char| c.is_ascii_digit()) {
        format!("V{}", pascal)
    } else {
        pascal
    };
    sanitize_keyword(&safe, "cpp")
}

/// Modern C++20/C++23 Code Generator.
pub struct CppCodegen {
    options: CppOptions,
    context: CppLanguageContext,
}

/// Emitted C++ identifier for a named type, disambiguated across namespaces
/// for the IR currently being generated.
fn type_ident(q: &QName) -> String {
    lookup_type_name(q, || to_cpp_type_name(&q.local))
}

impl CppCodegen {
    pub fn new(options: CppOptions) -> Self {
        Self {
            options,
            context: CppLanguageContext,
        }
    }

    fn unique_field_name(
        &self,
        struct_name: &str,
        raw: &str,
        seen: &mut HashSet<String>,
    ) -> String {
        let base = to_cpp_field_name(raw);
        let mut name = base.clone();
        if name.eq_ignore_ascii_case(struct_name) {
            name.push_str("_value");
        }
        let stem = name.clone();
        let mut index = 2;
        while seen.contains(&name) || name.eq_ignore_ascii_case(struct_name) {
            name = format!("{}_{}", stem, index);
            index += 1;
        }
        seen.insert(name.clone());
        name
    }

    fn unique_enum_variant(&self, raw: &str, seen: &mut HashSet<String>) -> String {
        let base = to_cpp_enum_variant(raw);
        let mut name = base.clone();
        let mut index = 2;
        while seen.contains(&name) {
            name = format!("{}_{}", base, index);
            index += 1;
        }
        seen.insert(name.clone());
        name
    }

    /// Generate complete header-only source (`.hpp`).
    pub fn generate_header(&self, ir: &SchemaIR) -> String {
        set_type_name_map(build_type_name_map(ir, to_cpp_type_name));
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
            "// @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)"
        )
        .unwrap();
        writeln!(out, "// Target: Modern C++20/C++23 (Header-Only)").unwrap();
        writeln!(out, "#pragma once\n").unwrap();

        self.emit_includes(&mut out, ir);

        let ns = to_cpp_namespace(&self.options.namespace);
        writeln!(out, "namespace {} {{\n", ns).unwrap();

        self.emit_utilities(&mut out);
        self.emit_forward_declarations(&mut out, ir);
        self.emit_types(&mut out, ir);
        self.emit_root_aliases(&mut out, ir);

        writeln!(out, "\n}} // namespace {}", ns).unwrap();

        if self.options.backend.is_glaze() {
            self.emit_glaze_meta(&mut out, ir);
        }

        out
    }

    /// Generate complete C++20 module interface unit (`.cppm`).
    pub fn generate_module_unit(&self, ir: &SchemaIR, module_name: &str) -> String {
        set_type_name_map(build_type_name_map(ir, to_cpp_type_name));
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
            "// @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)"
        )
        .unwrap();
        writeln!(out, "// Target: Modern C++20/C++23 Module Interface Unit").unwrap();
        writeln!(out, "module;\n").unwrap();

        self.emit_includes(&mut out, ir);

        let mod_id = if module_name.is_empty() {
            "polyxml.models"
        } else {
            module_name
        };
        writeln!(out, "\nexport module {};\n", mod_id).unwrap();

        let ns = to_cpp_namespace(&self.options.namespace);
        writeln!(out, "export namespace {} {{\n", ns).unwrap();

        self.emit_utilities(&mut out);
        self.emit_forward_declarations(&mut out, ir);
        self.emit_types(&mut out, ir);
        self.emit_root_aliases(&mut out, ir);

        writeln!(out, "\n}} // namespace {}", ns).unwrap();

        if self.options.backend.is_glaze() {
            self.emit_glaze_meta(&mut out, ir);
        }

        out
    }

    /// Generate single-string module depending on configured `CppMode`.
    pub fn generate_module(&self, ir: &SchemaIR) -> String {
        match self.options.mode {
            CppMode::HeaderOnly => self.generate_header(ir),
            CppMode::Module => {
                let mod_name = self.options.namespace.replace("::", ".");
                self.generate_module_unit(ir, &mod_name)
            }
        }
    }

    /// Generate full bundle of files: C++ source, plus CMake and Meson definitions if enabled.
    pub fn generate_files(&self, ir: &SchemaIR, base_name: &str) -> Vec<(String, String)> {
        let mut files = Vec::new();
        let stem = if base_name.is_empty() {
            "models"
        } else {
            base_name
        };

        match self.options.mode {
            CppMode::HeaderOnly => {
                files.push((format!("{}.hpp", stem), self.generate_header(ir)));
            }
            CppMode::Module => {
                files.push((
                    format!("{}.cppm", stem),
                    self.generate_module_unit(ir, stem),
                ));
            }
        }

        if self.options.emit_cmake {
            files.push(("CMakeLists.txt".to_string(), self.generate_cmake(stem)));
            files.push((
                "PolyXMLConfig.cmake".to_string(),
                self.generate_cmake_config(stem),
            ));
        }

        if self.options.emit_meson {
            files.push(("meson.build".to_string(), self.generate_meson(stem)));
        }

        files
    }

    /// Generate CMakeLists.txt definition for integration via add_subdirectory or FetchContent.
    pub fn generate_cmake(&self, project_name: &str) -> String {
        let safe_name = AsSnakeCase(project_name).to_string();
        if self.options.mode == CppMode::Module {
            format!(
                r#"cmake_minimum_required(VERSION 3.28)
project({safe_name}_models LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library({safe_name})
target_sources({safe_name}
    PUBLIC
        FILE_SET CXX_MODULES FILES
            {safe_name}.cppm
)
target_compile_features({safe_name} PUBLIC cxx_std_20)
"#
            )
        } else {
            format!(
                r#"cmake_minimum_required(VERSION 3.20)
project({safe_name}_models LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library({safe_name} INTERFACE)
target_include_directories({safe_name} INTERFACE
    $<BUILD_INTERFACE:${{CMAKE_CURRENT_SOURCE_DIR}}>
    $<INSTALL_INTERFACE:include>
)
target_compile_features({safe_name} INTERFACE cxx_std_20)
"#
            )
        }
    }

    /// Generate standalone PolyXMLConfig.cmake for `find_package(PolyXML)`.
    pub fn generate_cmake_config(&self, project_name: &str) -> String {
        let target_name = AsPascalCase(project_name).to_string();
        format!(
            r#"# PolyXML Generated CMake Configuration File
if(NOT TARGET PolyXML::{target_name})
    add_library(PolyXML::{target_name} INTERFACE IMPORTED)
    set_target_properties(PolyXML::{target_name} PROPERTIES
        INTERFACE_INCLUDE_DIRECTORIES "${{CMAKE_CURRENT_LIST_DIR}}"
        INTERFACE_COMPILE_FEATURES cxx_std_20
    )
endif()
"#
        )
    }

    /// Generate modern meson.build file for Meson build system integration.
    pub fn generate_meson(&self, project_name: &str) -> String {
        let safe_name = AsSnakeCase(project_name).to_string();
        format!(
            r#"project('{safe_name}_models', 'cpp',
  version: '0.1.0',
  default_options: ['cpp_std=c++20']
)

{safe_name}_inc = include_directories('.')
{safe_name}_dep = declare_dependency(
  include_directories: {safe_name}_inc
)
"#
        )
    }

    fn emit_includes(&self, out: &mut String, ir: &SchemaIR) {
        if self.options.validate_facets
            && ir.emitted_types().any(|t| match t {
                TypeDef::Simple(s) => !s.facets.patterns.is_empty(),
                TypeDef::Struct(s) => s
                    .fields
                    .iter()
                    .any(|f| f.facets.as_ref().is_some_and(|f| !f.patterns.is_empty())),
                _ => false,
            })
        {
            writeln!(out, "#include <regex>").unwrap();
        }
        writeln!(out, "#include <concepts>").unwrap();
        writeln!(out, "#include <cstdint>").unwrap();
        writeln!(out, "#include <memory>").unwrap();
        writeln!(out, "#include <optional>").unwrap();
        writeln!(out, "#include <string>").unwrap();
        writeln!(out, "#include <string_view>").unwrap();
        writeln!(out, "#include <variant>").unwrap();
        writeln!(out, "#include <vector>").unwrap();

        if self.options.backend.is_glaze() {
            writeln!(out, "#include <glaze/glaze.hpp>").unwrap();
        }
        writeln!(out).unwrap();
    }

    fn emit_glaze_meta(&self, out: &mut String, ir: &SchemaIR) {
        let ns = to_cpp_namespace(&self.options.namespace);
        writeln!(out, "\n// Glaze compile-time reflection metadata\n").unwrap();

        // 1. Enums
        for type_def in ir.emitted_types() {
            if let TypeDef::Enum(enum_def) = type_def {
                let enum_name = type_ident(&enum_def.qname);
                let full_type = format!("{}::{}", ns, enum_name);
                writeln!(out, "template <>").unwrap();
                writeln!(out, "struct glz::meta<{}> {{", full_type).unwrap();
                writeln!(out, "    using T = {};", full_type).unwrap();
                if enum_def.variants.is_empty() {
                    writeln!(out, "    static constexpr auto value = enumerate();").unwrap();
                } else {
                    writeln!(out, "    static constexpr auto value = enumerate(").unwrap();
                    let mut seen = HashSet::new();
                    let variant_names: Vec<String> = enum_def
                        .variants
                        .iter()
                        .map(|v| self.unique_enum_variant(&v.name, &mut seen))
                        .collect();
                    for (i, (v, var_name)) in
                        enum_def.variants.iter().zip(&variant_names).enumerate()
                    {
                        let comma = if i + 1 < enum_def.variants.len() {
                            ","
                        } else {
                            ""
                        };
                        writeln!(
                            out,
                            "        {}, T::{}{}",
                            super::cpp_string_literal(&v.value),
                            var_name,
                            comma
                        )
                        .unwrap();
                    }
                    writeln!(out, "    );").unwrap();
                }
                writeln!(out, "}};\n").unwrap();
            }
        }

        // 2. Structs
        let sorted_qnames = self.topological_sort_types(ir);
        for qname in sorted_qnames {
            if let Some(TypeDef::Struct(s)) = ir.types.get(&qname) {
                let struct_name = type_ident(&s.qname);
                let full_type = format!("{}::{}", ns, struct_name);
                writeln!(out, "template <>").unwrap();
                writeln!(out, "struct glz::meta<{}> {{", full_type).unwrap();
                writeln!(out, "    using T = {};", full_type).unwrap();

                let mut field_bindings = Vec::new();
                let mut seen = HashSet::new();

                // Check for simple_content_base "value"
                if let Some(ref base_qname) = s.base_type {
                    let has_value_field = s
                        .fields
                        .iter()
                        .any(|f| f.name == "value" || f.kind == crate::ir::FieldKind::Text);
                    if !has_value_field
                        && (PrimitiveType::from_xsd_name(&base_qname.local).is_some()
                            || matches!(ir.types.get(base_qname), Some(TypeDef::Simple(_))))
                    {
                        seen.insert("value".to_string());
                        field_bindings.push(r#""value", &T::value"#.to_string());
                    }
                }

                for f in &s.fields {
                    let field_name = self.unique_field_name(&struct_name, &f.name, &mut seen);
                    field_bindings.push(format!("{:?}, &T::{}", f.xml_name, field_name));
                }

                if field_bindings.is_empty() {
                    writeln!(out, "    static constexpr auto value = object();").unwrap();
                } else {
                    writeln!(out, "    static constexpr auto value = object(").unwrap();
                    for (i, fb) in field_bindings.iter().enumerate() {
                        let comma = if i + 1 < field_bindings.len() {
                            ","
                        } else {
                            ""
                        };
                        writeln!(out, "        {}{}", fb, comma).unwrap();
                    }
                    writeln!(out, "    );").unwrap();
                }
                writeln!(out, "}};\n").unwrap();
            }
        }
    }

    fn emit_utilities(&self, out: &mut String) {
        writeln!(
            out,
            "// Canonical C++20 pattern matching visitor for std::variant
template <class... Ts>
struct overloaded : Ts... {{
    using Ts::operator()...;
}};
template <class... Ts>
overloaded(Ts...) -> overloaded<Ts...>;

// Concept validating PolyXML C++20 equality comparable value types
template <typename T>
concept XmlModel = requires(T a) {{
    {{ a == a }} -> std::convertible_to<bool>;
}};\n"
        )
        .unwrap();
    }

    fn emit_forward_declarations(&self, out: &mut String, ir: &SchemaIR) {
        let mut structs: Vec<String> = ir
            .emitted_types()
            .filter_map(|t| {
                if let TypeDef::Struct(s) = t {
                    Some(type_ident(&s.qname))
                } else {
                    None
                }
            })
            .collect();
        structs.sort();

        if !structs.is_empty() {
            writeln!(out, "// Forward declarations").unwrap();
            for s in structs {
                writeln!(out, "struct {};", s).unwrap();
            }
            writeln!(out).unwrap();
        }
    }

    fn emit_types(&self, out: &mut String, ir: &SchemaIR) {
        if ir.emitted_types().any(|def| match def {
            TypeDef::Struct(s) => s
                .fields
                .iter()
                .any(|f| super::unbounded_integer(&f.type_ref, ir).is_some()),
            TypeDef::Simple(s) => super::unbounded_integer(&s.base_type, ir).is_some(),
            TypeDef::Union(u) => u
                .branches
                .iter()
                .any(|b| super::unbounded_integer(&b.type_ref, ir).is_some()),
            _ => false,
        }) {
            out.push_str(r#"
inline std::string_view polyxml_integer_trim(std::string_view text) noexcept {
    const auto first=text.find_first_not_of(" \t\r\n"); if(first==std::string_view::npos)return {};
    return text.substr(first,text.find_last_not_of(" \t\r\n")-first+1);
}
inline bool polyxml_integer_valid(std::string_view text,int kind) noexcept {
    text=polyxml_integer_trim(text);bool negative=false;
    if(!text.empty()&&(text.front()=='+'||text.front()=='-')){negative=text.front()=='-';text.remove_prefix(1);}
    if(text.empty())return false;bool nonzero=false;
    for(char c:text){if(c<'0'||c>'9')return false;nonzero|=c!='0';}
    int sign=nonzero?(negative?-1:1):0;
    return !((kind==1&&sign<=0)||(kind==2&&sign<0)||(kind==-1&&sign>=0)||(kind==-2&&sign>0));
}
inline int polyxml_integer_compare(std::string_view a,std::string_view b) noexcept {
    a=polyxml_integer_trim(a);b=polyxml_integer_trim(b);
    bool an=!a.empty()&&a.front()=='-',bn=!b.empty()&&b.front()=='-';
    if(!a.empty()&&(a.front()=='+'||a.front()=='-'))a.remove_prefix(1);
    if(!b.empty()&&(b.front()=='+'||b.front()=='-'))b.remove_prefix(1);
    while(!a.empty()&&a.front()=='0')a.remove_prefix(1);
    while(!b.empty()&&b.front()=='0')b.remove_prefix(1);
    an=an&&!a.empty();bn=bn&&!b.empty();if(an!=bn)return an?-1:1;
    int cmp=a.size()<b.size()?-1:a.size()>b.size()?1:a.compare(b);return an?-cmp:cmp;
}
"#);
        }
        let sorted_qnames = self.topological_sort_types(ir);

        // Aliases must follow their base aliases, which may sort later by name.
        for qname in &sorted_qnames {
            if let Some(TypeDef::Simple(simple)) =
                ir.types.get(qname).filter(|_| !ir.is_external_type(qname))
            {
                self.emit_simple_type(out, simple, ir);
            }
        }

        // Emit enums next
        for type_def in ir.emitted_types() {
            if let TypeDef::Enum(enum_def) = type_def {
                self.emit_enum(out, enum_def);
            }
        }

        // Cut cyclic value dependencies through struct fields or union branches,
        // then emit types in dependency order.
        let mut prepared = ir.clone();
        let sorted_qnames = self.order_with_forward_cuts(&mut prepared);

        for qname in sorted_qnames {
            if prepared.is_external_type(&qname) {
                continue;
            }
            if let Some(type_def) = prepared.types.get(&qname) {
                match type_def {
                    TypeDef::Union(u) => self.emit_union(out, u),
                    TypeDef::Struct(s) => self.emit_struct(out, s, &prepared),
                    TypeDef::Simple(_) | TypeDef::Enum(_) => {}
                }
            }
        }
    }

    fn emit_simple_type(&self, out: &mut String, simple: &SimpleTypeDef, ir: &SchemaIR) {
        if let Some(ref doc) = simple.documentation {
            for line in doc.lines() {
                writeln!(out, "/// {}", line).unwrap();
            }
        }
        let type_name = type_ident(&simple.qname);
        let base = if simple.facets.patterns.is_empty() {
            &simple.base_type
        } else {
            super::primitive_base(&simple.base_type, ir)
        };
        let base_type = self.context.map_type_ref(base);
        writeln!(out, "using {} = {};\n", type_name, base_type).unwrap();
        if self.options.validate_facets && !simple.facets.patterns.is_empty() {
            writeln!(out, "[[nodiscard]] inline bool validate_{}_patterns(std::string_view value) noexcept {{\n    try {{", type_name).unwrap();
            for (i, pattern) in simple.facets.patterns.iter().enumerate() {
                writeln!(out, "        static const std::regex pattern_{}({:?});\n        if (!std::regex_match(value.begin(), value.end(), pattern_{})) return false;", i, pattern, i).unwrap();
            }
            writeln!(out, "        return true;\n    }} catch (const std::regex_error&) {{ return false; }}\n}}\n").unwrap();
        }
    }

    fn emit_enum(&self, out: &mut String, enum_def: &EnumDef) {
        if let Some(ref doc) = enum_def.documentation {
            for line in doc.lines() {
                writeln!(out, "/// {}", line).unwrap();
            }
        }

        let enum_name = type_ident(&enum_def.qname);
        writeln!(out, "enum class {} {{", enum_name).unwrap();

        let mut seen = HashSet::new();
        let variant_names: Vec<String> = enum_def
            .variants
            .iter()
            .map(|v| self.unique_enum_variant(&v.name, &mut seen))
            .collect();

        for (variant, var_name) in enum_def.variants.iter().zip(&variant_names) {
            if let Some(ref doc) = variant.documentation {
                writeln!(out, "    /// {}", doc).unwrap();
            }
            writeln!(out, "    {},", var_name).unwrap();
        }
        writeln!(out, "}};\n").unwrap();

        if self.options.emit_enum_converters {
            // to_string
            writeln!(
                out,
                "[[nodiscard]] inline constexpr std::string_view to_string({} value) noexcept {{",
                enum_name
            )
            .unwrap();
            writeln!(out, "    switch (value) {{").unwrap();
            for (variant, var_name) in enum_def.variants.iter().zip(&variant_names) {
                writeln!(
                    out,
                    "        case {}::{}: return {};",
                    enum_name,
                    var_name,
                    super::cpp_string_literal(&variant.value)
                )
                .unwrap();
            }
            writeln!(out, "    }}").unwrap();
            writeln!(out, "    return \"\";").unwrap();
            writeln!(out, "}}\n").unwrap();

            // from_string — stem follows the disambiguated type name only when
            // it differs from the natural one, preserving the historical
            // snake-of-local form otherwise.
            let natural = to_cpp_type_name(&enum_def.qname.local);
            let stem = if enum_name == natural {
                AsSnakeCase(&enum_def.qname.local).to_string()
            } else {
                AsSnakeCase(&enum_name).to_string()
            };
            let func_name = format!("{stem}_from_string");
            writeln!(
                out,
                "[[nodiscard]] inline std::optional<{}> {}(std::string_view s) noexcept {{",
                enum_name, func_name
            )
            .unwrap();
            for (variant, var_name) in enum_def.variants.iter().zip(&variant_names) {
                writeln!(
                    out,
                    "    if (s == {}) return {}::{};",
                    super::cpp_string_literal(&variant.value),
                    enum_name,
                    var_name
                )
                .unwrap();
            }
            writeln!(out, "    return std::nullopt;").unwrap();
            writeln!(out, "}}\n").unwrap();
        }
    }

    fn emit_union(&self, out: &mut String, u: &UnionDef) {
        if let Some(ref doc) = u.documentation {
            for line in doc.lines() {
                writeln!(out, "/// {}", line).unwrap();
            }
        }

        let union_name = type_ident(&u.qname);

        // Check if branches have distinct types and no primitives
        let mut branch_types = Vec::new();
        let mut seen_types = HashSet::new();
        let mut needs_wrappers = false;

        for b in &u.branches {
            let mapped = self.context.map_type_ref(&b.type_ref);
            if matches!(b.type_ref, TypeRef::Primitive(_)) || seen_types.contains(&mapped) {
                needs_wrappers = true;
            }
            seen_types.insert(mapped.clone());
            branch_types.push((b, mapped));
        }

        let mut variant_params = Vec::new();

        if needs_wrappers {
            // Emit wrapper structs for each branch to avoid duplicate variant types and give named semantics
            for (branch, mapped_type) in &branch_types {
                let wrapper_name =
                    format!("{}{}", union_name, to_cpp_type_name(&branch.variant_name));
                if let Some(ref doc) = branch.documentation {
                    writeln!(out, "/// {}", doc).unwrap();
                }
                writeln!(out, "struct {} {{", wrapper_name).unwrap();
                writeln!(out, "    {} value = {{}};", mapped_type).unwrap();
                if self.options.emit_equality_operators {
                    writeln!(
                        out,
                        "    bool operator==(const {}&) const = default;",
                        wrapper_name
                    )
                    .unwrap();
                }
                writeln!(out, "}};\n").unwrap();
                variant_params.push(wrapper_name);
            }
        } else {
            for (_, mapped) in &branch_types {
                variant_params.push(mapped.clone());
            }
        }

        writeln!(
            out,
            "using {} = std::variant<{}>;\n",
            union_name,
            variant_params.join(", ")
        )
        .unwrap();
    }

    fn emit_struct(&self, out: &mut String, s: &StructDef, ir: &SchemaIR) {
        if let Some(ref doc) = s.documentation {
            for line in doc.lines() {
                writeln!(out, "/// {}", line).unwrap();
            }
        }

        let struct_name = type_ident(&s.qname);
        let mut base_clause = String::new();
        let mut simple_content_base: Option<String> = None;

        if let Some(ref base_qname) = s.base_type {
            if matches!(ir.types.get(base_qname), Some(TypeDef::Struct(_))) {
                base_clause = format!(" : public {}", type_ident(base_qname));
            } else if let Some(prim) = PrimitiveType::from_xsd_name(&base_qname.local) {
                simple_content_base = Some(self.context.map_primitive(prim).to_string());
            } else if let Some(TypeDef::Simple(st)) = ir.types.get(base_qname) {
                simple_content_base = Some(type_ident(&st.qname));
            }
        }

        writeln!(out, "struct {}{} {{", struct_name, base_clause).unwrap();

        if let Some(ref base_type_str) = simple_content_base {
            let has_value_field = s
                .fields
                .iter()
                .any(|f| f.name == "value" || f.kind == crate::ir::FieldKind::Text);
            if !has_value_field {
                let init = if base_type_str == "double" || base_type_str == "float" {
                    " = 0.0"
                } else if base_type_str.starts_with("std::int")
                    || base_type_str.starts_with("std::uint")
                {
                    " = 0"
                } else {
                    " = {}"
                };
                writeln!(out, "    {} value{};", base_type_str, init).unwrap();
            }
        }

        let mut seen = HashSet::new();
        if simple_content_base.is_some() {
            let has_value_field = s
                .fields
                .iter()
                .any(|f| f.name == "value" || f.kind == crate::ir::FieldKind::Text);
            if !has_value_field {
                seen.insert("value".to_string());
            }
        }
        let field_names: Vec<String> = s
            .fields
            .iter()
            .map(|f| self.unique_field_name(&struct_name, &f.name, &mut seen))
            .collect();

        // Fields
        for (f, field_name) in s.fields.iter().zip(&field_names) {
            if let Some(ref doc) = f.documentation {
                writeln!(out, "    /// {}", doc).unwrap();
            }
            let (field_type, init_val) = self.resolve_field_type_and_init(f);
            writeln!(out, "    {} {}{};", field_type, field_name, init_val).unwrap();
        }

        if self.options.emit_equality_operators {
            writeln!(
                out,
                "\n    bool operator==(const {}&) const = default;",
                struct_name
            )
            .unwrap();
        }

        if self.options.validate_facets {
            self.emit_struct_validator(out, s, ir, &field_names);
        }

        writeln!(out, "}};\n").unwrap();
    }

    fn resolve_field_type_and_init(&self, f: &crate::ir::FieldDef) -> (String, String) {
        let base_type = self.context.map_type_ref(&f.type_ref);

        if f.cardinality.is_list() {
            if f.is_cycle_cut {
                (
                    format!("std::vector<std::unique_ptr<{}>>", base_type),
                    " = {}".to_string(),
                )
            } else {
                (format!("std::vector<{}>", base_type), " = {}".to_string())
            }
        } else if f.is_cycle_cut {
            (
                format!("std::unique_ptr<{}>", base_type),
                " = nullptr".to_string(),
            )
        } else if f.cardinality.is_optional() || f.nillable {
            (
                format!("std::optional<{}>", base_type),
                " = std::nullopt".to_string(),
            )
        } else {
            // Value field
            let init = match &f.default_value {
                Some(v) => match f.type_ref {
                    TypeRef::Primitive(p) if p.is_unbounded_integer() => format!(" = {v:?}"),
                    TypeRef::Primitive(PrimitiveType::Boolean) => {
                        format!(" = {}", v.to_lowercase())
                    }
                    TypeRef::Primitive(PrimitiveType::Float) => format!(" = {}f", v),
                    TypeRef::Primitive(PrimitiveType::Double)
                    | TypeRef::Primitive(PrimitiveType::Decimal) => format!(" = {}", v),
                    TypeRef::Primitive(PrimitiveType::String)
                    | TypeRef::Primitive(PrimitiveType::Token)
                    | TypeRef::Primitive(PrimitiveType::NormalizedString) => {
                        format!(" = \"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
                    }
                    TypeRef::Primitive(_) => format!(" = {}", v),
                    TypeRef::Named(_) => format!(" = {}", v),
                    _ => " = {}".to_string(),
                },
                None => match f.type_ref {
                    TypeRef::Primitive(p) if p.is_unbounded_integer() => " = {}".into(),
                    TypeRef::Primitive(PrimitiveType::Boolean) => " = false".to_string(),
                    TypeRef::Primitive(PrimitiveType::Float) => " = 0.0f".to_string(),
                    TypeRef::Primitive(PrimitiveType::Double)
                    | TypeRef::Primitive(PrimitiveType::Decimal) => " = 0.0".to_string(),
                    TypeRef::Primitive(PrimitiveType::Byte)
                    | TypeRef::Primitive(PrimitiveType::Short)
                    | TypeRef::Primitive(PrimitiveType::Int)
                    | TypeRef::Primitive(PrimitiveType::Integer)
                    | TypeRef::Primitive(PrimitiveType::Long)
                    | TypeRef::Primitive(PrimitiveType::PositiveInteger)
                    | TypeRef::Primitive(PrimitiveType::NegativeInteger)
                    | TypeRef::Primitive(PrimitiveType::NonPositiveInteger)
                    | TypeRef::Primitive(PrimitiveType::NonNegativeInteger)
                    | TypeRef::Primitive(PrimitiveType::UnsignedByte)
                    | TypeRef::Primitive(PrimitiveType::UnsignedShort)
                    | TypeRef::Primitive(PrimitiveType::UnsignedInt)
                    | TypeRef::Primitive(PrimitiveType::UnsignedLong) => " = 0".to_string(),
                    _ => " = {}".to_string(),
                },
            };
            (base_type, init)
        }
    }

    fn emit_struct_validator(
        &self,
        out: &mut String,
        s: &StructDef,
        ir: &SchemaIR,
        field_names: &[String],
    ) {
        writeln!(out, "\n    [[nodiscard]] bool validate() const noexcept {{").unwrap();

        let mut has_checks = false;
        for (f, field_name) in s.fields.iter().zip(field_names) {
            if let Some(p) = super::unbounded_integer(&f.type_ref, ir) {
                let kind = match p {
                    PrimitiveType::PositiveInteger => 1,
                    PrimitiveType::NonNegativeInteger => 2,
                    PrimitiveType::NegativeInteger => -1,
                    PrimitiveType::NonPositiveInteger => -2,
                    _ => 0,
                };
                let (target, close) = if f.cardinality.is_list() || f.type_ref.is_list() {
                    writeln!(out, "        for(const auto& value : {field_name}) {{").unwrap();
                    ("value".to_string(), true)
                } else if f.cardinality.is_optional() || f.nillable {
                    writeln!(out, "        if({field_name}.has_value()){{").unwrap();
                    (format!("*{field_name}"), true)
                } else {
                    (field_name.clone(), false)
                };
                writeln!(
                    out,
                    "        if(!polyxml_integer_valid({target},{kind}))return false;"
                )
                .unwrap();
                if let Some(facets) = &f.facets {
                    self.emit_integer_facets(out, facets, &target);
                }
                if let TypeRef::Named(name) = &f.type_ref {
                    if let Some(TypeDef::Simple(simple)) = ir.types.get(name) {
                        self.emit_integer_facets(out, &simple.facets, &target);
                    }
                }
                if close {
                    out.push_str("        }\n");
                }
                continue;
            }
            if let Some(simple) = super::patterned_simple(&f.type_ref, ir) {
                let field = field_name.clone();
                let name = type_ident(&simple.qname);
                let is_string = self
                    .context
                    .map_type_ref(super::primitive_base(&simple.base_type, ir))
                    == "std::string";
                let render = |target: String| {
                    if is_string {
                        target
                    } else {
                        format!("std::to_string({target})")
                    }
                };
                if f.cardinality.is_list() || f.type_ref.is_list() {
                    writeln!(out, "        for (const auto& value : {}) {{ if (!validate_{}_patterns({})) return false; }}", field, name, render("value".into())).unwrap();
                } else if f.cardinality.is_optional() || f.nillable {
                    writeln!(out, "        if ({}.has_value()) {{ if (!validate_{}_patterns({})) return false; }}", field, name, render(format!("*{}", field))).unwrap();
                } else {
                    writeln!(
                        out,
                        "        if (!validate_{}_patterns({})) return false;",
                        name,
                        render(field.clone())
                    )
                    .unwrap();
                }
            }
            if let Some(ref facets) = f.facets {
                let is_opt = f.cardinality.is_optional() || f.nillable;

                if is_opt {
                    writeln!(out, "        if ({}.has_value()) {{", field_name).unwrap();
                    self.emit_facet_checks(
                        out,
                        facets,
                        &format!("(*{})", field_name),
                        "            ",
                    );
                    writeln!(out, "        }}").unwrap();
                } else {
                    self.emit_facet_checks(out, facets, field_name, "        ");
                }
                has_checks = true;
            }
        }

        let _ = has_checks;
        writeln!(out, "        return true;").unwrap();

        writeln!(out, "    }}").unwrap();
    }

    fn emit_integer_facets(&self, out: &mut String, facets: &RestrictionFacets, target: &str) {
        for (bound, op) in [
            (&facets.min_inclusive, "<"),
            (&facets.max_inclusive, ">"),
            (&facets.min_exclusive, "<="),
            (&facets.max_exclusive, ">="),
        ] {
            if let Some(bound) = bound {
                writeln!(
                    out,
                    "        if(polyxml_integer_compare({target},{bound:?}) {op} 0)return false;"
                )
                .unwrap();
            }
        }
    }

    fn emit_facet_checks(
        &self,
        out: &mut String,
        facets: &RestrictionFacets,
        target: &str,
        indent: &str,
    ) {
        for pattern in &facets.patterns {
            writeln!(out, "{}try {{ static const std::regex pattern({:?}); if (!std::regex_match({}, pattern)) return false; }} catch (const std::regex_error&) {{ return false; }}", indent, pattern, target).unwrap();
        }
        if let Some(min_len) = facets.min_length {
            writeln!(
                out,
                "{}if ({}.size() < {}) return false;",
                indent, target, min_len
            )
            .unwrap();
        }
        if let Some(max_len) = facets.max_length {
            writeln!(
                out,
                "{}if ({}.size() > {}) return false;",
                indent, target, max_len
            )
            .unwrap();
        }
        if let Some(len) = facets.length {
            writeln!(
                out,
                "{}if ({}.size() != {}) return false;",
                indent, target, len
            )
            .unwrap();
        }
        if let Some(ref min_inc) = facets.min_inclusive {
            writeln!(out, "{}if ({} < {}) return false;", indent, target, min_inc).unwrap();
        }
        if let Some(ref max_inc) = facets.max_inclusive {
            writeln!(out, "{}if ({} > {}) return false;", indent, target, max_inc).unwrap();
        }
    }

    fn emit_root_aliases(&self, out: &mut String, ir: &SchemaIR) {
        if !self.options.emit_root_aliases || ir.elements.is_empty() {
            return;
        }

        writeln!(out, "\n// Root XML Element Type Aliases").unwrap();
        for elem in ir.elements.values() {
            let elem_alias = to_cpp_type_name(&elem.qname.local);
            let target_type = self.context.map_type_ref(&elem.type_ref);
            if elem_alias != target_type {
                if let Some(ref doc) = elem.documentation {
                    for line in doc.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            writeln!(out, "/// {}", trimmed).unwrap();
                        }
                    }
                }
                writeln!(out, "using {} = {};", elem_alias, target_type).unwrap();
            }
        }
    }

    /// Box cyclic value edges until the C++ dependency graph can be ordered.
    fn order_with_forward_cuts(&self, ir: &mut SchemaIR) -> Vec<QName> {
        loop {
            let order = self.topological_sort_types(ir);
            let mut candidate: Option<(bool, QName, usize)> = None;
            for (owner, def) in &ir.types {
                let TypeDef::Struct(structure) = def else {
                    continue;
                };
                for (index, field) in structure.fields.iter().enumerate() {
                    if field.is_cycle_cut || field.cardinality.is_list() {
                        continue;
                    }
                    let TypeRef::Named(target) = &field.type_ref else {
                        continue;
                    };
                    if !matches!(ir.types.get(target), Some(TypeDef::Struct(_)))
                        || !self.has_cpp_dependency_path(ir, target, owner, &mut HashSet::new())
                    {
                        continue;
                    }
                    let optional = field.cardinality.is_optional() || field.nillable;
                    if candidate
                        .as_ref()
                        .is_none_or(|(was_optional, _, _)| optional && !was_optional)
                    {
                        candidate = Some((optional, owner.clone(), index));
                    }
                }
            }
            let Some((_, owner, index)) = candidate else {
                let union_cut = ir.types.iter().find_map(|(owner, def)| {
                    let TypeDef::Union(union) = def else {
                        return None;
                    };
                    union
                        .branches
                        .iter()
                        .enumerate()
                        .find_map(|(index, branch)| {
                            let TypeRef::Named(target) = &branch.type_ref else {
                                return None;
                            };
                            (matches!(ir.types.get(target), Some(TypeDef::Struct(_)))
                                && self.has_cpp_dependency_path(
                                    ir,
                                    target,
                                    owner,
                                    &mut HashSet::new(),
                                ))
                            .then(|| (owner.clone(), index))
                        })
                });
                let Some((owner, index)) = union_cut else {
                    return order;
                };
                if let Some(TypeDef::Union(union)) = ir.types.get_mut(&owner) {
                    let branch = &mut union.branches[index];
                    branch.type_ref = TypeRef::Boxed(Box::new(branch.type_ref.clone()));
                }
                continue;
            };
            if let Some(TypeDef::Struct(structure)) = ir.types.get_mut(&owner) {
                structure.fields[index].is_cycle_cut = true;
            }
        }
    }

    fn has_cpp_dependency_path(
        &self,
        ir: &SchemaIR,
        current: &QName,
        goal: &QName,
        seen: &mut HashSet<QName>,
    ) -> bool {
        if current == goal {
            return true;
        }
        if !seen.insert(current.clone()) {
            return false;
        }
        let mut deps = Vec::new();
        match ir.types.get(current) {
            Some(TypeDef::Struct(structure)) => {
                if let Some(base) = &structure.base_type {
                    deps.push(base.clone());
                }
                for field in &structure.fields {
                    if !field.is_cycle_cut
                        && !(field.cardinality.is_list()
                            && matches!(&field.type_ref, TypeRef::Named(target) if matches!(ir.types.get(target), Some(TypeDef::Struct(_)))))
                    {
                        self.collect_type_dependencies(&field.type_ref, ir, current, &mut deps);
                    }
                }
            }
            Some(TypeDef::Union(union)) => {
                for branch in &union.branches {
                    if !matches!(&branch.type_ref, TypeRef::Boxed(inner) if matches!(inner.as_ref(), TypeRef::Named(target) if matches!(ir.types.get(target), Some(TypeDef::Struct(_)))))
                    {
                        self.collect_type_dependencies(&branch.type_ref, ir, current, &mut deps);
                    }
                }
            }
            Some(TypeDef::Simple(simple)) => {
                self.collect_type_dependencies(&simple.base_type, ir, current, &mut deps);
            }
            _ => {}
        }
        deps.iter()
            .any(|dep| self.has_cpp_dependency_path(ir, dep, goal, seen))
    }

    /// Put definitions before every value use that requires a complete type.
    fn topological_sort_types(&self, ir: &SchemaIR) -> Vec<QName> {
        let mut in_degree: HashMap<QName, usize> = HashMap::new();
        let mut adj: HashMap<QName, Vec<QName>> = HashMap::new();

        for qname in ir.types.keys() {
            in_degree.insert(qname.clone(), 0);
            adj.insert(qname.clone(), Vec::new());
        }

        for (qname, type_def) in &ir.types {
            let mut deps = Vec::new();
            match type_def {
                TypeDef::Struct(s) => {
                    if let Some(ref base) = s.base_type {
                        if ir.types.contains_key(base) && base != qname {
                            deps.push(base.clone());
                        }
                    }
                    for f in &s.fields {
                        // Skip cycle cuts because they use std::unique_ptr (only need forward declarations)
                        if !f.is_cycle_cut
                            && !(f.cardinality.is_list()
                                && matches!(&f.type_ref, TypeRef::Named(target) if matches!(ir.types.get(target), Some(TypeDef::Struct(_)))))
                        {
                            self.collect_type_dependencies(&f.type_ref, ir, qname, &mut deps);
                        }
                    }
                }
                TypeDef::Union(u) => {
                    for b in &u.branches {
                        if !matches!(&b.type_ref, TypeRef::Boxed(inner) if matches!(inner.as_ref(), TypeRef::Named(target) if matches!(ir.types.get(target), Some(TypeDef::Struct(_)))))
                        {
                            self.collect_type_dependencies(&b.type_ref, ir, qname, &mut deps);
                        }
                    }
                }
                TypeDef::Simple(s) => {
                    self.collect_type_dependencies(&s.base_type, ir, qname, &mut deps);
                }
                TypeDef::Enum(_) => {}
            }

            for dep in deps {
                if let Some(list) = adj.get_mut(&dep) {
                    list.push(qname.clone());
                    *in_degree.get_mut(qname).unwrap() += 1;
                }
            }
        }

        let mut queue = VecDeque::new();
        // Use BTreeMap ordering for deterministic output
        let sorted_keys: Vec<QName> = ir.types.keys().cloned().collect();
        for qname in &sorted_keys {
            if *in_degree.get(qname).unwrap_or(&0) == 0 {
                queue.push_back(qname.clone());
            }
        }

        let mut sorted = Vec::new();
        while let Some(u) = queue.pop_front() {
            sorted.push(u.clone());
            if let Some(neighbors) = adj.get(&u) {
                for v in neighbors {
                    let deg = in_degree.get_mut(v).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(v.clone());
                    }
                }
            }
        }

        // Add any remaining (e.g. if cycle) to prevent dropping types
        for qname in sorted_keys {
            if !sorted.contains(&qname) {
                sorted.push(qname);
            }
        }

        sorted
    }

    fn collect_type_dependencies(
        &self,
        type_ref: &TypeRef,
        ir: &SchemaIR,
        current: &QName,
        deps: &mut Vec<QName>,
    ) {
        match type_ref {
            TypeRef::Named(target) => {
                if ir.types.contains_key(target) && target != current && !deps.contains(target) {
                    deps.push(target.clone());
                }
            }
            TypeRef::List(inner) | TypeRef::Boxed(inner) => {
                self.collect_type_dependencies(inner, ir, current, deps);
            }
            TypeRef::Primitive(_) => {}
        }
    }
}
