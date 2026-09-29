use polyxml::codegen::typescript::{
    to_ts_field_identifier, to_ts_type_name, to_ts_variant_name, TypeScriptBackend,
    TypeScriptCodegen, TypeScriptOptions,
};
use polyxml::ir::{
    Cardinality, EnumDef, EnumValue, FieldDef, FieldKind, PrimitiveType, QName, RestrictionFacets,
    SchemaIR, SimpleTypeDef, StructDef, TypeDef, TypeRef, UnionBranch, UnionDef,
};
use polyxml::schema_parser::XsdParser;
use std::{fs, process::Command};
use tempfile::tempdir;

#[test]
fn mixed_content_inheritance_compiles_with_widened_items() {
    let schema = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ts_mixed_content_inheritance.xsd"
    );
    let ir = XsdParser::new().parse_file(schema).unwrap();
    let code = TypeScriptCodegen::new(TypeScriptOptions::default()).generate_module(&ir);
    assert!(code.contains("export type Ed = Omit<Bin, \"items\"> & {"));
    if Command::new("tsc").arg("--version").output().is_ok() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("models.ts");
        fs::write(&source, code).unwrap();
        let result = Command::new("tsc")
            .args(["--noEmit", "--skipLibCheck", "--target", "ES2020"])
            .arg(&source)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
    }
}

#[test]
fn test_ts_identifier_sanitization() {
    assert_eq!(to_ts_field_identifier("type"), "type_");
    assert_eq!(to_ts_field_identifier("class"), "class_");
    assert_eq!(to_ts_field_identifier("debugger"), "debugger_");
    assert_eq!(to_ts_field_identifier("export"), "export_");
    assert_eq!(to_ts_field_identifier("function"), "function_");
    assert_eq!(to_ts_field_identifier("normalField"), "normalField");
    assert_eq!(to_ts_field_identifier("snake_case_field"), "snakeCaseField");
    assert_eq!(to_ts_field_identifier("100mDash"), "_100mDash");

    assert_eq!(to_ts_type_name("order-status"), "OrderStatus");
    assert_eq!(to_ts_type_name("customer_record"), "CustomerRecord");
    assert_eq!(to_ts_type_name("100Percent"), "Type100percent");

    assert_eq!(to_ts_variant_name("pending"), "Pending");
    assert_eq!(to_ts_variant_name("in-progress"), "InProgress");
    assert_eq!(to_ts_variant_name("10-day-hold"), "Value10DayHold");
}

#[test]
fn test_ts_interface_and_enum_codegen() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/shop");

    // Enum: OrderStatus
    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/shop"), "OrderStatus"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "pending".into(),
                value: "pending".into(),
                documentation: Some("Pending review".into()),
            },
            EnumValue {
                name: "shipped".into(),
                value: "shipped".into(),
                documentation: None,
            },
            EnumValue {
                name: "cancelled".into(),
                value: "cancelled".into(),
                documentation: None,
            },
        ],
        documentation: Some("Status of order processing".into()),
    }));

    // Struct: Customer
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/shop"), "Customer"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
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
                documentation: Some("Customer unique ID".into()),
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "name".into(),
                xml_name: "name".into(),
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
                name: "type".into(), // keyword
                xml_name: "type".into(),
                namespace: None,
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
                type_ref: TypeRef::Named(QName::new(
                    Some("https://example.com/shop"),
                    "OrderStatus",
                )),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
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
        ],
        documentation: Some("Customer record with orders".into()),
    }));

    let options = TypeScriptOptions {
        backend: TypeScriptBackend::None,
        emit_zod: false,
        use_interface: true,
        readonly_fields: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = TypeScriptCodegen::new(options);
    let code = codegen.generate_module(&ir);

    // Verify Enum type & companion const object
    assert!(code.contains("export type OrderStatus ="));
    assert!(code.contains("\"pending\""));
    assert!(code.contains("\"shipped\""));
    assert!(code.contains("\"cancelled\""));
    assert!(code.contains("export const OrderStatus = {"));
    assert!(code.contains("Pending: \"pending\","));
    assert!(code.contains("Shipped: \"shipped\","));
    assert!(code.contains("} as const;"));

    // Verify Customer interface
    assert!(code.contains("export interface Customer {"));
    assert!(code.contains("id: number;"));
    assert!(code.contains("name: string;"));
    assert!(code.contains("type_?: string | null;"));
    assert!(code.contains("status: OrderStatus;"));
    assert!(code.contains("tags: string[];"));
    assert!(code.contains("/** Customer record with orders */"));
}

