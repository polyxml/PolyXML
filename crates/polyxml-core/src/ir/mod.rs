pub mod chunker;
pub mod particle;
pub use particle::Particle;
pub mod tarjan;

pub use chunker::{partition_topological_chunks, ChunkPlan, TypeChunk};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

/// Fully qualified XML Name (Namespace URI + Local Name).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QName {
    pub namespace: Option<String>,
    pub local: String,
}

impl QName {
    pub fn new(namespace: Option<impl Into<String>>, local: impl Into<String>) -> Self {
        Self {
            namespace: namespace.map(Into::into),
            local: local.into(),
        }
    }

    pub fn local(local: impl Into<String>) -> Self {
        Self {
            namespace: None,
            local: local.into(),
        }
    }

    pub fn parse(s: &str, default_ns: Option<&str>) -> Self {
        if let Some((prefix, local)) = s.split_once(':') {
            Self {
                namespace: Some(prefix.to_string()),
                local: local.to_string(),
            }
        } else {
            Self {
                namespace: default_ns.map(|ns| ns.to_string()),
                local: s.to_string(),
            }
        }
    }
}

impl fmt::Display for QName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref ns) = self.namespace {
            write!(f, "{{{}}}{}", ns, self.local)
        } else {
            write!(f, "{}", self.local)
        }
    }
}

/// Standard W3C XML Schema Built-in Primitive Types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PrimitiveType {
    String,
    Boolean,
    Decimal,
    Float,
    Double,
    Integer,
    NegativeInteger,
    NonNegativeInteger,
    PositiveInteger,
    NonPositiveInteger,
    Long,
    Int,
    Short,
    Byte,
    UnsignedLong,
    UnsignedInt,
    UnsignedShort,
    UnsignedByte,
    Duration,
    DateTime,
    Time,
    Date,
    GYearMonth,
    GYear,
    GMonthDay,
    GDay,
    GMonth,
    HexBinary,
    Base64Binary,
    AnyUri,
    QName,
    NormalizedString,
    Token,
    Language,
    NMTOKEN,
    NMTOKENS,
    Name,
    NCName,
    Id,
    IdRef,
    IdRefs,
    Entity,
    Entities,
    AnyType,
    AnySimpleType,
}

impl PrimitiveType {
    pub fn from_xsd_name(name: &str) -> Option<Self> {
        let local = name
            .strip_prefix("xs:")
            .or_else(|| name.strip_prefix("xsd:"))
            .unwrap_or(name);
        match local {
            "string" => Some(Self::String),
            "boolean" => Some(Self::Boolean),
            "decimal" => Some(Self::Decimal),
            "float" => Some(Self::Float),
            "double" => Some(Self::Double),
            "integer" => Some(Self::Integer),
            "negativeInteger" => Some(Self::NegativeInteger),
            "nonNegativeInteger" => Some(Self::NonNegativeInteger),
            "positiveInteger" => Some(Self::PositiveInteger),
            "nonPositiveInteger" => Some(Self::NonPositiveInteger),
            "long" => Some(Self::Long),
            "int" => Some(Self::Int),
            "short" => Some(Self::Short),
            "byte" => Some(Self::Byte),
            "unsignedLong" => Some(Self::UnsignedLong),
            "unsignedInt" => Some(Self::UnsignedInt),
            "unsignedShort" => Some(Self::UnsignedShort),
            "unsignedByte" => Some(Self::UnsignedByte),
            "duration" => Some(Self::Duration),
            "dateTime" => Some(Self::DateTime),
            "time" => Some(Self::Time),
            "date" => Some(Self::Date),
            "gYearMonth" => Some(Self::GYearMonth),
            "gYear" => Some(Self::GYear),
            "gMonthDay" => Some(Self::GMonthDay),
            "gDay" => Some(Self::GDay),
            "gMonth" => Some(Self::GMonth),
            "hexBinary" => Some(Self::HexBinary),
            "base64Binary" => Some(Self::Base64Binary),
            "anyURI" => Some(Self::AnyUri),
            "QName" => Some(Self::QName),
            "normalizedString" => Some(Self::NormalizedString),
            "token" => Some(Self::Token),
            "language" => Some(Self::Language),
            "NMTOKEN" => Some(Self::NMTOKEN),
            "NMTOKENS" => Some(Self::NMTOKENS),
            "Name" => Some(Self::Name),
            "NCName" => Some(Self::NCName),
            "ID" => Some(Self::Id),
            "IDREF" => Some(Self::IdRef),
            "IDREFS" => Some(Self::IdRefs),
            "ENTITY" => Some(Self::Entity),
            "ENTITIES" => Some(Self::Entities),
            "anyType" => Some(Self::AnyType),
            "anySimpleType" => Some(Self::AnySimpleType),
            _ => None,
        }
    }
}

