use std::collections::HashSet;
use std::fmt::Write as FmtWrite;

use heck::AsSnakeCase;

use super::{to_cpp_type_name, type_ident, CppCodegen};
use crate::codegen::{cpp_string_literal, LanguageContext};
use crate::ir::{
    EnumDef, FieldDef, FieldKind, PrimitiveType, SchemaIR, StructDef, TypeDef, TypeRef, UnionDef,
};

enum CppValueShape<'a> {
    Struct(&'a StructDef),
    Enum(&'a EnumDef),
    Union(&'a UnionDef),
    LexicalList(&'a TypeRef),
    Primitive(PrimitiveType),
    AnyString,
}

struct InheritedField<'a> {
    owner_name: String,
    cpp_name: String,
    field: &'a FieldDef,
}

impl CppCodegen {
    /// Emit the self-contained XML parser and scalar conversion runtime helpers (`namespace detail`).
    pub(super) fn emit_xml_runtime(&self, out: &mut String) {
        out.push_str(
            r#"#ifndef POLYXML_XML_CODEC_DEFINED
#define POLYXML_XML_CODEC_DEFINED
namespace detail {

[[nodiscard]] inline std::string_view local_name(std::string_view qname) noexcept {
    const auto colon = qname.find(':');
    return colon == std::string_view::npos ? qname : qname.substr(colon + 1);
}

[[nodiscard]] inline std::string_view trim_ws(std::string_view sv) noexcept {
    const auto first = sv.find_first_not_of(" \t\r\n");
    if (first == std::string_view::npos) {
        return {};
    }
    const auto last = sv.find_last_not_of(" \t\r\n");
    return sv.substr(first, last - first + 1);
}

[[nodiscard]] inline bool is_ws(char c) noexcept {
    return c == ' ' || c == '\t' || c == '\r' || c == '\n';
}

inline void append_xml_escaped(std::string& out, std::string_view sv, bool attr) {
    for (char c : sv) {
        switch (c) {
            case '&': out.append("&amp;"); break;
            case '<': out.append("&lt;"); break;
            case '>': out.append("&gt;"); break;
            case '"': out.append(attr ? "&quot;" : "\""); break;
            case '\'': out.append(attr ? "&apos;" : "'"); break;
            default: out.push_back(c); break;
        }
    }
}

inline void append_utf8(std::string& out, std::uint32_t cp) {
    if (cp <= 0x7Fu) {
        out.push_back(static_cast<char>(cp));
    } else if (cp <= 0x7FFu) {
        out.push_back(static_cast<char>(0xC0u | (cp >> 6)));
        out.push_back(static_cast<char>(0x80u | (cp & 0x3Fu)));
    } else if (cp <= 0xFFFFu) {
        out.push_back(static_cast<char>(0xE0u | (cp >> 12)));
        out.push_back(static_cast<char>(0x80u | ((cp >> 6) & 0x3Fu)));
        out.push_back(static_cast<char>(0x80u | (cp & 0x3Fu)));
    } else if (cp <= 0x10FFFFu) {
        out.push_back(static_cast<char>(0xF0u | (cp >> 18)));
        out.push_back(static_cast<char>(0x80u | ((cp >> 12) & 0x3Fu)));
        out.push_back(static_cast<char>(0x80u | ((cp >> 6) & 0x3Fu)));
        out.push_back(static_cast<char>(0x80u | (cp & 0x3Fu)));
    } else {
        throw std::runtime_error("Invalid Unicode code point in XML character reference");
    }
}

[[nodiscard]] inline std::string xml_unescape(std::string_view sv) {
    std::string out;
    out.reserve(sv.size());
    std::size_t i = 0;
    while (i < sv.size()) {
        if (sv[i] != '&') {
            out.push_back(sv[i]);
            ++i;
            continue;
        }
        const auto semi = sv.find(';', i + 1);
        if (semi == std::string_view::npos) {
            out.push_back('&');
            ++i;
            continue;
        }
        const auto ent = sv.substr(i + 1, semi - (i + 1));
        if (ent == "amp") {
            out.push_back('&');
        } else if (ent == "lt") {
            out.push_back('<');
        } else if (ent == "gt") {
            out.push_back('>');
        } else if (ent == "quot") {
            out.push_back('"');
        } else if (ent == "apos") {
            out.push_back('\'');
        } else if (!ent.empty() && ent.front() == '#') {
            std::uint32_t cp = 0;
            int base = 10;
            auto digits = ent.substr(1);
            if (!digits.empty() && (digits.front() == 'x' || digits.front() == 'X')) {
                base = 16;
                digits.remove_prefix(1);
            }
            if (digits.empty()) {
                throw std::runtime_error("Empty XML numeric character reference");
            }
            const auto [ptr, ec] = std::from_chars(digits.data(), digits.data() + digits.size(), cp, base);
            if (ec != std::errc{} || ptr != digits.data() + digits.size()) {
                throw std::runtime_error("Invalid XML numeric character reference");
            }
            append_utf8(out, cp);
        } else {
            out.push_back('&');
            out.append(ent);
            out.push_back(';');
        }
        i = semi + 1;
    }
    return out;
}

struct XmlNode {
    std::string tag;
    std::vector<std::pair<std::string, std::string>> attrs;
    std::string text;
    std::vector<XmlNode> children;

    [[nodiscard]] std::optional<std::string_view> attr(std::string_view local) const noexcept {
        for (const auto& [k, v] : attrs) {
            if (k == "xmlns" || k.rfind("xmlns:", 0) == 0) {
                continue;
            }
            if (k == local || local_name(k) == local) {
                return std::string_view(v);
            }
        }
        return std::nullopt;
    }

    [[nodiscard]] bool is_nil() const noexcept {
        for (const auto& [k, v] : attrs) {
            if (local_name(k) == "nil" && (k.find(':') != std::string::npos || k == "nil")) {
                return v == "true" || v == "1";
            }
        }
        return false;
    }
};

inline void skip_misc(std::string_view xml, std::size_t& pos) {
    while (pos < xml.size()) {
        while (pos < xml.size() && is_ws(xml[pos])) {
            ++pos;
        }
        if (pos + 4 <= xml.size() && xml.substr(pos, 4) == "<!--") {
            const auto end = xml.find("-->", pos + 4);
            if (end == std::string_view::npos) {
                throw std::runtime_error("Unterminated XML comment");
            }
            pos = end + 3;
            continue;
        }
        if (pos + 2 <= xml.size() && xml.substr(pos, 2) == "<?") {
            const auto end = xml.find("?>", pos + 2);
            if (end == std::string_view::npos) {
                throw std::runtime_error("Unterminated XML processing instruction");
            }
            pos = end + 2;
            continue;
        }
        if (pos + 2 <= xml.size() && xml.substr(pos, 2) == "<!" &&
            (pos + 9 > xml.size() || xml.substr(pos, 9) != "<![CDATA[")) {
            int depth = 1;
            pos += 2;
            while (pos < xml.size() && depth > 0) {
                if (xml[pos] == '<') {
                    ++depth;
                } else if (xml[pos] == '>') {
                    --depth;
                }
                ++pos;
            }
            if (depth > 0) {
                throw std::runtime_error("Unterminated XML declaration");
            }
            continue;
        }
        break;
    }
}

[[nodiscard]] inline XmlNode parse_xml_element(std::string_view xml, std::size_t& pos) {
    if (pos >= xml.size() || xml[pos] != '<') {
        throw std::runtime_error("Expected XML start tag");
    }
    ++pos;
    const auto tag_start = pos;
    while (pos < xml.size() && !is_ws(xml[pos]) && xml[pos] != '>' && xml[pos] != '/') {
        ++pos;
    }
    if (pos == tag_start) {
        throw std::runtime_error("Empty XML tag name");
    }
    XmlNode node;
    node.tag = std::string(xml.substr(tag_start, pos - tag_start));

    while (pos < xml.size()) {
        while (pos < xml.size() && is_ws(xml[pos])) {
            ++pos;
        }
        if (pos >= xml.size()) {
            throw std::runtime_error("Unexpected EOF inside XML start tag");
        }
        if (xml[pos] == '/') {
            if (pos + 1 >= xml.size() || xml[pos + 1] != '>') {
                throw std::runtime_error("Malformed XML self-closing tag");
            }
            pos += 2;
            return node;
        }
        if (xml[pos] == '>') {
            ++pos;
            break;
        }
        const auto name_start = pos;
        while (pos < xml.size() && !is_ws(xml[pos]) && xml[pos] != '=' && xml[pos] != '>' && xml[pos] != '/') {
            ++pos;
        }
        const auto attr_name = xml.substr(name_start, pos - name_start);
        while (pos < xml.size() && is_ws(xml[pos])) {
            ++pos;
        }
        if (pos >= xml.size() || xml[pos] != '=') {
            throw std::runtime_error("Expected '=' after XML attribute name");
        }
        ++pos;
        while (pos < xml.size() && is_ws(xml[pos])) {
            ++pos;
        }
        if (pos >= xml.size() || (xml[pos] != '"' && xml[pos] != '\'')) {
            throw std::runtime_error("Expected quoted XML attribute value");
        }
        const char quote = xml[pos++];
        const auto val_start = pos;
        const auto val_end = xml.find(quote, pos);
        if (val_end == std::string_view::npos) {
            throw std::runtime_error("Unterminated XML attribute value");
        }
        node.attrs.emplace_back(std::string(attr_name), xml_unescape(xml.substr(val_start, val_end - val_start)));
        pos = val_end + 1;
    }

    while (pos < xml.size()) {
        const auto lt = xml.find('<', pos);
        if (lt == std::string_view::npos) {
            throw std::runtime_error("Unexpected EOF before closing XML tag");
        }
        if (lt > pos) {
            node.text.append(xml_unescape(xml.substr(pos, lt - pos)));
        }
        pos = lt;
        if (pos + 2 <= xml.size() && xml.substr(pos, 2) == "</") {
            pos += 2;
            const auto close_start = pos;
            while (pos < xml.size() && !is_ws(xml[pos]) && xml[pos] != '>') {
                ++pos;
            }
            const auto close_tag = xml.substr(close_start, pos - close_start);
            while (pos < xml.size() && is_ws(xml[pos])) {
                ++pos;
            }
            if (pos >= xml.size() || xml[pos] != '>') {
                throw std::runtime_error("Expected '>' in closing XML tag");
            }
            ++pos;
            if (close_tag != node.tag) {
                throw std::runtime_error("Mismatched closing XML tag");
            }
            return node;
        }
        if (pos + 9 <= xml.size() && xml.substr(pos, 9) == "<![CDATA[") {
            const auto end = xml.find("]]>", pos + 9);
            if (end == std::string_view::npos) {
                throw std::runtime_error("Unterminated XML CDATA section");
            }
            node.text.append(xml.substr(pos + 9, end - (pos + 9)));
            pos = end + 3;
            continue;
        }
        if (pos + 4 <= xml.size() && xml.substr(pos, 4) == "<!--") {
            const auto end = xml.find("-->", pos + 4);
            if (end == std::string_view::npos) {
                throw std::runtime_error("Unterminated XML comment");
            }
            pos = end + 3;
            continue;
        }
        if (pos + 2 <= xml.size() && xml.substr(pos, 2) == "<?") {
            const auto end = xml.find("?>", pos + 2);
            if (end == std::string_view::npos) {
                throw std::runtime_error("Unterminated XML processing instruction");
            }
            pos = end + 2;
            continue;
        }
        node.children.push_back(parse_xml_element(xml, pos));
    }

    throw std::runtime_error("Unexpected EOF in XML document");
}

[[nodiscard]] inline XmlNode parse_xml_document(std::string_view xml) {
    if (xml.size() >= 3 &&
        static_cast<unsigned char>(xml[0]) == 0xEFu &&
        static_cast<unsigned char>(xml[1]) == 0xBBu &&
        static_cast<unsigned char>(xml[2]) == 0xBFu) {
        xml.remove_prefix(3);
    }
    std::size_t pos = 0;
    skip_misc(xml, pos);
    if (pos >= xml.size() || xml[pos] != '<') {
        throw std::runtime_error("Expected root XML element");
    }
    return parse_xml_element(xml, pos);
}

[[nodiscard]] inline bool parse_bool(std::string_view s) {
    s = trim_ws(s);
    if (s == "true" || s == "1") return true;
    if (s == "false" || s == "0") return false;
    throw std::runtime_error("Invalid XML boolean value");
}

[[nodiscard]] inline std::string_view format_bool(bool v) noexcept {
    return v ? "true" : "false";
}

template <typename Int>
[[nodiscard]] inline Int parse_int(std::string_view s) {
    s = trim_ws(s);
    if (!s.empty() && s.front() == '+') {
        s.remove_prefix(1);
        if (s.empty() || s.front() == '-') {
            throw std::runtime_error("Invalid XML integer value");
        }
    }
    if (s.empty()) {
        throw std::runtime_error("Invalid XML integer value");
    }
    Int value = 0;
    const auto [ptr, ec] = std::from_chars(s.data(), s.data() + s.size(), value);
    if (ec != std::errc{} || ptr != s.data() + s.size()) {
        throw std::runtime_error("Invalid XML integer value");
    }
    return value;
}

template <typename Int>
[[nodiscard]] inline std::string format_int(Int v) {
    if constexpr (std::is_signed_v<Int>) {
        return std::to_string(static_cast<long long>(v));
    } else {
        return std::to_string(static_cast<unsigned long long>(v));
    }
}

template <typename Float>
[[nodiscard]] inline Float parse_float(std::string_view s) {
    s = trim_ws(s);
    if (s == "INF" || s == "+INF") return std::numeric_limits<Float>::infinity();
    if (s == "-INF") return -std::numeric_limits<Float>::infinity();
    if (s == "NaN") return std::numeric_limits<Float>::quiet_NaN();
    if (s.empty()) {
        throw std::runtime_error("Invalid XML floating-point value");
    }
    std::string tmp(s);
    char* end = nullptr;
    errno = 0;
    Float val{};
    if constexpr (std::is_same_v<Float, float>) {
        val = std::strtof(tmp.c_str(), &end);
    } else {
        val = static_cast<Float>(std::strtod(tmp.c_str(), &end));
    }
    if (end != tmp.c_str() + tmp.size() || errno == ERANGE) {
        throw std::runtime_error("Invalid XML floating-point value");
    }
    return val;
}

template <typename Float>
[[nodiscard]] inline std::string format_float(Float v) {
    if (std::isnan(v)) return "NaN";
    if (std::isinf(v)) return v < 0 ? "-INF" : "INF";
    std::ostringstream oss;
    oss << std::setprecision(std::numeric_limits<Float>::digits10) << v;
    return oss.str();
}

[[nodiscard]] inline std::vector<std::uint8_t> parse_bytes(std::string_view s) {
    s = trim_ws(s);
    std::vector<std::uint8_t> out;
    if (s.size() % 2 != 0) {
        throw std::runtime_error("Invalid hex binary length");
    }
    auto nibble = [](char c) -> int {
        if (c >= '0' && c <= '9') return c - '0';
        if (c >= 'a' && c <= 'f') return c - 'a' + 10;
        if (c >= 'A' && c <= 'F') return c - 'A' + 10;
        return -1;
    };
    out.reserve(s.size() / 2);
    for (std::size_t i = 0; i < s.size(); i += 2) {
        const int hi = nibble(s[i]);
        const int lo = nibble(s[i + 1]);
        if (hi < 0 || lo < 0) {
            throw std::runtime_error("Invalid hex binary digit");
        }
        out.push_back(static_cast<std::uint8_t>((hi << 4) | lo));
    }
    return out;
}

[[nodiscard]] inline std::string format_bytes(const std::vector<std::uint8_t>& bytes) {
    static constexpr char HEX[] = "0123456789ABCDEF";
    std::string out;
    out.reserve(bytes.size() * 2);
    for (std::uint8_t b : bytes) {
        out.push_back(HEX[(b >> 4) & 0xFu]);
        out.push_back(HEX[b & 0xFu]);
    }
    return out;
}

} // namespace detail
#endif

"#,
        );
    }

    /// Emit codec method declarations inside a generated C++ struct.
    pub(super) fn emit_struct_codec_decls(&self, out: &mut String, s: &StructDef) {
        let struct_name = type_ident(&s.qname);
        let default_tag = cpp_string_literal(&s.qname.local);
        writeln!(
            out,
            "\n    void write_xml_to(std::string& px_out, std::string_view px_tag = {default_tag}) const;"
        )
        .unwrap();
        writeln!(
            out,
            "    [[nodiscard]] std::string to_xml(std::string_view px_tag = {default_tag}) const;"
        )
        .unwrap();
        writeln!(
            out,
            "    [[nodiscard]] static {struct_name} from_xml_node(const detail::XmlNode& px_node);"
        )
        .unwrap();
        writeln!(
            out,
            "    [[nodiscard]] static {struct_name} from_xml(std::string_view px_xml);"
        )
        .unwrap();
    }

    /// Collect all inherited + own fields in root-to-leaf order with exact C++ owner + field names.
    fn collect_inherited_fields<'a>(
        &self,
        s: &'a StructDef,
        ir: &'a SchemaIR,
    ) -> Vec<InheritedField<'a>> {
        let mut chain = vec![s];
        let mut current = s.base_type.as_ref();
        let mut seen_bases = HashSet::new();
        seen_bases.insert(s.qname.clone());
        while let Some(base_qname) = current {
            if !seen_bases.insert(base_qname.clone()) {
                break;
            }
            let Some(TypeDef::Struct(base_s)) = ir.types.get(base_qname) else {
                break;
            };
            chain.push(base_s);
            current = base_s.base_type.as_ref();
        }
        chain.reverse();

        let mut fields = Vec::new();
        for anc in chain {
            let anc_name = type_ident(&anc.qname);
            let has_sc_base = self.has_simple_content_base(anc, ir);
            let names = self.struct_field_names(anc, &anc_name, has_sc_base);
            for (field, cpp_name) in anc.fields.iter().zip(names) {
                fields.push(InheritedField {
                    owner_name: anc_name.clone(),
                    cpp_name,
                    field,
                });
            }
        }
        fields
    }

    fn has_simple_content_base(&self, s: &StructDef, ir: &SchemaIR) -> bool {
        s.base_type.as_ref().is_some_and(|base_qname| {
            !matches!(ir.types.get(base_qname), Some(TypeDef::Struct(_)))
                && (PrimitiveType::from_xsd_name(&base_qname.local).is_some()
                    || matches!(ir.types.get(base_qname), Some(TypeDef::Simple(_))))
        })
    }

    fn resolve_simple_content_base_type(&self, s: &StructDef, ir: &SchemaIR) -> Option<TypeRef> {
        let has_explicit_value = s
            .fields
            .iter()
            .any(|f| f.name == "value" || f.kind == FieldKind::Text);
        if has_explicit_value {
            return None;
        }
        let base_qname = s.base_type.as_ref()?;
        if let Some(prim) = PrimitiveType::from_xsd_name(&base_qname.local) {
            return Some(TypeRef::Primitive(prim));
        }
        if matches!(ir.types.get(base_qname), Some(TypeDef::Simple(_))) {
            return Some(TypeRef::Named(base_qname.clone()));
        }
        None
    }

    fn resolve_cpp_shape<'a>(&self, type_ref: &'a TypeRef, ir: &'a SchemaIR) -> CppValueShape<'a> {
        let mut current = type_ref;
        let mut seen = HashSet::new();
        loop {
            match current {
                TypeRef::Boxed(inner) => current = inner.as_ref(),
                TypeRef::List(inner) => return CppValueShape::LexicalList(inner.as_ref()),
                TypeRef::Primitive(p) => return CppValueShape::Primitive(*p),
                TypeRef::Named(qname) => {
                    if let Some(prim) = PrimitiveType::from_xsd_name(&qname.local) {
                        return CppValueShape::Primitive(prim);
                    }
                    if !seen.insert(qname.clone()) {
                        return CppValueShape::AnyString;
                    }
                    match ir.types.get(qname) {
                        Some(TypeDef::Struct(st)) if !ir.is_external_type(qname) => {
                            return CppValueShape::Struct(st);
                        }
                        Some(TypeDef::Enum(en)) => return CppValueShape::Enum(en),
                        Some(TypeDef::Union(un)) => return CppValueShape::Union(un),
                        Some(TypeDef::Simple(sim)) => current = &sim.base_type,
                        _ => return CppValueShape::AnyString,
                    }
                }
            }
        }
    }

    fn enum_parse_expr(&self, enum_def: &EnumDef, raw_expr: &str) -> String {
        let enum_name = type_ident(&enum_def.qname);
        if self.options.emit_enum_converters {
            let natural = to_cpp_type_name(&enum_def.qname.local);
            let stem = if enum_name == natural {
                AsSnakeCase(&enum_def.qname.local).to_string()
            } else {
                AsSnakeCase(&enum_name).to_string()
            };
            let func_name = format!("{stem}_from_string");
            format!(
                "[&]() -> {enum_name} {{ if (auto px_e = {func_name}({raw_expr})) return *px_e; throw std::runtime_error(\"Invalid {enum_name} enum value\"); }}()"
            )
        } else {
            let mut seen = HashSet::new();
            let mut body = String::from("[&]() -> ");
            body.push_str(&enum_name);
            body.push_str(" { std::string_view px_sv = ");
            body.push_str(raw_expr);
            body.push_str("; ");
            for v in &enum_def.variants {
                let var_name = self.unique_enum_variant(&v.name, &mut seen);
                let _ = write!(
                    body,
                    "if (px_sv == {}) return {enum_name}::{var_name}; ",
                    cpp_string_literal(&v.value)
                );
            }
            let _ = write!(
                body,
                "throw std::runtime_error(\"Invalid {enum_name} enum value\"); }}()"
            );
            body
        }
    }

    fn enum_format_expr(&self, enum_def: &EnumDef, val_expr: &str) -> String {
        if self.options.emit_enum_converters {
            format!("to_string({val_expr})")
        } else {
            let enum_name = type_ident(&enum_def.qname);
            let mut seen = HashSet::new();
            let mut body = format!("[&]() -> std::string_view {{ switch ({val_expr}) {{ ");
            for v in &enum_def.variants {
                let var_name = self.unique_enum_variant(&v.name, &mut seen);
                let _ = write!(
                    body,
                    "case {enum_name}::{var_name}: return {}; ",
                    cpp_string_literal(&v.value)
                );
            }
            body.push_str("} return \"\"; }()");
            body
        }
    }

    fn cpp_scalar_parse_expr(&self, type_ref: &TypeRef, raw_expr: &str, ir: &SchemaIR) -> String {
        match self.resolve_cpp_shape(type_ref, ir) {
            CppValueShape::Enum(e) => self.enum_parse_expr(e, raw_expr),
            CppValueShape::Primitive(p) => match p {
                PrimitiveType::Boolean => format!("detail::parse_bool({raw_expr})"),
                PrimitiveType::Float => format!("detail::parse_float<float>({raw_expr})"),
                PrimitiveType::Double | PrimitiveType::Decimal => {
                    format!("detail::parse_float<double>({raw_expr})")
                }
                PrimitiveType::Byte => format!("detail::parse_int<std::int8_t>({raw_expr})"),
                PrimitiveType::Short => format!("detail::parse_int<std::int16_t>({raw_expr})"),
                PrimitiveType::Int => format!("detail::parse_int<std::int32_t>({raw_expr})"),
                PrimitiveType::Long => format!("detail::parse_int<std::int64_t>({raw_expr})"),
                PrimitiveType::UnsignedByte => {
                    format!("detail::parse_int<std::uint8_t>({raw_expr})")
                }
                PrimitiveType::UnsignedShort => {
                    format!("detail::parse_int<std::uint16_t>({raw_expr})")
                }
                PrimitiveType::UnsignedInt => {
                    format!("detail::parse_int<std::uint32_t>({raw_expr})")
                }
                PrimitiveType::UnsignedLong => {
                    format!("detail::parse_int<std::uint64_t>({raw_expr})")
                }
                PrimitiveType::Base64Binary | PrimitiveType::HexBinary => {
                    format!("detail::parse_bytes({raw_expr})")
                }
                _ => format!("std::string({raw_expr})"),
            },
            CppValueShape::AnyString
            | CppValueShape::Struct(_)
            | CppValueShape::Union(_)
            | CppValueShape::LexicalList(_) => {
                format!("std::string({raw_expr})")
            }
        }
    }

    fn cpp_scalar_format_expr(&self, type_ref: &TypeRef, val_expr: &str, ir: &SchemaIR) -> String {
        match self.resolve_cpp_shape(type_ref, ir) {
            CppValueShape::Enum(e) => self.enum_format_expr(e, val_expr),
            CppValueShape::Primitive(p) => match p {
                PrimitiveType::Boolean => format!("detail::format_bool({val_expr})"),
                PrimitiveType::Float | PrimitiveType::Double | PrimitiveType::Decimal => {
                    format!("detail::format_float({val_expr})")
                }
                PrimitiveType::Byte
                | PrimitiveType::Short
                | PrimitiveType::Int
                | PrimitiveType::Long
                | PrimitiveType::UnsignedByte
                | PrimitiveType::UnsignedShort
                | PrimitiveType::UnsignedInt
                | PrimitiveType::UnsignedLong => format!("detail::format_int({val_expr})"),
                PrimitiveType::Base64Binary | PrimitiveType::HexBinary => {
                    format!("detail::format_bytes({val_expr})")
                }
                _ => format!("std::string_view({val_expr})"),
            },
            CppValueShape::AnyString
            | CppValueShape::Struct(_)
            | CppValueShape::Union(_)
            | CppValueShape::LexicalList(_) => {
                format!("std::string_view({val_expr})")
            }
        }
    }

    fn cpp_union_metadata(&self, u: &UnionDef) -> (Vec<String>, bool, bool) {
        let union_name = type_ident(&u.qname);
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
        let mut params = Vec::new();
        if needs_wrappers {
            let mut seen_wrappers = HashSet::new();
            for (b, _) in &branch_types {
                let base_wrapper = format!("{}{}", union_name, to_cpp_type_name(&b.variant_name));
                let mut wrapper_name = base_wrapper.clone();
                let mut idx = 2;
                while !seen_wrappers.insert(wrapper_name.clone()) {
                    wrapper_name = format!("{base_wrapper}_{idx}");
                    idx += 1;
                }
                params.push(wrapper_name);
            }
        } else {
            for (_, mapped) in &branch_types {
                params.push(mapped.clone());
            }
        }
        let is_erased = params.len() > 16;
        (params, needs_wrappers, is_erased)
    }

    /// Emit `write_xml_to`, `to_xml`, `from_xml_node`, and `from_xml` definitions for a struct.
    pub(super) fn emit_struct_codec_impl(
        &self,
        out: &mut String,
        s: &StructDef,
        ir: &SchemaIR,
        inline: bool,
    ) {
        let struct_name = type_ident(&s.qname);
        let inline_kw = if inline { "inline " } else { "" };
        let fields = self.collect_inherited_fields(s, ir);

        let attr_fields: Vec<_> = fields
            .iter()
            .filter(|f| f.field.kind == FieldKind::Attribute)
            .collect();
        let elem_fields: Vec<_> = fields
            .iter()
            .filter(|f| f.field.kind == FieldKind::Element)
            .collect();
        let text_field = fields.iter().find(|f| f.field.kind == FieldKind::Text);
        let sc_base = self.resolve_simple_content_base_type(s, ir);

        // 1. write_xml_to
        writeln!(
            out,
            "{inline_kw}void {struct_name}::write_xml_to(std::string& px_out, std::string_view px_tag) const {{"
        )
        .unwrap();
        out.push_str("    px_out.push_back('<');\n    px_out.append(px_tag);\n");

        for inf in &attr_fields {
            let member = format!("this->{}::{}", inf.owner_name, inf.cpp_name);
            let f = inf.field;
            let xml_attr = cpp_string_literal(&f.xml_name);
            let optional = f.cardinality.is_optional() || f.nillable;
            if f.is_cycle_cut || f.type_ref.is_boxed() {
                let inner_expr = format!("(*{member})");
                let text_expr = self.cpp_scalar_format_expr(&f.type_ref, &inner_expr, ir);
                writeln!(out, "    if ({member}) {{").unwrap();
                writeln!(
                    out,
                    "        px_out.push_back(' '); px_out.append({xml_attr}); px_out.append(\"=\\\"\");\n        detail::append_xml_escaped(px_out, {text_expr}, true);\n        px_out.push_back('\"');"
                )
                .unwrap();
                out.push_str("    }\n");
            } else if optional {
                let inner_expr = format!("(*{member})");
                let text_expr = self.cpp_scalar_format_expr(&f.type_ref, &inner_expr, ir);
                writeln!(out, "    if ({member}.has_value()) {{").unwrap();
                writeln!(
                    out,
                    "        px_out.push_back(' '); px_out.append({xml_attr}); px_out.append(\"=\\\"\");\n        detail::append_xml_escaped(px_out, {text_expr}, true);\n        px_out.push_back('\"');"
                )
                .unwrap();
                out.push_str("    }\n");
            } else {
                let text_expr = self.cpp_scalar_format_expr(&f.type_ref, &member, ir);
                writeln!(
                    out,
                    "    px_out.push_back(' '); px_out.append({xml_attr}); px_out.append(\"=\\\"\");\n    detail::append_xml_escaped(px_out, {text_expr}, true);\n    px_out.push_back('\"');"
                )
                .unwrap();
            }
        }
        out.push_str("    px_out.push_back('>');\n");

        if elem_fields.is_empty() {
            if let Some(tf) = text_field {
                let member = format!("this->{}::{}", tf.owner_name, tf.cpp_name);
                let optional = tf.field.cardinality.is_optional() || tf.field.nillable;
                if optional {
                    let inner = format!("(*{member})");
                    writeln!(out, "    if ({member}.has_value()) {{").unwrap();
                    self.emit_cpp_text_write(out, &tf.field.type_ref, &inner, "        ", ir);
                    out.push_str("    }\n");
                } else {
                    self.emit_cpp_text_write(out, &tf.field.type_ref, &member, "    ", ir);
                }
            } else if let Some(ref base_ref) = sc_base {
                self.emit_cpp_text_write(out, base_ref, "this->value", "    ", ir);
            }
        }

        for inf in &elem_fields {
            let member = format!("this->{}::{}", inf.owner_name, inf.cpp_name);
            self.emit_cpp_element_write(out, inf.field, &member, ir);
        }

        out.push_str("    px_out.append(\"</\");\n    px_out.append(px_tag);\n    px_out.push_back('>');\n}\n\n");

        // 2. to_xml
        writeln!(
            out,
            "{inline_kw}std::string {struct_name}::to_xml(std::string_view px_tag) const {{\n    std::string px_out;\n    write_xml_to(px_out, px_tag);\n    return px_out;\n}}\n"
        )
        .unwrap();

        // 3. from_xml_node
        writeln!(
            out,
            "{inline_kw}{struct_name} {struct_name}::from_xml_node(const detail::XmlNode& px_node) {{\n    (void)px_node;\n    {struct_name} px_res{{}};"
        )
        .unwrap();

        for inf in &attr_fields {
            let target = format!("px_res.{}::{}", inf.owner_name, inf.cpp_name);
            let xml_attr = cpp_string_literal(&inf.field.xml_name);
            let parse_expr = self.cpp_scalar_parse_expr(&inf.field.type_ref, "*px_raw", ir);
            writeln!(
                out,
                "    if (const auto px_raw = px_node.attr({xml_attr})) {{"
            )
            .unwrap();
            if inf.field.is_cycle_cut || inf.field.type_ref.is_boxed() {
                let base_ty = self.unboxed_cpp_type(&inf.field.type_ref);
                writeln!(
                    out,
                    "        {target} = std::make_unique<{base_ty}>({parse_expr});"
                )
                .unwrap();
            } else {
                writeln!(out, "        {target} = {parse_expr};").unwrap();
            }
            out.push_str("    }\n");
        }

        if elem_fields.is_empty() {
            if let Some(tf) = text_field {
                let target = format!("px_res.{}::{}", tf.owner_name, tf.cpp_name);
                let parse_expr =
                    self.emit_cpp_single_element_parse_expr(&tf.field.type_ref, "px_node", ir);
                let optional = tf.field.cardinality.is_optional() || tf.field.nillable;
                if optional {
                    writeln!(
                        out,
                        "    if (!detail::trim_ws(px_node.text).empty()) {target} = {parse_expr};"
                    )
                    .unwrap();
                } else {
                    writeln!(out, "    {target} = {parse_expr};").unwrap();
                }
            } else if let Some(ref base_ref) = sc_base {
                let parse_expr = self.emit_cpp_single_element_parse_expr(base_ref, "px_node", ir);
                writeln!(out, "    px_res.value = {parse_expr};").unwrap();
            }
        }

        if !elem_fields.is_empty() {
            out.push_str("    for (const auto& px_child : px_node.children) {\n");
            out.push_str("        const auto px_tag = detail::local_name(px_child.tag);\n");
            let mut first = true;
            for inf in &elem_fields {
                let target = format!("px_res.{}::{}", inf.owner_name, inf.cpp_name);
                self.emit_cpp_element_read(out, inf.field, &target, &mut first, ir);
            }
            out.push_str("    }\n");
        }

        out.push_str("    return px_res;\n}\n\n");

        // 4. from_xml
        writeln!(
            out,
            "{inline_kw}{struct_name} {struct_name}::from_xml(std::string_view px_xml) {{\n    const auto px_node = detail::parse_xml_document(px_xml);\n    return from_xml_node(px_node);\n}}\n"
        )
        .unwrap();
    }

    fn unboxed_cpp_type(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Boxed(inner) => self.unboxed_cpp_type(inner),
            other => self.context.map_type_ref(other),
        }
    }

    fn emit_cpp_text_write(
        &self,
        out: &mut String,
        type_ref: &TypeRef,
        val_expr: &str,
        indent: &str,
        ir: &SchemaIR,
    ) {
        if let CppValueShape::LexicalList(item_ref) = self.resolve_cpp_shape(type_ref, ir) {
            writeln!(out, "{indent}{{ bool px_first = true;").unwrap();
            writeln!(out, "{indent}  for (const auto& px_tok : {val_expr}) {{").unwrap();
            writeln!(out, "{indent}      if (!px_first) px_out.push_back(' ');").unwrap();
            writeln!(out, "{indent}      px_first = false;").unwrap();
            let tok_fmt = self.cpp_scalar_format_expr(item_ref, "px_tok", ir);
            writeln!(
                out,
                "{indent}      detail::append_xml_escaped(px_out, {tok_fmt}, false);"
            )
            .unwrap();
            writeln!(out, "{indent}  }} }}").unwrap();
        } else {
            let expr = self.cpp_scalar_format_expr(type_ref, val_expr, ir);
            writeln!(
                out,
                "{indent}detail::append_xml_escaped(px_out, {expr}, false);"
            )
            .unwrap();
        }
    }

    fn emit_cpp_single_element_write(
        &self,
        out: &mut String,
        type_ref: &TypeRef,
        xml_name: &str,
        val_expr: &str,
        indent: &str,
        ir: &SchemaIR,
    ) {
        let tag_lit = cpp_string_literal(xml_name);
        match self.resolve_cpp_shape(type_ref, ir) {
            CppValueShape::Struct(_) => {
                writeln!(out, "{indent}{val_expr}.write_xml_to(px_out, {tag_lit});").unwrap();
            }
            CppValueShape::LexicalList(item_ref) => {
                writeln!(out, "{indent}{{ px_out.push_back('<'); px_out.append({tag_lit}); px_out.push_back('>');").unwrap();
                writeln!(out, "{indent}  bool px_first = true;").unwrap();
                writeln!(out, "{indent}  for (const auto& px_tok : {val_expr}) {{").unwrap();
                writeln!(out, "{indent}      if (!px_first) px_out.push_back(' ');").unwrap();
                writeln!(out, "{indent}      px_first = false;").unwrap();
                let tok_fmt = self.cpp_scalar_format_expr(item_ref, "px_tok", ir);
                writeln!(
                    out,
                    "{indent}      detail::append_xml_escaped(px_out, {tok_fmt}, false);"
                )
                .unwrap();
                writeln!(out, "{indent}  }}").unwrap();
                writeln!(out, "{indent}  px_out.append(\"</\"); px_out.append({tag_lit}); px_out.push_back('>'); }}").unwrap();
            }
            CppValueShape::Union(u) => {
                let (params, needs_wrappers, is_erased) = self.cpp_union_metadata(u);
                if xml_name.is_empty() {
                    writeln!(out, "{indent}switch ({val_expr}.index()) {{").unwrap();
                } else {
                    writeln!(
                        out,
                        "{indent}px_out.push_back('<'); px_out.append({tag_lit}); px_out.push_back('>');"
                    )
                    .unwrap();
                    writeln!(out, "{indent}switch ({val_expr}.index()) {{").unwrap();
                }
                for (idx, (branch, param)) in u.branches.iter().zip(&params).enumerate() {
                    if branch.xml_name == "#text" {
                        writeln!(out, "{indent}    case {idx}: break;").unwrap();
                        continue;
                    }
                    let access = if is_erased {
                        format!("(*static_cast<const {param}*>({val_expr}.data.get()))")
                    } else {
                        format!("std::get<{idx}>({val_expr})")
                    };
                    let inner_val = if needs_wrappers {
                        format!("{access}.value")
                    } else {
                        access
                    };
                    let (deref_val, needs_null_guard, raw_ptr) = if branch.type_ref.is_boxed() {
                        (format!("(*{inner_val})"), true, inner_val)
                    } else {
                        (inner_val, false, String::new())
                    };
                    writeln!(out, "{indent}    case {idx}: {{").unwrap();
                    if is_erased {
                        writeln!(out, "{indent}        if (!{val_expr}.data) break;").unwrap();
                    }
                    if needs_null_guard {
                        writeln!(out, "{indent}        if ({raw_ptr}) {{").unwrap();
                        if u.is_lexical() {
                            let fmt = self.cpp_scalar_format_expr(&branch.type_ref, &deref_val, ir);
                            writeln!(out, "{indent}            detail::append_xml_escaped(px_out, {fmt}, false);").unwrap();
                        } else {
                            self.emit_cpp_single_element_write(
                                out,
                                &branch.type_ref,
                                &branch.xml_name,
                                &deref_val,
                                &format!("{indent}            "),
                                ir,
                            );
                        }
                        writeln!(out, "{indent}        }}").unwrap();
                    } else if u.is_lexical() {
                        let fmt = self.cpp_scalar_format_expr(&branch.type_ref, &deref_val, ir);
                        writeln!(
                            out,
                            "{indent}        detail::append_xml_escaped(px_out, {fmt}, false);"
                        )
                        .unwrap();
                    } else {
                        self.emit_cpp_single_element_write(
                            out,
                            &branch.type_ref,
                            &branch.xml_name,
                            &deref_val,
                            &format!("{indent}        "),
                            ir,
                        );
                    }
                    writeln!(out, "{indent}        break;\n{indent}    }}").unwrap();
                }
                writeln!(out, "{indent}    default: break;\n{indent}}}").unwrap();
                if !xml_name.is_empty() {
                    writeln!(
                        out,
                        "{indent}px_out.append(\"</\"); px_out.append({tag_lit}); px_out.push_back('>');"
                    )
                    .unwrap();
                }
            }
            _ => {
                let fmt = self.cpp_scalar_format_expr(type_ref, val_expr, ir);
                writeln!(
                    out,
                    "{indent}px_out.push_back('<'); px_out.append({tag_lit}); px_out.push_back('>');\n{indent}detail::append_xml_escaped(px_out, {fmt}, false);\n{indent}px_out.append(\"</\"); px_out.append({tag_lit}); px_out.push_back('>');"
                )
                .unwrap();
            }
        }
    }

    fn emit_cpp_element_write(&self, out: &mut String, f: &FieldDef, member: &str, ir: &SchemaIR) {
        let is_list = f.cardinality.is_list();
        let is_boxed = f.is_cycle_cut || f.type_ref.is_boxed();
        let is_optional = f.cardinality.is_optional() || f.nillable;

        if is_list {
            writeln!(out, "    for (const auto& px_item : {member}) {{").unwrap();
            if is_boxed {
                out.push_str("        if (px_item) {\n");
                self.emit_cpp_single_element_write(
                    out,
                    &f.type_ref,
                    &f.xml_name,
                    "(*px_item)",
                    "            ",
                    ir,
                );
                out.push_str("        }\n");
            } else {
                self.emit_cpp_single_element_write(
                    out,
                    &f.type_ref,
                    &f.xml_name,
                    "px_item",
                    "        ",
                    ir,
                );
            }
            out.push_str("    }\n");
        } else if is_boxed {
            writeln!(out, "    if ({member}) {{").unwrap();
            self.emit_cpp_single_element_write(
                out,
                &f.type_ref,
                &f.xml_name,
                &format!("(*{member})"),
                "        ",
                ir,
            );
            if f.nillable && !f.cardinality.is_optional() && !f.xml_name.is_empty() {
                let tag_lit = cpp_string_literal(&f.xml_name);
                writeln!(
                    out,
                    "    }} else {{\n        px_out.push_back('<'); px_out.append({tag_lit}); px_out.append(\" xmlns:xsi=\\\"http://www.w3.org/2001/XMLSchema-instance\\\" xsi:nil=\\\"true\\\"/>\");\n    }}"
                )
                .unwrap();
            } else {
                out.push_str("    }\n");
            }
        } else if is_optional {
            writeln!(out, "    if ({member}.has_value()) {{").unwrap();
            self.emit_cpp_single_element_write(
                out,
                &f.type_ref,
                &f.xml_name,
                &format!("(*{member})"),
                "        ",
                ir,
            );
            if f.nillable && !f.cardinality.is_optional() && !f.xml_name.is_empty() {
                let tag_lit = cpp_string_literal(&f.xml_name);
                writeln!(
                    out,
                    "    }} else {{\n        px_out.push_back('<'); px_out.append({tag_lit}); px_out.append(\" xmlns:xsi=\\\"http://www.w3.org/2001/XMLSchema-instance\\\" xsi:nil=\\\"true\\\"/>\");\n    }}"
                )
                .unwrap();
            } else {
                out.push_str("    }\n");
            }
        } else {
            self.emit_cpp_single_element_write(out, &f.type_ref, &f.xml_name, member, "    ", ir);
        }
    }

    fn emit_cpp_single_element_parse_expr(
        &self,
        type_ref: &TypeRef,
        node_expr: &str,
        ir: &SchemaIR,
    ) -> String {
        match self.resolve_cpp_shape(type_ref, ir) {
            CppValueShape::Struct(st) => {
                let target = type_ident(&st.qname);
                format!("{target}::from_xml_node({node_expr})")
            }
            CppValueShape::LexicalList(item_ref) => {
                let vec_ty = self.unboxed_cpp_type(type_ref);
                let parse_tok = self.cpp_scalar_parse_expr(item_ref, "px_tok", ir);
                format!(
                    "[&]() -> {vec_ty} {{ {vec_ty} px_vec; std::string_view px_rem = detail::trim_ws({node_expr}.text); while (!px_rem.empty()) {{ std::size_t px_sp = 0; while (px_sp < px_rem.size() && !detail::is_ws(px_rem[px_sp])) ++px_sp; const auto px_tok = px_rem.substr(0, px_sp); px_vec.push_back({parse_tok}); px_rem.remove_prefix(px_sp); px_rem = detail::trim_ws(px_rem); }} return px_vec; }}()"
                )
            }
            CppValueShape::Union(u) if u.is_lexical() => {
                let union_name = type_ident(&u.qname);
                let (params, needs_wrappers, _) = self.cpp_union_metadata(u);
                let mut code =
                    format!("[&]() -> {union_name} {{ const auto& px_raw = {node_expr}.text; ");
                for (branch, param) in u.branches.iter().zip(&params) {
                    let parsed = self.cpp_scalar_parse_expr(&branch.type_ref, "px_raw", ir);
                    let wrapped = if branch.type_ref.is_boxed() {
                        let base_ty = self.unboxed_cpp_type(&branch.type_ref);
                        let mk = format!("std::make_unique<{base_ty}>({parsed})");
                        if needs_wrappers {
                            format!("{param}{{{mk}}}")
                        } else {
                            mk
                        }
                    } else if needs_wrappers {
                        format!("{param}{{{parsed}}}")
                    } else {
                        parsed
                    };
                    let _ = write!(
                        code,
                        "try {{ return {union_name}({wrapped}); }} catch (...) {{}} "
                    );
                }
                let _ = write!(
                    code,
                    "throw std::runtime_error(\"No {union_name} union branch accepted value\"); }}()"
                );
                code
            }
            _ => {
                let text_expr = format!("{node_expr}.text");
                self.cpp_scalar_parse_expr(type_ref, &text_expr, ir)
            }
        }
    }

    fn emit_cpp_element_read(
        &self,
        out: &mut String,
        f: &FieldDef,
        target: &str,
        first: &mut bool,
        ir: &SchemaIR,
    ) {
        let is_list = f.cardinality.is_list();
        let is_boxed = f.is_cycle_cut || f.type_ref.is_boxed();
        let base_ty = self.unboxed_cpp_type(&f.type_ref);

        if f.xml_name.is_empty() {
            if let CppValueShape::Union(u) = self.resolve_cpp_shape(&f.type_ref, ir) {
                let union_name = type_ident(&u.qname);
                let (params, needs_wrappers, _) = self.cpp_union_metadata(u);
                for (branch, param) in u.branches.iter().zip(&params) {
                    if branch.xml_name == "#text" {
                        continue;
                    }
                    let kw = if *first { "if" } else { "else if" };
                    *first = false;
                    writeln!(
                        out,
                        "        {kw} (px_tag == {}) {{",
                        cpp_string_literal(&branch.xml_name)
                    )
                    .unwrap();
                    let inner_parse =
                        self.emit_cpp_single_element_parse_expr(&branch.type_ref, "px_child", ir);
                    let branch_expr = if branch.type_ref.is_boxed() {
                        let inner_base = self.unboxed_cpp_type(&branch.type_ref);
                        let mk = format!("std::make_unique<{inner_base}>({inner_parse})");
                        if needs_wrappers {
                            format!("{param}{{{mk}}}")
                        } else {
                            mk
                        }
                    } else if needs_wrappers {
                        format!("{param}{{{inner_parse}}}")
                    } else {
                        inner_parse
                    };
                    let union_val = format!("{union_name}({branch_expr})");
                    self.emit_cpp_assign_value(
                        out,
                        target,
                        &base_ty,
                        &union_val,
                        is_list,
                        is_boxed,
                        "            ",
                    );
                    out.push_str("        }\n");
                }
                return;
            }
        }

        let kw = if *first { "if" } else { "else if" };
        *first = false;
        writeln!(
            out,
            "        {kw} (px_tag == {}) {{",
            cpp_string_literal(&f.xml_name)
        )
        .unwrap();

        if f.nillable {
            out.push_str("            if (px_child.is_nil()) {\n");
            if is_list {
                // Skip nil entries cleanly when list element isn't nullable
            } else if is_boxed {
                writeln!(out, "                {target} = nullptr;").unwrap();
            } else {
                writeln!(out, "                {target} = std::nullopt;").unwrap();
            }
            out.push_str("            } else {\n");
        }

        let indent = if f.nillable {
            "                "
        } else {
            "            "
        };
        if let CppValueShape::Union(u) = self.resolve_cpp_shape(&f.type_ref, ir) {
            if !u.is_lexical() {
                let union_name = type_ident(&u.qname);
                let (params, needs_wrappers, _) = self.cpp_union_metadata(u);
                writeln!(
                    out,
                    "{indent}if (!px_child.children.empty()) {{\n{indent}    const auto& px_u = px_child.children.front();\n{indent}    const auto px_utag = detail::local_name(px_u.tag);"
                )
                .unwrap();
                let mut ufirst = true;
                for (branch, param) in u.branches.iter().zip(&params) {
                    if branch.xml_name == "#text" {
                        continue;
                    }
                    let ukw = if ufirst { "if" } else { "else if" };
                    ufirst = false;
                    let inner_parse =
                        self.emit_cpp_single_element_parse_expr(&branch.type_ref, "px_u", ir);
                    let branch_expr = if branch.type_ref.is_boxed() {
                        let inner_base = self.unboxed_cpp_type(&branch.type_ref);
                        let mk = format!("std::make_unique<{inner_base}>({inner_parse})");
                        if needs_wrappers {
                            format!("{param}{{{mk}}}")
                        } else {
                            mk
                        }
                    } else if needs_wrappers {
                        format!("{param}{{{inner_parse}}}")
                    } else {
                        inner_parse
                    };
                    writeln!(
                        out,
                        "{indent}    {ukw} (px_utag == {}) {{",
                        cpp_string_literal(&branch.xml_name)
                    )
                    .unwrap();
                    let union_val = format!("{union_name}({branch_expr})");
                    self.emit_cpp_assign_value(
                        out,
                        target,
                        &base_ty,
                        &union_val,
                        is_list,
                        is_boxed,
                        &format!("{indent}        "),
                    );
                    writeln!(out, "{indent}    }}").unwrap();
                }
                writeln!(out, "{indent}}}").unwrap();
                if f.nillable {
                    out.push_str("            }\n");
                }
                out.push_str("        }\n");
                return;
            }
        }

        let parse_expr = self.emit_cpp_single_element_parse_expr(&f.type_ref, "px_child", ir);
        self.emit_cpp_assign_value(
            out,
            target,
            &base_ty,
            &parse_expr,
            is_list,
            is_boxed,
            indent,
        );

        if f.nillable {
            out.push_str("            }\n");
        }
        out.push_str("        }\n");
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_cpp_assign_value(
        &self,
        out: &mut String,
        target: &str,
        base_ty: &str,
        expr: &str,
        is_list: bool,
        is_boxed: bool,
        indent: &str,
    ) {
        if is_list && is_boxed {
            writeln!(
                out,
                "{indent}{target}.push_back(std::make_unique<{base_ty}>({expr}));"
            )
            .unwrap();
        } else if is_list {
            writeln!(out, "{indent}{target}.push_back({expr});").unwrap();
        } else if is_boxed {
            writeln!(
                out,
                "{indent}{target} = std::make_unique<{base_ty}>({expr});"
            )
            .unwrap();
        } else {
            writeln!(out, "{indent}{target} = {expr};").unwrap();
        }
    }
}