#[test]
fn test_ts_discriminated_union_choice() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/union");

    ir.add_type(TypeDef::Union(UnionDef {
        qname: QName::new(Some("https://example.com/union"), "ContactChoice"),
        branches: vec![
            UnionBranch {
                variant_name: "email".into(),
                xml_name: "email".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: Some("Email address".into()),
            },
            UnionBranch {
                variant_name: "phone".into(),
                xml_name: "phone".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: None,
            },
        ],
        documentation: Some("Choice of contact method".into()),
    }));

    let options = TypeScriptOptions {
        backend: TypeScriptBackend::None,
        emit_zod: false,
        use_interface: true,
        readonly_fields: true,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = TypeScriptCodegen::new(options);
    let code = codegen.generate_module(&ir);

    assert!(code.contains("export type ContactChoice ="));
    assert!(code.contains("{ readonly kind: \"Email\"; readonly value: string; }"));
    assert!(code.contains("{ readonly kind: \"Phone\"; readonly value: string; }"));
}

#[test]
fn test_ts_zod_schema_generation() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/zod");

    // Simple restriction with facets
    let facets = RestrictionFacets {
        min_length: Some(3),
        max_length: Some(10),
        patterns: vec!["^[A-Z0-9]+$".into()],
        ..Default::default()
    };

    ir.add_type(TypeDef::Simple(Box::new(SimpleTypeDef {
        qname: QName::new(Some("https://example.com/zod"), "PostalCode"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        facets,
        documentation: Some("Constrained postal code".into()),
    })));

    // Numeric restriction with inclusive range
    let num_facets = RestrictionFacets {
        min_inclusive: Some("18".into()),
        max_inclusive: Some("120".into()),
        ..Default::default()
    };

    ir.add_type(TypeDef::Simple(Box::new(SimpleTypeDef {
        qname: QName::new(Some("https://example.com/zod"), "Age"),
        base_type: TypeRef::Primitive(PrimitiveType::Int),
        facets: num_facets,
        documentation: None,
    })));

    // Struct using the simple types
    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/zod"), "Person"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "age".into(),
                xml_name: "age".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(Some("https://example.com/zod"), "Age")),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "postalCode".into(),
                xml_name: "postalCode".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(Some("https://example.com/zod"), "PostalCode")),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: None,
    }));

    let options = TypeScriptOptions {
        backend: TypeScriptBackend::None,
        emit_zod: true,
        use_interface: true,
        readonly_fields: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = TypeScriptCodegen::new(options);
    let code = codegen.generate_module(&ir);

    // Verify import
    assert!(code.contains("import { z } from \"zod\";"));

    // Verify PostalCodeSchema
    assert!(code.contains("export const PostalCodeSchema = z.string().min(3).max(10).regex(new RegExp(\"^(?:^[A-Z0-9]+$)(?![\\\\s\\\\S])\"));"));

    // Verify AgeSchema
    assert!(code.contains("export const AgeSchema = z.number().int().gte(18).lte(120);"));

    // Verify PersonSchema
    assert!(code.contains("export const PersonSchema = z.object({"));
    assert!(code.contains("age: AgeSchema,"));
    assert!(code.contains("postalCode: PostalCodeSchema.optional(),"));
}

#[test]
fn test_ts_recursive_cycle_zod_lazy() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/tree");

    let tree_qname = QName::new(Some("https://example.com/tree"), "TreeNode");
    ir.add_type(TypeDef::Struct(StructDef {
        qname: tree_qname.clone(),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "value".into(),
                xml_name: "value".into(),
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
                name: "children".into(),
                xml_name: "child".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(tree_qname.clone()),
                cardinality: Cardinality::unbounded(0),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: true, // Cut point identified by Tarjan SCC
            },
        ],
        documentation: Some("Recursive tree node".into()),
    }));

    let options = TypeScriptOptions {
        backend: TypeScriptBackend::None,
        emit_zod: true,
        use_interface: true,
        readonly_fields: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = TypeScriptCodegen::new(options);
    let code = codegen.generate_module(&ir);

    // Verify TypeScript interface is generated cleanly
    assert!(code.contains("export interface TreeNode {"));
    assert!(code.contains("value: string;"));
    assert!(code.contains("children: TreeNode[];"));

    // Verify Zod schema uses lazy cycle handling and explicit type annotation
    assert!(
        code.contains("export const TreeNodeSchema: z.ZodType<TreeNode> = z.lazy(() => z.object({")
    );
    assert!(code.contains("children: z.array(TreeNodeSchema),"));
    assert!(code.contains("}));"));
}

