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
