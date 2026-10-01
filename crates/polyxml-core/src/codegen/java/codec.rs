//! Reflection-free, streaming StAX companion codecs. Each reader consumes one element,
//! leaving the cursor on its END_ELEMENT so nested codecs compose without buffering.
use super::models::bean_suffix;
use super::*;
use crate::ir::{FieldDef, FieldKind};

impl JavaCodegen {
    /// Check direct-codec constraints before emitting compilation units.
    /// Wildcard XML and runtime xsi:type dispatch require a schema-aware dynamic runtime.
    pub fn validate_direct_codecs(&self, ir: &SchemaIR) -> Result<(), String> {
        fn check_type(ty: &TypeRef, ir: &SchemaIR) -> Result<(), String> {
            match ty {
                TypeRef::Primitive(PrimitiveType::AnyType | PrimitiveType::AnySimpleType) => {
                    Err("direct codecs do not support xs:anyType or xs:anySimpleType".into())
                }
                TypeRef::Named(q) if !ir.types.contains_key(q) => {
                    Err(format!("unresolved direct-codec type: {}", q.local))
                }
                TypeRef::Boxed(t) | TypeRef::List(t) => check_type(t, ir),
                _ => Ok(()),
            }
        }
        let names: HashSet<_> = ir.emitted_types().map(|d| type_ident(d.qname())).collect();
        for def in ir.emitted_types() {
            let name = type_ident(def.qname());
            if names.contains(&format!("{name}Codec")) {
                return Err(format!(
                    "direct codec name {name}Codec collides with a schema type"
                ));
            }
            match def {
                TypeDef::Struct(s) => {
                    for f in &s.fields {
                        if matches!(f.kind, FieldKind::Any | FieldKind::AnyAttribute) {
                            return Err(format!(
                                "direct codecs do not support wildcard field {}.{}",
                                name, f.name
                            ));
                        }
                        check_type(&f.type_ref, ir)?;
                        if matches!(f.kind, FieldKind::Attribute | FieldKind::Text) {
                            if let TypeRef::Named(q) = &f.type_ref {
                                if matches!(ir.types.get(q), Some(TypeDef::Struct(_)))
                                    || matches!(ir.types.get(q), Some(TypeDef::Union(u)) if !u.is_lexical())
                                {
                                    return Err(format!(
                                        "non-scalar XML text/attribute {}.{}",
                                        name, f.name
                                    ));
                                }
                            }
                        }
                    }
                }
                TypeDef::Simple(s) => check_type(
                    if s.facets.patterns.is_empty() {
                        &s.base_type
                    } else {
                        crate::codegen::primitive_base(&s.base_type, ir)
                    },
                    ir,
                )?,
                TypeDef::Union(u) => {
                    for b in &u.branches {
                        check_type(&b.type_ref, ir)?;
                    }
                }
                TypeDef::Enum(_) => {}
            }
        }
        Ok(())
    }

