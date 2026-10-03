use polyxml::codegen::python::{
    to_enum_identifier, to_field_identifier, PythonBackend, PythonCodegen, PythonOptions,
};
use polyxml::ir::{
    Cardinality, EnumDef, EnumValue, FieldDef, FieldKind, OccursLimit, PrimitiveType, QName,
    RestrictionFacets, SchemaIR, SimpleTypeDef, StructDef, TypeDef, TypeRef, UnionBranch, UnionDef,
};
use polyxml::schema_parser::XsdParser;

#[test]
fn multiline_xsd_documentation_stays_inside_python_comments() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test">
      <xs:simpleType name="Code"><xs:annotation><xs:documentation>First line
        Second line</xs:documentation></xs:annotation><xs:restriction base="xs:string"/></xs:simpleType>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let code = PythonCodegen::new(PythonOptions::default()).generate_module(&ir);
    assert!(code.contains("# First line\n#         Second line"));
}

#[test]
fn test_identifier_sanitization() {
    assert_eq!(to_enum_identifier("pending"), "PENDING");
    assert_eq!(to_enum_identifier("in-progress"), "IN_PROGRESS");
    assert_eq!(to_enum_identifier("2024-Q1"), "VALUE_2024_Q1");
    assert_eq!(to_enum_identifier("10"), "VALUE_10");
    assert_eq!(to_enum_identifier(""), "EMPTY");
    assert_eq!(to_enum_identifier("class"), "CLASS");

    assert_eq!(to_field_identifier("class"), "class_");
    assert_eq!(to_field_identifier("from"), "from_");
    assert_eq!(to_field_identifier("type"), "type_");
    assert_eq!(to_field_identifier("100mDash"), "_100m_dash");
    assert_eq!(to_field_identifier("normalField"), "normal_field");
}

#[test]
fn test_python_dataclass_codegen() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/shop");

    // Enum
    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/shop"), "OrderStatus"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "pending".into(),
                value: "pending".into(),
                documentation: Some("Order is pending payment".into()),
            },
            EnumValue {
                name: "10-day-hold".into(),
                value: "10-day-hold".into(),
                documentation: None,
            },
            EnumValue {
                name: "class".into(),
                value: "class".into(),
                documentation: None,
            },
        ],
        documentation: Some("State of an order".into()),
    }));

    // Struct
    let fields = vec![
        FieldDef {
            name: "id".into(),
            xml_name: "id".into(),
            namespace: None,
            kind: FieldKind::Attribute,
            type_ref: TypeRef::Primitive(PrimitiveType::Int),
            cardinality: Cardinality::required_one(),
            nillable: false,
            default_value: None,
            fixed_value: None,
            documentation: Some("Unique identifier".into()),
            facets: None,
            is_cycle_cut: false,
        },
        FieldDef {
            name: "type".into(), // keyword
            xml_name: "type".into(),
            namespace: Some("https://example.com/shop".into()),
            kind: FieldKind::Element,
            type_ref: TypeRef::Primitive(PrimitiveType::String),
            cardinality: Cardinality::optional_one(),
            nillable: true,
            default_value: None,
            fixed_value: None,
            documentation: None,
            facets: None,
            is_cycle_cut: false,
        },
        FieldDef {
            name: "status".into(),
            xml_name: "status".into(),
            namespace: None,
            kind: FieldKind::Element,
            type_ref: TypeRef::Named(QName::new(Some("https://example.com/shop"), "OrderStatus")),
            cardinality: Cardinality::required_one(),
            nillable: false,
            default_value: Some("pending".into()),
            fixed_value: None,
            documentation: None,
            facets: None,
            is_cycle_cut: false,
        },
        FieldDef {
            name: "tags".into(),
            xml_name: "tag".into(),
            namespace: None,
            kind: FieldKind::Element,
            type_ref: TypeRef::Primitive(PrimitiveType::String),
            cardinality: Cardinality::unbounded(0),
            nillable: false,
            default_value: None,
            fixed_value: None,
            documentation: None,
            facets: None,
            is_cycle_cut: false,
        },
    ];

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/shop"), "Order"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields,
        documentation: Some("Represents a customer purchase order".into()),
    }));

    let codegen = PythonCodegen::new(PythonOptions {
        backend: PythonBackend::Dataclass,
        slots: true,
        kw_only: true,
        pep695_aliases: true,
        emit_meta: true,
        emit_root_aliases: true,
        emit_codecs: true,
        emit_json_metadata: true,
        custom_header: None,
    });

    let code = codegen.generate_module(&ir);

    assert!(code.contains("from __future__ import annotations"));
    assert!(code.contains("from dataclasses import dataclass, field"));
    assert!(code.contains("from enum import StrEnum"));
    assert!(code.contains("class OrderStatus(StrEnum):"));
    assert!(code.contains("PENDING = \"pending\""));
    assert!(code.contains("VALUE_10_DAY_HOLD = \"10-day-hold\""));
    assert!(code.contains("CLASS = \"class\""));

    assert!(code.contains("@dataclass(slots=True, kw_only=True)"));
    assert!(code.contains("class Order:"));
    assert!(code.contains("class Meta:"));
    assert!(code.contains("name = \"Order\""));
    assert!(code.contains("namespace = \"https://example.com/shop\""));

    // Check fields
    assert!(code.contains("id: int = field(metadata={\"type\": \"Attribute\", \"name\": \"id\", \"json_name\": \"id\"})"));
    assert!(code.contains("type_: str | None = field(default=None, metadata={\"type\": \"Element\", \"name\": \"type\", \"json_name\": \"type\", \"namespace\": \"https://example.com/shop\", \"nillable\": True})"));
    assert!(code.contains("status: OrderStatus = field(default=\"pending\", metadata={\"type\": \"Element\", \"name\": \"status\", \"json_name\": \"status\", \"default\": \"pending\"})"));
    assert!(code.contains("tags: list[str] = field(default_factory=list, metadata={\"type\": \"Element\", \"name\": \"tag\", \"json_name\": \"tag\"})"));
}

