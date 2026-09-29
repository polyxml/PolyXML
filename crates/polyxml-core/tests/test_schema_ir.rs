use minijinja::context;
use polyxml::codegen::{create_template_engine, sanitize_keyword, LanguageContext};
use polyxml::ir::tarjan::TarjanCycleDetector;
use polyxml::ir::{
    Cardinality, FieldDef, FieldKind, OccursLimit, PrimitiveType, QName, SchemaIR, StructDef,
    TypeDef, TypeRef,
};
use polyxml::schema_parser::XsdParser;
use tempfile::tempdir;

#[test]
fn missing_included_schema_is_an_error() {
    let dir = tempdir().unwrap();
    for (name, include) in [
        ("empty.xsd", r#"<xs:include schemaLocation="missing.xsd"/>"#),
        (
            "start.xsd",
            r#"<xs:include schemaLocation="missing.xsd"></xs:include>"#,
        ),
    ] {
        let path = dir.path().join(name);
        std::fs::write(
            &path,
            format!(
                r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">{include}</xs:schema>"#
            ),
        )
        .unwrap();
        let error = XsdParser::new().parse_file(&path).unwrap_err();
        assert!(error.to_string().contains("missing.xsd"), "{error}");
    }
}

#[test]
fn consumed_child_subtrees_do_not_swallow_following_types() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test">
      <xs:complexType name="First"><xs:sequence>
        <xs:element name="nested"><xs:complexType><xs:sequence><xs:element name="value" type="xs:string"/></xs:sequence></xs:complexType></xs:element>
        <xs:group ref="missing"><xs:annotation><xs:documentation>ignored</xs:documentation></xs:annotation></xs:group>
      </xs:sequence></xs:complexType>
      <xs:complexType name="Second"><xs:sequence><xs:element name="other" type="xs:string"/></xs:sequence></xs:complexType>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let first = match &ir.types[&QName::new(Some("urn:test"), "First")] {
        TypeDef::Struct(value) => value,
        other => panic!("expected struct: {other:?}"),
    };
    assert_eq!(first.fields.len(), 1);
    assert!(ir
        .types
        .contains_key(&QName::new(Some("urn:test"), "Second")));
}

#[test]
fn consumed_particles_do_not_swallow_following_declarations() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test" xmlns:t="urn:test">
      <xs:group name="Shared"><xs:sequence>
        <xs:element name="inside"><xs:annotation><xs:documentation>text</xs:documentation></xs:annotation></xs:element>
      </xs:sequence></xs:group>
      <xs:complexType name="First"><xs:sequence>
        <xs:group ref="t:Shared"><xs:annotation/></xs:group>
        <xs:element name="child" type="xs:string"><xs:annotation/></xs:element>
      </xs:sequence></xs:complexType>
      <xs:complexType name="Second"><xs:sequence><xs:element name="value" type="xs:string"/></xs:sequence></xs:complexType>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let first = match &ir.types[&QName::new(Some("urn:test"), "First")] {
        TypeDef::Struct(value) => value,
        _ => panic!("expected struct"),
    };
    assert!(first.fields.iter().any(|field| field.name == "inside"));
    assert!(first.fields.iter().any(|field| field.name == "child"));
    assert!(ir
        .types
        .contains_key(&QName::new(Some("urn:test"), "Second")));
}

#[test]
fn global_attribute_reference_uses_declared_type() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test" xmlns:t="urn:test">
      <xs:complexType name="Object"><xs:attribute ref="t:id" use="required"/></xs:complexType>
      <xs:attribute name="id" type="xs:ID"><xs:annotation><xs:documentation>identifier</xs:documentation></xs:annotation></xs:attribute>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let object = match &ir.types[&QName::new(Some("urn:test"), "Object")] {
        TypeDef::Struct(value) => value,
        _ => panic!("expected struct"),
    };
    assert_eq!(
        object.fields[0].type_ref,
        TypeRef::Primitive(PrimitiveType::Id)
    );
}

#[test]
fn xml_prefix_is_available_without_explicit_declaration() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("xml.xsd"),
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="http://www.w3.org/XML/1998/namespace">
          <xs:attribute name="lang" type="xs:language"/>
        </xs:schema>"#,
    )
    .unwrap();
    let main = dir.path().join("dc.xsd");
    std::fs::write(
        &main,
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:dc">
          <xs:import namespace="http://www.w3.org/XML/1998/namespace" schemaLocation="xml.xsd"/>
          <xs:complexType name="SimpleLiteral"><xs:attribute ref="xml:lang"/></xs:complexType>
        </xs:schema>"#,
    )
    .unwrap();
    let ir = XsdParser::new().parse_file(&main).unwrap();
    let TypeDef::Struct(literal) = &ir.types[&QName::new(Some("urn:dc"), "SimpleLiteral")] else {
        panic!("expected SimpleLiteral struct");
    };
    assert_eq!(
        literal.fields[0].type_ref,
        TypeRef::Primitive(PrimitiveType::Language)
    );
}