    pub(super) fn generate_codec(&self, def: &TypeDef, ir: &SchemaIR) -> String {
        let name = type_ident(def.qname());
        let mut out = String::new();
        self.emit_file_header(&mut out);
        out.push_str("import javax.xml.stream.*;\nimport java.io.*;\n\n");
        let _ = writeln!(
            out,
            "public final class {name}Codec {{\n    private {name}Codec() {{}}"
        );
        let root = ir
            .elements
            .values()
            .find(|e| e.type_ref == TypeRef::Named(def.qname().clone()))
            .map(|e| &e.qname)
            .unwrap_or(def.qname());
        out.push_str(
            r#"    private static final ThreadLocal<XMLInputFactory> READER_FACTORY = ThreadLocal.withInitial(() -> {
        XMLInputFactory factory = XMLInputFactory.newFactory();
        factory.setProperty(XMLInputFactory.SUPPORT_DTD, false);
        factory.setProperty("javax.xml.stream.isSupportingExternalEntities", false);
        return factory;
    });
    private static final ThreadLocal<XMLOutputFactory> WRITER_FACTORY =
        ThreadLocal.withInitial(XMLOutputFactory::newFactory);
"#,
        );
        let _ = writeln!(
            out,
            r#"    public static {name} readXml(InputStream input) throws XMLStreamException {{
        XMLStreamReader reader = READER_FACTORY.get().createXMLStreamReader(input);
        try {{ return readXml(reader); }} finally {{ reader.close(); }}
    }}
    public static void writeXml({name} value, OutputStream output) throws XMLStreamException {{
        XMLStreamWriter writer = WRITER_FACTORY.get().createXMLStreamWriter(output, "UTF-8");
        try {{ writeXml(value, writer); writer.flush(); }} finally {{ writer.close(); }}
    }}
    public static void writeXml({name} value, XMLStreamWriter writer) throws XMLStreamException {{
        writeXml(value, writer, {local:?}, {ns:?});
    }}
    private static void start(XMLStreamWriter writer, String local, String ns) throws XMLStreamException {{
        writer.writeStartElement("", local, ns);
        writer.writeDefaultNamespace(ns);
    }}
    private static void skip(XMLStreamReader reader) throws XMLStreamException {{
        int depth = 1;
        while (depth > 0 && reader.hasNext()) {{
            int event = reader.next();
            if (event == XMLStreamConstants.START_ELEMENT) depth++;
            else if (event == XMLStreamConstants.END_ELEMENT) depth--;
        }}
    }}
    private static void writeNil(XMLStreamWriter writer) throws XMLStreamException {{
        writer.writeNamespace("xsi", "http://www.w3.org/2001/XMLSchema-instance");
        writer.writeAttribute("xsi", "http://www.w3.org/2001/XMLSchema-instance", "nil", "true");
    }}
    private static boolean nil(XMLStreamReader reader) {{
        String value = reader.getAttributeValue("http://www.w3.org/2001/XMLSchema-instance", "nil");
        return "true".equals(value) || "1".equals(value);
    }}
    private static boolean booleanValue(String value) {{
        return switch (value.trim()) {{
            case "true", "1" -> true;
            case "false", "0" -> false;
            default -> throw new IllegalArgumentException("Invalid XML boolean: " + value);
        }};
    }}
    private static String floatingValue(String value) {{
        return switch (value.trim()) {{ case "INF" -> "Infinity"; case "-INF" -> "-Infinity"; default -> value.trim(); }};
    }}
    private static String floatingText(Number value) {{
        return value.toString().replace("Infinity", "INF");
    }}
    public static {name} readXml(XMLStreamReader reader) throws XMLStreamException {{
        while (reader.getEventType() != XMLStreamConstants.START_ELEMENT && reader.hasNext()) reader.next();
        if (reader.getEventType() != XMLStreamConstants.START_ELEMENT) throw new XMLStreamException("Expected start element");
        if (nil(reader)) {{ skip(reader); return null; }}
        if (reader.getAttributeValue("http://www.w3.org/2001/XMLSchema-instance", "type") != null)
            throw new XMLStreamException("Runtime xsi:type dispatch is not supported; select a concrete codec", reader.getLocation());
        try {{
"#,
            local = root.local,
            ns = root.namespace.as_deref().unwrap_or("")
        );
        match def {
            TypeDef::Struct(s) => self.emit_read_struct(&mut out, s, ir),
            TypeDef::Simple(s) => {
                let expr = self.parse_scalar(
                    if s.facets.patterns.is_empty() {
                        &s.base_type
                    } else {
                        crate::codegen::primitive_base(&s.base_type, ir)
                    },
                    "reader.getElementText()",
                    ir,
                );
                let _ = writeln!(out, "            return new {name}({expr});");
            }
            TypeDef::Enum(_) => {
                let _ = writeln!(
                    out,
                    "            return {name}.fromValue(reader.getElementText());"
                );
            }
            TypeDef::Union(u) => {
                if u.is_lexical() {
                    out.push_str("            String text = reader.getElementText();\n");
                    for b in &u.branches {
                        let expr = self.parse_scalar(&b.type_ref, "text", ir);
                        let variant = union_variant_name(b);
                        let _ = writeln!(out, "            try {{ return new {name}.{variant}({expr}); }} catch (RuntimeException ignored) {{ }}");
                    }
                    let _ = writeln!(out, "            throw new XMLStreamException(\"No {} union member accepts: \" + text);", name);
                } else {
                    let _=writeln!(out,"            {name} result = null;\n            while (reader.hasNext()) {{\n                int event = reader.next();\n                if (event == XMLStreamConstants.END_ELEMENT) return result;\n                if (event != XMLStreamConstants.START_ELEMENT) continue;");
                    for (i, b) in u.branches.iter().enumerate() {
                        let expr = self.read_value(&b.type_ref, ir);
                        let variant = union_variant_name(b);
                        let _=writeln!(out,"                {}if (reader.getLocalName().equals({:?})) result = new {name}.{variant}({expr});",if i==0 {""}else{"else "},b.xml_name);
                    }
                    out.push_str("                else skip(reader);\n            }\n            return result;\n");
                }
            }
        }
        out.push_str("        } catch (IllegalArgumentException | java.time.DateTimeException ex) { throw new XMLStreamException(\"Invalid XML value\", reader.getLocation(), ex); }\n    }\n");
        let concrete_check = if !self.options.use_records && matches!(def, TypeDef::Struct(_)) {
            format!("        if (value != null && value.getClass() != {name}.class) throw new XMLStreamException(\"Use the concrete type's codec to preserve derived fields\");\n")
        } else {
            String::new()
        };
        let is_union = matches!(def, TypeDef::Union(_));
        if is_union {
            let _ = writeln!(out, "    public static void writeXml({name} value, XMLStreamWriter writer, String local, String ns) throws XMLStreamException {{\n        boolean hasLocal = local != null && !local.isEmpty();\n        if (hasLocal) start(writer, local, ns);\n        if (value == null) {{\n            if (hasLocal) {{\n                writer.writeNamespace(\"xsi\", \"http://www.w3.org/2001/XMLSchema-instance\");\n                writer.writeAttribute(\"xsi\", \"http://www.w3.org/2001/XMLSchema-instance\", \"nil\", \"true\");\n                writer.writeEndElement();\n            }}\n            return;\n        }}");
        } else {
            let _=writeln!(out,"    public static void writeXml({name} value, XMLStreamWriter writer, String local, String ns) throws XMLStreamException {{\n{concrete_check}        start(writer, local, ns);\n        if (value == null) {{\n            writer.writeNamespace(\"xsi\", \"http://www.w3.org/2001/XMLSchema-instance\");\n            writer.writeAttribute(\"xsi\", \"http://www.w3.org/2001/XMLSchema-instance\", \"nil\", \"true\");\n            writer.writeEndElement(); return;\n        }}");
        }
        match def {
            TypeDef::Struct(s) => self.emit_write_struct(&mut out, s, ir),
            TypeDef::Enum(_) => out.push_str("        writer.writeCharacters(value.getValue());\n"),
            TypeDef::Simple(s) => {
                let access = if self.options.use_records {
                    "value.value()"
                } else {
                    "value.getValue()"
                };
                let expr = self.format_scalar(
                    if s.facets.patterns.is_empty() {
                        &s.base_type
                    } else {
                        crate::codegen::primitive_base(&s.base_type, ir)
                    },
                    access,
                    ir,
                );
                let _ = writeln!(out, "        writer.writeCharacters({expr});");
            }
            TypeDef::Union(u) => {
                for (i, b) in u.branches.iter().enumerate() {
                    let variant = union_variant_name(b);
                    let _ = writeln!(
                        out,
                        "        {}if (value instanceof {name}.{variant} branch) {{",
                        if i == 0 { "" } else { "else " }
                    );
                    let access = if self.options.use_records {
                        "branch.value()"
                    } else {
                        "branch.getValue()"
                    };
                    if u.is_lexical() {
                        let text = self.format_scalar(&b.type_ref, access, ir);
                        let _ = writeln!(out, "            writer.writeCharacters({text});");
                    } else if u.is_mixed_content() && b.xml_name == "#text" {
                        let _ = writeln!(out, "            writer.writeCharacters({access});");
                    } else {
                        self.emit_write_value(
                            &mut out,
                            &b.type_ref,
                            access,
                            &b.xml_name,
                            u.qname.namespace.as_deref().unwrap_or(""),
                            ir,
                            "            ",
                        );
                    }
                    out.push_str("        }\n");
                }
            }
        }
        if is_union {
            out.push_str("        if (hasLocal) writer.writeEndElement();\n    }\n}\n");
        } else {
            out.push_str("        writer.writeEndElement();\n    }\n}\n");
        }
        out
    }

    fn emit_read_struct(&self, out: &mut String, s: &StructDef, ir: &SchemaIR) {
        let name = type_ident(&s.qname);
        if s.is_abstract && !self.options.use_records {
            out.push_str("            throw new XMLStreamException(\"Cannot instantiate abstract XML type without a concrete codec\");\n");
            return;
        }
        let fields = self.model_fields(s, ir);
        for (f, id) in &fields {
            let _ = writeln!(
                out,
                "            {} {id} = {};",
                self.field_type(f),
                self.field_initial(f)
            );
        }
        for (f, id) in &fields {
            if f.kind == FieldKind::Attribute {
                let _=writeln!(out,"            {{ String raw = reader.getAttributeValue({:?}, {:?}); if (raw != null) {{",f.namespace.as_deref().unwrap_or(""),f.xml_name);
                let expr = self.parse_scalar(&f.type_ref, "raw", ir);
                self.assign_value(out, f, id, &expr, "                ");
                out.push_str("            } }\n");
            }
        }
        let text_field = fields.iter().find(|(f, _)| f.kind == FieldKind::Text);
        if text_field.is_some() {
            out.push_str("            StringBuilder text = new StringBuilder();\n");
        }
        out.push_str("            while (reader.hasNext()) {\n                int event = reader.next();\n                if (event == XMLStreamConstants.END_ELEMENT) break;\n");
        if text_field.is_some() {
            out.push_str("                if (event == XMLStreamConstants.CHARACTERS || event == XMLStreamConstants.CDATA) text.append(reader.getText());\n");
        }
        if ir.has_ordered_content(s) {
            if let Some((field, id, union)) = fields.iter().find_map(|(field, id)| {
                self.resolve_union_def(&field.type_ref, ir)
                    .filter(|union| union.is_mixed_content())
                    .map(|union| (field, id, union))
            }) {
                let union_name = type_ident(&union.qname);
                let text_branch = union
                    .branches
                    .iter()
                    .find(|branch| branch.xml_name == "#text");
                if let Some(text_branch) = text_branch {
                    let variant = union_variant_name(text_branch);
                    let val = format!("new {union_name}.{variant}(reader.getText())");
                    out.push_str("                if (event == XMLStreamConstants.CHARACTERS || event == XMLStreamConstants.CDATA || event == XMLStreamConstants.SPACE) {\n");
                    self.assign_value(out, field, id, &val, "                    ");
                    out.push_str("                    continue;\n                }\n");
                }
            }
        }
        out.push_str("                if (event != XMLStreamConstants.START_ELEMENT) continue;\n");
        let mut first = true;
        for (f, id) in &fields {
            if f.kind != FieldKind::Element {
                continue;
            }
            if f.xml_name.is_empty() {
                if let Some(u) = self.resolve_union_def(&f.type_ref, ir) {
                    let union_name = type_ident(&u.qname);
                    for branch in &u.branches {
                        if branch.xml_name == "#text" {
                            continue;
                        }
                        let _ = writeln!(
                            out,
                            "                {}if (reader.getLocalName().equals({:?})) {{",
                            if first { "" } else { "else " },
                            branch.xml_name
                        );
                        first = false;
                        let expr = self.read_value(&branch.type_ref, ir);
                        let variant = union_variant_name(branch);
                        let union_val = format!("new {union_name}.{variant}({expr})");
                        self.assign_value(out, f, id, &union_val, "                    ");
                        out.push_str("                }\n");
                    }
                    continue;
                }
            }
            let _=writeln!(out,"                {}if (reader.getLocalName().equals({:?}) && java.util.Objects.equals(reader.getNamespaceURI() == null ? \"\" : reader.getNamespaceURI(), {:?})) {{",if first {""} else {"else "},f.xml_name,f.namespace.as_deref().unwrap_or(""));
            first = false;
            let expr = self.read_value(&f.type_ref, ir);
            if f.nillable {
                out.push_str("                    if (nil(reader)) { skip(reader);\n");
                self.assign_value(out, f, id, "null", "                        ");
                out.push_str("                    } else {\n");
            }
            self.assign_value(out, f, id, &expr, "                    ");
            if f.nillable {
                out.push_str("                    }\n");
            }
            out.push_str("                }\n");
        }
        if first {
            out.push_str("                skip(reader);\n");
        } else {
            out.push_str("                else skip(reader);\n");
        }
        out.push_str("            }\n");
        if let Some((f, id)) = text_field {
            let expr = self.parse_scalar(&f.type_ref, "text.toString()", ir);
            self.assign_value(out, f, id, &expr, "            ");
        }
        if self.options.use_records {
            let _ = writeln!(
                out,
                "            return new {name}({});",
                fields
                    .iter()
                    .map(|(_, id)| id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else {
            let _ = writeln!(out, "            {name} result = new {name}();");
            for (_, id) in fields {
                let _ = writeln!(out, "            result.set{}({id});", bean_suffix(&id));
            }
            out.push_str("            return result;\n");
        }
    }

    fn assign_value(&self, out: &mut String, f: &FieldDef, id: &str, expr: &str, indent: &str) {
        if f.cardinality.is_list() {
            let _ = writeln!(out, "{indent}{id}.add({expr});");
        } else if self.options.use_records
            && !f.type_ref.is_list()
            && (f.cardinality.is_optional() || f.nillable)
        {
            let _ = writeln!(out, "{indent}{id} = java.util.Optional.ofNullable({expr});");
        } else {
            let _ = writeln!(out, "{indent}{id} = {expr};");
        }
    }

    fn emit_write_struct(&self, out: &mut String, s: &StructDef, ir: &SchemaIR) {
        let mut fields = self.model_fields(s, ir);
        fields.sort_by_key(|(f, _)| f.kind != FieldKind::Attribute);
        for (idx, (f, id)) in fields.iter().enumerate() {
            if matches!(f.kind, FieldKind::Any | FieldKind::AnyAttribute) {
                continue;
            }
            let access = format!("value.{}", self.accessor(f, id));
            let list = f.cardinality.is_list();
            let optional = self.options.use_records
                && !list
                && !f.type_ref.is_list()
                && (f.cardinality.is_optional() || f.nillable);
            let expr = if optional {
                format!("{access}.orElse(null)")
            } else {
                access
            };
            let local = format!("field{idx}");
            let _ = writeln!(out, "        var {local} = {expr};");
            let nullable = !matches!(
                self.field_type(f).as_str(),
                "boolean" | "byte" | "short" | "int" | "long" | "float" | "double"
            );
            if nullable {
                let _ = writeln!(out, "        if ({local} != null) {{");
            }
            if list {
                let _ = writeln!(out, "        for (var item : {local}) {{");
            }
            let value = if list { "item" } else { &local };
            if f.kind == FieldKind::Attribute {
                let text = self.format_scalar(&f.type_ref, value, ir);
                if let Some(ns) = &f.namespace {
                    let _=writeln!(out,"        writer.writeNamespace(\"a{idx}\", {ns:?});\n        writer.writeAttribute(\"a{idx}\", {ns:?}, {:?}, {text});",f.xml_name);
                } else {
                    let _ = writeln!(
                        out,
                        "        writer.writeAttribute({:?}, {text});",
                        f.xml_name
                    );
                }
            } else if f.kind == FieldKind::Text {
                let text = self.format_scalar(&f.type_ref, value, ir);
                let _ = writeln!(out, "        writer.writeCharacters({text});");
            } else {
                self.emit_write_value(
                    out,
                    &f.type_ref,
                    value,
                    &f.xml_name,
                    f.namespace.as_deref().unwrap_or(""),
                    ir,
                    "        ",
                );
            }
            if list {
                out.push_str("        }\n");
            }
            if nullable {
                out.push_str("        }\n");
                if f.nillable && !list && f.kind == FieldKind::Element {
                    let _ = writeln!(out, "        else {{ start(writer, {:?}, {:?}); writeNil(writer); writer.writeEndElement(); }}", f.xml_name, f.namespace.as_deref().unwrap_or(""));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_write_value(
        &self,
        out: &mut String,
        ty: &TypeRef,
        value: &str,
        local: &str,
        ns: &str,
        ir: &SchemaIR,
        indent: &str,
    ) {
        match ty {
            TypeRef::Boxed(inner) => {
                self.emit_write_value(out, inner, value, local, ns, ir, indent)
            }
            TypeRef::Named(q) => {
                let name = type_ident(q);
                let _ = writeln!(
                    out,
                    "{indent}{name}Codec.writeXml({value}, writer, {local:?}, {ns:?});"
                );
            }
            _ => {
                let text = self.format_scalar(ty, value, ir);
                let _=writeln!(out,"{indent}start(writer, {local:?}, {ns:?});\n{indent}if (java.util.Objects.isNull({value})) writeNil(writer); else writer.writeCharacters({text});\n{indent}writer.writeEndElement();");
            }
        }
    }

    fn resolve_union_def<'a>(&self, ty: &'a TypeRef, ir: &'a SchemaIR) -> Option<&'a UnionDef> {
        match ty {
            TypeRef::Named(q) => {
                if let Some(TypeDef::Union(u)) = ir.types.get(q) {
                    Some(u)
                } else {
                    None
                }
            }
            TypeRef::Boxed(t) | TypeRef::List(t) => self.resolve_union_def(t, ir),
            _ => None,
        }
    }

    fn read_value(&self, ty: &TypeRef, ir: &SchemaIR) -> String {
        match ty {
            TypeRef::Named(q) => format!("{}Codec.readXml(reader)", type_ident(q)),
            TypeRef::Boxed(t) => self.read_value(t, ir),
            _ => self.parse_scalar(ty, "reader.getElementText()", ir),
        }
    }

    fn parse_scalar(&self, ty: &TypeRef, raw: &str, ir: &SchemaIR) -> String {
        match ty {
            TypeRef::Boxed(t) => self.parse_scalar(t, raw, ir),
            TypeRef::List(t) => {
                let parse = self.parse_scalar(t, "token", ir);
                // XML list values are whitespace-separated lexical tokens.
                format!("java.util.Arrays.stream(({raw}).trim().split(\"\\\\s+\")).filter(token -> !token.isEmpty()).map(token -> {parse}).collect(java.util.stream.Collectors.toCollection(java.util.ArrayList::new))")
            }
            TypeRef::Named(q) => match ir.types.get(q) {
                Some(TypeDef::Enum(_)) => {
                    format!("{}.fromValue({raw})", type_ident(q))
                }
                Some(TypeDef::Simple(s)) => format!(
                    "new {}({})",
                    type_ident(q),
                    self.parse_scalar(
                        if s.facets.patterns.is_empty() {
                            &s.base_type
                        } else {
                            crate::codegen::primitive_base(&s.base_type, ir)
                        },
                        raw,
                        ir
                    )
                ),
                _ => "null".into(),
            },
            TypeRef::Primitive(p) => match p {
                PrimitiveType::Boolean => format!("booleanValue({raw})"),
                PrimitiveType::Float => format!("Float.parseFloat(floatingValue({raw}))"),
                PrimitiveType::Double => format!("Double.parseDouble(floatingValue({raw}))"),
                PrimitiveType::Decimal => format!("new java.math.BigDecimal(({raw}).trim())"),
                PrimitiveType::Date => format!("java.time.LocalDate.parse(({raw}).trim())"),
                PrimitiveType::Time => format!("java.time.LocalTime.parse(({raw}).trim())"),
                PrimitiveType::DateTime => format!("java.time.Instant.parse(({raw}).trim())"),
                PrimitiveType::Duration => format!("java.time.Duration.parse(({raw}).trim())"),
                PrimitiveType::Base64Binary => {
                    format!("java.util.Base64.getMimeDecoder().decode({raw})")
                }
                PrimitiveType::HexBinary => {
                    format!("java.util.HexFormat.of().parseHex(({raw}).trim())")
                }
                _ => match self.context.map_primitive(*p) {
                    "byte" => format!("Byte.parseByte(({raw}).trim())"),
                    "short" => format!("Short.parseShort(({raw}).trim())"),
                    "int" => format!("Integer.parseInt(({raw}).trim())"),
                    "long" => format!("Long.parseLong(({raw}).trim())"),
                    _ => raw.into(),
                },
            },
        }
    }

    fn format_scalar(&self, ty: &TypeRef, value: &str, ir: &SchemaIR) -> String {
        match ty {
            TypeRef::Boxed(t)=>self.format_scalar(t,value,ir),
            TypeRef::List(t)=>format!("{value}.stream().map(item -> {}).collect(java.util.stream.Collectors.joining(\" \"))",self.format_scalar(t,"item",ir)),
            TypeRef::Named(q)=>match ir.types.get(q) {
                Some(TypeDef::Enum(_))=>format!("{value}.getValue()"),
                Some(TypeDef::Simple(s))=>self.format_scalar(if s.facets.patterns.is_empty() { &s.base_type } else { crate::codegen::primitive_base(&s.base_type, ir) },&format!("{value}.{}",if self.options.use_records {"value()"}else{"getValue()"}),ir),
                _=>format!("String.valueOf({value})"),
            },
            TypeRef::Primitive(PrimitiveType::Base64Binary)=>format!("java.util.Base64.getEncoder().encodeToString({value})"),
            TypeRef::Primitive(PrimitiveType::HexBinary)=>format!("java.util.HexFormat.of().formatHex({value})"),
            TypeRef::Primitive(PrimitiveType::Float|PrimitiveType::Double)=>format!("floatingText({value})"),
            _=>format!("String.valueOf({value})"),
        }
    }
}
