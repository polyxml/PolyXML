use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize};
use std::sync::Arc;

#[test]
fn fixed_constraints_cover_input_and_output_in_value_space() {
    let ir = XsdParser::new()
        .parse_str(include_str!("../../../research/fixtures/fixed_values.xsd"))
        .unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    for xml in [
        "<Root><Code>0013</Code><Flag>1</Flag></Root>",
        "<Root><Code/><Flag/></Root>",
    ] {
        let value = deserialize(xml.as_bytes(), Arc::clone(&schema)).unwrap();
        let output = serialize("Root", &value, &schema, None).unwrap();
        deserialize(&output, Arc::clone(&schema)).unwrap();
    }
    for xml in [
        "<Root><Code>wrong</Code></Root>",
        "<Root mode='wrong'><Code>0013</Code></Root>",
        "<Root><Code>0013</Code><Flag>false</Flag></Root>",
    ] {
        assert!(deserialize(xml.as_bytes(), Arc::clone(&schema))
            .unwrap_err()
            .to_string()
            .contains("Fixed value constraint"));
    }
}

#[test]
fn repeated_particle_cardinality_belongs_to_the_group() {
    let ir = XsdParser::new()
        .parse_str(include_str!(
            "../../../research/fixtures/repeated_sequence.xsd"
        ))
        .unwrap();
    let model = &ir.content_models[&polyxml::ir::QName::local("RootType")];
    assert!(model.has_repeated_sequence());
    assert!(model.pattern().contains("{0,2}"));
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let group = "<First>a</First><Second>b</Second>";
    for count in 0..=2 {
        let value = deserialize(
            format!("<Root>{}</Root>", group.repeat(count)).as_bytes(),
            Arc::clone(&schema),
        )
        .unwrap();
        serialize("Root", &value, &schema, None).unwrap();
    }
    for content in [
        group.repeat(3),
        "<First>a</First>".into(),
        "<First>a</First><First>b</First><Second>c</Second><Second>d</Second>".into(),
    ] {
        assert!(deserialize(
            format!("<Root>{content}</Root>").as_bytes(),
            Arc::clone(&schema)
        )
        .is_err());
    }
}