#[test]
fn test_python_pydantic_codegen_with_facets() {
    let mut ir = SchemaIR::new();

    // Simple type with facets (Age: 0 <= age <= 120)
    ir.add_type(TypeDef::Simple(Box::new(SimpleTypeDef {
        qname: QName::local("Age"),
        base_type: TypeRef::Primitive(PrimitiveType::Int),
        facets: RestrictionFacets {
            min_inclusive: Some("0".into()),
            max_inclusive: Some("120".into()),
            ..Default::default()
        },
        documentation: Some("Human age constrained between 0 and 120".into()),
    })));

    // Struct with facets and Pydantic validators
    let fields = vec![
        FieldDef {
            name: "username".into(),
            xml_name: "username".into(),
            namespace: None,
            kind: FieldKind::Element,
            type_ref: TypeRef::Primitive(PrimitiveType::String),
            cardinality: Cardinality::required_one(),
            nillable: false,
            default_value: None,
            fixed_value: None,
            documentation: None,
            facets: Some(RestrictionFacets {
                min_length: Some(3),
                max_length: Some(20),
                patterns: vec!["^[a-zA-Z0-9_]+$".into()],
                ..Default::default()
            }),
            is_cycle_cut: false,
        },
        FieldDef {
            name: "user_age".into(),
            xml_name: "age".into(),
            namespace: None,
            kind: FieldKind::Element,
            type_ref: TypeRef::Named(QName::local("Age")),
            cardinality: Cardinality::required_one(),
            nillable: false,
            default_value: None,
            fixed_value: None,
            documentation: None,
            facets: None,
            is_cycle_cut: false,
        },
    ];

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::local("User"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields,
        documentation: None,
    }));

    let codegen = PythonCodegen::new(PythonOptions {
        backend: PythonBackend::Pydantic,
        slots: true,
        kw_only: true,
        pep695_aliases: true,
        emit_meta: true,
        emit_root_aliases: true,
        emit_codecs: true,
        emit_json_metadata: true,
        custom_header: None,
    });

    let code = codegen.generate_module(&ir);

    assert!(code.contains("from pydantic import BaseModel, ConfigDict, Field"));
    assert!(code.contains("from typing import Annotated"));
    assert!(code.contains("type Age = Annotated[int, Field(ge=0, le=120)]"));
    assert!(code.contains("class User(BaseModel):"));
    assert!(code.contains("model_config = ConfigDict(defer_build=True, populate_by_name=True)"));
    assert!(code.contains("username: str = Field(..., json_schema_extra={\"type\": \"Element\", \"name\": \"username\", \"json_name\": \"username\"}, min_length=3, max_length=20, pattern=r\"\\A(?:^[a-zA-Z0-9_]+$)\\z\")"));
    assert!(code.contains(
        "user_age: Age = Field(..., alias=\"age\", serialization_alias=\"age\", json_schema_extra={\"type\": \"Element\", \"name\": \"age\", \"json_name\": \"age\"})"
    ));
}