#[test]
fn derived_complex_type_keeps_base_and_bounded_choice() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" targetNamespace="urn:test">
      <xs:complexType name="Base"><xs:attribute name="id" type="xs:string"/></xs:complexType>
      <xs:complexType name="Node"><xs:complexContent><xs:extension base="t:Base"><xs:choice>
        <xs:element name="left" type="xs:string"/><xs:element name="right" type="xs:int"/>
      </xs:choice></xs:extension></xs:complexContent></xs:complexType>
      <xs:complexType name="Child"><xs:complexContent><xs:extension base="t:Node">
        <xs:attribute name="flag" type="xs:boolean"/>
      </xs:extension></xs:complexContent></xs:complexType>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let TypeDef::Struct(node) = &ir.types[&QName::new(Some("urn:test"), "Node")] else {
        panic!("derived choice must remain a struct");
    };
    assert_eq!(node.base_type, Some(QName::new(Some("urn:test"), "Base")));
    assert_eq!(node.fields.len(), 1);
    assert_eq!(node.fields[0].name, "choice");
    let TypeRef::Named(choice_name) = &node.fields[0].type_ref else {
        panic!("choice field must refer to a union");
    };
    let TypeDef::Union(choice) = &ir.types[choice_name] else {
        panic!("choice field must refer to a union");
    };
    assert_eq!(choice.branches.len(), 2);
}

#[test]
fn test_parse_lexical_union_member_types_and_inline_members() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" targetNamespace="urn:test">
      <xs:simpleType name="DateOrCode"><xs:union memberTypes="xs:date xs:string">
        <xs:simpleType><xs:restriction base="xs:int"><xs:minInclusive value="1"/></xs:restriction></xs:simpleType>
      </xs:union></xs:simpleType>
      <xs:element name="value" type="t:DateOrCode"/>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let union = match &ir.types[&QName::new(Some("urn:test"), "DateOrCode")] {
        TypeDef::Union(union) => union,
        other => panic!("expected lexical union, got {other:?}"),
    };
    assert_eq!(union.branches.len(), 3);
    assert!(union
        .branches
        .iter()
        .all(|branch| branch.xml_name.is_empty()));
    assert_eq!(
        union.branches[0].type_ref,
        TypeRef::Primitive(PrimitiveType::Date)
    );
    assert_eq!(
        union.branches[1].type_ref,
        TypeRef::Primitive(PrimitiveType::String)
    );
    let TypeRef::Named(inline) = &union.branches[2].type_ref else {
        panic!("inline simple type was not registered")
    };
    assert!(matches!(ir.types.get(inline), Some(TypeDef::Simple(_))));
}

