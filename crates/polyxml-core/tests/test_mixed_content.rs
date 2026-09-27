use polyxml::codegen::cpp::{CppCodegen, CppOptions};
use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions};
use polyxml::codegen::go::{GoCodegen, GoOptions};
use polyxml::codegen::java::{JavaCodegen, JavaOptions};
use polyxml::codegen::python::{PythonCodegen, PythonOptions};
use polyxml::codegen::rust::{RustCodegen, RustOptions};
use polyxml::codegen::typescript::{TypeScriptCodegen, TypeScriptOptions};
use polyxml::ir::{FieldKind, QName, TypeDef, TypeRef};
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize_with_options, ModelSchema, PolyValue};
use std::sync::Arc;

#[test]
fn parser_builds_ordered_mixed_content_union() {
    let ir = XsdParser::new()
        .parse_str(include_str!("fixtures/mixed_content.xsd"))
        .unwrap();
    let TypeDef::Struct(rich_text) = &ir.types[&QName::new(Some("urn:polyxml:mixed"), "RichText")]
    else {
        panic!("RichText must be a struct")
    };
    assert!(rich_text.is_mixed);
    assert_eq!(rich_text.fields.len(), 2);
    assert_eq!(rich_text.fields[0].kind, FieldKind::Attribute);
    let items = &rich_text.fields[1];
    assert_eq!(items.name, "items");
    let TypeRef::Named(item_type) = &items.type_ref else {
        panic!("items must be named")
    };
    let TypeDef::Union(union) = &ir.types[item_type] else {
        panic!("items must be a union")
    };
    assert!(union.is_mixed_content());
    assert_eq!(
        union
            .branches
            .iter()
            .map(|branch| branch.xml_name.as_str())
            .collect::<Vec<_>>(),
        ["#text", "b", "i", "card"]
    );
}

#[test]
fn mixed_content_preserves_nested_child_order() {
    let ir = XsdParser::new()
        .parse_str(include_str!("fixtures/mixed_content.xsd"))
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("description")).unwrap();
    let xml = br#"<description>open <card code="c1"><title>One</title></card> close</description>"#;
    let value = deserialize(xml, Arc::clone(&schema)).unwrap();
    let encoded =
        serialize_with_options("description", &value, &schema, None, Some(false), None).unwrap();
    assert_eq!(encoded, xml);
    assert_eq!(deserialize(&encoded, schema).unwrap(), value);
}

#[test]
fn generated_models_keep_typed_content_items_in_all_targets() {
    let ir = XsdParser::new()
        .parse_str(include_str!("fixtures/mixed_content.xsd"))
        .unwrap();
    let rust = RustCodegen::new(RustOptions::default()).generate_module(&ir);
    assert!(rust.contains("pub enum RichTextItem"));
    assert!(rust.contains("pub items: Vec<RichTextItem"));
    assert!(rust.contains("writer.write_event(Event::Text(BytesText::new(val.as_ref())))"));

    let python = PythonCodegen::new(PythonOptions::default()).generate_module(&ir);
    assert!(python.contains("class RichTextItem:"));
    assert!(python.contains("mixed_branches = ("));
    assert!(python.contains("items: list[RichTextItem]"));

    let cpp = CppCodegen::new(CppOptions::default()).generate_module(&ir);
    assert!(cpp.contains("using RichTextItem = std::variant<"));
    assert!(cpp.contains("std::vector<RichTextItem> items"));

    let ts = TypeScriptCodegen::new(TypeScriptOptions::default()).generate_module(&ir);
    assert!(ts.contains("export type RichTextItem ="));
    assert!(ts.contains("items: RichTextItem[]"));

    let go = GoCodegen::new(GoOptions::default()).generate_module(&ir);
    assert!(go.contains("Items []RichTextItem"));
    assert!(go.contains("func (v RichText) MarshalXML"));
    assert!(go.contains("xml.CharData(*item.Text)"));

    let csharp = CSharpCodegen::new(CSharpOptions::default()).generate_module(&ir);
    assert!(csharp.contains("List<RichTextItem> Items"));
    assert!(csharp.contains("System.Xml.Serialization.IXmlSerializable"));
    assert!(csharp.contains("writer.WriteString(text.Value)"));

    let java = JavaCodegen::new(JavaOptions::default()).generate_module(&ir, "Models");
    assert!(java.contains("sealed interface RichTextItem"));
    assert!(java.contains("java.util.List<RichTextItem> items"));
}

#[test]
fn mixed_items_field_avoids_existing_attribute_name() {
    let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:complexType name="Note" mixed="true"><xs:attribute name="items" type="xs:string"/></xs:complexType>
      <xs:element name="note" type="Note"/>
    </xs:schema>"#;
    let ir = XsdParser::new().parse_str(xsd).unwrap();
    let TypeDef::Struct(note) = &ir.types[&QName::new(None::<&str>, "Note")] else {
        panic!("expected Note")
    };
    assert_eq!(note.fields[0].name, "items");
    assert_eq!(note.fields[1].name, "items_2");
    let schema = ModelSchema::from_ir(&ir, Some("note")).unwrap();
    let xml = b"<note items=\"label\">text</note>";
    let value = deserialize(xml, Arc::clone(&schema)).unwrap();
    let encoded = serialize_with_options("note", &value, &schema, None, Some(false), None).unwrap();
    assert_eq!(encoded, xml);
}

#[test]
fn mixed_content_preserves_text_and_child_order() {
    let ir = XsdParser::new()
        .parse_str(include_str!("fixtures/mixed_content.xsd"))
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("description")).unwrap();
    let xml =
        br#"<description lang="en">before <b>bold</b> between <i>italic</i> after</description>"#;
    let value = deserialize(xml, Arc::clone(&schema)).unwrap();
    let PolyValue::Record { values, .. } = &value else {
        panic!("expected record")
    };
    let Some(PolyValue::List(items)) = &values[1] else {
        panic!("expected ordered items")
    };
    assert_eq!(items.len(), 5);
    let kinds: Vec<_> = items
        .iter()
        .map(|item| {
            let PolyValue::Object(tagged) = item else {
                panic!("expected tagged item")
            };
            tagged["kind"].as_str().unwrap()
        })
        .collect();
    assert_eq!(kinds, ["#text", "b", "#text", "i", "#text"]);
    let encoded =
        serialize_with_options("description", &value, &schema, None, Some(false), None).unwrap();
    assert_eq!(
        String::from_utf8(encoded.clone()).unwrap(),
        std::str::from_utf8(xml).unwrap()
    );
    assert_eq!(deserialize(&encoded, schema).unwrap(), value);

    let indented = serialize_with_options(
        "description",
        &value,
        &ModelSchema::from_ir(&ir, Some("description")).unwrap(),
        Some(2),
        Some(false),
        None,
    )
    .unwrap();
    assert_eq!(indented, xml);
}