#[test]
fn test_python_choice_union_codegen() {
    let mut ir = SchemaIR::new();

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::local("CardPayment"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef::new(
            "card_number",
            "cardNumber",
            FieldKind::Element,
            TypeRef::Primitive(PrimitiveType::String),
        )],
        documentation: None,
    }));

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::local("BankTransfer"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef::new(
            "iban",
            "iban",
            FieldKind::Element,
            TypeRef::Primitive(PrimitiveType::String),
        )],
        documentation: None,
    }));

    ir.add_type(TypeDef::Union(UnionDef {
        qname: QName::local("PaymentMethod"),
        branches: vec![
            UnionBranch {
                variant_name: "Card".into(),
                xml_name: "card".into(),
                namespace: None,
                type_ref: TypeRef::Named(QName::local("CardPayment")),
                documentation: None,
            },
            UnionBranch {
                variant_name: "Bank".into(),
                xml_name: "bank".into(),
                namespace: None,
                type_ref: TypeRef::Named(QName::local("BankTransfer")),
                documentation: None,
            },
        ],
        documentation: Some("Payment choice".into()),
    }));

    let codegen = PythonCodegen::new(PythonOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("type PaymentMethod = CardPayment | BankTransfer"));
}

#[test]
fn test_python_recursive_type_codegen() {
    let mut ir = SchemaIR::new();

    let fields = vec![
        FieldDef::new(
            "name",
            "name",
            FieldKind::Element,
            TypeRef::Primitive(PrimitiveType::String),
        ),
        FieldDef {
            name: "sub_departments".into(),
            xml_name: "subDepartment".into(),
            namespace: None,
            kind: FieldKind::Element,
            type_ref: TypeRef::List(Box::new(TypeRef::Named(QName::local("Department")))),
            cardinality: Cardinality {
                min_occurs: 0,
                max_occurs: OccursLimit::Unbounded,
            },
            nillable: false,
            default_value: None,
            fixed_value: None,
            documentation: None,
            facets: None,
            is_cycle_cut: false,
        },
        FieldDef {
            name: "parent".into(),
            xml_name: "parent".into(),
            namespace: None,
            kind: FieldKind::Element,
            type_ref: TypeRef::Boxed(Box::new(TypeRef::Named(QName::local("Department")))),
            cardinality: Cardinality::optional_one(),
            nillable: true,
            default_value: None,
            fixed_value: None,
            documentation: None,
            facets: None,
            is_cycle_cut: true,
        },
    ];

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::local("Department"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields,
        documentation: None,
    }));

    let codegen = PythonCodegen::new(PythonOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("class Department:"));
    assert!(code.contains("sub_departments: list[Department] = field(default_factory=list"));
    assert!(code.contains("parent: Department | None = field(default=None"));
}

#[test]
fn test_python_codecs_generation() {
    let mut ir = SchemaIR::new();
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::local("Item"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef::new(
            "name",
            "name",
            FieldKind::Element,
            TypeRef::Primitive(PrimitiveType::String),
        )],
        documentation: None,
    }));

    // With codecs enabled
    let codegen_enabled = PythonCodegen::new(PythonOptions {
        emit_codecs: true,
        ..Default::default()
    });
    let code_enabled = codegen_enabled.generate_module(&ir);
    assert!(code_enabled.contains("def from_xml(cls, data: bytes | str) -> Self:"));
    assert!(code_enabled.contains("def to_xml("));
    assert!(code_enabled.contains("return polyxml.deserialize(raw_bytes, cls)"));
    assert!(code_enabled.contains(
        "return polyxml.serialize(self, indent=indent, namespaces=namespaces, ns_map=ns_map)"
    ));
    assert!(code_enabled.contains("def from_json(cls, data: bytes | str) -> Self:"));
    assert!(code_enabled.contains("def to_json("));
    assert!(code_enabled.contains("return polyxml.deserialize_json(data, cls)"));
    assert!(code_enabled
        .contains("return polyxml.serialize_json(self, indent=indent, by_alias=by_alias)"));

    // With codecs disabled
    let codegen_disabled = PythonCodegen::new(PythonOptions {
        emit_codecs: false,
        ..Default::default()
    });
    let code_disabled = codegen_disabled.generate_module(&ir);
    assert!(!code_disabled.contains("def from_xml("));
    assert!(!code_disabled.contains("def to_xml("));
    assert!(!code_disabled.contains("def from_json("));
    assert!(!code_disabled.contains("def to_json("));
}