/// A reference to a type in the SchemaIR.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeRef {
    Primitive(PrimitiveType),
    Named(QName),
    Boxed(Box<TypeRef>),
    List(Box<TypeRef>),
}

impl TypeRef {
    pub fn string() -> Self {
        Self::Primitive(PrimitiveType::String)
    }

    pub fn named(qname: QName) -> Self {
        Self::Named(qname)
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    pub fn is_boxed(&self) -> bool {
        matches!(self, Self::Boxed(_))
    }
}

/// Repetition limit for an element or compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OccursLimit {
    Count(usize),
    Unbounded,
}

/// Cardinality definition (minOccurs .. maxOccurs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cardinality {
    pub min_occurs: usize,
    pub max_occurs: OccursLimit,
}

impl Cardinality {
    pub fn required_one() -> Self {
        Self {
            min_occurs: 1,
            max_occurs: OccursLimit::Count(1),
        }
    }

    pub fn optional_one() -> Self {
        Self {
            min_occurs: 0,
            max_occurs: OccursLimit::Count(1),
        }
    }

    pub fn unbounded(min_occurs: usize) -> Self {
        Self {
            min_occurs,
            max_occurs: OccursLimit::Unbounded,
        }
    }

    pub fn is_optional(&self) -> bool {
        self.min_occurs == 0 && matches!(self.max_occurs, OccursLimit::Count(1))
    }

    pub fn is_list(&self) -> bool {
        match self.max_occurs {
            OccursLimit::Unbounded => true,
            OccursLimit::Count(n) => n > 1,
        }
    }
}

impl Default for Cardinality {
    fn default() -> Self {
        Self::required_one()
    }
}

/// W3C XML Schema Restriction Facets for Simple Types.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RestrictionFacets {
    pub min_inclusive: Option<String>,
    pub max_inclusive: Option<String>,
    pub min_exclusive: Option<String>,
    pub max_exclusive: Option<String>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub length: Option<usize>,
    /// AND-combined derivation steps. Each entry is one regex, with alternatives
    /// from the same restriction normalized to `(first)|(second)` (OR).
    /// A singleton restriction retains its original regex verbatim.
    pub patterns: Vec<String>,
    pub enumerations: Vec<String>,
    pub white_space: Option<String>,
    pub total_digits: Option<usize>,
    pub fraction_digits: Option<usize>,
}

impl RestrictionFacets {
    pub fn is_empty(&self) -> bool {
        self.min_inclusive.is_none()
            && self.max_inclusive.is_none()
            && self.min_exclusive.is_none()
            && self.max_exclusive.is_none()
            && self.min_length.is_none()
            && self.max_length.is_none()
            && self.length.is_none()
            && self.patterns.is_empty()
            && self.enumerations.is_empty()
            && self.white_space.is_none()
            && self.total_digits.is_none()
            && self.fraction_digits.is_none()
    }
}

/// Classification of a field within a compound structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldKind {
    Element,
    Attribute,
    Text,
    Any,
    AnyAttribute,
}

/// A normalized field definition in a struct or choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub xml_name: String,
    pub namespace: Option<String>,
    pub kind: FieldKind,
    pub type_ref: TypeRef,
    pub cardinality: Cardinality,
    pub nillable: bool,
    pub default_value: Option<String>,
    pub fixed_value: Option<String>,
    pub documentation: Option<String>,
    pub facets: Option<RestrictionFacets>,
    pub is_cycle_cut: bool,
}

impl FieldDef {
    pub fn new(
        name: impl Into<String>,
        xml_name: impl Into<String>,
        kind: FieldKind,
        type_ref: TypeRef,
    ) -> Self {
        Self {
            name: name.into(),
            xml_name: xml_name.into(),
            namespace: None,
            kind,
            type_ref,
            cardinality: Cardinality::required_one(),
            nillable: false,
            default_value: None,
            fixed_value: None,
            documentation: None,
            facets: None,
            is_cycle_cut: false,
        }
    }
}

