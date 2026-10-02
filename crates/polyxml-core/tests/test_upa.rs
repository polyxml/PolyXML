use polyxml::schema_parser::XsdParser;

fn schema(content: &str) -> String {
    format!(
        r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="Root"><xs:complexType>{content}</xs:complexType></xs:element></xs:schema>"#
    )
}
fn check(content: &str, valid: bool) {
    let result = XsdParser::new().parse_str(&schema(content));
    if valid {
        assert!(result.is_ok(), "{content}: {result:?}");
    } else {
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("Unique Particle Attribution (UPA)"),
            "{content}: {error}"
        );
    }
}

#[test]
fn optional_siblings_and_choice_lookahead_violate_upa() {
    let repro = include_str!("../../../research/fixtures/wave8/30_upa_violation.xsd");
    assert!(XsdParser::new()
        .parse_str(repro)
        .unwrap_err()
        .to_string()
        .contains("UPA"));
    check(
        r#"<xs:choice><xs:element name="a" type="xs:string"/><xs:element name="a" type="xs:string"/></xs:choice>"#,
        false,
    );
    check(
        r#"<xs:choice><xs:sequence><xs:element name="a" type="xs:string"/><xs:element name="b" type="xs:string"/></xs:sequence><xs:sequence><xs:element name="a" type="xs:string"/><xs:element name="c" type="xs:string"/></xs:sequence></xs:choice>"#,
        false,
    );
    check(
        r#"<xs:sequence><xs:element name="a" type="xs:string" minOccurs="0"/><xs:element name="separator" type="xs:string" minOccurs="0"/><xs:element name="a" type="xs:string"/></xs:sequence>"#,
        false,
    );
    check(
        r#"<xs:sequence><xs:element name="a" type="xs:string" minOccurs="0"/><xs:element name="separator" type="xs:string"/><xs:element name="a" type="xs:string"/></xs:sequence>"#,
        true,
    );
}

