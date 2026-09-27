use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use heck::{AsKebabCase, AsLowerCamelCase, AsPascalCase, AsShoutySnakeCase, AsSnakeCase};
use minijinja::{Environment, Error as JinjaError, Value};
use thiserror::Error;

pub mod cpp;
pub mod csharp;
pub mod go;
pub mod java;
pub mod python;
pub mod rust;
pub mod typescript;

pub use cpp::{CppBackend, CppCodegen, CppMode, CppOptions};
pub use csharp::{CSharpCodegen, CSharpOptions, CSharpRecordKind};
pub use go::{GoBackend, GoCodegen, GoOptions};
pub use java::{JavaBackend, JavaCodegen, JavaOptions};
pub use python::{
    PythonAotCodegen, PythonAotCrate, PythonAotOptions, PythonBackend, PythonCodegen, PythonOptions,
};
pub use rust::{RustCodegen, RustOptions};
pub use typescript::{TypeScriptBackend, TypeScriptCodegen, TypeScriptOptions};

use crate::ir::{FieldDef, PrimitiveType, QName, SchemaIR, StructDef, TypeRef};

#[derive(Debug, Error)]
pub enum CodegenError {
    #[error("Template engine error: {0}")]
    Template(#[from] JinjaError),

    #[error("Language generation error: {0}")]
    Generation(String),
}

/// Adapter trait feeding normalized schema context and language conventions into templates.
pub trait LanguageContext: Send + Sync {
    /// Return the target language name (e.g. "python", "rust", "cpp", "java", "ts", "go", "csharp").
    fn target_language(&self) -> &'static str;

    /// Sanitize an identifier to avoid collisions with reserved keywords.
    fn sanitize_identifier(&self, id: &str) -> String {
        sanitize_keyword(id, self.target_language())
    }

    /// Map a primitive XSD type to the target language's native type representation.
    fn map_primitive(&self, prim: PrimitiveType) -> &'static str;

    /// Map a canonical TypeRef to the target language's type string.
    fn map_type_ref(&self, type_ref: &TypeRef) -> String;
}

thread_local! {
    /// Disambiguated type identifiers for the IR currently being generated,
    /// keyed by qualified name. Refreshed at the start of
    /// every `generate_module` call; lookups for names absent from the map
    /// (e.g. references to unloaded external types) fall back to the
    /// language's default local-name derivation.
    static TYPE_NAME_MAP: RefCell<HashMap<QName, String>> = RefCell::new(HashMap::new());
}

/// Install the disambiguated type-name map for the current thread.
/// Called by each codegen's `generate_module`.
pub fn set_type_name_map(map: HashMap<QName, String>) {
    TYPE_NAME_MAP.with(|cell| *cell.borrow_mut() = map);
}

/// Resolve the emitted identifier for a named type, falling back to the
/// language-specific derivation when the type is not part of the current IR.
pub fn lookup_type_name(qname: &QName, fallback: impl FnOnce() -> String) -> String {
    TYPE_NAME_MAP
        .with(|cell| cell.borrow().get(qname).cloned())
        .unwrap_or_else(fallback)
}

/// Assign a unique identifier to every type in the IR. Types whose
/// language-cased names collide (same local name in different namespaces)
/// get numeric suffixes; the first type in `BTreeMap` order keeps the bare
/// name, making the assignment deterministic.
pub fn build_type_name_map<F: Fn(&str) -> String>(
    ir: &SchemaIR,
    name_of: F,
) -> HashMap<QName, String> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut map = HashMap::new();
    let mut keys: Vec<&QName> = ir.types.keys().collect();
    keys.sort_by_key(|q| (ir.is_external_type(q), *q));
    for qname in keys {
        let base = name_of(&qname.local);
        let mut name = base.clone();
        let mut n = 2u32;
        while !taken.insert(name.clone()) {
            name = format!("{base}{n}");
            n += 1;
        }
        map.insert(qname.clone(), name);
    }
    map
}

