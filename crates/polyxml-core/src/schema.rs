use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    Attribute,
    Element,
    Text,
    AnyAttribute,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarType {
    String,
    Int,
    Float,
    Bool,
    Decimal,
    XmlDate,
    XmlDateTime,
    XmlTime,
    XmlDuration,
    XmlGregorian(crate::ir::PrimitiveType),
    Enum(Vec<String>),
    Pattern(Box<ScalarType>, Vec<String>),
    Union(Vec<ScalarType>),
    Any,
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Scalar(ScalarType),
    List(Box<ValueType>),
    Nested(Arc<ModelSchema>),
}

#[derive(Debug, Clone)]
pub struct MixedBranchSchema {
    pub variant_name: String,
    pub xml_name: Vec<u8>,
    pub namespace: Option<String>,
    pub val_type: ValueType,
}

#[derive(Debug, Clone)]
pub struct MixedContentSchema {
    pub items_index: usize,
    pub branches: Vec<MixedBranchSchema>,
}

impl MixedContentSchema {
    pub fn branch(&self, local_name: &[u8]) -> Option<&MixedBranchSchema> {
        self.branches
            .iter()
            .find(|branch| branch.xml_name == local_name)
    }
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub xml_name: Vec<u8>,
    pub namespace: Option<String>,
    pub kind: FieldKind,
    pub val_type: ValueType,
    pub required: bool,
    pub default_value: Option<String>,
}