#[test]
fn occurrence_ranges_keep_particle_identity_and_count_boundaries() {
    for (range, valid) in [
        ("", true),
        (r#"minOccurs="2" maxOccurs="2""#, true),
        (r#"minOccurs="1000000" maxOccurs="1000000""#, true),
        (r#"minOccurs="2" maxOccurs="3""#, false),
        (r#"minOccurs="0" maxOccurs="0""#, true),
        (r#"minOccurs="0" maxOccurs="unbounded""#, false),
    ] {
        check(
            &format!(
                r#"<xs:sequence><xs:element name="a" type="xs:string" {range}/><xs:element name="a" type="xs:string"/></xs:sequence>"#
            ),
            valid,
        );
    }
    check(
        r#"<xs:sequence><xs:element name="a" type="xs:string" minOccurs="0" maxOccurs="unbounded"/></xs:sequence>"#,
        true,
    );
    for (range, valid) in [
        (r#"minOccurs="2" maxOccurs="2""#, true),
        (r#"minOccurs="0" maxOccurs="unbounded""#, false),
    ] {
        check(
            &format!(
                r#"<xs:sequence><xs:sequence {range}><xs:element name="a" type="xs:string"/><xs:element name="b" type="xs:string"/></xs:sequence><xs:element name="a" type="xs:string"/></xs:sequence>"#
            ),
            valid,
        );
    }
    check(
        r#"<xs:sequence><xs:sequence minOccurs="0" maxOccurs="unbounded"><xs:element name="a" type="xs:string"/><xs:element name="b" type="xs:string" minOccurs="0"/></xs:sequence><xs:element name="b" type="xs:string"/></xs:sequence>"#,
        false,
    );
}

#[test]
fn all_groups_and_wildcard_namespaces_are_checked() {
    check(
        r#"<xs:all><xs:element name="a" type="xs:string"/><xs:element name="a" type="xs:string"/></xs:all>"#,
        false,
    );
    let many = (0..32)
        .map(|i| format!(r#"<xs:element name="a{i}" type="xs:string"/>"#))
        .collect::<String>();
    check(&format!("<xs:all>{many}</xs:all>"), true);
    check(
        r#"<xs:choice><xs:any processContents="lax"/><xs:element name="a" type="xs:string"/></xs:choice>"#,
        false,
    );
    check(
        r#"<xs:choice><xs:any namespace="urn:a" processContents="lax"/><xs:any namespace="urn:a urn:b" processContents="lax"/></xs:choice>"#,
        false,
    );
    check(
        r#"<xs:choice><xs:any namespace="urn:a" processContents="lax"/><xs:any namespace="urn:b" processContents="lax"/></xs:choice>"#,
        true,
    );
    check(
        r###"<xs:choice><xs:any namespace="##other" processContents="lax"/><xs:element name="a" type="xs:string"/></xs:choice>"###,
        true,
    );
}

#[test]
fn local_form_and_in_scope_qnames_distinguish_namespaces() {
    let source = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:root" elementFormDefault="qualified"><xs:element name="Root"><xs:complexType><xs:choice><xs:element name="a" type="xs:string"/><xs:element name="a" type="xs:string" form="unqualified"/></xs:choice></xs:complexType></xs:element></xs:schema>"#;
    assert!(XsdParser::new().parse_str(source).is_ok());
    let ambiguous = source.replace("form=\"unqualified\"", "form=\"qualified\"");
    assert!(XsdParser::new()
        .parse_str(&ambiguous)
        .unwrap_err()
        .to_string()
        .contains("UPA"));
}

#[test]
fn model_groups_derivation_and_substitution_members_are_checked() {
    let template = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:group name="G"><xs:sequence><xs:element name="a" type="xs:string" minOccurs="0"/></xs:sequence></xs:group><xs:element name="Root"><xs:complexType><xs:sequence><xs:group ref="G"/><xs:element name="a" type="xs:string"/></xs:sequence></xs:complexType></xs:element></xs:schema>"#;
    assert!(XsdParser::new()
        .parse_str(template)
        .unwrap_err()
        .to_string()
        .contains("UPA"));
    assert!(XsdParser::new()
        .parse_str(&template.replace("minOccurs=\"0\"", "minOccurs=\"1\""))
        .is_ok());
    for (minimum, valid) in [(0, false), (1, true)] {
        let source = format!(
            r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:complexType name="Base"><xs:sequence><xs:element name="a" type="xs:string" minOccurs="{minimum}"/></xs:sequence></xs:complexType><xs:complexType name="Derived"><xs:complexContent><xs:extension base="Base"><xs:sequence><xs:element name="a" type="xs:string"/></xs:sequence></xs:extension></xs:complexContent></xs:complexType><xs:element name="Root" type="Derived"/></xs:schema>"#
        );
        let result = XsdParser::new().parse_str(&source);
        if valid {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(result.unwrap_err().to_string().contains("UPA"));
        }
    }
    let source = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="head" type="xs:string" abstract="true"/><xs:element name="member" type="xs:string" substitutionGroup="head"/><xs:element name="Root"><xs:complexType><xs:choice><xs:element ref="head"/><xs:element ref="member"/></xs:choice></xs:complexType></xs:element></xs:schema>"#;
    assert!(XsdParser::new()
        .parse_str(source)
        .unwrap_err()
        .to_string()
        .contains("UPA"));
}

#[test]
fn imported_groups_and_chameleon_wildcards_keep_source_namespaces() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("base.xsd"),r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:base" elementFormDefault="qualified"><xs:element name="a" type="xs:string"/><xs:group name="G"><xs:sequence><xs:element name="a" type="xs:string" minOccurs="0"/></xs:sequence></xs:group></xs:schema>"#).unwrap();
    std::fs::write(dir.path().join("root.xsd"),r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:b="urn:base"><xs:import namespace="urn:base" schemaLocation="base.xsd"/><xs:element name="Root"><xs:complexType><xs:sequence><xs:group ref="b:G"/><xs:element ref="b:a"/></xs:sequence></xs:complexType></xs:element></xs:schema>"#).unwrap();
    assert!(XsdParser::new()
        .parse_file(dir.path().join("root.xsd"))
        .unwrap_err()
        .to_string()
        .contains("UPA"));
    for (form, valid) in [("qualified", false), ("unqualified", true)] {
        std::fs::write(dir.path().join("included.xsd"),format!(r###"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:group name="G"><xs:choice><xs:element name="a" type="xs:string" form="{form}"/><xs:any namespace="##targetNamespace" processContents="lax"/></xs:choice></xs:group></xs:schema>"###)).unwrap();
        let mut parser = XsdParser::new();
        for ns in ["urn:one", "urn:two"] {
            std::fs::write(dir.path().join("root.xsd"),format!(r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="{ns}" xmlns:t="{ns}"><xs:include schemaLocation="included.xsd"/><xs:element name="Root"><xs:complexType><xs:group ref="t:G"/></xs:complexType></xs:element></xs:schema>"#)).unwrap();
            // A distinct path exercises cached chameleon include re-keying.
            let path = dir.path().join(if ns == "urn:one" {
                "one.xsd"
            } else {
                "two.xsd"
            });
            std::fs::copy(dir.path().join("root.xsd"), &path).unwrap();
            let result = parser.parse_file(path);
            if valid {
                assert!(result.is_ok(), "{form}: {result:?}");
            } else {
                assert!(result.unwrap_err().to_string().contains("UPA"));
            }
        }
    }
}
