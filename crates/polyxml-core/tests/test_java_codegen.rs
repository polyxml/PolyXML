use std::fs;
use std::process::Command;
use tempfile::tempdir;

use polyxml::codegen::java::{
    to_java_enum_constant, to_java_field_identifier, to_java_type_name, JavaBackend, JavaCodegen,
    JavaOptions,
};
use polyxml::ir::{
    Cardinality, EnumDef, EnumValue, FieldDef, FieldKind, PrimitiveType, QName, RestrictionFacets,
    SchemaIR, SimpleTypeDef, StructDef, TypeDef, TypeRef, UnionBranch, UnionDef,
};

#[test]
fn test_java_identifier_sanitization() {
    assert_eq!(to_java_field_identifier("class"), "class_");
    assert_eq!(to_java_field_identifier("record"), "record_");
    assert_eq!(to_java_field_identifier("sealed"), "sealed_");
    assert_eq!(to_java_field_identifier("permits"), "permits_");
    assert_eq!(to_java_field_identifier("import"), "import_");
    assert_eq!(to_java_field_identifier("default"), "default_");
    assert_eq!(to_java_field_identifier("normalField"), "normalField");
    assert_eq!(
        to_java_field_identifier("snake_case_field"),
        "snakeCaseField"
    );
    assert_eq!(to_java_field_identifier("100mDash"), "_100mDash");

    assert_eq!(to_java_type_name("order-status"), "OrderStatus");
    assert_eq!(to_java_type_name("customer_record"), "CustomerRecord");
    assert_eq!(to_java_type_name("100Percent"), "Type100percent");

    assert_eq!(to_java_enum_constant("pending"), "PENDING");
    assert_eq!(to_java_enum_constant("in-progress"), "IN_PROGRESS");
    assert_eq!(to_java_enum_constant("10-day-hold"), "VALUE_10_DAY_HOLD");
    assert_eq!(to_java_enum_constant(""), "EMPTY");
}