impl FieldSchema {
    pub fn new(
        name: impl Into<String>,
        xml_name: &[u8],
        kind: FieldKind,
        val_type: ValueType,
    ) -> Self {
        Self {
            name: name.into(),
            xml_name: xml_name.to_vec(),
            namespace: None,
            kind,
            val_type,
            required: false,
            default_value: None,
        }
    }

    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn with_namespace(self, ns: impl Into<String>) -> Self {
        self.namespace(ns)
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

#[derive(Debug, Clone)]
pub struct ModelSchema {
    pub name: String,
    pub xml_name: Vec<u8>,
    pub namespace: Option<String>,
    pub fields: Vec<FieldSchema>,
    pub element_map: HashMap<Vec<u8>, usize>,
    pub attribute_map: HashMap<Vec<u8>, usize>,
    pub text_field: Option<usize>,
    pub any_attribute_field: Option<usize>,
    pub any_element_field: Option<usize>,
    pub mixed_content: Option<MixedContentSchema>,
    /// Declared `abstract="true"` in the source schema.
    pub is_abstract: bool,
    /// Require the document element to match this schema's expanded QName.
    pub strict_root: bool,
    /// Concrete derivations eligible for `xsi:type` dispatch. Python may
    /// refresh this registry when subclasses are defined after first use.
    variants: Arc<RwLock<Vec<Arc<ModelSchema>>>>,
}

impl ModelSchema {
    pub fn builder(name: impl Into<String>) -> ModelSchemaBuilder {
        ModelSchemaBuilder::new(name)
    }

    /// Register the concrete derivations eligible for `xsi:type` dispatch.
    /// Replaces the registry when subclasses are discovered after first use.
    pub fn set_variants(&self, variants: Vec<Arc<ModelSchema>>) {
        *self.variants.write().unwrap_or_else(|p| p.into_inner()) = variants;
    }

    /// Registered derivations for `xsi:type` dispatch (empty when none).
    pub fn variants(&self) -> Vec<Arc<ModelSchema>> {
        self.variants
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    /// Whether any `xsi:type` derivations are registered for this type.
    pub fn has_variants(&self) -> bool {
        !self
            .variants
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .is_empty()
    }

    /// Look up a derivation by the QName local part of an `xsi:type` value.
    pub fn find_variant(&self, local: &[u8]) -> Option<Arc<ModelSchema>> {
        self.variants
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .find(|v| v.xml_name.as_slice() == local)
            .cloned()
    }

    /// Look up a derivation by its complete QName.
    pub fn find_variant_qname(&self, namespace: &str, local: &[u8]) -> Option<Arc<ModelSchema>> {
        self.variants
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .find(|v| {
                v.xml_name.as_slice() == local && v.namespace.as_deref().unwrap_or("") == namespace
            })
            .cloned()
    }

    /// Whether `candidate` is a registered derivation of `self`.
    pub fn matches_variant(&self, candidate: &ModelSchema) -> bool {
        self.variants
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .any(|v| {
                std::ptr::eq(v.as_ref(), candidate)
                    || (v.xml_name == candidate.xml_name && v.namespace == candidate.namespace)
            })
    }

    /// Construct a runtime `ModelSchema` from a compiled `SchemaIR`.
    pub fn from_ir(
        ir: &crate::ir::SchemaIR,
        root_name: Option<&str>,
    ) -> crate::error::Result<Arc<ModelSchema>> {
        use crate::ir::{PrimitiveType, TypeDef, TypeRef};
        use std::collections::HashSet;

        let (name, qname_opt, type_ref) = if let Some(target) = root_name {
            if let Some((qname, elem)) = ir.elements.iter().find(|(q, _)| q.local == target) {
                (
                    elem.qname.local.clone(),
                    Some(qname.clone()),
                    elem.type_ref.clone(),
                )
            } else if let Some((qname, _)) = ir.types.iter().find(|(q, _)| q.local == target) {
                (
                    qname.local.clone(),
                    Some(qname.clone()),
                    TypeRef::Named(qname.clone()),
                )
            } else {
                return Err(crate::error::PolyXmlError::SchemaError(format!(
                    "Root element or type '{}' not found in schema",
                    target
                )));
            }
        } else if let Some((_, elem)) = ir.elements.iter().next() {
            (
                elem.qname.local.clone(),
                Some(elem.qname.clone()),
                elem.type_ref.clone(),
            )
        } else if let Some((qname, _)) = ir.types.iter().next() {
            (
                qname.local.clone(),
                Some(qname.clone()),
                TypeRef::Named(qname.clone()),
            )
        } else {
            return Err(crate::error::PolyXmlError::SchemaError(
                "SchemaIR contains no elements or types".into(),
            ));
        };

        fn map_primitive(prim: PrimitiveType) -> ScalarType {
            match prim {
                PrimitiveType::String
                | PrimitiveType::NormalizedString
                | PrimitiveType::Token
                | PrimitiveType::Language
                | PrimitiveType::Name
                | PrimitiveType::NCName
                | PrimitiveType::Id
                | PrimitiveType::IdRef
                | PrimitiveType::IdRefs
                | PrimitiveType::Entity
                | PrimitiveType::Entities
                | PrimitiveType::NMTOKEN
                | PrimitiveType::NMTOKENS
                | PrimitiveType::AnyUri
                | PrimitiveType::QName => ScalarType::String,
                PrimitiveType::Boolean => ScalarType::Bool,
                PrimitiveType::Decimal => ScalarType::Decimal,
                PrimitiveType::Float | PrimitiveType::Double => ScalarType::Float,
                PrimitiveType::Duration => ScalarType::XmlDuration,
                PrimitiveType::DateTime => ScalarType::XmlDateTime,
                PrimitiveType::Time => ScalarType::XmlTime,
                PrimitiveType::Date => ScalarType::XmlDate,
                PrimitiveType::GDay
                | PrimitiveType::GMonth
                | PrimitiveType::GYear
                | PrimitiveType::GYearMonth
                | PrimitiveType::GMonthDay => ScalarType::XmlGregorian(prim),
                PrimitiveType::Int
                | PrimitiveType::Integer
                | PrimitiveType::NonPositiveInteger
                | PrimitiveType::NegativeInteger
                | PrimitiveType::Long
                | PrimitiveType::Short
                | PrimitiveType::Byte
                | PrimitiveType::NonNegativeInteger
                | PrimitiveType::UnsignedLong
                | PrimitiveType::UnsignedInt
                | PrimitiveType::UnsignedShort
                | PrimitiveType::UnsignedByte
                | PrimitiveType::PositiveInteger => ScalarType::Int,
                PrimitiveType::Base64Binary | PrimitiveType::HexBinary => ScalarType::String,
                _ => ScalarType::String,
            }
        }

        fn build_type(
            tr: &TypeRef,
            ir: &crate::ir::SchemaIR,
            visited: &mut HashSet<crate::ir::QName>,
        ) -> ValueType {
            match tr {
                TypeRef::Primitive(prim) => ValueType::Scalar(map_primitive(*prim)),
                TypeRef::List(inner) => ValueType::List(Box::new(build_type(inner, ir, visited))),
                TypeRef::Boxed(inner) => build_type(inner, ir, visited),
                TypeRef::Named(qname) => {
                    if let Some(type_def) = ir.types.get(qname) {
                        match type_def {
                            TypeDef::Struct(s) => {
                                if visited.contains(&s.qname) {
                                    ValueType::Nested(ModelSchema::builder(&s.qname.local).build())
                                } else {
                                    visited.insert(s.qname.clone());
                                    let child_schema = build_struct(s, ir, visited);
                                    visited.remove(&s.qname);
                                    ValueType::Nested(child_schema)
                                }
                            }
                            TypeDef::Simple(sim) => {
                                let base = build_type(&sim.base_type, ir, visited);
                                if sim.facets.patterns.is_empty() {
                                    base
                                } else if let ValueType::Scalar(scalar) = base {
                                    ValueType::Scalar(ScalarType::Pattern(
                                        Box::new(scalar),
                                        sim.facets.patterns.clone(),
                                    ))
                                } else {
                                    base
                                }
                            }
                            TypeDef::Enum(e) => ValueType::Scalar(ScalarType::Enum(
                                e.variants.iter().map(|v| v.value.clone()).collect(),
                            )),
                            TypeDef::Union(u) if u.is_lexical() => {
                                ValueType::Scalar(ScalarType::Union(
                                    u.branches
                                        .iter()
                                        .map(|b| match build_type(&b.type_ref, ir, visited) {
                                            ValueType::Scalar(scalar) => scalar,
                                            _ => ScalarType::Any,
                                        })
                                        .collect(),
                                ))
                            }
                            TypeDef::Union(_) => ValueType::Scalar(ScalarType::String),
                        }
                    } else {
                        ValueType::Scalar(ScalarType::String)
                    }
                }
            }
        }

        fn build_field(
            f: &crate::ir::FieldDef,
            ir: &crate::ir::SchemaIR,
            visited: &mut HashSet<crate::ir::QName>,
        ) -> FieldSchema {
            let kind = match f.kind {
                crate::ir::FieldKind::Attribute => FieldKind::Attribute,
                crate::ir::FieldKind::Element => FieldKind::Element,
                crate::ir::FieldKind::Text => FieldKind::Text,
                crate::ir::FieldKind::AnyAttribute => FieldKind::AnyAttribute,
                crate::ir::FieldKind::Any => FieldKind::Any,
            };

            let mut val_type = build_type(&f.type_ref, ir, visited);
            if f.cardinality.is_list() && !matches!(val_type, ValueType::List(_)) {
                val_type = ValueType::List(Box::new(val_type));
            }

            let mut field_schema = FieldSchema::new(&f.name, f.xml_name.as_bytes(), kind, val_type);
            field_schema.default_value = f.default_value.clone();
            if let Some(ref ns) = f.namespace {
                field_schema = field_schema.namespace(ns);
            }
            if f.cardinality.min_occurs > 0 && !f.cardinality.is_optional() {
                field_schema = field_schema.required();
            }
            field_schema
        }

        /// Whether `d` transitively extends `base` via `xs:extension`.
        fn derives_from(
            ir: &crate::ir::SchemaIR,
            d: &crate::ir::StructDef,
            base: &crate::ir::QName,
        ) -> bool {
            let mut seen: HashSet<crate::ir::QName> = HashSet::new();
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

        fn build_struct(
            s: &crate::ir::StructDef,
            ir: &crate::ir::SchemaIR,
            visited: &mut HashSet<crate::ir::QName>,
        ) -> Arc<ModelSchema> {
            let mut builder = ModelSchema::builder(&s.qname.local);
            if let Some(ref ns) = s.qname.namespace {
                builder = builder.namespace(ns);
            }
            builder = builder.is_abstract(s.is_abstract);

            // Flatten the xs:extension content model: base-chain fields
            // precede the struct's own fields (mirrors the Java codegen).
            let mut seen_bases: HashSet<crate::ir::QName> = HashSet::new();
            seen_bases.insert(s.qname.clone());
            let mut chain: Vec<&crate::ir::StructDef> = Vec::new();
            let mut cur = s.base_type.as_ref();
            while let Some(base_q) = cur {
                if !seen_bases.insert(base_q.clone()) {
                    break;
                }
                match ir.types.get(base_q) {
                    Some(TypeDef::Struct(base_s)) => {
                        chain.push(base_s);
                        cur = base_s.base_type.as_ref();
                    }
                    _ => break,
                }
            }
            for base_s in chain.into_iter().rev() {
                for f in &base_s.fields {
                    builder = builder.field(build_field(f, ir, visited));
                }
            }
            for f in &s.fields {
                builder = builder.field(build_field(f, ir, visited));
            }

            let mut schema = builder.build();
            if ir.has_ordered_content(s) {
                let item_field = s
                    .fields
                    .iter()
                    .find(|field| field.xml_name.is_empty() && matches!(&field.type_ref, TypeRef::Named(qname) if matches!(ir.types.get(qname), Some(TypeDef::Union(union)) if union.is_mixed_content())));
                if let Some(crate::ir::FieldDef {
                    type_ref: TypeRef::Named(item_type),
                    ..
                }) = item_field
                {
                    if let Some(TypeDef::Union(union)) = ir.types.get(item_type) {
                        let items_index = schema
                            .fields
                            .iter()
                            .position(|field| {
                                field.name == item_field.expect("mixed items field").name
                            })
                            .expect("mixed items field");
                        let branches = union
                            .branches
                            .iter()
                            .filter(|branch| branch.xml_name != "#text")
                            .map(|branch| MixedBranchSchema {
                                variant_name: branch.variant_name.clone(),
                                xml_name: branch.xml_name.as_bytes().to_vec(),
                                namespace: branch.namespace.clone(),
                                val_type: build_type(&branch.type_ref, ir, visited),
                            })
                            .collect();
                        Arc::get_mut(&mut schema).expect("new schema").mixed_content =
                            Some(MixedContentSchema {
                                items_index,
                                branches,
                            });
                    }
                }
            }

            // xsi:type dispatch registry: every transitive
            // derivation of this type, matched by namespace and local name.
            let derived: Vec<crate::ir::QName> = ir
                .types
                .iter()
                .filter_map(|(q, td)| match td {
                    TypeDef::Struct(d) if d.qname != s.qname && derives_from(ir, d, &s.qname) => {
                        Some(q.clone())
                    }
                    _ => None,
                })
                .collect();
            let mut variants = Vec::with_capacity(derived.len());
            for dq in derived {
                if visited.contains(&dq) {
                    continue;
                }
                visited.insert(dq.clone());
                if let Some(TypeDef::Struct(d)) = ir.types.get(&dq) {
                    variants.push(build_struct(d, ir, visited));
                }
                visited.remove(&dq);
            }
            if !variants.is_empty() {
                schema.set_variants(variants);
            }

            schema
        }

        let mut visited = HashSet::new();
        if let Some(qn) = qname_opt.as_ref() {
            visited.insert(qn.clone());
        }

        match type_ref {
            TypeRef::Named(ref qname) if ir.types.contains_key(qname) => {
                if let Some(TypeDef::Struct(s)) = ir.types.get(qname) {
                    Ok(build_struct(s, ir, &mut visited))
                } else {
                    let mut b = ModelSchema::builder(name);
                    b = b.field(FieldSchema::new(
                        "value",
                        b"value",
                        FieldKind::Text,
                        build_type(&type_ref, ir, &mut visited),
                    ));
                    Ok(b.build())
                }
            }
            _ => {
                let mut b = ModelSchema::builder(name);
                b = b.field(FieldSchema::new(
                    "value",
                    b"value",
                    FieldKind::Text,
                    build_type(&type_ref, ir, &mut visited),
                ));
                Ok(b.build())
            }
        }
    }
}

pub struct ModelSchemaBuilder {
    name: String,
    xml_name: Option<Vec<u8>>,
    namespace: Option<String>,
    fields: Vec<FieldSchema>,
    is_abstract: bool,
    strict_root: bool,
}

impl ModelSchemaBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            xml_name: None,
            namespace: None,
            fields: Vec::new(),
            is_abstract: false,
            strict_root: false,
        }
    }

    /// Mark the type as `abstract="true"`.
    pub fn is_abstract(mut self, is_abstract: bool) -> Self {
        self.is_abstract = is_abstract;
        self
    }

    pub fn xml_name(mut self, xml_name: &[u8]) -> Self {
        self.xml_name = Some(xml_name.to_vec());
        self
    }

    pub fn strict_root(mut self, strict_root: bool) -> Self {
        self.strict_root = strict_root;
        self
    }

    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn with_namespace(self, ns: impl Into<String>) -> Self {
        self.namespace(ns)
    }

    pub fn field(mut self, field: FieldSchema) -> Self {
        self.fields.push(field);
        self
    }

    pub fn build(self) -> Arc<ModelSchema> {
        let mut element_map = HashMap::new();
        let mut attribute_map = HashMap::new();
        let mut text_field = None;
        let mut any_attribute_field = None;
        let mut any_element_field = None;

        for (idx, field) in self.fields.iter().enumerate() {
            match field.kind {
                FieldKind::Attribute => {
                    attribute_map.insert(field.xml_name.clone(), idx);
                    // Match local name if prefix exists (e.g., "prefix:attr" -> "attr")
                    if let Some(pos) = field.xml_name.iter().position(|&b| b == b':') {
                        attribute_map.insert(field.xml_name[pos + 1..].to_vec(), idx);
                    }
                }
                FieldKind::Element => {
                    element_map.insert(field.xml_name.clone(), idx);
                    if let Some(pos) = field.xml_name.iter().position(|&b| b == b':') {
                        element_map.insert(field.xml_name[pos + 1..].to_vec(), idx);
                    }
                }
                FieldKind::Text => {
                    text_field = Some(idx);
                }
                FieldKind::AnyAttribute => {
                    any_attribute_field = Some(idx);
                }
                FieldKind::Any => {
                    any_element_field = Some(idx);
                }
            }
        }

        let xml_name = self
            .xml_name
            .unwrap_or_else(|| self.name.as_bytes().to_vec());

        Arc::new(ModelSchema {
            name: self.name,
            xml_name,
            namespace: self.namespace,
            fields: self.fields,
            element_map,
            attribute_map,
            text_field,
            any_attribute_field,
            any_element_field,
            mixed_content: None,
            is_abstract: self.is_abstract,
            strict_root: self.strict_root,
            variants: Arc::new(RwLock::new(Vec::new())),
        })
    }
}
