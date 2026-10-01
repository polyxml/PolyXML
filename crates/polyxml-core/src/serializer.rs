use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Cursor;

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use crate::converters::ValueConverter;
use crate::error::{PolyXmlError, Result};
use crate::schema::{FieldKind, ModelSchema, ScalarType, ValueType};
use crate::value::PolyValue;

/// XML Schema Instance namespace, needed to write `xsi:type` selectors.
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

#[derive(Debug, Clone)]
pub struct NamespaceContext {
    pub uri_to_prefix: HashMap<String, String>,
    pub prefix_to_uri: Vec<(String, String)>,
}

impl NamespaceContext {
    pub fn build(
        root_schema: &ModelSchema,
        user_ns_map: Option<&HashMap<String, String>>,
    ) -> Option<Self> {
        let mut uris = Vec::new();
        Self::collect_namespaces(root_schema, &mut uris);

        if uris.is_empty() && user_ns_map.is_none_or(|m| m.is_empty()) {
            return None;
        }

        let mut uri_to_prefix = HashMap::new();
        let mut prefix_to_uri = Vec::new();

        // 1. Process user ns_map
        if let Some(map) = user_ns_map {
            for (k, v) in map {
                let (prefix, uri) = if v.contains("://") || v.starts_with("urn:") {
                    (k.clone(), v.clone())
                } else if k.contains("://") || k.starts_with("urn:") {
                    (v.clone(), k.clone())
                } else {
                    (k.clone(), v.clone())
                };

                let clean_prefix = if prefix == "None" {
                    String::new()
                } else {
                    prefix
                };
                if !uri.is_empty() && !uri_to_prefix.contains_key(&uri) {
                    uri_to_prefix.insert(uri.clone(), clean_prefix.clone());
                    prefix_to_uri.push((clean_prefix, uri));
                }
            }
        }

        // 2. Assign prefixes for remaining schema URIs
        let mut ns_counter = 0;
        for uri in uris {
            if !uri_to_prefix.contains_key(&uri) {
                let prefix = match uri.as_str() {
                    "http://www.w3.org/2001/XMLSchema-instance" => "xsi".to_string(),
                    "http://www.w3.org/2001/XMLSchema" => "xs".to_string(),
                    "http://www.w3.org/XML/1998/namespace" => "xml".to_string(),
                    _ => loop {
                        let candidate = format!("ns{}", ns_counter);
                        ns_counter += 1;
                        if !prefix_to_uri.iter().any(|(p, _)| p == &candidate) {
                            break candidate;
                        }
                    },
                };

                uri_to_prefix.insert(uri.clone(), prefix.clone());
                prefix_to_uri.push((prefix, uri));
            }
        }

        Some(Self {
            uri_to_prefix,
            prefix_to_uri,
        })
    }

    fn collect_namespaces(schema: &ModelSchema, uris: &mut Vec<String>) {
        if let Some(ref ns) = schema.namespace {
            if !ns.is_empty() && !uris.contains(ns) {
                uris.push(ns.clone());
            }
        }
        for field in &schema.fields {
            if let Some(ref ns) = field.namespace {
                if !ns.is_empty() && !uris.contains(ns) {
                    uris.push(ns.clone());
                }
            }
            match &field.val_type {
                ValueType::Nested(nested) => Self::collect_namespaces(nested, uris),
                ValueType::List(inner) => {
                    if let ValueType::Nested(nested) = inner.as_ref() {
                        Self::collect_namespaces(nested, uris);
                    }
                }
                _ => {}
            }
        }
        if let Some(mixed) = &schema.mixed_content {
            for branch in &mixed.branches {
                if let Some(ns) = &branch.namespace {
                    if !ns.is_empty() && !uris.contains(ns) {
                        uris.push(ns.clone());
                    }
                }
                if let ValueType::Nested(nested) = &branch.val_type {
                    Self::collect_namespaces(nested, uris);
                }
            }
        }

        // xsi:type dispatch: variant namespaces must be in
        // scope, and the XML Schema Instance namespace is required to
        // write the xsi:type selector itself.
        let variants = schema.variants();
        if !variants.is_empty() {
            if !uris.iter().any(|u| u == XSI_NS) {
                uris.push(XSI_NS.to_string());
            }
            for variant in variants {
                Self::collect_namespaces(&variant, uris);
            }
        }
    }

