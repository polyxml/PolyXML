use super::*;
use crate::codegen::flatten_fields;
use crate::ir::{FieldDef, FieldKind};

impl JavaCodegen {
    pub(super) fn emit_value_class(
        &self,
        out: &mut String,
        name: &str,
        ty: &TypeRef,
        facets: Option<&RestrictionFacets>,
        interface: Option<&str>,
        indent: &str,
    ) {
        let nested = if indent.is_empty() { "" } else { "static " };
        let implements = interface.map(|v| format!(", {v}")).unwrap_or_default();
        let ty_name = self.context.map_type_ref(ty);
        let _ = writeln!(out, "{indent}public {nested}final class {name} implements java.io.Serializable{implements} {{\n{indent}    private static final long serialVersionUID = 1L;");
        if self.options.bean_validation {
            let mut value_field = FieldDef::new("value", "value", FieldKind::Text, ty.clone());
            value_field.facets = facets.cloned();
            for annotation in self.bean_annotations(&value_field) {
                let _ = writeln!(out, "{indent}    {annotation}");
            }
        }
        if self.options.backend.is_jackson() {
            let _ = writeln!(out, "{indent}    @JsonValue @JacksonXmlText");
        }
        let _ = writeln!(out, "{indent}    private {ty_name} value;\n{indent}    public {name}() {{}}\n{indent}    public {name}({ty_name} value) {{ setValue(value); }}\n{indent}    public {ty_name} getValue() {{ return value; }}\n{indent}    public void setValue({ty_name} value) {{");
        if self.options.validate_facets {
            if let Some(facets) = facets {
                for check in self.build_facet_checks("value", ty, facets, false, ty.is_list()) {
                    let _ = writeln!(out, "{indent}        {check}");
                }
            }
        }
        let _ = writeln!(out, "{indent}        this.value = value;\n{indent}    }}\n{indent}    @Override public boolean equals(Object obj) {{ return obj instanceof {name} other && java.util.Objects.deepEquals(value, other.value); }}\n{indent}    @Override public int hashCode() {{ return java.util.Arrays.deepHashCode(new Object[] {{ value }}); }}\n{indent}    @Override public String toString() {{ return \"{name}[\" + value + \"]\"; }}");
        if self.options.backend.is_jackson() {
            let _ = writeln!(out, "{indent}    @JsonCreator public static {name} of({ty_name} value) {{ return new {name}(value); }}");
        }
        let _ = writeln!(out, "{indent}}}");
    }