/// Create a pre-configured MiniJinja environment with PolyXML case filters and keyword sanitization.
pub fn create_template_engine() -> Environment<'static> {
    let mut env = Environment::new();

    // Register case transformation filters
    env.add_filter("pascal_case", |val: Value| -> String {
        AsPascalCase(val.as_str().unwrap_or_default()).to_string()
    });

    env.add_filter("snake_case", |val: Value| -> String {
        AsSnakeCase(val.as_str().unwrap_or_default()).to_string()
    });

    env.add_filter("camel_case", |val: Value| -> String {
        AsLowerCamelCase(val.as_str().unwrap_or_default()).to_string()
    });

    env.add_filter("screaming_snake_case", |val: Value| -> String {
        AsShoutySnakeCase(val.as_str().unwrap_or_default()).to_string()
    });

    env.add_filter("kebab_case", |val: Value| -> String {
        AsKebabCase(val.as_str().unwrap_or_default()).to_string()
    });

    // Keyword sanitization filter: {{ field_name | sanitize_keyword("rust") }}
    env.add_filter(
        "sanitize_keyword",
        |val: Value, lang: Option<String>| -> String {
            let name = val.as_str().unwrap_or_default();
            let target = lang.as_deref().unwrap_or("generic");
            sanitize_keyword(name, target)
        },
    );

    env
}

/// Sanitize an identifier against reserved keywords for a given target language.
pub fn sanitize_keyword(name: &str, target_language: &str) -> String {
    let keywords = match target_language.to_lowercase().as_str() {
        "rust" => rust_keywords(),
        "python" => python_keywords(),
        "cpp" | "c++" => cpp_keywords(),
        "java" => java_keywords(),
        "ts" | "typescript" | "javascript" | "js" => ts_keywords(),
        "go" => go_keywords(),
        "csharp" | "c#" | "cs" => csharp_keywords(),
        _ => generic_keywords(),
    };

    if keywords.contains(name) {
        match target_language.to_lowercase().as_str() {
            "rust" => {
                // In Rust, 'self', 'Self', 'super', and 'crate' cannot be raw identifiers
                if matches!(name, "self" | "Self" | "super" | "crate") {
                    format!("{}_", name)
                } else {
                    format!("r#{}", name)
                }
            }
            "csharp" | "c#" | "cs" => format!("@{}", name),
            _ => format!("{}_", name),
        }
    } else {
        name.to_string()
    }
}

/// Normalizes an identifier or enumeration symbol by mapping common mathematical
/// and punctuation symbols into English words so that non-alphanumeric XML
/// enumeration values (e.g. "+", "-", "*", "100%", "2G") produce valid identifiers
/// across all target languages.
pub fn normalize_symbol_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "Empty".to_string();
    }

    match trimmed {
        "+" => return "Plus".to_string(),
        "-" => return "Minus".to_string(),
        "*" => return "Star".to_string(),
        "/" => return "Slash".to_string(),
        "%" => return "Percent".to_string(),
        "=" => return "Equal".to_string(),
        "<" => return "LessThan".to_string(),
        ">" => return "GreaterThan".to_string(),
        "<=" => return "LessThanOrEqual".to_string(),
        ">=" => return "GreaterThanOrEqual".to_string(),
        "!=" => return "NotEqual".to_string(),
        "&" => return "And".to_string(),
        "|" => return "Or".to_string(),
        "~" => return "Tilde".to_string(),
        "^" => return "Caret".to_string(),
        "@" => return "At".to_string(),
        "#" => return "Hash".to_string(),
        "$" => return "Dollar".to_string(),
        _ => {}
    }

    let mut out = String::new();
    for c in trimmed.chars() {
        match c {
            '%' => out.push_str("Percent"),
            '+' => out.push_str("Plus"),
            '#' => out.push_str("Hash"),
            '@' => out.push_str("At"),
            '$' => out.push_str("Dollar"),
            '=' => out.push_str("Equal"),
            _ => out.push(c),
        }
    }
    out
}

fn rust_keywords() -> &'static HashSet<&'static str> {
    static RUST_KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    RUST_KEYWORDS.get_or_init(|| {
        [
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
            "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
            "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become",
            "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual",
            "yield", "try",
        ]
        .into_iter()
        .collect()
    })
}

fn python_keywords() -> &'static HashSet<&'static str> {
    static PY_KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    PY_KEYWORDS.get_or_init(|| {
        [
            "False",
            "None",
            "True",
            "false",
            "none",
            "true",
            "and",
            "as",
            "assert",
            "async",
            "await",
            "break",
            "class",
            "continue",
            "def",
            "del",
            "elif",
            "else",
            "except",
            "finally",
            "for",
            "from",
            "global",
            "if",
            "import",
            "in",
            "is",
            "lambda",
            "nonlocal",
            "not",
            "or",
            "pass",
            "raise",
            "return",
            "try",
            "while",
            "with",
            "yield",
            "match",
            "case",
            "type",
            "field",
            "dataclass",
            "self",
        ]
        .into_iter()
        .collect()
    })
}