/// A value-backed enumeration variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumValue {
    pub name: String,
    pub value: String,
    pub documentation: Option<String>,
}

/// A strongly typed enumeration definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumDef {
    pub qname: QName,
    pub base_type: TypeRef,
    pub variants: Vec<EnumValue>,
    pub documentation: Option<String>,
}

/// A single variant/branch within an `xs:choice` union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionBranch {
    pub variant_name: String,
    pub xml_name: String,
    pub namespace: Option<String>,
    pub type_ref: TypeRef,
    pub documentation: Option<String>,
}

/// A strongly typed discriminated union / sum type representing `xs:choice` or `xs:union`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionDef {
    pub qname: QName,
    pub branches: Vec<UnionBranch>,
    pub documentation: Option<String>,
}

impl UnionDef {
    /// Lexical `xs:union` members share one element's text, unlike `xs:choice`
    /// branches which each carry a distinct child element name.
    pub fn is_lexical(&self) -> bool {
        !self.branches.is_empty()
            && self
                .branches
                .iter()
                .all(|branch| branch.xml_name.is_empty())
    }

    /// The `#text` branch identifies an ordered mixed-content item stream.
    pub fn is_mixed_content(&self) -> bool {
        self.branches
            .iter()
            .any(|branch| branch.xml_name == "#text")
    }
}

/// A simple type definition with restriction facets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimpleTypeDef {
    pub qname: QName,
    pub base_type: TypeRef,
    pub facets: RestrictionFacets,
    pub documentation: Option<String>,
}

/// A compound record (struct) definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDef {
    pub qname: QName,
    pub base_type: Option<QName>,
    pub is_abstract: bool,
    #[serde(default)]
    pub is_mixed: bool,
    pub fields: Vec<FieldDef>,
    pub documentation: Option<String>,
}

/// Canonical type definition in PolyXML-IR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeDef {
    Struct(StructDef),
    Enum(EnumDef),
    Union(UnionDef),
    Simple(Box<SimpleTypeDef>),
}

impl TypeDef {
    pub fn qname(&self) -> &QName {
        match self {
            Self::Struct(s) => &s.qname,
            Self::Enum(e) => &e.qname,
            Self::Union(u) => &u.qname,
            Self::Simple(st) => &st.qname,
        }
    }

    pub fn documentation(&self) -> Option<&str> {
        match self {
            Self::Struct(s) => s.documentation.as_deref(),
            Self::Enum(e) => e.documentation.as_deref(),
            Self::Union(u) => u.documentation.as_deref(),
            Self::Simple(st) => st.documentation.as_deref(),
        }
    }
}

/// A top-level element declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementDef {
    pub qname: QName,
    pub type_ref: TypeRef,
    pub substitution_group: Option<QName>,
    pub nillable: bool,
    pub documentation: Option<String>,
}

/// A declared XML namespace mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespaceDeclaration {
    pub prefix: String,
    pub uri: String,
    pub schema_location: Option<String>,
}

/// The complete canonical intermediate representation of one or more compiled schemas.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SchemaIR {
    pub target_namespace: Option<String>,
    pub namespaces: Vec<NamespaceDeclaration>,
    pub types: BTreeMap<QName, TypeDef>,
    pub elements: BTreeMap<QName, ElementDef>,
    /// Declared types of global attributes, used to resolve `<xs:attribute ref>`.
    #[serde(default)]
    pub attributes: BTreeMap<QName, TypeRef>,
    pub substitution_groups: HashMap<QName, Vec<QName>>,
    /// Element-only content which needs an ordered item stream to avoid losing
    /// repeated particle or substitution-member order during serialization.
    #[serde(default)]
    pub ordered_types: std::collections::BTreeSet<QName>,
    #[serde(default)]
    pub content_models: BTreeMap<QName, Particle>,
    /// Type owners outside the module currently being emitted. Definitions
    /// remain in `types` so generators can resolve their field semantics.
    #[serde(default)]
    pub external_types: BTreeMap<QName, String>,
}