#[test]
fn test_java_records_and_enums_generation() {
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
                facets: Some(RestrictionFacets {
                    min_length: Some(2),
                    max_length: Some(50),
                    patterns: vec!["^[A-Za-z ]+$".into()],
                    ..Default::default()
                }),
                is_cycle_cut: false,
            },
            FieldDef {
                name: "class".into(), // keyword in Java
                xml_name: "class".into(),
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

    let options = JavaOptions {
        package_name: "com.example.shop".to_string(),
        backend: JavaBackend::Standard,
        use_records: true,
        emit_builder: false,
        emit_direct_codec: false,
        validate_facets: true,
        bean_validation: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = JavaCodegen::new(options);
    let files = codegen.generate_files(&ir);

    assert_eq!(files.len(), 2);
    let files_map: std::collections::HashMap<_, _> = files.into_iter().collect();

    // Verify OrderStatus.java
    let order_status_code = files_map.get("OrderStatus.java").unwrap();
    assert!(order_status_code.contains("package com.example.shop;"));
    assert!(order_status_code.contains("public enum OrderStatus {"));
    assert!(order_status_code.contains("PENDING(\"pending\"),"));
    assert!(order_status_code.contains("SHIPPED(\"shipped\"),"));
    assert!(order_status_code.contains("CANCELLED(\"cancelled\");"));
    assert!(order_status_code.contains("public String getValue()"));
    assert!(order_status_code.contains("public static OrderStatus fromValue(String value)"));

    // Verify Customer.java
    let customer_code = files_map.get("Customer.java").unwrap();
    assert!(customer_code.contains("package com.example.shop;"));
    assert!(customer_code.contains("public record Customer("));
    assert!(customer_code.contains("int id,"));
    assert!(customer_code.contains("String name,"));
    assert!(customer_code.contains("java.util.Optional<String> class_,"));
    assert!(customer_code.contains("OrderStatus status,"));
    assert!(customer_code.contains("java.util.List<String> tags"));
    assert!(customer_code.contains("Objects.requireNonNull(name, \"name must not be null\");"));
    assert!(customer_code.contains(
        "if (name.length() < 2) throw new IllegalArgumentException(\"name minLength is 2\");"
    ));
    assert!(customer_code.contains(
        "if (name.length() > 50) throw new IllegalArgumentException(\"name maxLength is 50\");"
    ));

    // Verify compilation with javac -Werror
    let dir = tempdir().unwrap();
    for (filename, content) in &files_map {
        fs::write(dir.path().join(filename), content).unwrap();
    }

    let javac = Command::new("javac")
        .arg("-Werror")
        .arg(dir.path().join("OrderStatus.java"))
        .arg(dir.path().join("Customer.java"))
        .output();

    if let Ok(out) = javac {
        assert!(
            out.status.success(),
            "javac failed on generated records: {}\nstdout: {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }
}

#[test]
fn test_java_sealed_interface_choice() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/payment");

    ir.add_type(TypeDef::Union(UnionDef {
        qname: QName::new(Some("https://example.com/payment"), "PaymentChoice"),
        branches: vec![
            UnionBranch {
                variant_name: "creditCard".into(),
                xml_name: "creditCard".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: Some("Credit card token".into()),
            },
            UnionBranch {
                variant_name: "directDebit".into(),
                xml_name: "directDebit".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: Some("IBAN direct debit".into()),
            },
        ],
        documentation: Some("Choice of payment method".into()),
    }));

    let options = JavaOptions {
        package_name: "com.example.payment".to_string(),
        backend: JavaBackend::Standard,
        use_records: true,
        emit_builder: false,
        emit_direct_codec: false,
        validate_facets: true,
        bean_validation: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = JavaCodegen::new(options);
    let files = codegen.generate_files(&ir);

    assert_eq!(files.len(), 1);
    let (filename, content) = &files[0];
    assert_eq!(filename, "PaymentChoice.java");
    assert!(content.contains("package com.example.payment;"));
    assert!(content.contains("public sealed interface PaymentChoice permits PaymentChoice.CreditCard, PaymentChoice.DirectDebit {"));
    assert!(content.contains("record CreditCard(String value) implements PaymentChoice {}"));
    assert!(content.contains("record DirectDebit(String value) implements PaymentChoice {}"));

    // Verify compilation with javac -Werror
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("PaymentChoice.java"), content).unwrap();

    let javac = Command::new("javac")
        .arg("-Werror")
        .arg(dir.path().join("PaymentChoice.java"))
        .output();

    if let Ok(out) = javac {
        assert!(
            out.status.success(),
            "javac failed on sealed interface: {}\nstdout: {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }
}

#[test]
fn test_java_module_container_class() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/banking");

    ir.add_type(TypeDef::Simple(Box::new(SimpleTypeDef {
        qname: QName::new(Some("https://example.com/banking"), "Iban"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        facets: RestrictionFacets {
            min_length: Some(15),
            max_length: Some(34),
            patterns: vec!["^[A-Z]{2}[0-9]{2}[A-Z0-9]+$".into()],
            ..Default::default()
        },
        documentation: Some("International Bank Account Number".into()),
    })));

    let options = JavaOptions {
        package_name: "com.example.banking".to_string(),
        backend: JavaBackend::Standard,
        use_records: true,
        emit_builder: false,
        emit_direct_codec: false,
        validate_facets: true,
        bean_validation: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = JavaCodegen::new(options);
    let code = codegen.generate_module(&ir, "BankingModels");

    assert!(code.contains("package com.example.banking;"));
    assert!(code.contains("public final class BankingModels {"));
    assert!(code.contains("private BankingModels() {}"));
    assert!(code.contains("public static record Iban(String value) {"));
    assert!(code.contains(
        "if (value.length() < 15) throw new IllegalArgumentException(\"value minLength is 15\");"
    ));
    assert!(code.contains(
        "if (value.length() > 34) throw new IllegalArgumentException(\"value maxLength is 34\");"
    ));

    // Verify compilation with javac -Werror
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("BankingModels.java"), code).unwrap();

    let javac = Command::new("javac")
        .arg("-Werror")
        .arg(dir.path().join("BankingModels.java"))
        .output();

    if let Ok(out) = javac {
        assert!(
            out.status.success(),
            "javac failed on BankingModels.java: {}\nstdout: {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }
}

#[test]
fn test_java_jackson_backend_struct_annotations() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/crm");

    ir.add_type(TypeDef::Struct(StructDef {
        qname: QName::new(Some("https://example.com/crm"), "Contact"),
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
                documentation: None,
                facets: None,
                is_cycle_cut: false,
            },
            FieldDef {
                name: "email".into(),
                xml_name: "email".into(),
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
                name: "phone".into(),
                xml_name: "phone".into(),
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
        documentation: Some("CRM contact record".into()),
    }));

    let options = JavaOptions {
        package_name: "com.example.crm".to_string(),
        backend: JavaBackend::Jackson,
        use_records: true,
        emit_builder: false,
        emit_direct_codec: false,
        validate_facets: true,
        bean_validation: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = JavaCodegen::new(options);
    let files = codegen.generate_files(&ir);

    assert_eq!(files.len(), 1);
    let (filename, code) = &files[0];
    assert_eq!(filename, "Contact.java");

    // Jackson imports
    assert!(code.contains("import com.fasterxml.jackson.annotation.*;"));
    assert!(code.contains("import com.fasterxml.jackson.dataformat.xml.annotation.*;"));

    // Class-level annotations
    assert!(code.contains("@JsonIgnoreProperties(ignoreUnknown = true)"));
    assert!(code.contains("@JsonInclude(JsonInclude.Include.NON_EMPTY)"));
    assert!(code.contains("@JacksonXmlRootElement(localName = \"Contact\""));

    // Field annotations — attribute
    assert!(code.contains("@JsonProperty(\"id\")"));
    assert!(code.contains("@JacksonXmlProperty(localName = \"id\", isAttribute = true)"));

    // Field annotations — element
    assert!(code.contains("@JsonProperty(\"email\")"));
    assert!(code.contains("@JacksonXmlProperty(localName = \"email\", isAttribute = false)"));

    // Optional field gets NON_EMPTY
    assert!(code.contains("@JsonProperty(\"phone\")"));

    // List field gets wrapper suppression
    assert!(code.contains("@JacksonXmlElementWrapper(useWrapping = false)"));
}

#[test]
fn test_java_jackson_backend_enum_annotations() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/orders");

    ir.add_type(TypeDef::Enum(EnumDef {
        qname: QName::new(Some("https://example.com/orders"), "Priority"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        variants: vec![
            EnumValue {
                name: "low".into(),
                value: "low".into(),
                documentation: None,
            },
            EnumValue {
                name: "high".into(),
                value: "high".into(),
                documentation: None,
            },
        ],
        documentation: None,
    }));

    let options = JavaOptions {
        package_name: "com.example.orders".to_string(),
        backend: JavaBackend::Jackson,
        use_records: true,
        emit_builder: false,
        emit_direct_codec: false,
        validate_facets: true,
        bean_validation: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = JavaCodegen::new(options);
    let files = codegen.generate_files(&ir);

    assert_eq!(files.len(), 1);
    let (_, code) = &files[0];

    // @JsonValue on getValue()
    assert!(code.contains("@JsonValue"));
    assert!(code.contains("public String getValue()"));

    // @JsonCreator on fromValue()
    assert!(code.contains("@JsonCreator"));
    assert!(code.contains("public static Priority fromValue(String value)"));
}

#[test]
fn test_java_jackson_backend_union_annotations() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/messaging");

    ir.add_type(TypeDef::Union(UnionDef {
        qname: QName::new(Some("https://example.com/messaging"), "MessageChannel"),
        branches: vec![
            UnionBranch {
                variant_name: "sms".into(),
                xml_name: "sms".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: None,
            },
            UnionBranch {
                variant_name: "email".into(),
                xml_name: "email".into(),
                namespace: None,
                type_ref: TypeRef::Primitive(PrimitiveType::String),
                documentation: None,
            },
        ],
        documentation: None,
    }));

    let options = JavaOptions {
        package_name: "com.example.messaging".to_string(),
        backend: JavaBackend::Jackson,
        use_records: true,
        emit_builder: false,
        emit_direct_codec: false,
        validate_facets: true,
        bean_validation: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = JavaCodegen::new(options);
    let files = codegen.generate_files(&ir);

    assert_eq!(files.len(), 1);
    let (_, code) = &files[0];

    // Polymorphic type info
    assert!(code.contains("@JsonTypeInfo(use = JsonTypeInfo.Id.NAME"));
    assert!(code.contains("@JsonSubTypes({"));
    assert!(code.contains("@JsonSubTypes.Type(value = MessageChannel.Sms.class, name = \"sms\")"));
    assert!(
        code.contains("@JsonSubTypes.Type(value = MessageChannel.Email.class, name = \"email\")")
    );

    // Variant type names
    assert!(code.contains("@JsonTypeName(\"sms\")"));
    assert!(code.contains("@JsonTypeName(\"email\")"));
}

#[test]
fn test_java_jackson_backend_simple_type_annotations() {
    let mut ir = SchemaIR::new().with_target_namespace("https://example.com/types");

    ir.add_type(TypeDef::Simple(Box::new(SimpleTypeDef {
        qname: QName::new(Some("https://example.com/types"), "CurrencyCode"),
        base_type: TypeRef::Primitive(PrimitiveType::String),
        facets: RestrictionFacets {
            length: Some(3),
            patterns: vec!["^[A-Z]{3}$".into()],
            ..Default::default()
        },
        documentation: Some("ISO 4217 currency code".into()),
    })));

    let options = JavaOptions {
        package_name: "com.example.types".to_string(),
        backend: JavaBackend::Jackson,
        use_records: true,
        emit_builder: false,
        emit_direct_codec: false,
        validate_facets: true,
        bean_validation: false,
        emit_root_aliases: true,
        custom_header: None,
    };
    let codegen = JavaCodegen::new(options);
    let files = codegen.generate_files(&ir);

    assert_eq!(files.len(), 1);
    let (_, code) = &files[0];

    // @JsonValue @JacksonXmlText on value component
    assert!(code.contains("@JsonValue @JacksonXmlText String value"));

    // @JsonCreator factory method
    assert!(code.contains("@JsonCreator"));
    assert!(code.contains("public static CurrencyCode of(String value)"));
    assert!(code.contains("return new CurrencyCode(value);"));

    // Validation still present
    assert!(code.contains("if (value.length() != 3)"));
}

#[test]
fn test_java_jackson_backend_from_str_loose() {
    assert_eq!(
        JavaBackend::from_str_loose("jackson"),
        Some(JavaBackend::Jackson)
    );
    assert_eq!(
        JavaBackend::from_str_loose("Jackson"),
        Some(JavaBackend::Jackson)
    );
    assert_eq!(
        JavaBackend::from_str_loose("spring"),
        Some(JavaBackend::Jackson)
    );
    assert_eq!(
        JavaBackend::from_str_loose("spring-boot"),
        Some(JavaBackend::Jackson)
    );
    assert_eq!(
        JavaBackend::from_str_loose("enterprise"),
        Some(JavaBackend::Jackson)
    );
    assert_eq!(
        JavaBackend::from_str_loose("standard"),
        Some(JavaBackend::Standard)
    );
    assert_eq!(
        JavaBackend::from_str_loose("std"),
        Some(JavaBackend::Standard)
    );
    assert_eq!(JavaBackend::from_str_loose("unknown"), None);
}

#[test]
fn test_java_bean_validation_annotations_are_opt_in() {
    let schema = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="Code"><xs:restriction base="xs:string">
        <xs:minLength value="2"/><xs:maxLength value="8"/><xs:pattern value="[A-Z]+"/>
      </xs:restriction></xs:simpleType>
      <xs:simpleType name="Count"><xs:restriction base="xs:int">
        <xs:minInclusive value="1"/><xs:maxInclusive value="10"/>
      </xs:restriction></xs:simpleType>
      <xs:simpleType name="Price"><xs:restriction base="xs:decimal">
        <xs:minExclusive value="0.0"/><xs:maxExclusive value="99.9"/>
      </xs:restriction></xs:simpleType>
      <xs:complexType name="Order"><xs:sequence>
        <xs:element name="code" type="Code"/>
        <xs:element name="count" type="Count"/>
        <xs:element name="price" type="Price"/>
      </xs:sequence></xs:complexType>
    </xs:schema>"#;
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(schema)
        .unwrap();
    let plain = JavaCodegen::new(JavaOptions::default()).generate_files(&ir);
    assert!(plain
        .iter()
        .all(|(_, code)| !code.contains("jakarta.validation")));
    let files = JavaCodegen::new(JavaOptions {
        bean_validation: true,
        ..Default::default()
    })
    .generate_files(&ir);
    let source = |name: &str| {
        files
            .iter()
            .find(|(file, _)| file == name)
            .unwrap()
            .1
            .as_str()
    };
    assert!(source("Code.java").contains("@Size(min = 2, max = 8)"));
    assert!(source("Code.java")
        .contains("@jakarta.validation.constraints.Pattern(regexp = \"[A-Z]+\")"));
    assert!(source("Count.java").contains("@Min(1)"));
    assert!(source("Count.java").contains("@Max(10)"));
    assert!(source("Price.java").contains("@DecimalMin(value = \"0.0\", inclusive = false)"));
    assert!(source("Price.java").contains("@DecimalMax(value = \"99.9\", inclusive = false)"));
    assert!(source("Order.java").contains("@NotNull"));
    assert!(source("Order.java").contains("@jakarta.validation.Valid"));
}

#[test]
fn test_java_mutable_builders_and_direct_codecs_execute() {
    let schema = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="Code"><xs:restriction base="xs:string"><xs:minLength value="2"/></xs:restriction></xs:simpleType>
      <xs:complexType name="Base"><xs:attribute name="id" type="xs:int" use="required"/></xs:complexType>
      <xs:complexType name="Item"><xs:complexContent><xs:extension base="Base"><xs:sequence>
        <xs:element name="label" type="xs:string"/>
        <xs:element name="active" type="xs:boolean"/>
        <xs:element name="code" type="Code" minOccurs="0"/>
        <xs:element name="tags" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
        <xs:element name="child" type="Item" minOccurs="0"/>
        <xs:element name="bytes" type="xs:hexBinary" minOccurs="0"/>
      </xs:sequence></xs:extension></xs:complexContent></xs:complexType>
      <xs:element name="item" type="Item"/>
    </xs:schema>"#;
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(schema)
        .unwrap();
    let generator = JavaCodegen::new(JavaOptions {
        package_name: String::new(),
        use_records: false,
        emit_builder: true,
        emit_direct_codec: true,
        ..Default::default()
    });
    let dir = tempdir().unwrap();
    let files = generator.generate_files(&ir);
    let item = &files.iter().find(|(n, _)| n == "Item.java").unwrap().1;
    assert!(item.contains("extends Base"));
    assert!(item.contains("public boolean isActive()"));
    assert!(item.contains("public ItemBuilder id(int id)"));
    let codec = &files.iter().find(|(n, _)| n == "ItemCodec.java").unwrap().1;
    assert!(codec.contains("ThreadLocal<XMLInputFactory>"));
    assert!(codec.contains("READER_FACTORY.get().createXMLStreamReader(input)"));
    assert!(codec.contains("WRITER_FACTORY.get().createXMLStreamWriter(output"));
    assert!(!codec.contains("XMLInputFactory.newFactory();\n        XMLStreamReader"));
    for (name, body) in files {
        fs::write(dir.path().join(name), body).unwrap();
    }
    fs::write(dir.path().join("Main.java"), r#"
import java.io.*;
import java.nio.charset.StandardCharsets;
public class Main {
    public static void main(String[] args) throws Exception {
        var builder = Item.builder().id(7).label("a<&").active(true).tags(new java.util.ArrayList<>(java.util.List.of("one", "two")));
        Item item = builder.build();
        item.setCode(new Code("OK"));
        item.setBytes(new byte[] {0, 15, -1});
        item.setChild(Item.builder().id(8).label("child").build());
        Item other = builder.build();
        item.getTags().add("three");
        if (other.getTags().size() != 2) throw new AssertionError("Builder shared a list");
        other.setTags(null); other.getTags().add("reset");
        if (!item.isActive() || item.getId() != 7) throw new AssertionError("Bean accessors");
        var bytes = new ByteArrayOutputStream();
        ItemCodec.writeXml(item, bytes);
        try { BaseCodec.writeXml(item, new ByteArrayOutputStream()); throw new AssertionError("Derived fields silently dropped"); } catch (javax.xml.stream.XMLStreamException expected) {}
        Item copy = ItemCodec.readXml(new ByteArrayInputStream(bytes.toByteArray()));
        if (!item.equals(copy) || item.hashCode() != copy.hashCode()) throw new AssertionError(bytes + " -> " + copy);
        copy.setId(20);
        if (item.equals(copy)) throw new AssertionError("Inherited equals");
        try { new Code("x"); throw new AssertionError("Missing facet validation"); } catch (IllegalArgumentException expected) {}
        String xml = "<item id='4'><unknown><nested/></unknown><label>x</label><active>1</active></item>";
        if (!ItemCodec.readXml(new ByteArrayInputStream(xml.getBytes(StandardCharsets.UTF_8))).isActive()) throw new AssertionError("XML boolean");
        try { ItemCodec.readXml(new ByteArrayInputStream("<item><active>bad</active></item>".getBytes())); throw new AssertionError("Bad boolean accepted"); } catch (javax.xml.stream.XMLStreamException expected) {}
    }
}
"#).unwrap();
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }
    let sources: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    let result = Command::new("javac").args(&sources).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new("java")
        .args(["-cp", dir.path().to_str().unwrap(), "Main"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_java_record_builder_and_nested_codec_execute() {
    let ir = polyxml::schema_parser::XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:test" xmlns:t="urn:test" elementFormDefault="qualified">
      <xs:complexType name="Base"><xs:attribute name="id" type="xs:int" use="required"/></xs:complexType>
      <xs:complexType name="Message"><xs:complexContent><xs:extension base="t:Base"><xs:sequence><xs:element name="text" type="xs:string"/><xs:element name="count" type="xs:int" minOccurs="0"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType>
      <xs:complexType name="Empty"/>
    </xs:schema>"#).unwrap();
    let generator = JavaCodegen::new(JavaOptions {
        package_name: String::new(),
        emit_builder: true,
        emit_direct_codec: true,
        ..Default::default()
    });
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Models.java"),
        generator.generate_module(&ir, "Models"),
    )
    .unwrap();
    fs::write(dir.path().join("Main.java"),r#"import java.io.*;
public class Main {
 public static void main(String[] args) throws Exception {
  var message = Models.Message.builder().id(5).text("hello").build();
  if (message.count().isPresent()) throw new AssertionError();
  var out = new ByteArrayOutputStream(); Models.MessageCodec.writeXml(message,out);
  if (!Models.MessageCodec.readXml(new ByteArrayInputStream(out.toByteArray())).equals(message)) throw new AssertionError(out.toString());
  if (Models.Empty.builder().build() == null) throw new AssertionError();
 }
}"#).unwrap();
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }
    let result = Command::new("javac")
        .current_dir(dir.path())
        .args(["Models.java", "Main.java"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new("java")
        .current_dir(dir.path())
        .args(["-cp", ".", "Main"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn test_direct_codec_rejects_wildcards_and_name_collisions() {
    let generator = JavaCodegen::new(JavaOptions {
        emit_direct_codec: true,
        ..Default::default()
    });
    for schema in [
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="Open"><xs:sequence><xs:any/></xs:sequence></xs:complexType></xs:schema>"#,
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="Item"/><xs:complexType name="ItemCodec"/></xs:schema>"#,
    ] {
        let ir = polyxml::schema_parser::XsdParser::new()
            .parse_str(schema)
            .unwrap();
        assert!(generator.validate_direct_codecs(&ir).is_err());
    }
}

#[test]
fn test_java_direct_nil_lists_choices_and_namespaces() {
    let mut ir=polyxml::schema_parser::XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:t="urn:test" targetNamespace="urn:test" elementFormDefault="qualified">
      <xs:complexType name="Choice"><xs:choice><xs:element name="text" type="xs:string"/><xs:element name="count" type="xs:int"/></xs:choice></xs:complexType>
      <xs:simpleType name="Status"><xs:restriction base="xs:string"><xs:enumeration value="ready"/><xs:enumeration value="done"/></xs:restriction></xs:simpleType>
      <xs:complexType name="Envelope"><xs:sequence>
        <xs:element name="values" type="xs:string" minOccurs="0" maxOccurs="unbounded" nillable="true"/>
        <xs:element name="numbers" type="xs:string" minOccurs="0"/>
        <xs:element name="choice" type="t:Choice"/>
        <xs:element name="status" type="t:Status"/>
        <xs:element name="nullable" type="xs:int" nillable="true"/>
        <xs:element name="infinity" type="xs:double"/>
      </xs:sequence></xs:complexType>
    </xs:schema>"#).unwrap();
    if let TypeDef::Struct(s) = ir
        .types
        .get_mut(&QName::new(Some("urn:test"), "Envelope"))
        .unwrap()
    {
        s.fields
            .iter_mut()
            .find(|f| f.name == "numbers")
            .unwrap()
            .type_ref = TypeRef::List(Box::new(TypeRef::Primitive(PrimitiveType::Boolean)));
    }
    for records in [false, true] {
        let generator = JavaCodegen::new(JavaOptions {
            package_name: String::new(),
            use_records: records,
            emit_builder: true,
            emit_direct_codec: true,
            ..Default::default()
        });
        generator.validate_direct_codecs(&ir).unwrap();
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("Models.java"),
            generator.generate_module(&ir, "Models"),
        )
        .unwrap();
        let nil = if records {
            "java.util.Optional.empty()"
        } else {
            "null"
        };
        let values = if records {
            "copy.values()"
        } else {
            "copy.getValues()"
        };
        let numbers = if records {
            "copy.numbers()"
        } else {
            "copy.getNumbers()"
        };
        fs::write(dir.path().join("Main.java"),format!(r#"import java.io.*;
public class Main {{ public static void main(String[] args) throws Exception {{
 var source=Models.Envelope.builder().values(new java.util.ArrayList<>(java.util.Arrays.asList("one",null,"three")))
   .numbers(java.util.List.of(true,false)).choice(new Models.Choice.Text("hi<&")).status(Models.Status.READY)
   .nullable({nil}).infinity(Double.POSITIVE_INFINITY).build();
 var output=new ByteArrayOutputStream(); Models.EnvelopeCodec.writeXml(source,output);
 String xml=output.toString(java.nio.charset.StandardCharsets.UTF_8);
 if(!xml.contains("xsi:nil") || !xml.contains("INF") || !xml.contains("urn:test")) throw new AssertionError(xml);
 var copy=Models.EnvelopeCodec.readXml(new ByteArrayInputStream(output.toByteArray()));
 if(!source.equals(copy) || {values}.get(1)!=null || !{numbers}.equals(java.util.List.of(true,false))) throw new AssertionError(xml+" -> "+copy);
}}
}}"#)).unwrap();
        if Command::new("javac").arg("-version").output().is_err() {
            continue;
        }
        let result = Command::new("javac")
            .current_dir(dir.path())
            .args(["Models.java", "Main.java"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "records={records}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let result = Command::new("java")
            .current_dir(dir.path())
            .args(["-cp", ".", "Main"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "records={records}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

// ---------------------------------------------------------------------------
// xsd:extension in plain record mode (the default): records cannot `extends`,
// so inherited fields must be inlined as components. Previously only the
// builder/direct-codec modes ran the base walk, silently dropping inherited
// fields from the default record output.
// ---------------------------------------------------------------------------

/// Slice one record's component header from `record Name(` through `) {`.
fn record_header<'a>(code: &'a str, name: &str) -> &'a str {
    let marker = format!("record {name}(");
    let start = code
        .find(&marker)
        .unwrap_or_else(|| panic!("expected {marker:?} in:\n{code}"));
    let rest = &code[start..];
    let end = rest
        .find(") {")
        .unwrap_or_else(|| panic!("unterminated record {name}:\n{code}"));
    &rest[..end + 3]
}

fn extension_schema() -> &'static str {
    r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"
            targetNamespace="urn:jx" xmlns:t="urn:jx" elementFormDefault="qualified">
        <xs:complexType name="Base">
            <xs:attribute name="id" type="xs:int" use="required"/>
        </xs:complexType>
        <xs:complexType name="Message">
            <xs:complexContent>
                <xs:extension base="t:Base">
                    <xs:sequence>
                        <xs:element name="text" type="xs:string"/>
                        <xs:element name="count" type="xs:int" minOccurs="0"/>
                    </xs:sequence>
                </xs:extension>
            </xs:complexContent>
        </xs:complexType>
        <xs:complexType name="Measurement">
            <xs:simpleContent>
                <xs:extension base="xs:decimal">
                    <xs:attribute name="unit" type="xs:string"/>
                </xs:extension>
            </xs:simpleContent>
        </xs:complexType>
        <xs:complexType name="PreciseMeasurement">
            <xs:simpleContent>
                <xs:extension base="t:Measurement">
                    <xs:attribute name="precision" type="xs:int"/>
                </xs:extension>
            </xs:simpleContent>
        </xs:complexType>
    </xs:schema>"#
}

#[test]
fn test_java_plain_record_inlines_base_fields() {
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(extension_schema())
        .expect("parse extension schema");
    // Defaults: use_records=true with no builder and no direct codec — the
    // mode that used to bypass the base walk entirely.
    let generator = JavaCodegen::new(JavaOptions {
        package_name: String::new(),
        ..Default::default()
    });
    let models = generator.generate_module(&ir, "Models");

    let message = record_header(&models, "Message");
    assert!(
        message.contains("int id"),
        "record Message dropped the inherited id attribute:\n{message}"
    );
    assert!(
        message.contains("String text"),
        "own text missing:\n{message}"
    );
    assert!(message.contains("count"), "own count missing:\n{message}");
    let id_at = message.find("int id").expect("id");
    let text_at = message.find("String text").expect("text");
    assert!(
        id_at < text_at,
        "base components must precede derived ones:\n{message}"
    );

    // simpleContent chain: `unit` inherited, `value` present exactly once —
    // a suffixed duplicate would still match the " value" prefix below.
    let derived = record_header(&models, "PreciseMeasurement");
    assert!(
        derived.contains("unit"),
        "record PreciseMeasurement dropped the inherited unit attribute:\n{derived}"
    );
    assert!(
        derived.contains("precision"),
        "own precision missing:\n{derived}"
    );
    assert_eq!(
        derived.matches(" value").count(),
        1,
        "`value` must be shadowed by the derived declaration, not duplicated:\n{derived}"
    );
}

#[test]
fn test_java_plain_record_extension_javac_execute() {
    let ir = polyxml::schema_parser::XsdParser::new()
        .parse_str(extension_schema())
        .expect("parse extension schema");
    let generator = JavaCodegen::new(JavaOptions {
        package_name: String::new(),
        ..Default::default()
    });
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Models.java"),
        generator.generate_module(&ir, "Models"),
    )
    .unwrap();
    fs::write(
        dir.path().join("Main.java"),
        r#"import java.util.Optional;
public class Main {
    public static void main(String[] args) {
        var m = new Models.Message(5, "hello", Optional.of(3));
        if (m.id() != 5) throw new AssertionError("inherited id attribute lost");
        if (!m.text().equals("hello")) throw new AssertionError("own text lost");
        if (m.count().orElse(-1) != 3) throw new AssertionError("own count lost");
        var p = new Models.PreciseMeasurement(
            Optional.of("km"),
            new Models.Measurement(new java.math.BigDecimal("2.5"), Optional.empty()),
            Optional.of(3));
        if (!p.unit().orElse("").equals("km")) throw new AssertionError("inherited unit lost");
        if (p.precision().orElse(-1) != 3) throw new AssertionError("own precision lost");
    }
}"#,
    )
    .unwrap();
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }
    let result = Command::new("javac")
        .current_dir(dir.path())
        .args(["Models.java", "Main.java"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new("java")
        .current_dir(dir.path())
        .args(["-cp", ".", "Main"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn jackson3_preserves_core_annotations_and_maps_record_text() {
    use polyxml::schema_parser::XsdParser;
    let ir = XsdParser::new()
        .parse_str(include_str!("../../../tests/java-spring/schema.xsd"))
        .unwrap();
    for records in [false, true] {
        let sources = JavaCodegen::new(JavaOptions {
            backend: JavaBackend::Jackson3,
            use_records: records,
            emit_builder: true,
            bean_validation: true,
            ..Default::default()
        })
        .generate_files(&ir);
        let amount = sources
            .iter()
            .find(|(name, _)| name == "Amount.java")
            .unwrap()
            .1
            .as_str();
        assert!(amount.contains("import com.fasterxml.jackson.annotation.*;"));
        assert!(amount.contains("import tools.jackson.dataformat.xml.annotation.*;"));
        assert!(!amount.contains("import com.fasterxml.jackson.dataformat"));
        assert!(amount.contains("@JacksonXmlText"));
        assert!(amount.contains("@NotNull"));
        assert!(amount.contains("AmountBuilder"));
        assert!(amount.contains("@JsonPropertyOrder({\"value\", \"currency\"})"));
    }
    for alias in [
        "jackson3",
        "jackson-3",
        "jackson_3",
        "spring-boot-4",
        " JACKSON3 ",
    ] {
        assert_eq!(
            JavaBackend::from_str_loose(alias),
            Some(JavaBackend::Jackson3)
        );
    }
    assert_eq!(
        JavaBackend::from_str_loose("spring-boot"),
        Some(JavaBackend::Jackson)
    );
}
