pub mod tarjan;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
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
    pub substitution_groups: HashMap<QName, Vec<QName>>,
    /// Type owners outside the module currently being emitted. Definitions
    /// remain in `types` so generators can resolve their field semantics.
    #[serde(default)]
    pub external_types: BTreeMap<QName, String>,
}

impl SchemaIR {
    pub fn is_external_type(&self, qname: &QName) -> bool {
        self.external_types.contains_key(qname)
    }

    pub fn emitted_types(&self) -> impl Iterator<Item = &TypeDef> {
        self.types
            .values()
            .filter(|ty| !self.is_external_type(ty.qname()))
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