impl SchemaIR {
    pub fn has_ordered_content(&self, structure: &StructDef) -> bool {
        structure.is_mixed || self.ordered_types.contains(&structure.qname)
    }
    /// Keep the selected global elements and all types that can occur beneath them.
    /// The schema is parsed in full before this operation, so references can be
    /// followed across imports and includes.
    pub fn select_root_elements(&self, roots: &[String]) -> Result<Self, String> {
        if roots.is_empty() {
            return Ok(self.clone());
        }

        let mut selected_elements = BTreeSet::new();
        for name in roots {
            let matches: Vec<_> = if let Some(qualified) = name.strip_prefix('{') {
                let (namespace, local) = qualified.split_once('}').ok_or_else(|| {
                    format!("Invalid root element '{name}': expected {{namespace}}local-name")
                })?;
                if local.is_empty() {
                    return Err(format!("Invalid root element '{name}': missing local name"));
                }
                self.elements
                    .keys()
                    .filter(|q| {
                        q.namespace.as_deref().unwrap_or("") == namespace && q.local == local
                    })
                    .cloned()
                    .collect()
            } else {
                self.elements
                    .keys()
                    .filter(|q| q.local == *name)
                    .cloned()
                    .collect()
            };
            match matches.as_slice() {
                [] => return Err(format!("Root element '{name}' was not found in the schema")),
                [qname] => {
                    selected_elements.insert(qname.clone());
                }
                _ => {
                    return Err(format!(
                        "Root element '{name}' is ambiguous; use {{namespace}}local-name"
                    ))
                }
            }
        }

        let mut descendants: HashMap<QName, Vec<QName>> = HashMap::new();
        for (qname, def) in &self.types {
            let base = match def {
                TypeDef::Struct(s) => s.base_type.as_ref(),
                TypeDef::Simple(s) => named_ref(&s.base_type),
                TypeDef::Enum(e) => named_ref(&e.base_type),
                TypeDef::Union(_) => None,
            };
            if let Some(base) = base {
                descendants
                    .entry(base.clone())
                    .or_default()
                    .push(qname.clone());
            }
        }

        let mut selected_types = BTreeSet::new();
        let mut pending_elements: Vec<_> = selected_elements.iter().cloned().collect();
        let mut pending_types: Vec<(QName, bool)> = Vec::new();
        let mut expanded_polymorphic = BTreeSet::new();
        while !pending_elements.is_empty() || !pending_types.is_empty() {
            while let Some(qname) = pending_elements.pop() {
                if let Some(element) = self.elements.get(&qname) {
                    push_named_ref(&element.type_ref, true, &mut pending_types);
                    if let Some(head) = &element.substitution_group {
                        if selected_elements.insert(head.clone()) {
                            pending_elements.push(head.clone());
                        }
                    }
                }
                if let Some(members) = self.substitution_groups.get(&qname) {
                    for member in members {
                        if selected_elements.insert(member.clone()) {
                            pending_elements.push(member.clone());
                        }
                    }
                }
            }
            if let Some((qname, polymorphic)) = pending_types.pop() {
                if polymorphic && expanded_polymorphic.insert(qname.clone()) {
                    if let Some(children) = descendants.get(&qname) {
                        pending_types.extend(children.iter().cloned().map(|child| (child, true)));
                    }
                }
                if !selected_types.insert(qname.clone()) {
                    continue;
                }
                if let Some(def) = self.types.get(&qname) {
                    match def {
                        TypeDef::Struct(s) => {
                            if let Some(base) = &s.base_type {
                                pending_types.push((base.clone(), false));
                            }
                            for field in &s.fields {
                                push_named_ref(&field.type_ref, true, &mut pending_types);
                                if field.kind == FieldKind::Element {
                                    let element = referenced_element_name(
                                        field.namespace.as_deref(),
                                        &field.xml_name,
                                    );
                                    if self.substitution_groups.contains_key(&element)
                                        && selected_elements.insert(element.clone())
                                    {
                                        pending_elements.push(element);
                                    }
                                }
                            }
                        }
                        TypeDef::Simple(s) => {
                            push_named_ref(&s.base_type, false, &mut pending_types)
                        }
                        TypeDef::Enum(e) => push_named_ref(&e.base_type, false, &mut pending_types),
                        TypeDef::Union(u) => {
                            for branch in &u.branches {
                                push_named_ref(&branch.type_ref, true, &mut pending_types);
                                let element = referenced_element_name(
                                    branch.namespace.as_deref(),
                                    &branch.xml_name,
                                );
                                if self.substitution_groups.contains_key(&element)
                                    && selected_elements.insert(element.clone())
                                {
                                    pending_elements.push(element);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut result = self.clone();
        result
            .types
            .retain(|qname, _| selected_types.contains(qname));
        result
            .elements
            .retain(|qname, _| selected_elements.contains(qname));
        result
            .external_types
            .retain(|qname, _| selected_types.contains(qname));
        result.substitution_groups.retain(|head, members| {
            if !selected_elements.contains(head) {
                return false;
            }
            members.retain(|member| selected_elements.contains(member));
            !members.is_empty()
        });
        Ok(result)
    }

    pub fn is_external_type(&self, qname: &QName) -> bool {
        if qname
            .namespace
            .as_deref()
            .is_some_and(|ns| ns.starts_with("urn:polyxml:builtins"))
        {
            return false;
        }
        self.external_types.contains_key(qname)
    }

    pub fn emitted_types(&self) -> impl Iterator<Item = &TypeDef> {
        self.types
            .values()
            .filter(|ty| !self.is_external_type(ty.qname()))
    }

    /// Partition local emitted types into topologically sorted SCC chunks.
    pub fn partition_topological_chunks(&self, budget: usize) -> ChunkPlan {
        chunker::partition_topological_chunks(self, budget)
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_target_namespace(mut self, target_ns: impl Into<String>) -> Self {
        self.target_namespace = Some(target_ns.into());
        self
    }

    pub fn add_type(&mut self, type_def: TypeDef) {
        self.types.insert(type_def.qname().clone(), type_def);
    }

    pub fn add_element(&mut self, element_def: ElementDef) {
        if let Some(ref head) = element_def.substitution_group {
            self.substitution_groups
                .entry(head.clone())
                .or_default()
                .push(element_def.qname.clone());
        }
        self.elements.insert(element_def.qname.clone(), element_def);
    }

    pub fn find_type(&self, qname: &QName) -> Option<&TypeDef> {
        self.types.get(qname)
    }

    pub fn find_element(&self, qname: &QName) -> Option<&ElementDef> {
        self.elements.get(qname)
    }

    /// Resolve cycles in the type graph using Tarjan's SCC algorithm,
    /// boxing minimal cut-point fields.
    pub fn resolve_cycles(&mut self) {
        tarjan::resolve_cycles(self);
    }
}

fn named_ref(type_ref: &TypeRef) -> Option<&QName> {
    match type_ref {
        TypeRef::Named(qname) => Some(qname),
        TypeRef::Boxed(inner) | TypeRef::List(inner) => named_ref(inner),
        TypeRef::Primitive(_) => None,
    }
}

fn push_named_ref(type_ref: &TypeRef, polymorphic: bool, pending: &mut Vec<(QName, bool)>) {
    if let Some(qname) = named_ref(type_ref) {
        pending.push((qname.clone(), polymorphic));
    }
}

fn referenced_element_name(namespace: Option<&str>, xml_name: &str) -> QName {
    let local = xml_name.rsplit(':').next().unwrap_or(xml_name);
    QName::new(namespace, local)
}

#[cfg(test)]
mod root_selection_tests {
    use super::*;

    fn structure(name: &str, base: Option<&str>, fields: Vec<FieldDef>) -> TypeDef {
        TypeDef::Struct(StructDef {
            qname: QName::new(Some("urn:test"), name),
            base_type: base.map(|name| QName::new(Some("urn:test"), name)),
            is_abstract: false,
            is_mixed: false,
            fields,
            documentation: None,
        })
    }

    fn element(name: &str, ty: &str) -> ElementDef {
        ElementDef {
            qname: QName::new(Some("urn:test"), name),
            type_ref: TypeRef::named(QName::new(Some("urn:test"), ty)),
            substitution_group: None,
            nillable: false,
            documentation: None,
        }
    }

    #[test]
    fn keeps_references_descendants_and_substitutions() {
        let mut ir = SchemaIR::new();
        let mut field = FieldDef::new(
            "item",
            "Item",
            FieldKind::Element,
            TypeRef::List(Box::new(TypeRef::named(QName::new(
                Some("urn:test"),
                "Choice",
            )))),
        );
        field.namespace = Some("urn:test".to_string());
        ir.add_type(structure("RootType", None, vec![field]));
        ir.add_type(structure("Base", None, vec![]));
        ir.add_type(structure("Derived", Some("Base"), vec![]));
        ir.add_type(TypeDef::Union(UnionDef {
            qname: QName::new(Some("urn:test"), "Choice"),
            branches: vec![UnionBranch {
                variant_name: "Base".into(),
                xml_name: "Base".into(),
                namespace: Some("urn:test".into()),
                type_ref: TypeRef::named(QName::new(Some("urn:test"), "Base")),
                documentation: None,
            }],
            documentation: None,
        }));
        ir.add_type(structure("Unrelated", None, vec![]));
        ir.add_element(element("Root", "RootType"));
        ir.add_element(element("UnrelatedRoot", "Unrelated"));
        let mut member = element("Member", "Derived");
        member.substitution_group = Some(QName::new(Some("urn:test"), "Root"));
        ir.add_element(member);

        let selected = ir.select_root_elements(&["Root".into()]).unwrap();
        assert_eq!(selected.elements.len(), 2);
        assert_eq!(selected.types.len(), 4);
        assert!(selected
            .types
            .contains_key(&QName::new(Some("urn:test"), "Derived")));
        assert!(!selected
            .types
            .contains_key(&QName::new(Some("urn:test"), "Unrelated")));
        let combined = ir
            .select_root_elements(&["Root".into(), "UnrelatedRoot".into()])
            .unwrap();
        assert_eq!(combined.elements.len(), 3);
        assert!(combined
            .types
            .contains_key(&QName::new(Some("urn:test"), "Unrelated")));
    }

    #[test]
    fn rejects_missing_and_ambiguous_roots() {
        let mut ir = SchemaIR::new();
        ir.add_element(element("Root", "RootType"));
        let mut second = element("Root", "RootType");
        second.qname.namespace = Some("urn:other".into());
        ir.add_element(second);
        assert!(ir
            .select_root_elements(&["Root".into()])
            .unwrap_err()
            .contains("ambiguous"));
        assert_eq!(
            ir.select_root_elements(&["{urn:test}Root".into()])
                .unwrap()
                .elements
                .len(),
            1
        );
        assert!(ir
            .select_root_elements(&["Missing".into()])
            .unwrap_err()
            .contains("not found"));
    }

    #[test]
    fn inherited_base_does_not_select_sibling_types() {
        let mut ir = SchemaIR::new();
        ir.add_type(structure("CommonBase", None, vec![]));
        ir.add_type(structure("SelectedType", Some("CommonBase"), vec![]));
        ir.add_type(structure("SiblingType", Some("CommonBase"), vec![]));
        ir.add_element(element("Selected", "SelectedType"));
        let selected = ir.select_root_elements(&["Selected".into()]).unwrap();
        assert!(selected
            .types
            .contains_key(&QName::new(Some("urn:test"), "CommonBase")));
        assert!(selected
            .types
            .contains_key(&QName::new(Some("urn:test"), "SelectedType")));
        assert!(!selected
            .types
            .contains_key(&QName::new(Some("urn:test"), "SiblingType")));
    }

    #[test]
    fn field_reference_keeps_substitution_group_members() {
        let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            xmlns:t="urn:test" targetNamespace="urn:test" elementFormDefault="qualified">
            <xs:element name="Head" type="xs:string" abstract="true"/>
            <xs:element name="Member" type="xs:string" substitutionGroup="t:Head"/>
            <xs:complexType name="ContainerType"><xs:sequence>
                <xs:element ref="t:Head"/>
            </xs:sequence></xs:complexType>
            <xs:element name="Container" type="t:ContainerType"/>
        </xs:schema>"#;
        let mut parser = crate::schema_parser::XsdParser::new();
        let ir = parser.parse_str(xsd).unwrap();
        let selected = ir.select_root_elements(&["Container".into()]).unwrap();
        assert!(selected
            .elements
            .contains_key(&QName::new(Some("urn:test"), "Head")));
        assert!(selected
            .elements
            .contains_key(&QName::new(Some("urn:test"), "Member")));
    }
}