#[test]
fn test_python_generated_tag_and_custom_header() {
    let mut ir = SchemaIR::new();
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::local("Item"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef::new(
            "name",
            "name",
            FieldKind::Element,
            TypeRef::Primitive(PrimitiveType::String),
        )],
        documentation: None,
    }));

    // Without custom header: emits default @generated tag
    let default_codegen = PythonCodegen::new(PythonOptions::default());
    let default_code = default_codegen.generate_module(&ir);
    assert!(default_code
        .contains("# @generated by PolyXML Compiler (https://github.com/polyxml/PolyXML)"));
    assert!(default_code.contains("from __future__ import annotations"));

    // With custom header: prepends custom header
    let custom_codegen = PythonCodegen::new(PythonOptions {
        custom_header: Some("# ruff: noqa\n# mypy: ignore-errors".into()),
        ..Default::default()
    });
    let custom_code = custom_codegen.generate_module(&ir);
    assert!(custom_code
        .starts_with("# ruff: noqa\n# mypy: ignore-errors\n\n# @generated by PolyXML Compiler"));
}

#[test]
fn test_python_abstract_meta_emission() {
    // Abstract complex types mark Meta.abstract so the runtime
    // can raise a clear error for xsi:type values with no derivations.
    let mut ir = SchemaIR::new().with_target_namespace("urn:veh");
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:veh"), "Vehicle"),
        base_type: None,
        is_abstract: true,
        is_mixed: false,
        fields: vec![FieldDef::new(
            "id",
            "id",
            FieldKind::Element,
            TypeRef::Primitive(PrimitiveType::String),
        )],
        documentation: None,
    }));
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("urn:veh"), "Car"),
        base_type: Some(QName::new(Some("urn:veh"), "Vehicle")),
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef::new(
            "doors",
            "doors",
            FieldKind::Element,
            TypeRef::Primitive(PrimitiveType::Int),
        )],
        documentation: None,
    }));

    let code = PythonCodegen::new(PythonOptions::default()).generate_module(&ir);

    assert!(
        code.contains("class Vehicle:"),
        "abstract base must still emit:\n{code}"
    );
    assert_eq!(
        code.matches("abstract = True").count(),
        1,
        "only the abstract type may set Meta.abstract:\n{code}"
    );
    assert!(
        code.contains("class Car(Vehicle):"),
        "concrete derivation must inherit the base:\n{code}"
    );
}