#[test]
fn test_parse_complex_type_and_facets() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:iso:std:iso:20022:tech:xsd:pain.001.001.09" elementFormDefault="qualified">
        <xs:simpleType name="Max35Text">
            <xs:restriction base="xs:string">
                <xs:minLength value="1"/>
                <xs:maxLength value="35"/>
                <xs:pattern value="[a-zA-Z0-9]+"/>
            </xs:restriction>
        </xs:simpleType>

        <xs:simpleType name="PaymentMethodCode">
            <xs:restriction base="xs:string">
                <xs:enumeration value="CHK"/>
                <xs:enumeration value="TRF"/>
                <xs:enumeration value="DD"/>
            </xs:restriction>
        </xs:simpleType>

        <xs:complexType name="PostalAddress">
            <xs:sequence>
                <xs:element name="StrtNm" type="xs:string" minOccurs="1" maxOccurs="1"/>
                <xs:element name="BldgNb" type="xs:string" minOccurs="0" maxOccurs="1"/>
                <xs:element name="PstCd" type="xs:string"/>
                <xs:element name="TwnNm" type="xs:string"/>
                <xs:element name="Ctry" type="xs:string" minOccurs="0"/>
            </xs:sequence>
            <xs:attribute name="id" type="xs:string" use="required"/>
            <xs:attribute name="type" type="xs:string" use="optional"/>
        </xs:complexType>

        <xs:element name="PstlAdr" type="PostalAddress"/>
    </xs:schema>"#;

    let mut parser = XsdParser::new();
    let ir = parser.parse_str(xsd).expect("Failed to parse XSD");

    assert_eq!(
        ir.target_namespace.as_deref(),
        Some("urn:iso:std:iso:20022:tech:xsd:pain.001.001.09")
    );

    // Verify SimpleType with facets
    let max35_qname = QName::new(
        Some("urn:iso:std:iso:20022:tech:xsd:pain.001.001.09"),
        "Max35Text",
    );
    let max35 = ir.find_type(&max35_qname).expect("Max35Text should exist");
    if let TypeDef::Simple(ref st) = max35 {
        assert_eq!(st.facets.min_length, Some(1));
        assert_eq!(st.facets.max_length, Some(35));
        assert_eq!(st.facets.patterns, vec!["[a-zA-Z0-9]+"]);
    } else {
        panic!("Expected SimpleTypeDef");
    }

    // Verify Enum
    let code_qname = QName::new(
        Some("urn:iso:std:iso:20022:tech:xsd:pain.001.001.09"),
        "PaymentMethodCode",
    );
    let code_type = ir.find_type(&code_qname).expect("PaymentMethodCode exists");
    if let TypeDef::Enum(ref ed) = code_type {
        assert_eq!(ed.variants.len(), 3);
        assert_eq!(ed.variants[0].value, "CHK");
        assert_eq!(ed.variants[1].value, "TRF");
        assert_eq!(ed.variants[2].value, "DD");
    } else {
        panic!("Expected EnumDef");
    }

    // Verify Struct
    let addr_qname = QName::new(
        Some("urn:iso:std:iso:20022:tech:xsd:pain.001.001.09"),
        "PostalAddress",
    );
    let addr_type = ir.find_type(&addr_qname).expect("PostalAddress exists");
    if let TypeDef::Struct(ref s) = addr_type {
        assert_eq!(s.fields.len(), 7); // 5 elements + 2 attributes
        let street = &s.fields[0];
        assert_eq!(street.name, "strt_nm");
        assert_eq!(street.xml_name, "StrtNm");
        assert_eq!(street.kind, FieldKind::Element);
        assert_eq!(street.cardinality.min_occurs, 1);

        let building = &s.fields[1];
        assert_eq!(building.name, "bldg_nb");
        assert!(building.cardinality.is_optional());

        let id_attr = s.fields.iter().find(|f| f.name == "id").unwrap();
        assert_eq!(id_attr.kind, FieldKind::Attribute);
        assert_eq!(id_attr.cardinality.min_occurs, 1);

        let type_attr = s.fields.iter().find(|f| f.name == "type").unwrap();
        assert_eq!(type_attr.kind, FieldKind::Attribute);
        assert!(type_attr.cardinality.is_optional());
    } else {
        panic!("Expected StructDef");
    }

    // Verify Element
    let elem_qname = QName::new(
        Some("urn:iso:std:iso:20022:tech:xsd:pain.001.001.09"),
        "PstlAdr",
    );
    let elem = ir.find_element(&elem_qname).expect("PstlAdr exists");
    assert_eq!(elem.type_ref, TypeRef::Named(addr_qname));
}

#[test]
fn test_parse_choice_as_union() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test">
        <xs:complexType name="CreditCard">
            <xs:sequence>
                <xs:element name="CardNumber" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>

        <xs:complexType name="DirectDebit">
            <xs:sequence>
                <xs:element name="Iban" type="xs:string"/>
            </xs:sequence>
        </xs:complexType>

        <xs:complexType name="PaymentInstrumentChoice">
            <xs:choice>
                <xs:element name="CreditCard" type="CreditCard"/>
                <xs:element name="DirectDebit" type="DirectDebit"/>
            </xs:choice>
        </xs:complexType>
    </xs:schema>"#;

    let mut parser = XsdParser::new();
    let ir = parser.parse_str(xsd).expect("Failed to parse XSD");

    let choice_qname = QName::new(Some("urn:test"), "PaymentInstrumentChoice");
    let choice_type = ir.find_type(&choice_qname).expect("Choice type exists");
    if let TypeDef::Union(ref u) = choice_type {
        assert_eq!(u.branches.len(), 2);
        assert_eq!(u.branches[0].xml_name, "CreditCard");
        assert_eq!(u.branches[1].xml_name, "DirectDebit");
    } else {
        panic!("Expected UnionDef for xs:choice");
    }
}