fn cpp_keywords() -> &'static HashSet<&'static str> {
    static CPP_KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    CPP_KEYWORDS.get_or_init(|| {
        [
            "alignas",
            "alignof",
            "and",
            "and_eq",
            "asm",
            "atomic_cancel",
            "atomic_commit",
            "atomic_noexcept",
            "auto",
            "bitand",
            "bitor",
            "bool",
            "break",
            "case",
            "catch",
            "char",
            "char8_t",
            "char16_t",
            "char32_t",
            "class",
            "compl",
            "concept",
            "const",
            "consteval",
            "constexpr",
            "constinit",
            "const_cast",
            "continue",
            "co_await",
            "co_return",
            "co_yield",
            "decltype",
            "default",
            "delete",
            "do",
            "double",
            "dynamic_cast",
            "else",
            "enum",
            "explicit",
            "export",
            "extern",
            "false",
            "float",
            "for",
            "friend",
            "goto",
            "if",
            "inline",
            "int",
            "long",
            "mutable",
            "namespace",
            "new",
            "noexcept",
            "not",
            "not_eq",
            "nullptr",
            "operator",
            "or",
            "or_eq",
            "private",
            "protected",
            "public",
            "reflexpr",
            "register",
            "reinterpret_cast",
            "requires",
            "return",
            "short",
            "signed",
            "sizeof",
            "static",
            "static_assert",
            "static_cast",
            "struct",
            "switch",
            "synchronized",
            "template",
            "this",
            "thread_local",
            "throw",
            "true",
            "try",
            "typedef",
            "typeid",
            "typename",
            "union",
            "unsigned",
            "using",
            "virtual",
            "void",
            "volatile",
            "wchar_t",
            "while",
            "xor",
            "xor_eq",
        ]
        .into_iter()
        .collect()
    })
}

fn java_keywords() -> &'static HashSet<&'static str> {
    static JAVA_KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    JAVA_KEYWORDS.get_or_init(|| {
        [
            "abstract",
            "assert",
            "boolean",
            "break",
            "byte",
            "case",
            "catch",
            "char",
            "class",
            "const",
            "continue",
            "default",
            "do",
            "double",
            "else",
            "enum",
            "extends",
            "final",
            "finally",
            "float",
            "for",
            "goto",
            "if",
            "implements",
            "import",
            "instanceof",
            "int",
            "interface",
            "long",
            "native",
            "new",
            "package",
            "private",
            "protected",
            "public",
            "return",
            "short",
            "static",
            "strictfp",
            "super",
            "switch",
            "synchronized",
            "this",
            "throw",
            "throws",
            "transient",
            "try",
            "void",
            "volatile",
            "while",
            "record",
            "sealed",
            "permits",
            "var",
            "yield",
        ]
        .into_iter()
        .collect()
    })
}

fn ts_keywords() -> &'static HashSet<&'static str> {
    static TS_KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    TS_KEYWORDS.get_or_init(|| {
        [
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "else",
            "enum",
            "export",
            "extends",
            "false",
            "finally",
            "for",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "new",
            "null",
            "return",
            "super",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "typeof",
            "var",
            "void",
            "while",
            "with",
            "as",
            "implements",
            "interface",
            "let",
            "package",
            "private",
            "protected",
            "public",
            "static",
            "yield",
            "any",
            "boolean",
            "constructor",
            "declare",
            "get",
            "module",
            "require",
            "number",
            "set",
            "string",
            "symbol",
            "type",
            "from",
            "of",
        ]
        .into_iter()
        .collect()
    })
}

fn go_keywords() -> &'static HashSet<&'static str> {
    static GO_KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    GO_KEYWORDS.get_or_init(|| {
        [
            "break",
            "case",
            "chan",
            "const",
            "continue",
            "default",
            "defer",
            "else",
            "fallthrough",
            "for",
            "func",
            "go",
            "goto",
            "if",
            "import",
            "interface",
            "map",
            "package",
            "range",
            "return",
            "select",
            "struct",
            "switch",
            "type",
            "var",
        ]
        .into_iter()
        .collect()
    })
}