#[test]
fn test_python_aot_codegen() {
    assert_eq!(
        PythonBackend::from_str_loose("aot"),
        Some(PythonBackend::Aot)
    );
    assert_eq!(
        PythonBackend::from_str_loose("native"),
        Some(PythonBackend::Aot)
    );
    assert_eq!(
        PythonBackend::from_str_loose("pyo3"),
        Some(PythonBackend::Aot)
    );

    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/aero");
    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/aero"), "FlightStatus"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "Scheduled".into(),
                value: "Scheduled".into(),
                documentation: None,
            },
            EnumValue {
                name: "Active".into(),
                value: "Active".into(),
                documentation: None,
            },
        ],
        documentation: None,
    }));

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/aero"), "FlightPlan"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "flightId".into(),
                xml_name: "flightId".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "altitude".into(),
                xml_name: "altitude".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Primitive(PrimitiveType::Int),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "status".into(),
                xml_name: "status".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(
                    Some("https://example.com/aero"),
                    "FlightStatus",
                )),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: Some("Flight plan information".into()),
    }));

    use polyxml::codegen::python::{PythonAotCodegen, PythonAotOptions};
    let aot = PythonAotCodegen::new(PythonAotOptions {
        module_name: "aero_models".to_string(),
        custom_header: None,
    });
    let generated = aot.generate_crate(&ir);

    // Verify Cargo.toml
    assert!(generated.cargo_toml.contains("name = \"aero_models\""));
    assert!(generated.cargo_toml.contains("crate-type = [\"cdylib\"]"));
    assert!(generated.cargo_toml.contains("pyo3 = { version = \"0.29\""));

    // Verify pyproject.toml
    assert!(generated
        .pyproject_toml
        .contains("build-backend = \"maturin\""));
    assert!(generated
        .pyproject_toml
        .contains("module-name = \"aero_models\""));

    // Verify lib.rs
    assert!(generated.lib_rs.contains("use pyo3::prelude::*;"));
    assert!(generated
        .lib_rs
        .contains("#[pyclass(eq, eq_int, from_py_object)]"));
    assert!(generated.lib_rs.contains("pub enum FlightStatus {"));
    assert!(generated
        .lib_rs
        .contains("#[pyclass(get_all, set_all, from_py_object)]"));
    assert!(generated.lib_rs.contains("pub struct FlightPlan {"));
    assert!(generated.lib_rs.contains("#[pyo3(name = \"to_xml\")]"));
    assert!(generated
        .lib_rs
        .contains("pub fn py_to_xml(&self) -> pyo3::PyResult<String>"));
    assert!(generated.lib_rs.contains("#[pyo3(name = \"from_xml\")]"));
    assert!(generated
        .lib_rs
        .contains("pub fn py_from_xml(xml: &str) -> pyo3::PyResult<Self>"));
    assert!(generated.lib_rs.contains("#[pyo3(name = \"to_json\")]"));
    assert!(generated
        .lib_rs
        .contains("pub fn py_to_json(&self) -> pyo3::PyResult<String>"));
    assert!(generated.lib_rs.contains("#[pyo3(name = \"from_json\")]"));
    assert!(generated
        .lib_rs
        .contains("pub fn py_from_json(json_str: &str) -> pyo3::PyResult<Self>"));
    assert!(generated.lib_rs.contains(
        "fn aero_models(m: &pyo3::Bound<'_, pyo3::types::PyModule>) -> pyo3::PyResult<()>"
    ));
    assert!(generated.lib_rs.contains("m.add_class::<FlightPlan>()?;"));
    assert!(generated.lib_rs.contains("m.add_class::<FlightStatus>()?;"));

    // Verify pyi stub
    assert!(generated
        .pyi_stub
        .contains("class FlightStatus(enum.IntEnum):"));
    assert!(generated.pyi_stub.contains("class FlightPlan:"));
    assert!(generated.pyi_stub.contains("flight_id: str"));
    assert!(generated.pyi_stub.contains("altitude: int"));
    assert!(generated.pyi_stub.contains("def to_xml(self) -> str: ..."));
    assert!(generated
        .pyi_stub
        .contains("def from_xml(xml: str) -> FlightPlan: ..."));
}

#[test]
fn test_python_derived_meta_inheritance() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:complexType name="BaseType"><xs:sequence>
        <xs:element name="BaseField" type="xs:string"/>
      </xs:sequence></xs:complexType>
      <xs:complexType name="DerivedType"><xs:complexContent><xs:extension base="BaseType">
        <xs:sequence><xs:element name="ExtraField" type="xs:string"/></xs:sequence>
      </xs:extension></xs:complexContent></xs:complexType>
      <xs:element name="Root" type="DerivedType"/>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");
    let codegen = PythonCodegen::new(PythonOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("class BaseType:"));
    assert!(code.contains("    class Meta:"));
    assert!(code.contains("class DerivedType(BaseType):"));
    assert!(code.contains("    class Meta(BaseType.Meta):"));
}

#[test]
fn test_python_simplecontent_default_factory() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:complexType name="BooleanWithAttr">
        <xs:simpleContent>
          <xs:extension base="xs:boolean">
            <xs:attribute name="source" type="xs:string"/>
          </xs:extension>
        </xs:simpleContent>
      </xs:complexType>
      <xs:element name="Root">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="flag" type="BooleanWithAttr" default="true"/>
          </xs:sequence>
        </xs:complexType>
      </xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new().parse_str(xsd).expect("parse failed");
    let codegen = PythonCodegen::new(PythonOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains(
        "flag: BooleanWithAttr = field(default_factory=lambda: BooleanWithAttr(value=True)"
    ));
}