#[test]
fn test_substitution_groups() {
    let xsd = r#"<?xml version="1.0" encoding="UTF-8"?>
    <xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test">
        <xs:element name="BaseInstrument" type="xs:string"/>
        <xs:element name="Bond" type="xs:string" substitutionGroup="BaseInstrument"/>
        <xs:element name="Equity" type="xs:string" substitutionGroup="BaseInstrument"/>
    </xs:schema>"#;

    let mut parser = XsdParser::new();
    let ir = parser.parse_str(xsd).expect("Failed to parse XSD");

    let base_qname = QName::new(Some("urn:test"), "BaseInstrument");
    let subs = ir
        .substitution_groups
        .get(&base_qname)
        .expect("Substitution group exists");
    assert_eq!(subs.len(), 2);
    assert!(subs.contains(&QName::new(Some("urn:test"), "Bond")));
    assert!(subs.contains(&QName::new(Some("urn:test"), "Equity")));
}

#[test]
fn test_tarjan_self_recursive_cycle_detection_and_boxing() {
    let mut ir = SchemaIR::new();
    let node_qname = QName::local("TreeNode");

    // struct TreeNode { id: String, left: Option<TreeNode>, right: Option<TreeNode> }
    let struct_def = StructDef {
        qname: node_qname.clone(),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef::new(
                "id",
                "id",
                FieldKind::Element,
                TypeRef::Primitive(PrimitiveType::String),
            ),
            {
                let mut f = FieldDef::new(
                    "left",
                    "left",
                    FieldKind::Element,
                    TypeRef::Named(node_qname.clone()),
                );
                f.cardinality = Cardinality::optional_one();
                f
            },
            {
                let mut f = FieldDef::new(
                    "right",
                    "right",
                    FieldKind::Element,
                    TypeRef::Named(node_qname.clone()),
                );
                f.cardinality = Cardinality::optional_one();
                f
            },
        ],
        documentation: None,
    };
    ir.add_type(TypeDef::Struct(struct_def));

    // Verify Tarjan detects the cyclic SCC
    let mut detector = TarjanCycleDetector::new(&ir);
    let sccs = detector.find_cyclic_sccs();
    assert_eq!(sccs.len(), 1);
    assert_eq!(sccs[0], vec![node_qname.clone()]);

    // Resolve cycles
    ir.resolve_cycles();

    // Verify that at least one recursive field was cut and boxed
    if let Some(TypeDef::Struct(ref s)) = ir.find_type(&node_qname) {
        let left = &s.fields[1];
        let right = &s.fields[2];
        assert!(left.is_cycle_cut || right.is_cycle_cut);
        if left.is_cycle_cut {
            assert!(left.type_ref.is_boxed());
        }
        if right.is_cycle_cut {
            assert!(right.type_ref.is_boxed());
        }
    } else {
        panic!("Expected StructDef");
    }

    // Run Tarjan again to verify graph is now acyclic
    let mut detector_after = TarjanCycleDetector::new(&ir);
    assert!(detector_after.find_cyclic_sccs().is_empty());
}

#[test]
fn test_tarjan_mutual_recursive_cycle_detection_and_boxing() {
    let mut ir = SchemaIR::new();
    let qname_a = QName::local("Parent");
    let qname_b = QName::local("Child");

    // Parent has a Child (non-list, non-boxed)
    let parent = StructDef {
        qname: qname_a.clone(),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![FieldDef::new(
            "child",
            "child",
            FieldKind::Element,
            TypeRef::Named(qname_b.clone()),
        )],
        documentation: None,
    };

    // Child has an optional Parent back-reference
    let child = StructDef {
        qname: qname_b.clone(),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![{
            let mut f = FieldDef::new(
                "parent",
                "parent",
                FieldKind::Element,
                TypeRef::Named(qname_a.clone()),
            );
            f.cardinality = Cardinality::optional_one();
            f
        }],
        documentation: None,
    };

    ir.add_type(TypeDef::Struct(parent));
    ir.add_type(TypeDef::Struct(child));

    // Tarjan finds mutual cycle
    let mut detector = TarjanCycleDetector::new(&ir);
    let sccs = detector.find_cyclic_sccs();
    assert_eq!(sccs.len(), 1);
    assert_eq!(sccs[0].len(), 2);

    // Resolve cycles
    ir.resolve_cycles();

    // Verify minimal cut point was selected on optional back-reference
    if let Some(TypeDef::Struct(ref s)) = ir.find_type(&qname_b) {
        let parent_ref = &s.fields[0];
        assert!(parent_ref.is_cycle_cut);
        assert!(parent_ref.type_ref.is_boxed());
    } else {
        panic!("Expected Child StructDef");
    }

    // Now acyclic
    let mut detector_after = TarjanCycleDetector::new(&ir);
    assert!(detector_after.find_cyclic_sccs().is_empty());
}