#[test]
fn test_ts_backend_from_str_loose() {
    assert_eq!(
        TypeScriptBackend::from_str_loose("none"),
        Some(TypeScriptBackend::None)
    );
    assert_eq!(
        TypeScriptBackend::from_str_loose("interfaces"),
        Some(TypeScriptBackend::None)
    );
    assert_eq!(
        TypeScriptBackend::from_str_loose("standard"),
        Some(TypeScriptBackend::None)
    );
    assert_eq!(
        TypeScriptBackend::from_str_loose("zod"),
        Some(TypeScriptBackend::Zod)
    );
    assert_eq!(
        TypeScriptBackend::from_str_loose("valibot"),
        Some(TypeScriptBackend::Valibot)
    );
    assert_eq!(
        TypeScriptBackend::from_str_loose("vali"),
        Some(TypeScriptBackend::Valibot)
    );
    assert_eq!(
        TypeScriptBackend::from_str_loose("typebox"),
        Some(TypeScriptBackend::TypeBox)
    );
    assert_eq!(
        TypeScriptBackend::from_str_loose("sinclair"),
        Some(TypeScriptBackend::TypeBox)
    );
    assert_eq!(TypeScriptBackend::from_str_loose("unknown"), None);
}

#[test]
fn test_ts_valibot_schema_generation() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/valibot");

    let facets = RestrictionFacets {
        min_length: Some(3),
        max_length: Some(10),
        patterns: vec!["^[A-Z0-9]+$".into()],
        ..Default::default()
    };

    ir.add_type(TypeDef::Simple(Box::new(SimpleTypeDef {
        qname: QName::new(Some("https://example.com/valibot"), "PostalCode"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        facets,
        documentation: Some("Constrained postal code".into()),
    })));

    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/valibot"), "Status"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "active".into(),
                value: "active".into(),
                documentation: None,
            },
            EnumValue {
                name: "inactive".into(),
                value: "inactive".into(),
                documentation: None,
            },
        ],
        documentation: None,
    }));

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/valibot"), "Person"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "postalCode".into(),
                xml_name: "postalCode".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(
                    Some("https://example.com/valibot"),
                    "PostalCode",
                )),
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
                type_ref: TypeRef::Named(QName::new(Some("https://example.com/valibot"), "Status")),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: None,
    }));

    let options = TypeScriptOptions {
        backend: TypeScriptBackend::Valibot,
        emit_zod: false,
        use_interface: true,
        readonly_fields: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = TypeScriptCodegen::new(options);
    let code = codegen.generate_module(&ir);

    // Verify Valibot import
    assert!(code.contains("import * as v from \"valibot\";"));

    // Verify simple type facets with v.pipe
    assert!(code.contains("export const PostalCodeSchema = v.pipe(v.string(), v.minLength(3), v.maxLength(10), v.regex(/^(?:^[A-Z0-9]+$)(?![\\s\\S])/));"));

    // Verify enum picklist
    assert!(code.contains("export const StatusSchema = v.picklist([\"active\", \"inactive\"]);"));

    // Verify struct schema
    assert!(code.contains("export const PersonSchema = v.object({"));
    assert!(code.contains("postalCode: PostalCodeSchema,"));
    assert!(code.contains("status: v.optional(StatusSchema),"));
}

#[test]
fn test_ts_typebox_schema_generation() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/typebox");

    let facets = RestrictionFacets {
        min_length: Some(2),
        max_length: Some(8),
        ..Default::default()
    };

    ir.add_type(TypeDef::Simple(Box::new(SimpleTypeDef {
        qname: QName::new(Some("https://example.com/typebox"), "Code"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        facets,
        documentation: None,
    })));

    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/typebox"), "Role"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "admin".into(),
                value: "admin".into(),
                documentation: None,
            },
            EnumValue {
                name: "user".into(),
                value: "user".into(),
                documentation: None,
            },
        ],
        documentation: None,
    }));

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/typebox"), "Account"),
        base_type: None,
        is_abstract: false,
        is_mixed: false,
        fields: vec![
            FieldDef {
                name: "code".into(),
                xml_name: "code".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(Some("https://example.com/typebox"), "Code")),
                cardinality: Cardinality::required_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "role".into(),
                xml_name: "role".into(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(QName::new(Some("https://example.com/typebox"), "Role")),
                cardinality: Cardinality::optional_one(),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
        ],
        documentation: None,
    }));

    let options = TypeScriptOptions {
        backend: TypeScriptBackend::TypeBox,
        emit_zod: false,
        use_interface: true,
        readonly_fields: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = TypeScriptCodegen::new(options);
    let code = codegen.generate_module(&ir);

    // Verify TypeBox imports
    assert!(code.contains("import { Type, Static } from \"@sinclair/typebox\";"));

    // Verify TypeBox Code schema
    assert!(code.contains("export const CodeSchema = Type.String({ minLength: 2, maxLength: 8 });"));

    // Verify TypeBox enum
    assert!(code.contains(
        "export const RoleSchema = Type.Union([Type.Literal(\"admin\"), Type.Literal(\"user\")]);"
    ));

    // Verify TypeBox struct
    assert!(code.contains("export const AccountSchema = Type.Object({"));
    assert!(code.contains("code: CodeSchema,"));
    assert!(code.contains("role: Type.Optional(RoleSchema),"));
}