#[test]
fn test_python_any_attribute_codegen() {
    use polyxml::schema_parser::XsdParser;

    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:element name="Extensible">
        <xs:complexType>
          <xs:sequence>
            <xs:element name="name" type="xs:string"/>
          </xs:sequence>
          <xs:anyAttribute processContents="lax"/>
        </xs:complexType>
      </xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new()
        .parse_str(xsd)
        .expect("schema parse failed");
    let codegen = PythonCodegen::new(PythonOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("any_attribute: dict[str, str] = field(default_factory=dict, metadata={\"type\": \"Attributes\", \"name\": \"*\", \"json_name\": \"*\"})"));
}

#[test]
fn test_python_any_element_codegen() {
    let xsd = r###"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:audit:content" targetNamespace="urn:audit:content" elementFormDefault="qualified">
      <xs:element name="Body">
        <xs:complexType>
          <xs:choice>
            <xs:element name="Uri" type="xs:string"/>
            <xs:any namespace="##other" processContents="lax"/>
          </xs:choice>
          <xs:attribute name="content" type="xs:string"/>
        </xs:complexType>
      </xs:element>
    </xs:schema>"###;

    let ir = XsdParser::new()
        .parse_str(xsd)
        .expect("schema parse failed");
    let codegen = PythonCodegen::new(PythonOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(
        code.contains("\"type\": \"Wildcard\""),
        "Generated Python model must mark xs:any field with Wildcard metadata"
    );
    assert!(
        code.contains("any: object | None = field(default=None"),
        "Generated Python model must define xs:any field as optional object"
    );
}

#[test]
fn test_python_sequence_nested_inside_choice_codegen() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:element name="Root"><xs:complexType><xs:choice>
        <xs:sequence>
          <xs:element name="First" type="xs:string"/>
          <xs:element name="Second" type="xs:string"/>
        </xs:sequence>
        <xs:element name="Alternative" type="xs:string"/>
      </xs:choice></xs:complexType></xs:element>
    </xs:schema>"#;

    let ir = XsdParser::new()
        .parse_str(xsd)
        .expect("schema parse failed");
    let codegen = PythonCodegen::new(PythonOptions::default());
    let code = codegen.generate_module(&ir);

    assert!(code.contains("class RootType:"));
    assert!(code.contains("alternative: str | None = field("));
    assert!(code.contains("class RootTypeSequence:"));
    assert!(code.contains("first: str | None = field("));
    assert!(code.contains("second: str | None = field("));
}

#[test]
fn quoted_docstrings_preserve_text_and_remain_valid_python() {
    let ir = XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:simpleType name="Direction"><xs:annotation><xs:documentation>Default is "Outbound"</xs:documentation></xs:annotation><xs:restriction base="xs:string"><xs:enumeration value="Outbound"/></xs:restriction></xs:simpleType><xs:complexType name="Record"><xs:annotation><xs:documentation>Literal \path, "quotes" and """triples"""</xs:documentation></xs:annotation><xs:sequence><xs:element name="direction" type="Direction"/></xs:sequence></xs:complexType></xs:schema>"#).unwrap();
    for backend in [PythonBackend::Dataclass, PythonBackend::Pydantic] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("models.py");
        std::fs::write(
            &path,
            PythonCodegen::new(PythonOptions {
                backend,
                ..Default::default()
            })
            .generate_module(&ir),
        )
        .unwrap();
        let expected = serde_json::to_string(&vec![
            "Default is \"Outbound\"",
            r#"Literal \path, "quotes" and """triples""""#,
        ])
        .unwrap();
        let output = std::process::Command::new("python3").arg("-c").arg("import ast,json,sys; tree=ast.parse(open(sys.argv[1]).read()); docs=[ast.get_docstring(n) for n in tree.body if isinstance(n,ast.ClassDef) and ast.get_docstring(n) is not None]; assert docs==json.loads(sys.argv[2]),docs").arg(path).arg(expected).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