#[test]
fn test_list_fields_do_not_trigger_spurious_cycles() {
    let mut ir = SchemaIR::new();
    let folder_qname = QName::local("Folder");

    // A folder contains subfolders in a Vec/List (which already introduces heap indirection)
    let folder = StructDef {
        qname: folder_qname.clone(),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![{
            let mut f = FieldDef::new(
                "subfolders",
                "subfolder",
                FieldKind::Element,
                TypeRef::List(Box::new(TypeRef::Named(folder_qname.clone()))),
            );
            f.cardinality = Cardinality {
                min_occurs: 0,
                max_occurs: OccursLimit::Unbounded,
            };
            f
        }],
        documentation: None,
    };

    ir.add_type(TypeDef::Struct(folder));

    // Tarjan should detect 0 cyclic SCCs because lists are already heap allocated!
    let mut detector = TarjanCycleDetector::new(&ir);
    assert!(detector.find_cyclic_sccs().is_empty());
}

#[test]
fn test_minijinja_template_engine_and_filters() {
    let mut env = create_template_engine();

    // Test case transformation filters
    let template = "{{ text | pascal_case }} | {{ text | snake_case }} | {{ text | camel_case }} | {{ text | screaming_snake_case }} | {{ text | kebab_case }}";
    env.add_template("case_test", template).unwrap();

    let tmpl = env.get_template("case_test").unwrap();
    let rendered = tmpl.render(context!(text => "postal_address")).unwrap();
    assert_eq!(
        rendered,
        "PostalAddress | postal_address | postalAddress | POSTAL_ADDRESS | postal-address"
    );

    // Test keyword sanitization
    assert_eq!(sanitize_keyword("type", "rust"), "r#type");
    assert_eq!(sanitize_keyword("match", "rust"), "r#match");
    assert_eq!(sanitize_keyword("normal_name", "rust"), "normal_name");

    assert_eq!(sanitize_keyword("def", "python"), "def_");
    assert_eq!(sanitize_keyword("class", "python"), "class_");
    assert_eq!(sanitize_keyword("from", "python"), "from_");

    assert_eq!(sanitize_keyword("record", "csharp"), "@record");
    assert_eq!(sanitize_keyword("class", "csharp"), "@class");

    // Test keyword filter in template
    let kw_tmpl = "{{ field | sanitize_keyword('rust') }}";
    env.add_template("kw_test", kw_tmpl).unwrap();
    let rendered_kw = env
        .get_template("kw_test")
        .unwrap()
        .render(context!(field => "fn"))
        .unwrap();
    assert_eq!(rendered_kw, "r#fn");
}

struct DummyRustContext;
impl LanguageContext for DummyRustContext {
    fn target_language(&self) -> &'static str {
        "rust"
    }

    fn map_primitive(&self, prim: PrimitiveType) -> &'static str {
        match prim {
            PrimitiveType::String => "String",
            PrimitiveType::Int => "i32",
            PrimitiveType::Long => "i64",
            PrimitiveType::Boolean => "bool",
            PrimitiveType::Decimal | PrimitiveType::Double => "f64",
            _ => "String",
        }
    }

    fn map_type_ref(&self, type_ref: &TypeRef) -> String {
        match type_ref {
            TypeRef::Primitive(p) => self.map_primitive(*p).to_string(),
            TypeRef::Named(q) => q.local.clone(),
            TypeRef::Boxed(inner) => format!("Box<{}>", self.map_type_ref(inner)),
            TypeRef::List(inner) => format!("Vec<{}>", self.map_type_ref(inner)),
        }
    }
}

#[test]
fn test_language_context_adapter() {
    let ctx = DummyRustContext;
    assert_eq!(ctx.target_language(), "rust");
    assert_eq!(ctx.sanitize_identifier("type"), "r#type");
    assert_eq!(ctx.sanitize_identifier("my_field"), "my_field");
    assert_eq!(ctx.map_primitive(PrimitiveType::Int), "i32");
    assert_eq!(ctx.map_primitive(PrimitiveType::String), "String");

    let boxed_ref = TypeRef::Boxed(Box::new(TypeRef::Named(QName::local("TreeNode"))));
    assert_eq!(ctx.map_type_ref(&boxed_ref), "Box<TreeNode>");

    let list_ref = TypeRef::List(Box::new(TypeRef::Primitive(PrimitiveType::Int)));
    assert_eq!(ctx.map_type_ref(&list_ref), "Vec<i32>");
}