fn csharp_keywords() -> &'static HashSet<&'static str> {
    static CS_KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    CS_KEYWORDS.get_or_init(|| {
        [
            "abstract",
            "as",
            "base",
            "bool",
            "break",
            "byte",
            "case",
            "catch",
            "char",
            "checked",
            "class",
            "const",
            "continue",
            "decimal",
            "default",
            "delegate",
            "do",
            "double",
            "else",
            "enum",
            "event",
            "explicit",
            "extern",
            "false",
            "finally",
            "fixed",
            "float",
            "for",
            "foreach",
            "goto",
            "if",
            "implicit",
            "in",
            "int",
            "interface",
            "internal",
            "is",
            "lock",
            "long",
            "namespace",
            "new",
            "null",
            "object",
            "operator",
            "out",
            "override",
            "params",
            "private",
            "protected",
            "public",
            "readonly",
            "ref",
            "return",
            "sbyte",
            "sealed",
            "short",
            "sizeof",
            "stackalloc",
            "static",
            "string",
            "struct",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "typeof",
            "uint",
            "ulong",
            "unchecked",
            "unsafe",
            "ushort",
            "using",
            "virtual",
            "void",
            "volatile",
            "while",
            "record",
            "init",
            "scoped",
            "file",
            "nint",
            "nuint",
            "required",
        ]
        .into_iter()
        .collect()
    })
}

fn generic_keywords() -> &'static HashSet<&'static str> {
    static GENERIC_KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    GENERIC_KEYWORDS.get_or_init(|| {
        ["type", "class", "struct", "import", "export", "default"]
            .into_iter()
            .collect()
    })
}

/// Resolve a simple alias to its underlying primitive without looping on cycles.
pub(crate) fn primitive_base<'a>(ty: &'a TypeRef, ir: &'a SchemaIR) -> &'a TypeRef {
    let mut current = ty;
    let mut visited = HashSet::new();
    while let TypeRef::Named(q) = current {
        if !visited.insert(q) {
            break;
        }
        match ir.types.get(q) {
            Some(crate::ir::TypeDef::Simple(s)) => current = &s.base_type,
            _ => break,
        }
    }
    current
}

/// Patterned simple types referenced by fields, including list/boxed wrappers.
pub(crate) fn patterned_simple<'a>(
    ty: &TypeRef,
    ir: &'a SchemaIR,
) -> Option<&'a crate::ir::SimpleTypeDef> {
    match ty {
        TypeRef::Named(q) => match ir.types.get(q) {
            Some(crate::ir::TypeDef::Simple(s)) if !s.facets.patterns.is_empty() => Some(s),
            _ => None,
        },
        TypeRef::Boxed(inner) | TypeRef::List(inner) => patterned_simple(inner, ir),
        _ => None,
    }
}

/// Flattened field list for a struct: every field inherited through its
/// `xsd:extension` base chain (root first), followed by the struct's own
/// fields. Backends whose targets cannot represent the base chain directly —
/// Rust structs and Java records have no inheritance — must emit this exact
/// list; declaration, name allocation, and both codec directions must agree
/// on it or derived types silently drop inherited data.
///
/// When a derived type re-declares a field whose schema name matches an
/// inherited one (the `simpleContent` value field, for instance), the
/// most-derived declaration wins and the inherited duplicate is skipped, so
/// flattening never introduces suffixed `value2`-style duplicates. A base
/// chain that revisits a type is cut at the revisit.
pub(crate) fn flatten_fields<'a>(s: &'a StructDef, ir: &'a SchemaIR) -> Vec<&'a FieldDef> {
    fn collect_bases<'a>(
        s: &'a StructDef,
        ir: &'a SchemaIR,
        seen: &mut HashSet<QName>,
        out: &mut Vec<&'a StructDef>,
    ) {
        if !seen.insert(s.qname.clone()) {
            return;
        }
        if let Some(crate::ir::TypeDef::Struct(base)) =
            s.base_type.as_ref().and_then(|q| ir.types.get(q))
        {
            collect_bases(base, ir, seen, out);
            out.push(base);
        }
    }

    let mut bases = Vec::new();
    collect_bases(s, ir, &mut HashSet::new(), &mut bases);
    if bases.is_empty() {
        return s.fields.iter().collect();
    }

    // Claim schema names leaf-to-root so the most-derived declaration of a
    // name shadows any inherited duplicate. Names are claimed per struct, not
    // per field, so an attribute and an element sharing a name inside one
    // struct are both preserved.
    let mut claimed: HashSet<&str> = HashSet::new();
    let mut groups: Vec<Vec<&FieldDef>> = Vec::new();
    for st in std::iter::once(s).chain(bases.iter().rev().copied()) {
        let group: Vec<&FieldDef> = st
            .fields
            .iter()
            .filter(|f| !claimed.contains(f.name.as_str()))
            .collect();
        claimed.extend(group.iter().map(|f| f.name.as_str()));
        groups.push(group);
    }
    groups.reverse();
    groups.into_iter().flatten().collect()
}
