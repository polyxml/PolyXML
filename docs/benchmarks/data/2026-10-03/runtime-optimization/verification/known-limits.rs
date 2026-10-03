use polyxml::schema::ModelSchema;
use polyxml::schema_parser::XsdParser;
use polyxml::{deserialize, serialize};
use std::sync::Arc;

fn main() {
    let ir = XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:simpleType name="StateType"><xs:restriction base="xs:string"><xs:enumeration value="R&amp;D"/><xs:enumeration value="Ready"/></xs:restriction></xs:simpleType>
      <xs:element name="Root" type="StateType"/>
    </xs:schema>"#).unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let error = deserialize(b"<Root>R&amp;D</Root>", schema).unwrap_err();
    println!("escaped XSD enumeration limitation: {error:?}");

    let ir = XsdParser::new().parse_str(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
      <xs:element name="Root"><xs:complexType mixed="true"><xs:choice minOccurs="0" maxOccurs="unbounded">
        <xs:element name="Text" type="xs:string" nillable="true"/>
        <xs:element name="State" type="xs:string"/>
      </xs:choice></xs:complexType></xs:element>
    </xs:schema>"#).unwrap();
    let schema = ModelSchema::from_ir(&ir, Some("Root")).unwrap();
    let value = deserialize(b"<Root xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance'><Text xsi:nil='true'/><State>Ready</State></Root>", Arc::clone(&schema)).unwrap();
    let error = serialize("Root", &value, &schema, None).unwrap_err();
    println!("nil mixed scalar serialization limitation: {error:?}");
}