    pub(super) fn model_fields<'a>(
        &self,
        s: &'a StructDef,
        ir: &'a SchemaIR,
    ) -> Vec<(&'a FieldDef, String)> {
        // Records cannot `extends`, so every record mode must inline the base
        // chain — a plain record previously declared only its own components
        // and silently dropped inherited fields. Classes re-slice back to
        // their own tail for field declarations because they inherit instead.
        let fields = flatten_fields(s, ir);
        let mut seen = HashSet::new();
        fields
            .into_iter()
            .map(|f| (f, self.unique_field_name(&f.name, &mut seen)))
            .collect()
    }

    pub(super) fn field_type(&self, f: &FieldDef) -> String {
        if f.cardinality.is_list() {
            format!("java.util.List<{}>", self.context.boxed_type(&f.type_ref))
        } else if f.type_ref.is_list() {
            self.context.map_type_ref(&f.type_ref)
        } else if f.cardinality.is_optional() || f.nillable {
            let ty = self.context.boxed_type(&f.type_ref);
            if self.options.use_records {
                format!("java.util.Optional<{ty}>")
            } else {
                ty
            }
        } else {
            self.context.map_type_ref(&f.type_ref)
        }
    }

    pub(super) fn field_initial(&self, f: &FieldDef) -> String {
        if f.cardinality.is_list() || f.type_ref.is_list() {
            "new java.util.ArrayList<>()".into()
        } else if self.options.use_records && (f.cardinality.is_optional() || f.nillable) {
            "java.util.Optional.empty()".into()
        } else {
            match self.field_type(f).as_str() {
                "boolean" => "false",
                "byte" | "short" | "int" | "long" | "float" | "double" => "0",
                _ => "null",
            }
            .into()
        }
    }

    pub(super) fn accessor(&self, f: &FieldDef, id: &str) -> String {
        if self.options.use_records {
            format!("{id}()")
        } else {
            format!(
                "{}{}()",
                if self.field_type(f) == "boolean" {
                    "is"
                } else {
                    "get"
                },
                bean_suffix(id)
            )
        }
    }

    fn emit_property_annotations(&self, out: &mut String, f: &FieldDef, indent: &str) {
        if !self.options.backend.is_jackson() {
            return;
        }
        let _ = writeln!(out, "{indent}@JsonProperty({:?})", f.xml_name);
        if f.kind == FieldKind::Text {
            let _ = writeln!(out, "{indent}@JacksonXmlText");
        } else {
            let _ = writeln!(
                out,
                "{indent}@JacksonXmlProperty(localName = {:?}, isAttribute = {}, namespace = {:?})",
                f.xml_name,
                f.kind == FieldKind::Attribute,
                f.namespace.as_deref().unwrap_or("")
            );
        }
        if f.cardinality.is_list() {
            let _ = writeln!(
                out,
                "{indent}@JacksonXmlElementWrapper(useWrapping = false)"
            );
        }
    }

    pub(super) fn emit_pojo(
        &self,
        out: &mut String,
        s: &StructDef,
        name: &str,
        indent: &str,
        ir: &SchemaIR,
    ) {
        if let Some(doc) = &s.documentation {
            self.emit_docstring(out, doc, indent);
        }
        if self.options.backend.is_jackson() {
            let _ = writeln!(out, "{indent}@JsonIgnoreProperties(ignoreUnknown = true)\n{indent}@JsonInclude(JsonInclude.Include.NON_NULL)\n{indent}@JacksonXmlRootElement(localName = {:?}, namespace = {:?})", s.qname.local, s.qname.namespace.as_deref().unwrap_or(""));
            // Bind the explicitly annotated fields only; avoids duplicate XML properties after bean name normalization.
            let _ = writeln!(out, "{indent}@JsonAutoDetect(fieldVisibility = JsonAutoDetect.Visibility.NONE, getterVisibility = JsonAutoDetect.Visibility.NONE, isGetterVisibility = JsonAutoDetect.Visibility.NONE, setterVisibility = JsonAutoDetect.Visibility.NONE)");
        }
        let base = s
            .base_type
            .as_ref()
            .filter(|q| matches!(ir.types.get(q), Some(TypeDef::Struct(_))));
        let extends = base
            .map(|q| format!(" extends {}", type_ident(q)))
            .unwrap_or_default();
        let nested = if indent.is_empty() { "" } else { "static " };
        let abstract_kw = if s.is_abstract { "abstract " } else { "" };
        let _ = writeln!(out, "{indent}public {nested}{abstract_kw}class {name}{extends} implements java.io.Serializable {{\n{indent}    private static final long serialVersionUID = 1L;");
        let fields = self.model_fields(s, ir);
        let own = &fields[fields.len() - s.fields.len()..];
        for (f, id) in own {
            self.emit_property_annotations(out, f, &format!("{indent}    "));
            for annotation in self.bean_annotations(f) {
                let _ = writeln!(out, "{indent}    {annotation}");
            }
            let _ = writeln!(
                out,
                "{indent}    private {} {id} = {};",
                self.field_type(f),
                self.field_initial(f)
            );
        }
        let _ = writeln!(out, "\n{indent}    public {name}() {{}}");
        for (f, id) in own {
            let ty = self.field_type(f);
            let suffix = bean_suffix(id);
            let get = self.accessor(f, id);
            let _ = writeln!(out, "{indent}    public {ty} {get} {{");
            if f.cardinality.is_list() || f.type_ref.is_list() {
                let _ = writeln!(out, "{indent}        if (this.{id} == null) this.{id} = new java.util.ArrayList<>();");
            }
            let _ = writeln!(out, "{indent}        return this.{id};\n{indent}    }}\n{indent}    public void set{suffix}({ty} {id}) {{");
            if self.options.validate_facets {
                if let Some(facets) = &f.facets {
                    let checks = self.build_facet_checks(
                        id,
                        &f.type_ref,
                        facets,
                        false,
                        f.cardinality.is_list() || f.type_ref.is_list(),
                    );
                    let nullable = f.cardinality.is_optional() || f.nillable;
                    if nullable && !checks.is_empty() {
                        let _ = writeln!(out, "{indent}        if ({id} != null) {{");
                    }
                    for check in &checks {
                        let _ = writeln!(out, "{indent}        {check}");
                    }
                    if nullable && !checks.is_empty() {
                        let _ = writeln!(out, "{indent}        }}");
                    }
                }
            }
            let _ = writeln!(out, "{indent}        this.{id} = {id};\n{indent}    }}");
        }
        let mut equals = vec!["true".to_string()];
        if base.is_some() {
            equals.push("super.equals(other)".into());
        }
        equals.extend(
            own.iter()
                .map(|(_, id)| format!("java.util.Objects.deepEquals(this.{id}, other.{id})")),
        );
        let _ = writeln!(out, "{indent}    @Override public boolean equals(Object obj) {{\n{indent}        if (this == obj) return true;\n{indent}        if (obj == null || getClass() != obj.getClass()) return false;\n{indent}        {name} other = ({name}) obj;\n{indent}        return {};\n{indent}    }}", equals.join(" && "));
        let mut values: Vec<String> = own.iter().map(|(_, id)| format!("this.{id}")).collect();
        if base.is_some() {
            values.insert(0, "super.hashCode()".into());
        }
        let _ = writeln!(out, "{indent}    @Override public int hashCode() {{ return java.util.Arrays.deepHashCode(new Object[] {{ {} }}); }}", values.join(", "));
        let display = fields
            .iter()
            .map(|(f, id)| format!("\"{id}=\" + {}", self.accessor(f, id)))
            .collect::<Vec<_>>()
            .join(" + \", \" + ");
        let display = if display.is_empty() {
            "\"\"".into()
        } else {
            display
        };
        let _ = writeln!(out, "{indent}    @Override public String toString() {{ return \"{name}[\" + {display} + \"]\"; }}");
        if self.options.emit_builder {
            self.emit_builder(out, s, name, indent, ir);
        }
        let _ = writeln!(out, "{indent}}}");
    }

    pub(super) fn emit_builder(
        &self,
        out: &mut String,
        s: &StructDef,
        name: &str,
        indent: &str,
        ir: &SchemaIR,
    ) {
        let fields = self.model_fields(s, ir);
        let base = if self.options.use_records {
            None
        } else {
            s.base_type
                .as_ref()
                .filter(|q| matches!(ir.types.get(q), Some(TypeDef::Struct(_))))
        };
        let extends = base
            .map(|q| {
                let n = type_ident(q);
                format!(" extends {n}.{n}Builder")
            })
            .unwrap_or_default();
        let abstract_kw = if s.is_abstract && !self.options.use_records {
            "abstract "
        } else {
            ""
        };
        if abstract_kw.is_empty() {
            let _ = writeln!(out, "{indent}    public static {name}Builder builder() {{ return new {name}Builder(); }}");
        }
        let _ = writeln!(
            out,
            "{indent}    public static {abstract_kw}class {name}Builder{extends} {{"
        );
        let inherited = if self.options.use_records {
            0
        } else {
            fields.len() - s.fields.len()
        };
        for (i, (f, id)) in fields.iter().enumerate() {
            let ty = self.field_type(f);
            if i >= inherited {
                let _ = writeln!(
                    out,
                    "{indent}        protected {ty} {id} = {};",
                    self.field_initial(f)
                );
            }
            let _ = writeln!(out, "{indent}        public {name}Builder {id}({ty} {id}) {{ this.{id} = {id}; return this; }}");
        }
        if !abstract_kw.is_empty() {
            let _ = writeln!(out, "{indent}        public abstract {name} build();");
        } else {
            let _ = writeln!(out, "{indent}        public {name} build() {{");
            if self.options.use_records {
                let _ = writeln!(
                    out,
                    "{indent}            return new {name}({});",
                    fields
                        .iter()
                        .map(|(_, id)| id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            } else {
                let _ = writeln!(out, "{indent}            {name} result = new {name}();");
                for (f, id) in &fields {
                    let value = if f.cardinality.is_list() || f.type_ref.is_list() {
                        format!("{id} == null ? null : new java.util.ArrayList<>({id})")
                    } else {
                        id.clone()
                    };
                    let _ = writeln!(
                        out,
                        "{indent}            result.set{}({value});",
                        bean_suffix(id)
                    );
                }
                let _ = writeln!(out, "{indent}            return result;");
            }
            let _ = writeln!(out, "{indent}        }}");
        }
        let _ = writeln!(out, "{indent}    }}");
    }
}

pub(super) fn bean_suffix(id: &str) -> String {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => "Field".into(),
    }
}