    pub fn qualify_element<'a>(&'a self, local_name: &'a str, ns: Option<&str>) -> Cow<'a, str> {
        let clean_name = if let Some(pos) = local_name.find(':') {
            &local_name[pos + 1..]
        } else {
            local_name
        };
        if let Some(ns_uri) = ns {
            if let Some(prefix) = self.uri_to_prefix.get(ns_uri) {
                if !prefix.is_empty() {
                    return Cow::Owned(format!("{}:{}", prefix, clean_name));
                }
                return Cow::Borrowed(clean_name);
            }
        }
        Cow::Borrowed(local_name)
    }

    pub fn qualify_attribute<'a>(&'a self, local_name: &'a str, ns: Option<&str>) -> Cow<'a, str> {
        let clean_name = if let Some(pos) = local_name.find(':') {
            &local_name[pos + 1..]
        } else {
            local_name
        };
        if let Some(ns_uri) = ns {
            if let Some(prefix) = self.uri_to_prefix.get(ns_uri) {
                if !prefix.is_empty() {
                    return Cow::Owned(format!("{}:{}", prefix, clean_name));
                }
                return Cow::Borrowed(clean_name);
            }
        }
        Cow::Borrowed(local_name)
    }
}

pub struct XmlSerializer;

impl XmlSerializer {
    pub fn serialize(
        root_name: &str,
        value: &PolyValue,
        schema: &ModelSchema,
        indent: Option<usize>,
    ) -> Result<Vec<u8>> {
        Self::serialize_with_options(root_name, value, schema, indent, None, None)
    }

