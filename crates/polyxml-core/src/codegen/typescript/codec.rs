use std::collections::HashSet;
use std::fmt::Write as FmtWrite;

use super::{type_ident, TypeScriptCodegen};
use crate::codegen::string_literal;
use crate::ir::{
    EnumDef, FieldDef, FieldKind, PrimitiveType, SchemaIR, StructDef, TypeDef, TypeRef, UnionDef,
};

enum TsValueShape<'a> {
    Struct(&'a StructDef),
    Enum(&'a EnumDef),
    Union(&'a UnionDef),
    LexicalList(&'a TypeRef),
    Primitive(PrimitiveType),
    AnyUnknown,
}

impl TypeScriptCodegen {
    /// Emit the self-contained runtime XML parser and scalar helpers (`_XmlNode` and `_xmlCodec`).
    pub(super) fn emit_xml_codec_runtime(&self, out: &mut String) {
        out.push_str(
            r###"export interface _XmlNode {
  tag: string;
  attrs: Record<string, string>;
  text: string;
  children: _XmlNode[];
}

const _xmlCodec = {
  localName(qname: string): string {
    const idx = qname.indexOf(":");
    return idx >= 0 ? qname.slice(idx + 1) : qname;
  },
  escapeText(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  },
  escapeAttr(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&apos;");
  },
  unescape(s: string): string {
    return s.replace(/&(#x[0-9a-fA-F]+|#[0-9]+|amp|lt|gt|quot|apos);/g, (m, ent: string) => {
      switch (ent) {
        case "amp": return "&";
        case "lt": return "<";
        case "gt": return ">";
        case "quot": return "\"";
        case "apos": return "'";
        default: {
          const cp = ent.startsWith("#x") || ent.startsWith("#X")
            ? parseInt(ent.slice(2), 16)
            : parseInt(ent.slice(1), 10);
          if (!Number.isFinite(cp) || cp < 0 || cp > 0x10ffff) return m;
          return String.fromCodePoint(cp);
        }
      }
    });
  },
  getAttr(node: _XmlNode, local: string): string | undefined {
    for (const [k, v] of Object.entries(node.attrs)) {
      if (k === "xmlns" || k.startsWith("xmlns:")) continue;
      if (k === local || _xmlCodec.localName(k) === local) return v;
    }
    return undefined;
  },
  isNil(node: _XmlNode): boolean {
    for (const [k, v] of Object.entries(node.attrs)) {
      if (_xmlCodec.localName(k) === "nil" && (k.includes(":") || k === "nil")) {
        return v === "true" || v === "1";
      }
    }
    return false;
  },
  parseBool(raw: string): boolean {
    const t = raw.trim();
    if (t === "true" || t === "1") return true;
    if (t === "false" || t === "0") return false;
    throw new Error(`Invalid XML boolean: ${raw}`);
  },
  parseIntValue(raw: string): number {
    const t = raw.trim();
    if (!/^[+-]?\d+$/.test(t)) throw new Error(`Invalid XML integer: ${raw}`);
    return Number(t);
  },
  parseFloatValue(raw: string): number {
    const t = raw.trim();
    if (t === "INF" || t === "+INF") return Infinity;
    if (t === "-INF") return -Infinity;
    if (t === "NaN") return NaN;
    if (t.length === 0) throw new Error("Empty XML float value");
    const n = Number(t);
    if (Number.isNaN(n)) throw new Error(`Invalid XML float: ${raw}`);
    return n;
  },
  formatFloat(n: number): string {
    if (Number.isNaN(n)) return "NaN";
    if (!Number.isFinite(n)) return n < 0 ? "-INF" : "INF";
    return String(n);
  },
  bytesToHex(bytes: Uint8Array): string {
    let out = "";
    for (let i = 0; i < bytes.length; i++) {
      out += bytes[i].toString(16).padStart(2, "0").toUpperCase();
    }
    return out;
  },
  hexToBytes(raw: string): Uint8Array {
    const t = raw.trim();
    if (t.length % 2 !== 0 || !/^[0-9a-fA-F]*$/.test(t)) {
      throw new Error(`Invalid XML hexBinary: ${raw}`);
    }
    const out = new Uint8Array(t.length / 2);
    for (let i = 0; i < t.length; i += 2) {
      out[i / 2] = parseInt(t.slice(i, i + 2), 16);
    }
    return out;
  },
  parseEnum<T extends string>(raw: string, allowed: readonly T[], name: string): T {
    if ((allowed as readonly string[]).includes(raw)) return raw as T;
    throw new Error(`Invalid ${name} enum value: ${raw}`);
  },
  parseXmlDocument(input: string): _XmlNode {
    const xml = input.startsWith("\uFEFF") ? input.slice(1) : input;
    let pos = 0;
    const isWs = (c: string): boolean => c === " " || c === "\t" || c === "\r" || c === "\n";
    const skipWs = (): void => {
      while (pos < xml.length && isWs(xml[pos])) pos++;
    };
    const skipMisc = (): void => {
      while (pos < xml.length) {
        skipWs();
        if (xml.startsWith("<!--", pos)) {
          const end = xml.indexOf("-->", pos + 4);
          if (end < 0) throw new Error("Unterminated XML comment");
          pos = end + 3;
          continue;
        }
        if (xml.startsWith("<?", pos)) {
          const end = xml.indexOf("?>", pos + 2);
          if (end < 0) throw new Error("Unterminated XML processing instruction");
          pos = end + 2;
          continue;
        }
        if (xml.startsWith("<!", pos) && !xml.startsWith("<![CDATA[", pos)) {
          let depth = 1;
          pos += 2;
          while (pos < xml.length && depth > 0) {
            if (xml[pos] === "<") depth++;
            else if (xml[pos] === ">") depth--;
            pos++;
          }
          if (depth > 0) throw new Error("Unterminated XML declaration");
          continue;
        }
        break;
      }
    };
    const parseElem = (): _XmlNode => {
      if (xml[pos] !== "<") throw new Error("Expected XML start tag");
      pos++;
      const tagStart = pos;
      while (pos < xml.length && !isWs(xml[pos]) && xml[pos] !== ">" && xml[pos] !== "/") pos++;
      if (pos === tagStart) throw new Error("Empty XML tag name");
      const tag = xml.slice(tagStart, pos);
      const attrs: Record<string, string> = {};
      const children: _XmlNode[] = [];
      let text = "";
      while (pos < xml.length) {
        skipWs();
        if (pos >= xml.length) throw new Error("Unexpected EOF in XML start tag");
        if (xml.startsWith("/>", pos)) {
          pos += 2;
          return { tag, attrs, text, children };
        }
        if (xml[pos] === ">") {
          pos++;
          break;
        }
        const aStart = pos;
        while (pos < xml.length && !isWs(xml[pos]) && xml[pos] !== "=" && xml[pos] !== ">" && xml[pos] !== "/") pos++;
        const aName = xml.slice(aStart, pos);
        skipWs();
        if (xml[pos] !== "=") throw new Error(`Expected '=' after attribute ${aName}`);
        pos++;
        skipWs();
        const quote = xml[pos];
        if (quote !== "\"" && quote !== "'") throw new Error(`Expected quoted value for ${aName}`);
        pos++;
        const vEnd = xml.indexOf(quote, pos);
        if (vEnd < 0) throw new Error(`Unterminated attribute value for ${aName}`);
        attrs[aName] = _xmlCodec.unescape(xml.slice(pos, vEnd));
        pos = vEnd + 1;
      }
      while (pos < xml.length) {
        const lt = xml.indexOf("<", pos);
        if (lt < 0) throw new Error(`Unexpected EOF before </${tag}>`);
        if (lt > pos) text += _xmlCodec.unescape(xml.slice(pos, lt));
        pos = lt;
        if (xml.startsWith("</", pos)) {
          pos += 2;
          const cStart = pos;
          while (pos < xml.length && !isWs(xml[pos]) && xml[pos] !== ">") pos++;
          const closeTag = xml.slice(cStart, pos);
          skipWs();
          if (xml[pos] !== ">") throw new Error("Expected '>' in closing tag");
          pos++;
          if (closeTag !== tag) throw new Error(`Mismatched closing tag </${closeTag}> for <${tag}>`);
          return { tag, attrs, text, children };
        }
        if (xml.startsWith("<![CDATA[", pos)) {
          const end = xml.indexOf("]]>", pos + 9);
          if (end < 0) throw new Error("Unterminated CDATA section");
          text += xml.slice(pos + 9, end);
          pos = end + 3;
          continue;
        }
        if (xml.startsWith("<!--", pos)) {
          const end = xml.indexOf("-->", pos + 4);
          if (end < 0) throw new Error("Unterminated XML comment");
          pos = end + 3;
          continue;
        }
        if (xml.startsWith("<?", pos)) {
          const end = xml.indexOf("?>", pos + 2);
          if (end < 0) throw new Error("Unterminated XML processing instruction");
          pos = end + 2;
          continue;
        }
        children.push(parseElem());
      }
      throw new Error(`Unexpected EOF in <${tag}>`);
    };
    skipMisc();
    if (pos >= xml.length || xml[pos] !== "<") throw new Error("Expected root XML element");
    return parseElem();
  },
} as const;

"###,
        );
    }

    fn resolve_ts_shape<'a>(&self, type_ref: &'a TypeRef, ir: &'a SchemaIR) -> TsValueShape<'a> {
        let mut current = type_ref;
        let mut seen = HashSet::new();
        loop {
            match current {
                TypeRef::Boxed(inner) => current = inner.as_ref(),
                TypeRef::List(inner) => return TsValueShape::LexicalList(inner.as_ref()),
                TypeRef::Primitive(p) => return TsValueShape::Primitive(*p),
                TypeRef::Named(qname) => {
                    if let Some(prim) = PrimitiveType::from_xsd_name(&qname.local) {
                        return TsValueShape::Primitive(prim);
                    }
                    if !seen.insert(qname.clone()) {
                        return TsValueShape::AnyUnknown;
                    }
                    match ir.types.get(qname) {
                        Some(TypeDef::Struct(st)) => return TsValueShape::Struct(st),
                        Some(TypeDef::Enum(en)) => return TsValueShape::Enum(en),
                        Some(TypeDef::Union(un)) => return TsValueShape::Union(un),
                        Some(TypeDef::Simple(sim)) => current = &sim.base_type,
                        None => return TsValueShape::AnyUnknown,
                    }
                }
            }
        }
    }

    fn ts_scalar_parse_expr(&self, type_ref: &TypeRef, raw_expr: &str, ir: &SchemaIR) -> String {
        match self.resolve_ts_shape(type_ref, ir) {
            TsValueShape::Enum(e) => {
                let enum_name = type_ident(&e.qname);
                let lits: Vec<String> = e
                    .variants
                    .iter()
                    .map(|v| string_literal(&v.value))
                    .collect();
                format!(
                    "_xmlCodec.parseEnum({raw_expr}, [{}] as const, {enum_name:?})",
                    lits.join(", ")
                )
            }
            TsValueShape::Primitive(p) => match p {
                PrimitiveType::Boolean => format!("_xmlCodec.parseBool({raw_expr})"),
                PrimitiveType::Float | PrimitiveType::Double | PrimitiveType::Decimal => {
                    format!("_xmlCodec.parseFloatValue({raw_expr})")
                }
                PrimitiveType::Byte
                | PrimitiveType::Short
                | PrimitiveType::Int
                | PrimitiveType::Long
                | PrimitiveType::UnsignedByte
                | PrimitiveType::UnsignedShort
                | PrimitiveType::UnsignedInt
                | PrimitiveType::UnsignedLong => format!("_xmlCodec.parseIntValue({raw_expr})"),
                PrimitiveType::Base64Binary | PrimitiveType::HexBinary => {
                    format!("_xmlCodec.hexToBytes({raw_expr})")
                }
                PrimitiveType::AnyType | PrimitiveType::AnySimpleType => raw_expr.to_string(),
                _ => raw_expr.to_string(),
            },
            TsValueShape::AnyUnknown
            | TsValueShape::Struct(_)
            | TsValueShape::Union(_)
            | TsValueShape::LexicalList(_) => raw_expr.to_string(),
        }
    }

    fn ts_scalar_format_expr(&self, type_ref: &TypeRef, val_expr: &str, ir: &SchemaIR) -> String {
        match self.resolve_ts_shape(type_ref, ir) {
            TsValueShape::Enum(_) => val_expr.to_string(),
            TsValueShape::Primitive(p) => match p {
                PrimitiveType::Boolean => format!("({val_expr} ? \"true\" : \"false\")"),
                PrimitiveType::Float | PrimitiveType::Double | PrimitiveType::Decimal => {
                    format!("_xmlCodec.formatFloat({val_expr})")
                }
                PrimitiveType::Byte
                | PrimitiveType::Short
                | PrimitiveType::Int
                | PrimitiveType::Long
                | PrimitiveType::UnsignedByte
                | PrimitiveType::UnsignedShort
                | PrimitiveType::UnsignedInt
                | PrimitiveType::UnsignedLong => format!("String({val_expr})"),
                PrimitiveType::Base64Binary | PrimitiveType::HexBinary => {
                    format!("_xmlCodec.bytesToHex({val_expr})")
                }
                PrimitiveType::AnyType | PrimitiveType::AnySimpleType => {
                    format!("String({val_expr} ?? \"\")")
                }
                _ => val_expr.to_string(),
            },
            TsValueShape::AnyUnknown => format!("String({val_expr} ?? \"\")"),
            TsValueShape::Struct(_) | TsValueShape::Union(_) | TsValueShape::LexicalList(_) => {
                format!("String({val_expr})")
            }
        }
    }

    fn ts_union_variant_names(&self, u: &UnionDef) -> Vec<String> {
        let mut seen = HashSet::new();
        u.branches
            .iter()
            .map(|b| self.unique_variant_name(&b.variant_name, &mut seen))
            .collect()
    }

    /// Emit `serialize{Name}`, `deserialize{Name}`, and `{Name}Codec` for a generated struct.
    pub(super) fn emit_struct_codec(
        &self,
        out: &mut String,
        s: &StructDef,
        ts_name: &str,
        ir: &SchemaIR,
    ) {
        let all_fields = self.collect_struct_fields_with_bases(s, ir);
        let mut seen_ids = HashSet::new();
        let field_entries: Vec<(&FieldDef, String)> = all_fields
            .into_iter()
            .map(|f| {
                let id = self.unique_field_name(&f.name, &mut seen_ids);
                (f, id)
            })
            .collect();

        let attr_fields: Vec<_> = field_entries
            .iter()
            .filter(|(f, _)| f.kind == FieldKind::Attribute)
            .collect();
        let elem_fields: Vec<_> = field_entries
            .iter()
            .filter(|(f, _)| f.kind == FieldKind::Element)
            .collect();
        let text_field = field_entries
            .iter()
            .find(|(f, _)| f.kind == FieldKind::Text);

        let default_tag = s.qname.local.as_str();

        // 1. serialize{ts_name}
        let _ = writeln!(
            out,
            "export function serialize{ts_name}(value: {ts_name}, tagName: string = {default_tag:?}): string {{"
        );
        if field_entries.is_empty() {
            out.push_str("  void value;\n");
        }
        out.push_str("  let out = `<${tagName}`;\n");

        for (f, field_id) in &attr_fields {
            let val_expr = format!("value.{field_id}");
            let is_opt = f.cardinality.is_optional() || f.nillable;
            let formatted = self.ts_attr_format_expr(f, &val_expr, ir);
            if is_opt {
                let _ = writeln!(
                    out,
                    "  if ({val_expr} !== undefined && {val_expr} !== null) {{ out += ` {}=\"${{_xmlCodec.escapeAttr({formatted})}}\"`; }}",
                    f.xml_name
                );
            } else {
                let _ = writeln!(
                    out,
                    "  out += ` {}=\"${{_xmlCodec.escapeAttr({formatted})}}\"`;",
                    f.xml_name
                );
            }
        }
        out.push_str("  out += \">\";\n");

        if elem_fields.is_empty() {
            if let Some((f, field_id)) = text_field {
                let val_expr = format!("value.{field_id}");
                let is_opt = f.cardinality.is_optional() || f.nillable;
                if is_opt {
                    let _ = writeln!(
                        out,
                        "  if ({val_expr} !== undefined && {val_expr} !== null) {{"
                    );
                    self.emit_ts_text_write(out, &f.type_ref, &val_expr, "    ", ir);
                    out.push_str("  }\n");
                } else {
                    self.emit_ts_text_write(out, &f.type_ref, &val_expr, "  ", ir);
                }
            }
        }

        for (f, field_id) in &elem_fields {
            self.emit_ts_element_write(out, f, &format!("value.{field_id}"), ir);
        }

        out.push_str("  out += `</${tagName}>`;\n  return out;\n}\n\n");

        // 2. deserialize{ts_name}
        let _ = writeln!(
            out,
            "export function deserialize{ts_name}(input: string | _XmlNode): {ts_name} {{"
        );
        out.push_str("  const node = typeof input === \"string\" ? _xmlCodec.parseXmlDocument(input) : input;\n");
        if field_entries.is_empty() {
            out.push_str("  void node;\n");
        }
        out.push_str("  const raw: Record<string, unknown> = {};\n");

        // Pre-initialize arrays and non-optional primitive defaults.
        for (f, field_id) in &field_entries {
            if f.cardinality.is_list() || f.type_ref.is_list() {
                let _ = writeln!(out, "  raw[{field_id:?}] = [];");
            } else if !f.cardinality.is_optional() && !f.nillable {
                if let TsValueShape::Primitive(p) = self.resolve_ts_shape(&f.type_ref, ir) {
                    let zero = match p {
                        PrimitiveType::Boolean => Some("false"),
                        PrimitiveType::Float
                        | PrimitiveType::Double
                        | PrimitiveType::Decimal
                        | PrimitiveType::Byte
                        | PrimitiveType::Short
                        | PrimitiveType::Int
                        | PrimitiveType::Long
                        | PrimitiveType::UnsignedByte
                        | PrimitiveType::UnsignedShort
                        | PrimitiveType::UnsignedInt
                        | PrimitiveType::UnsignedLong => Some("0"),
                        PrimitiveType::Base64Binary | PrimitiveType::HexBinary => {
                            Some("new Uint8Array(0)")
                        }
                        PrimitiveType::AnyType | PrimitiveType::AnySimpleType => None,
                        _ => Some("\"\""),
                    };
                    if let Some(val) = zero {
                        let _ = writeln!(out, "  raw[{field_id:?}] = {val};");
                    }
                }
            }
        }

        for (f, field_id) in &attr_fields {
            let xml_attr = f.xml_name.as_str();
            let _ = writeln!(
                out,
                "  {{ const attrVal = _xmlCodec.getAttr(node, {xml_attr:?}); if (attrVal !== undefined) {{ raw[{field_id:?}] = {}; }} }}",
                self.ts_single_value_parse_expr(&f.type_ref, "attrVal", true, ir)
            );
        }

        if elem_fields.is_empty() {
            if let Some((f, field_id)) = text_field {
                let parsed = self.ts_single_value_parse_expr(&f.type_ref, "node.text", true, ir);
                if f.cardinality.is_optional() || f.nillable {
                    let _ = writeln!(
                        out,
                        "  if (node.text.trim().length > 0) raw[{field_id:?}] = {parsed};"
                    );
                } else {
                    let _ = writeln!(out, "  raw[{field_id:?}] = {parsed};");
                }
            }
        }

        if !elem_fields.is_empty() {
            out.push_str("  for (const child of node.children) {\n");
            out.push_str("    const childTag = _xmlCodec.localName(child.tag);\n");
            let mut first = true;
            for (f, field_id) in &elem_fields {
                self.emit_ts_element_read(out, f, field_id, &mut first, ir);
            }
            out.push_str("  }\n");
        }

        let _ = writeln!(out, "  return raw as unknown as {ts_name};\n}}\n");

        // 3. Companion Codec object
        let _ = writeln!(
            out,
            "export const {ts_name}Codec = {{\n  toXml: serialize{ts_name},\n  fromXml: deserialize{ts_name},\n}} as const;\n"
        );
    }

    fn ts_attr_format_expr(&self, f: &FieldDef, val_expr: &str, ir: &SchemaIR) -> String {
        if let TsValueShape::LexicalList(inner) = self.resolve_ts_shape(&f.type_ref, ir) {
            let elem_fmt = self.ts_scalar_format_expr(inner, "x", ir);
            format!("{val_expr}.map((x) => {elem_fmt}).join(\" \")")
        } else {
            self.ts_scalar_format_expr(&f.type_ref, val_expr, ir)
        }
    }

    fn emit_ts_text_write(
        &self,
        out: &mut String,
        type_ref: &TypeRef,
        val_expr: &str,
        indent: &str,
        ir: &SchemaIR,
    ) {
        if let TsValueShape::LexicalList(inner) = self.resolve_ts_shape(type_ref, ir) {
            let elem_fmt = self.ts_scalar_format_expr(inner, "x", ir);
            let _ = writeln!(
                out,
                "{indent}out += _xmlCodec.escapeText({val_expr}.map((x) => {elem_fmt}).join(\" \"));"
            );
        } else {
            let fmt = self.ts_scalar_format_expr(type_ref, val_expr, ir);
            let _ = writeln!(out, "{indent}out += _xmlCodec.escapeText({fmt});");
        }
    }

    fn emit_ts_single_element_write(
        &self,
        out: &mut String,
        type_ref: &TypeRef,
        xml_name: &str,
        val_expr: &str,
        indent: &str,
        ir: &SchemaIR,
    ) {
        match self.resolve_ts_shape(type_ref, ir) {
            TsValueShape::Struct(st) => {
                let child_codec = format!("{}Codec", type_ident(&st.qname));
                let _ = writeln!(
                    out,
                    "{indent}out += {child_codec}.toXml({val_expr}, {xml_name:?});"
                );
            }
            TsValueShape::LexicalList(item_ref) => {
                let item_fmt = self.ts_scalar_format_expr(item_ref, "x", ir);
                let _ = writeln!(
                    out,
                    "{indent}out += `<{xml_name}>${{_xmlCodec.escapeText({val_expr}.map((x) => {item_fmt}).join(\" \"))}}</{xml_name}>`;"
                );
            }
            TsValueShape::Union(u) => {
                if u.is_lexical() {
                    let _ = writeln!(
                        out,
                        "{indent}out += `<{xml_name}>${{_xmlCodec.escapeText({val_expr} instanceof Uint8Array ? _xmlCodec.bytesToHex({val_expr}) : String({val_expr}))}}</{xml_name}>`;"
                    );
                    return;
                }
                let kind_names = self.ts_union_variant_names(u);
                if !xml_name.is_empty() {
                    let _ = writeln!(out, "{indent}out += `<{xml_name}>`;");
                }
                let _ = writeln!(out, "{indent}switch ({val_expr}.kind) {{");
                for (branch, kind) in u.branches.iter().zip(&kind_names) {
                    let _ = writeln!(out, "{indent}  case {kind:?}:");
                    if branch.xml_name == "#text" {
                        let _ = writeln!(
                            out,
                            "{indent}    out += _xmlCodec.escapeText(String({val_expr}.value));"
                        );
                    } else {
                        self.emit_ts_single_element_write(
                            out,
                            &branch.type_ref,
                            &branch.xml_name,
                            &format!("{val_expr}.value"),
                            &format!("{indent}    "),
                            ir,
                        );
                    }
                    let _ = writeln!(out, "{indent}    break;");
                }
                let _ = writeln!(out, "{indent}}}");
                if !xml_name.is_empty() {
                    let _ = writeln!(out, "{indent}out += `</{xml_name}>`;");
                }
            }
            _ => {
                let fmt = self.ts_scalar_format_expr(type_ref, val_expr, ir);
                let _ = writeln!(
                    out,
                    "{indent}out += `<{xml_name}>${{_xmlCodec.escapeText({fmt})}}</{xml_name}>`;"
                );
            }
        }
    }

    fn emit_ts_element_write(&self, out: &mut String, f: &FieldDef, val_expr: &str, ir: &SchemaIR) {
        let is_elem_list = f.cardinality.is_list();
        let is_opt = f.cardinality.is_optional();

        if is_elem_list {
            if is_opt {
                let _ = writeln!(out, "  if ({val_expr} !== undefined) {{");
                let _ = writeln!(out, "    for (const item of {val_expr}) {{");
                self.emit_ts_single_element_write(
                    out,
                    &f.type_ref,
                    &f.xml_name,
                    "item",
                    "      ",
                    ir,
                );
                out.push_str("    }\n  }\n");
            } else {
                let _ = writeln!(out, "  for (const item of {val_expr}) {{");
                self.emit_ts_single_element_write(
                    out,
                    &f.type_ref,
                    &f.xml_name,
                    "item",
                    "    ",
                    ir,
                );
                out.push_str("  }\n");
            }
            return;
        }

        if is_opt && f.nillable {
            let _ = writeln!(
                out,
                "  if ({val_expr} === null) {{ out += \"<{xml} xmlns:xsi=\\\"http://www.w3.org/2001/XMLSchema-instance\\\" xsi:nil=\\\"true\\\"/>\"; }} else if ({val_expr} !== undefined) {{",
                xml = f.xml_name
            );
            self.emit_ts_single_element_write(out, &f.type_ref, &f.xml_name, val_expr, "    ", ir);
            out.push_str("  }\n");
        } else if f.nillable {
            let _ = writeln!(
                out,
                "  if ({val_expr} === null) {{ out += \"<{xml} xmlns:xsi=\\\"http://www.w3.org/2001/XMLSchema-instance\\\" xsi:nil=\\\"true\\\"/>\"; }} else {{",
                xml = f.xml_name
            );
            self.emit_ts_single_element_write(out, &f.type_ref, &f.xml_name, val_expr, "    ", ir);
            out.push_str("  }\n");
        } else if is_opt {
            let _ = writeln!(out, "  if ({val_expr} !== undefined) {{");
            self.emit_ts_single_element_write(out, &f.type_ref, &f.xml_name, val_expr, "    ", ir);
            out.push_str("  }\n");
        } else {
            self.emit_ts_single_element_write(out, &f.type_ref, &f.xml_name, val_expr, "  ", ir);
        }
    }

    fn ts_single_value_parse_expr(
        &self,
        type_ref: &TypeRef,
        source_expr: &str,
        is_text_expr: bool,
        ir: &SchemaIR,
    ) -> String {
        let text_expr = if is_text_expr {
            source_expr.to_string()
        } else {
            format!("{source_expr}.text")
        };
        match self.resolve_ts_shape(type_ref, ir) {
            TsValueShape::Struct(st) if !is_text_expr => {
                let child_codec = format!("{}Codec", type_ident(&st.qname));
                format!("{child_codec}.fromXml({source_expr})")
            }
            TsValueShape::LexicalList(item_ref) => {
                let item_parse = self.ts_scalar_parse_expr(item_ref, "tok", ir);
                format!(
                    "({text_expr}.trim().length === 0 ? [] : {text_expr}.trim().split(/\\s+/).map((tok) => {item_parse}))"
                )
            }
            TsValueShape::Union(u) if u.is_lexical() => {
                let mut parts = Vec::new();
                for branch in &u.branches {
                    let parsed = self.ts_scalar_parse_expr(&branch.type_ref, "txt", ir);
                    parts.push(format!("try {{ return {parsed}; }} catch {{}}"));
                }
                format!(
                    "((txt: string) => {{ {} throw new Error(`Invalid union value: ${{txt}}`); }})({text_expr})",
                    parts.join(" ")
                )
            }
            _ => self.ts_scalar_parse_expr(type_ref, &text_expr, ir),
        }
    }

    fn emit_ts_element_read(
        &self,
        out: &mut String,
        f: &FieldDef,
        field_id: &str,
        first: &mut bool,
        ir: &SchemaIR,
    ) {
        let is_elem_list = f.cardinality.is_list();
        let assign_expr = |rhs: &str, indent: &str, out: &mut String| {
            if is_elem_list {
                let _ = writeln!(out, "{indent}(raw[{field_id:?}] as unknown[]).push({rhs});");
            } else {
                let _ = writeln!(out, "{indent}raw[{field_id:?}] = {rhs};");
            }
        };

        if f.xml_name.is_empty() {
            if let TsValueShape::Union(u) = self.resolve_ts_shape(&f.type_ref, ir) {
                let kind_names = self.ts_union_variant_names(u);
                for (branch, kind) in u.branches.iter().zip(&kind_names) {
                    if branch.xml_name == "#text" {
                        continue;
                    }
                    let kw = if *first { "if" } else { "else if" };
                    *first = false;
                    let _ = writeln!(out, "    {kw} (childTag === {:?}) {{", branch.xml_name);
                    let rhs = self.ts_single_value_parse_expr(&branch.type_ref, "child", false, ir);
                    assign_expr(
                        &format!("{{ kind: {kind:?}, value: {rhs} }}"),
                        "      ",
                        out,
                    );
                    out.push_str("    }\n");
                }
                return;
            }
        }

        let kw = if *first { "if" } else { "else if" };
        *first = false;
        let _ = writeln!(out, "    {kw} (childTag === {:?}) {{", f.xml_name);

        if f.nillable {
            out.push_str("      if (_xmlCodec.isNil(child)) {\n");
            if !is_elem_list {
                let _ = writeln!(out, "        raw[{field_id:?}] = null;");
            }
            out.push_str("      } else {\n");
        }

        let indent = if f.nillable { "        " } else { "      " };
        if let TsValueShape::Union(u) = self.resolve_ts_shape(&f.type_ref, ir) {
            if !u.is_lexical() {
                let kind_names = self.ts_union_variant_names(u);
                let _ = writeln!(
                    out,
                    "{indent}if (child.children.length > 0) {{\n{indent}  const uChild = child.children[0];\n{indent}  const uTag = _xmlCodec.localName(uChild.tag);"
                );
                let mut ufirst = true;
                for (branch, kind) in u.branches.iter().zip(&kind_names) {
                    if branch.xml_name == "#text" {
                        continue;
                    }
                    let ukw = if ufirst { "if" } else { "else if" };
                    ufirst = false;
                    let _ = writeln!(out, "{indent}  {ukw} (uTag === {:?}) {{", branch.xml_name);
                    let rhs =
                        self.ts_single_value_parse_expr(&branch.type_ref, "uChild", false, ir);
                    assign_expr(
                        &format!("{{ kind: {kind:?}, value: {rhs} }}"),
                        &format!("{indent}    "),
                        out,
                    );
                    let _ = writeln!(out, "{indent}  }}");
                }
                let _ = writeln!(out, "{indent}}}");
                if f.nillable {
                    out.push_str("      }\n");
                }
                out.push_str("    }\n");
                return;
            }
        }

        let rhs = self.ts_single_value_parse_expr(&f.type_ref, "child", false, ir);
        assign_expr(&rhs, indent, out);

        if f.nillable {
            out.push_str("      }\n");
        }
        out.push_str("    }\n");
    }
}