    pub fn serialize_with_options(
        root_name: &str,
        value: &PolyValue,
        schema: &ModelSchema,
        indent: Option<usize>,
        enable_namespaces: Option<bool>,
        ns_map: Option<&HashMap<String, String>>,
    ) -> Result<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::with_capacity(512));
        let mut writer = match indent {
            Some(spaces) => Writer::new_with_indent(&mut buffer, b' ', spaces),
            None => Writer::new(&mut buffer),
        };

        let ns_ctx = match enable_namespaces {
            Some(false) => None,
            _ => NamespaceContext::build(schema, ns_map),
        };

        Self::write_model(
            &mut writer,
            root_name.as_bytes(),
            value,
            schema,
            ns_ctx.as_ref(),
            true,
            schema.namespace.as_deref(),
        )?;

        Ok(buffer.into_inner())
    }

    fn format_scalar_to<'a>(
        val: &'a PolyValue,
        buf: &'a mut [u8; lexical_core::BUFFER_SIZE],
    ) -> Option<&'a str> {
        match val {
            PolyValue::String(s) => Some(s.as_str()),
            PolyValue::Int(i) => {
                let bytes = lexical_core::write(*i, buf);
                std::str::from_utf8(bytes).ok()
            }
            PolyValue::Float(f) => {
                let bytes = lexical_core::write(*f, buf);
                std::str::from_utf8(bytes).ok()
            }
            PolyValue::Bool(b) => Some(if *b { "true" } else { "false" }),
            _ => None,
        }
    }

    fn validate_scalar(ty: &ValueType, text: &str, field_name: &str) -> Result<()> {
        if let ValueType::Scalar(
            scalar @ (ScalarType::XmlDate
            | ScalarType::XmlGregorian(_)
            | ScalarType::Enum(_)
            | ScalarType::Pattern(_, _)
            | ScalarType::Union(_)),
        ) = ty
        {
            ValueConverter::parse_scalar(scalar, text.as_bytes(), field_name)?;
        }
        Ok(())
    }

    fn write_model<W: std::io::Write>(
        writer: &mut Writer<W>,
        tag_name: &[u8],
        value: &PolyValue,
        schema: &ModelSchema,
        ns_ctx: Option<&NamespaceContext>,
        is_root: bool,
        element_ns: Option<&str>,
    ) -> Result<()> {
        match value {
            PolyValue::Record { .. } | PolyValue::Object(_) => {}
            _ => {
                return Err(PolyXmlError::SerializationError(
                    "Expected Object or Record value for model".into(),
                ));
            }
        }

        // xsi:type dispatch: when the value is a Record built
        // from a registered derivation of the declared schema, write the
        // concrete variant's fields and re-emit the xsi:type selector.
        let mut xsi_type: Option<String> = None;
        let schema = if let PolyValue::Record { schema: rec, .. } = value {
            if schema.matches_variant(rec) {
                xsi_type = match ns_ctx {
                    Some(ctx) => {
                        let name = std::str::from_utf8(&rec.xml_name)?;
                        Some(
                            ctx.qualify_element(name, rec.namespace.as_deref())
                                .into_owned(),
                        )
                    }
                    None => Some(String::from_utf8_lossy(&rec.xml_name).into_owned()),
                };
                rec.as_ref()
            } else {
                schema
            }
        } else {
            schema
        };

        let get_field = |idx: usize, name: &str| -> Option<&PolyValue> {
            match value {
                PolyValue::Record { values, .. } => values.get(idx).and_then(|v| v.as_ref()),
                PolyValue::Object(o) => o.get(name),
                _ => None,
            }
        };

        let local_tag = std::str::from_utf8(tag_name)?;
        let qualified_tag = if let Some(ctx) = ns_ctx {
            ctx.qualify_element(local_tag, element_ns)
        } else {
            Cow::Borrowed(local_tag)
        };

        let mut elem = BytesStart::new(qualified_tag.as_ref());

        // Emit xmlns declarations on root element
        if is_root {
            if let Some(ctx) = ns_ctx {
                for (prefix, uri) in &ctx.prefix_to_uri {
                    if prefix.is_empty() {
                        elem.push_attribute(("xmlns", uri.as_str()));
                    } else {
                        let attr_name = format!("xmlns:{}", prefix);
                        elem.push_attribute((attr_name.as_str(), uri.as_str()));
                    }
                }
            }
        }

        // 1. Collect and write attributes
        for (idx, field) in schema.fields.iter().enumerate() {
            if field.kind == FieldKind::Attribute {
                if let Some(val) = get_field(idx, &field.name) {
                    if !val.is_null() {
                        let local_attr = std::str::from_utf8(&field.xml_name)?;
                        let attr_name = if let Some(ctx) = ns_ctx {
                            ctx.qualify_attribute(local_attr, field.namespace.as_deref())
                        } else {
                            Cow::Borrowed(local_attr)
                        };
                        let mut buf = [0u8; lexical_core::BUFFER_SIZE];
                        if let Some(attr_str) = Self::format_scalar_to(val, &mut buf) {
                            Self::validate_scalar(&field.val_type, attr_str, &field.name)?;
                            elem.push_attribute((attr_name.as_ref(), attr_str));
                        }
                    }
                }
            } else if field.kind == FieldKind::AnyAttribute {
                if let Some(PolyValue::Object(map)) = get_field(idx, &field.name) {
                    let mut sorted_keys: Vec<_> = map.keys().collect();
                    sorted_keys.sort();
                    for k in sorted_keys {
                        if let Some(v) = map.get(k) {
                            let mut buf = [0u8; lexical_core::BUFFER_SIZE];
                            if let Some(attr_str) = Self::format_scalar_to(v, &mut buf) {
                                elem.push_attribute((k.as_str(), attr_str));
                            }
                        }
                    }
                }
            }
        }

        // Re-emit the xsi:type selector after content attributes.
        if let Some(ref xsi_val) = xsi_type {
            let attr_key = match ns_ctx {
                Some(ctx) => {
                    let key = ctx.qualify_attribute("type", Some(XSI_NS));
                    if key.as_ref() == "type" {
                        return Err(PolyXmlError::SerializationError(
                            "xsi:type requires a namespace prefix".into(),
                        ));
                    }
                    key.into_owned()
                }
                None => {
                    return Err(PolyXmlError::SerializationError(
                        "xsi:type requires namespaces to be enabled".into(),
                    ))
                }
            };
            elem.push_attribute((attr_key.as_str(), xsi_val.as_str()));
        }

        writer
            .write_event(Event::Start(elem))
            .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;

        // 2. Write text content if present
        if let Some(text_idx) = schema.text_field {
            let field = &schema.fields[text_idx];
            if let Some(val) = get_field(text_idx, &field.name) {
                let mut buf = [0u8; lexical_core::BUFFER_SIZE];
                if let Some(text_content) = Self::format_scalar_to(val, &mut buf) {
                    Self::validate_scalar(&field.val_type, text_content, &field.name)?;
                    if !text_content.is_empty() {
                        writer
                            .write_event(Event::Text(BytesText::new(text_content)))
                            .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;
                    }
                } else if let PolyValue::List(items) = val {
                    for item in items {
                        let mut item_buf = [0u8; lexical_core::BUFFER_SIZE];
                        if let Some(text_content) = Self::format_scalar_to(item, &mut item_buf) {
                            if let ValueType::List(inner) = &field.val_type {
                                Self::validate_scalar(inner, text_content, &field.name)?;
                            }
                            if !text_content.is_empty() {
                                writer
                                    .write_event(Event::Text(BytesText::new(text_content)))
                                    .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;
                            }
                        }
                    }
                }
            }
        }

        // 3. Write child elements in their original order for mixed content.
        if let Some(mixed) = &schema.mixed_content {
            if let Some(PolyValue::List(items)) =
                get_field(mixed.items_index, &schema.fields[mixed.items_index].name)
            {
                for item in items {
                    let tagged_field = |name: &str| -> Option<&PolyValue> {
                        match item {
                            PolyValue::Object(tagged) => tagged.get(name),
                            PolyValue::Record { schema, values } => schema
                                .fields
                                .iter()
                                .position(|field| field.name == name)
                                .and_then(|idx| values.get(idx))
                                .and_then(Option::as_ref),
                            _ => None,
                        }
                    };
                    let Some(kind) = tagged_field("kind").and_then(PolyValue::as_str) else {
                        return Err(PolyXmlError::SerializationError(
                            "Mixed content item has no kind".into(),
                        ));
                    };
                    let Some(content) = tagged_field("value") else {
                        return Err(PolyXmlError::SerializationError(
                            "Mixed content item has no value".into(),
                        ));
                    };
                    if kind == "#text" {
                        let Some(text) = content.as_str() else {
                            return Err(PolyXmlError::SerializationError(
                                "Mixed text item must be a string".into(),
                            ));
                        };
                        writer
                            .write_event(Event::Text(BytesText::new(text)))
                            .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;
                        continue;
                    }
                    let Some(branch) = mixed
                        .branches
                        .iter()
                        .find(|branch| branch.variant_name == kind)
                    else {
                        return Err(PolyXmlError::SerializationError(format!(
                            "Unknown mixed content kind: {kind}"
                        )));
                    };
                    let local_name = std::str::from_utf8(&branch.xml_name)?;
                    let qualified = if let Some(ctx) = ns_ctx {
                        ctx.qualify_element(
                            local_name,
                            branch.namespace.as_deref().or(schema.namespace.as_deref()),
                        )
                    } else {
                        Cow::Borrowed(local_name)
                    };
                    match &branch.val_type {
                        ValueType::Scalar(_) => {
                            let mut buf = [0u8; lexical_core::BUFFER_SIZE];
                            let Some(text) = Self::format_scalar_to(content, &mut buf) else {
                                return Err(PolyXmlError::SerializationError(format!(
                                    "Invalid mixed content value for {kind}"
                                )));
                            };
                            Self::validate_scalar(&branch.val_type, text, kind)?;
                            writer
                                .write_event(Event::Start(BytesStart::new(qualified.as_ref())))
                                .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;
                            writer
                                .write_event(Event::Text(BytesText::new(text)))
                                .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;
                            writer
                                .write_event(Event::End(BytesEnd::new(qualified.as_ref())))
                                .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;
                        }
                        ValueType::Nested(nested) => Self::write_model(
                            writer,
                            &branch.xml_name,
                            content,
                            nested,
                            ns_ctx,
                            false,
                            branch.namespace.as_deref().or(nested.namespace.as_deref()),
                        )?,
                        ValueType::List(_) => {
                            return Err(PolyXmlError::SerializationError(
                                "Nested lists are invalid mixed content branches".into(),
                            ))
                        }
                    }
                }
            }
        } else {
            for (idx, field) in schema.fields.iter().enumerate() {
                if field.kind == FieldKind::Element {
                    if let Some(val) = get_field(idx, &field.name) {
                        if val.is_null() {
                            continue;
                        }
                        let child_element_ns =
                            field.namespace.as_deref().or(schema.namespace.as_deref());

                        match &field.val_type {
                            ValueType::Scalar(_) => {
                                let mut buf = [0u8; lexical_core::BUFFER_SIZE];
                                if let Some(text) = Self::format_scalar_to(val, &mut buf) {
                                    Self::validate_scalar(&field.val_type, text, &field.name)?;
                                    let local_child = std::str::from_utf8(&field.xml_name)?;
                                    let child_tag = if let Some(ctx) = ns_ctx {
                                        ctx.qualify_element(local_child, child_element_ns)
                                    } else {
                                        Cow::Borrowed(local_child)
                                    };

                                    writer
                                        .write_event(Event::Start(BytesStart::new(
                                            child_tag.as_ref(),
                                        )))
                                        .map_err(|e| {
                                            PolyXmlError::SerializationError(e.to_string())
                                        })?;
                                    writer
                                        .write_event(Event::Text(BytesText::new(text)))
                                        .map_err(|e| {
                                            PolyXmlError::SerializationError(e.to_string())
                                        })?;
                                    writer
                                        .write_event(Event::End(BytesEnd::new(child_tag.as_ref())))
                                        .map_err(|e| {
                                            PolyXmlError::SerializationError(e.to_string())
                                        })?;
                                }
                            }
                            ValueType::List(inner) => {
                                if let PolyValue::List(items) = val {
                                    for item in items {
                                        match inner.as_ref() {
                                            ValueType::Scalar(_) => {
                                                let mut buf = [0u8; lexical_core::BUFFER_SIZE];
                                                if let Some(text) =
                                                    Self::format_scalar_to(item, &mut buf)
                                                {
                                                    Self::validate_scalar(
                                                        inner,
                                                        text,
                                                        &field.name,
                                                    )?;
                                                    let local_child =
                                                        std::str::from_utf8(&field.xml_name)?;
                                                    let child_tag = if let Some(ctx) = ns_ctx {
                                                        ctx.qualify_element(
                                                            local_child,
                                                            child_element_ns,
                                                        )
                                                    } else {
                                                        Cow::Borrowed(local_child)
                                                    };

                                                    writer
                                                        .write_event(Event::Start(BytesStart::new(
                                                            child_tag.as_ref(),
                                                        )))
                                                        .map_err(|e| {
                                                            PolyXmlError::SerializationError(
                                                                e.to_string(),
                                                            )
                                                        })?;
                                                    writer
                                                        .write_event(Event::Text(BytesText::new(
                                                            text,
                                                        )))
                                                        .map_err(|e| {
                                                            PolyXmlError::SerializationError(
                                                                e.to_string(),
                                                            )
                                                        })?;
                                                    writer
                                                        .write_event(Event::End(BytesEnd::new(
                                                            child_tag.as_ref(),
                                                        )))
                                                        .map_err(|e| {
                                                            PolyXmlError::SerializationError(
                                                                e.to_string(),
                                                            )
                                                        })?;
                                                }
                                            }
                                            ValueType::Nested(nested_schema) => {
                                                let nested_ns = field
                                                    .namespace
                                                    .as_deref()
                                                    .or(nested_schema.namespace.as_deref());
                                                Self::write_model(
                                                    writer,
                                                    &field.xml_name,
                                                    item,
                                                    nested_schema,
                                                    ns_ctx,
                                                    false,
                                                    nested_ns,
                                                )?;
                                            }
                                            ValueType::List(_) => {}
                                        }
                                    }
                                }
                            }
                            ValueType::Nested(nested_schema) => {
                                let nested_ns = field
                                    .namespace
                                    .as_deref()
                                    .or(nested_schema.namespace.as_deref());
                                Self::write_model(
                                    writer,
                                    &field.xml_name,
                                    val,
                                    nested_schema,
                                    ns_ctx,
                                    false,
                                    nested_ns,
                                )?;
                            }
                        }
                    }
                } else if field.kind == FieldKind::Any {
                    if let Some(val) = get_field(idx, &field.name) {
                        Self::write_any_wildcard_element(writer, val, ns_ctx)?;
                    }
                }
            }
        }

        // Close element
        writer
            .write_event(Event::End(BytesEnd::new(qualified_tag.as_ref())))
            .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;

        Ok(())
    }

    fn write_any_wildcard_element<W: std::io::Write>(
        writer: &mut quick_xml::Writer<W>,
        val: &PolyValue,
        ns_ctx: Option<&NamespaceContext>,
    ) -> Result<()> {
        match val {
            PolyValue::List(items) => {
                for item in items {
                    Self::write_single_wildcard_element(writer, item, ns_ctx)?;
                }
            }
            PolyValue::Object(_) => {
                Self::write_single_wildcard_element(writer, val, ns_ctx)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn write_single_wildcard_element<W: std::io::Write>(
        writer: &mut quick_xml::Writer<W>,
        val: &PolyValue,
        ns_ctx: Option<&NamespaceContext>,
    ) -> Result<()> {
        let PolyValue::Object(map) = val else {
            return Ok(());
        };
        let Some(PolyValue::String(qname)) = map.get("qname") else {
            return Ok(());
        };
        let (ns_uri, local) = if qname.starts_with('{') {
            if let Some(end) = qname.find('}') {
                (Some(&qname[1..end]), &qname[end + 1..])
            } else {
                (None, qname.as_str())
            }
        } else {
            (None, qname.as_str())
        };

        let (qualified, xmlns_attr) = if let Some(uri) = ns_uri {
            if let Some(ctx) = ns_ctx {
                if let Some(prefix) = ctx.uri_to_prefix.get(uri) {
                    if !prefix.is_empty() {
                        (Cow::Owned(format!("{}:{}", prefix, local)), None)
                    } else {
                        (Cow::Borrowed(local), None)
                    }
                } else {
                    (Cow::Borrowed(local), Some(uri))
                }
            } else {
                (Cow::Borrowed(local), Some(uri))
            }
        } else {
            (Cow::Borrowed(local), None)
        };

        let mut start = BytesStart::new(qualified.as_ref());
        if let Some(uri) = xmlns_attr {
            start.push_attribute(("xmlns", uri));
        }

        if let Some(PolyValue::Object(attrs)) = map.get("attributes") {
            let mut sorted_keys: Vec<_> = attrs.keys().collect();
            sorted_keys.sort();
            for k in sorted_keys {
                if let Some(v) = attrs.get(k) {
                    let mut buf = [0u8; lexical_core::BUFFER_SIZE];
                    if let Some(s) = Self::format_scalar_to(v, &mut buf) {
                        start.push_attribute((k.as_str(), s));
                    }
                }
            }
        }

        writer
            .write_event(Event::Start(start))
            .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;

        if let Some(PolyValue::String(text)) = map.get("text") {
            if !text.is_empty() {
                writer
                    .write_event(Event::Text(BytesText::new(text)))
                    .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;
            }
        }

        if let Some(PolyValue::List(children)) = map.get("children") {
            for child in children {
                Self::write_single_wildcard_element(writer, child, ns_ctx)?;
            }
        }

        writer
            .write_event(Event::End(BytesEnd::new(qualified.as_ref())))
            .map_err(|e| PolyXmlError::SerializationError(e.to_string()))?;

        Ok(())
    }
}
