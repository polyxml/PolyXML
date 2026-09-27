use std::collections::HashMap;
use std::sync::Arc;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use smallvec::{smallvec, SmallVec};

use crate::converters::ValueConverter;
use crate::error::{PolyXmlError, Result};
use crate::schema::{ModelSchema, ScalarType, ValueType};
use crate::value::PolyValue;

type NamespaceScope = Arc<HashMap<String, String>>;

pub(crate) struct StackFrame {
    schema: Arc<ModelSchema>,
    values: SmallVec<[Option<PolyValue>; 8]>,
    list_values: SmallVec<[Option<Vec<PolyValue>>; 4]>,
    frame_text_buf: Option<Vec<u8>>,
    mixed_parent_kind: Option<String>,
}

impl StackFrame {
    fn new(schema: Arc<ModelSchema>) -> Self {
        let field_count = schema.fields.len();
        let frame_text_buf = if schema.text_field.is_some() {
            Some(Vec::new())
        } else {
            None
        };
        Self {
            schema,
            values: smallvec![None; field_count],
            list_values: SmallVec::new(),
            frame_text_buf,
            mixed_parent_kind: None,
        }
    }

    fn push_mixed_item(&mut self, kind: &str, value: PolyValue) {
        if let Some(mixed) = &self.schema.mixed_content {
            let mut item = HashMap::new();
            item.insert("kind".into(), PolyValue::String(kind.into()));
            item.insert("value".into(), value);
            self.push_list_item(mixed.items_index, PolyValue::Object(item));
        }
    }

    fn push_mixed_text(&mut self, text: &[u8]) -> Result<()> {
        if self.schema.mixed_content.is_some() && !text.is_empty() {
            self.push_mixed_item(
                "#text",
                PolyValue::String(std::str::from_utf8(text)?.into()),
            );
        }
        Ok(())
    }

    #[inline]
    fn push_list_item(&mut self, idx: usize, val: PolyValue) {
        if self.list_values.is_empty() {
            self.list_values
                .resize_with(self.schema.fields.len(), || None);
        }
        if let Some(list_opt) = self.list_values.get_mut(idx) {
            match list_opt {
                Some(list) => list.push(val),
                None => *list_opt = Some(vec![val]),
            }
        }
    }

    fn finish(mut self) -> Result<PolyValue> {
        if !self.list_values.is_empty() {
            for (idx, list_opt) in self.list_values.into_iter().enumerate() {
                if let Some(list) = list_opt {
                    self.values[idx] = Some(PolyValue::List(list));
                }
            }
        }

        if let Some(text_idx) = self.schema.text_field {
            if let Some(ref text_buf) = self.frame_text_buf {
                if !text_buf.is_empty() {
                    let field = &self.schema.fields[text_idx];
                    match &field.val_type {
                        ValueType::Scalar(st) => {
                            let val = ValueConverter::parse_scalar(st, text_buf, &field.name)?;
                            self.values[text_idx] = Some(val);
                        }
                        ValueType::List(inner) => {
                            if let ValueType::Scalar(st) = &**inner {
                                let val = ValueConverter::parse_scalar(st, text_buf, &field.name)?;
                                self.values[text_idx] = Some(PolyValue::List(vec![val]));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(PolyValue::Record {
            schema: self.schema,
            values: self.values.into_vec().into_boxed_slice(),
        })
    }
}

pub struct XmlDeserializer;

fn is_nil_element(e: &BytesStart) -> bool {
    for attr in e.attributes().flatten() {
        let raw_key = attr.key.as_ref();
        if raw_key == "xmlns" || raw_key.starts_with("xmlns:") {
            continue;
        }
        let key = attr.key.local_name();
        if (key.as_ref() == "nil" || key.as_ref() == "xsi:nil")
            && (attr.value.as_ref() == "true" || attr.value.as_ref() == "1")
        {
            return true;
        }
    }
    false
}

/// Extend inherited namespace bindings for this element. Elements without
/// declarations share the existing map.
fn namespace_scope(parent: &NamespaceScope, e: &BytesStart) -> NamespaceScope {
    let mut scope = None;
    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref();
        if key == "xmlns" {
            scope
                .get_or_insert_with(|| (**parent).clone())
                .insert(String::new(), attr.value.as_ref().to_string());
        } else if let Some(prefix) = key.strip_prefix("xmlns:") {
            scope
                .get_or_insert_with(|| (**parent).clone())
                .insert(prefix.to_string(), attr.value.as_ref().to_string());
        }
    }
    scope.map(Arc::new).unwrap_or_else(|| Arc::clone(parent))
}

/// Raw unescaped value of the type attribute in the XML Schema Instance
/// namespace. The prefix is resolved from the element's namespace scope.
fn xsi_type_value(e: &BytesStart, scope: &HashMap<String, String>) -> Result<Option<Vec<u8>>> {
    for attr in e.attributes().flatten() {
        let raw_key = attr.key.as_ref();
        if raw_key == "xmlns" || raw_key.starts_with("xmlns:") {
            continue;
        }
        if let Some(prefix) = raw_key.strip_suffix(":type") {
            if scope
                .get(prefix)
                .is_some_and(|uri| uri == "http://www.w3.org/2001/XMLSchema-instance")
            {
                let unescaped = quick_xml::escape::unescape(attr.value.as_ref())?;
                return Ok(Some(unescaped.into_owned().into_bytes()));
            }
        }
    }
    Ok(None)
}

/// Resolve the concrete schema an element should be parsed with.
///
/// When the declared type has `xsi:type` derivations registered (or is
/// abstract), the wire attribute selects one; unknown types on an abstract
/// base error loudly instead of silently dropping concrete fields. Types
/// without dispatch information keep today's behavior of parsing as
/// declared, so a plain content attribute named `type` is never mistaken
/// for dispatch on a non-abstract type.
fn resolve_record_schema(
    declared: &Arc<ModelSchema>,
    e: &BytesStart,
    scope: &NamespaceScope,
) -> Result<Arc<ModelSchema>> {
    if !declared.is_abstract && !declared.has_variants() {
        return Ok(Arc::clone(declared));
    }
    let raw = match xsi_type_value(e, scope)? {
        Some(raw) => raw,
        None if !declared.is_abstract => return Ok(Arc::clone(declared)),
        None => {
            return Err(PolyXmlError::SchemaError(format!(
                "abstract type '{}' requires xsi:type naming a concrete derivation",
                declared.name
            )))
        }
    };
    let (prefix, local): (&[u8], &[u8]) = match raw.iter().position(|&b| b == b':') {
        Some(i) => (&raw[..i], &raw[i + 1..]),
        None => (b"", &raw[..]),
    };
    let prefix = std::str::from_utf8(prefix).unwrap_or("");
    let namespace = scope.get(prefix).map(String::as_str);
    if prefix.is_empty() || namespace.is_some() {
        if let Some(variant) = declared.find_variant_qname(namespace.unwrap_or(""), local) {
            if !variant.is_abstract {
                return Ok(variant);
            }
        }
    }
    if declared.is_abstract {
        let value = String::from_utf8_lossy(&raw);
        return Err(PolyXmlError::SchemaError(if declared.has_variants() {
            let known: Vec<String> = declared
                .variants()
                .iter()
                .map(|v| String::from_utf8_lossy(&v.xml_name).into_owned())
                .collect();
            format!(
                "xsi:type=\"{}\" does not match any known derivation of '{}' (known: {})",
                value,
                declared.name,
                known.join(", ")
            )
        } else {
            format!(
                "xsi:type=\"{}\" targets abstract type '{}', which has no registered \
                 derivations; deserialize the concrete type directly (escape hatch)",
                value, declared.name
            )
        }));
    }
    Ok(Arc::clone(declared))
}

fn append_general_ref(
    e: &quick_xml::events::BytesRef,
    active_scalar: bool,
    text_buf: &mut Vec<u8>,
    frame_text_buf: Option<&mut Vec<u8>>,
) -> Result<()> {
    let mut char_buf = [0u8; 4];
    let resolved_bytes: &[u8] = if e.is_char_ref() {
        if let Some(ch) = e.resolve_char_ref().map_err(PolyXmlError::XmlError)? {
            ch.encode_utf8(&mut char_buf).as_bytes()
        } else {
            b""
        }
    } else if let Some(s) = quick_xml::escape::resolve_xml_entity(e.as_ref()) {
        s.as_bytes()
    } else {
        e.as_ref().as_bytes()
    };

    if active_scalar {
        text_buf.extend_from_slice(resolved_bytes);
    } else if let Some(tb) = frame_text_buf {
        tb.extend_from_slice(resolved_bytes);
    }
    Ok(())
}

pub const DEFAULT_MAX_DEPTH: usize = 256;

impl XmlDeserializer {
    pub fn deserialize(xml_bytes: &[u8], root_schema: Arc<ModelSchema>) -> Result<PolyValue> {
        Self::deserialize_with_limit(xml_bytes, root_schema, DEFAULT_MAX_DEPTH)
    }

    pub fn deserialize_with_limit(
        xml_bytes: &[u8],
        root_schema: Arc<ModelSchema>,
        max_depth: usize,
    ) -> Result<PolyValue> {
        let mut reader = Reader::from_reader(xml_bytes);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    return Self::parse_sub_tree(
                        &mut reader,
                        root_schema,
                        e,
                        max_depth,
                        &Arc::new(HashMap::new()),
                    );
                }
                Ok(Event::Empty(ref e)) => {
                    let scope = namespace_scope(&Arc::new(HashMap::new()), e);
                    let schema = resolve_record_schema(&root_schema, e, &scope)?;
                    let mut frame = StackFrame::new(schema);
                    Self::parse_attributes(e, &mut frame)?;
                    return frame.finish();
                }
                Ok(Event::Eof) => break,
                Err(err) => {
                    return Err(PolyXmlError::XmlSyntaxError {
                        position: reader.buffer_position(),
                        source: err,
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        Err(PolyXmlError::SchemaError(
            "Unexpected end of XML stream".to_string(),
        ))
    }

    pub(crate) fn parse_sub_tree<R: std::io::BufRead>(
        reader: &mut Reader<R>,
        root_schema: Arc<ModelSchema>,
        root_start: &BytesStart,
        max_depth: usize,
        inherited_scope: &NamespaceScope,
    ) -> Result<PolyValue> {
        let mut stack: Vec<StackFrame> = Vec::with_capacity(16);
        let mut namespace_stack = vec![namespace_scope(inherited_scope, root_start)];
        let root_schema =
            resolve_record_schema(&root_schema, root_start, namespace_stack.last().unwrap())?;
        let mut root_frame = StackFrame::new(root_schema);
        Self::parse_attributes(root_start, &mut root_frame)?;
        stack.push(root_frame);

        let mut active_scalar_field: Option<(usize, ScalarType, bool)> = None;
        let mut active_mixed_scalar: Option<(String, ScalarType)> = None;
        let mut unknown_depth: usize = 0;
        let mut text_buf: Vec<u8> = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    namespace_stack.push(namespace_scope(namespace_stack.last().unwrap(), e));
                    if unknown_depth > 0 {
                        unknown_depth += 1;
                        continue;
                    }
                    if active_scalar_field.is_some() || active_mixed_scalar.is_some() {
                        unknown_depth = 1;
                        continue;
                    }
                    if stack.len() >= max_depth {
                        return Err(PolyXmlError::MaxDepthExceeded {
                            max_depth,
                            current: stack.len() + 1,
                        });
                    }
                    let local_name = e.local_name();
                    let is_nil = is_nil_element(e);

                    let current_schema = Arc::clone(&stack.last().unwrap().schema);
                    if let Some(&field_idx) = current_schema
                        .element_map
                        .get(local_name.as_ref().as_bytes())
                    {
                        let field = &current_schema.fields[field_idx];
                        match &field.val_type {
                            ValueType::Scalar(st) => {
                                if is_nil {
                                    stack.last_mut().unwrap().values[field_idx] =
                                        Some(PolyValue::Null);
                                } else {
                                    active_scalar_field = Some((field_idx, st.clone(), false));
                                    text_buf.clear();
                                }
                            }
                            ValueType::List(inner) => match inner.as_ref() {
                                ValueType::Scalar(st) => {
                                    if is_nil {
                                        stack
                                            .last_mut()
                                            .unwrap()
                                            .push_list_item(field_idx, PolyValue::Null);
                                    } else {
                                        active_scalar_field = Some((field_idx, st.clone(), true));
                                        text_buf.clear();
                                    }
                                }
                                ValueType::Nested(sub_schema) => {
                                    if is_nil {
                                        stack
                                            .last_mut()
                                            .unwrap()
                                            .push_list_item(field_idx, PolyValue::Null);
                                        unknown_depth = 1;
                                        continue;
                                    }
                                    let frame_schema = resolve_record_schema(
                                        sub_schema,
                                        e,
                                        namespace_stack.last().unwrap(),
                                    )?;
                                    let mut frame = StackFrame::new(frame_schema);
                                    Self::parse_attributes(e, &mut frame)?;
                                    stack.push(frame);
                                }
                                _ => {}
                            },
                            ValueType::Nested(sub_schema) => {
                                if is_nil {
                                    stack.last_mut().unwrap().values[field_idx] =
                                        Some(PolyValue::Null);
                                    unknown_depth = 1;
                                    continue;
                                }
                                let frame_schema = resolve_record_schema(
                                    sub_schema,
                                    e,
                                    namespace_stack.last().unwrap(),
                                )?;
                                let mut frame = StackFrame::new(frame_schema);
                                Self::parse_attributes(e, &mut frame)?;
                                stack.push(frame);
                            }
                        }
                    } else if let Some(mixed) = &current_schema.mixed_content {
                        if let Some(branch) = mixed.branch(local_name.as_ref().as_bytes()) {
                            if is_nil {
                                stack
                                    .last_mut()
                                    .unwrap()
                                    .push_mixed_item(&branch.variant_name, PolyValue::Null);
                                unknown_depth = 1;
                            } else {
                                match &branch.val_type {
                                    ValueType::Scalar(scalar) => {
                                        active_mixed_scalar =
                                            Some((branch.variant_name.clone(), scalar.clone()));
                                        text_buf.clear();
                                    }
                                    ValueType::Nested(sub_schema) => {
                                        let frame_schema = resolve_record_schema(
                                            sub_schema,
                                            e,
                                            namespace_stack.last().unwrap(),
                                        )?;
                                        let mut frame = StackFrame::new(frame_schema);
                                        frame.mixed_parent_kind = Some(branch.variant_name.clone());
                                        Self::parse_attributes(e, &mut frame)?;
                                        stack.push(frame);
                                    }
                                    ValueType::List(_) => unknown_depth = 1,
                                }
                            }
                        } else {
                            unknown_depth = 1;
                        }
                    } else {
                        unknown_depth = 1;
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    if unknown_depth > 0
                        || active_scalar_field.is_some()
                        || active_mixed_scalar.is_some()
                    {
                        continue;
                    }
                    let local_name = e.local_name();
                    let scope = namespace_scope(namespace_stack.last().unwrap(), e);
                    let is_nil = is_nil_element(e);

                    let current_schema = Arc::clone(&stack.last().unwrap().schema);
                    if let Some(&field_idx) = current_schema
                        .element_map
                        .get(local_name.as_ref().as_bytes())
                    {
                        let field = &current_schema.fields[field_idx];
                        match &field.val_type {
                            ValueType::Scalar(_) => {
                                let val = if is_nil {
                                    PolyValue::Null
                                } else {
                                    PolyValue::String(String::new())
                                };
                                stack.last_mut().unwrap().values[field_idx] = Some(val);
                            }
                            ValueType::List(inner) => match inner.as_ref() {
                                ValueType::Scalar(_) => {
                                    let val = if is_nil {
                                        PolyValue::Null
                                    } else {
                                        PolyValue::String(String::new())
                                    };
                                    stack.last_mut().unwrap().push_list_item(field_idx, val);
                                }
                                ValueType::Nested(sub_schema) => {
                                    if is_nil {
                                        stack
                                            .last_mut()
                                            .unwrap()
                                            .push_list_item(field_idx, PolyValue::Null);
                                        continue;
                                    }
                                    let frame_schema =
                                        resolve_record_schema(sub_schema, e, &scope)?;
                                    let mut frame = StackFrame::new(frame_schema);
                                    Self::parse_attributes(e, &mut frame)?;
                                    let instance = frame.finish()?;
                                    stack
                                        .last_mut()
                                        .unwrap()
                                        .push_list_item(field_idx, instance);
                                }
                                _ => {}
                            },
                            ValueType::Nested(sub_schema) => {
                                if is_nil {
                                    stack.last_mut().unwrap().values[field_idx] =
                                        Some(PolyValue::Null);
                                    continue;
                                }
                                let frame_schema = resolve_record_schema(sub_schema, e, &scope)?;
                                let mut frame = StackFrame::new(frame_schema);
                                Self::parse_attributes(e, &mut frame)?;
                                let instance = frame.finish()?;
                                stack.last_mut().unwrap().values[field_idx] = Some(instance);
                            }
                        }
                    } else if let Some(mixed) = &current_schema.mixed_content {
                        if let Some(branch) = mixed.branch(local_name.as_ref().as_bytes()) {
                            let value = if is_nil {
                                PolyValue::Null
                            } else {
                                match &branch.val_type {
                                    ValueType::Scalar(scalar) => ValueConverter::parse_scalar(
                                        scalar,
                                        b"",
                                        &branch.variant_name,
                                    )?,
                                    ValueType::Nested(sub_schema) => {
                                        let frame_schema =
                                            resolve_record_schema(sub_schema, e, &scope)?;
                                        let mut frame = StackFrame::new(frame_schema);
                                        Self::parse_attributes(e, &mut frame)?;
                                        frame.finish()?
                                    }
                                    ValueType::List(_) => PolyValue::Null,
                                }
                            };
                            stack
                                .last_mut()
                                .unwrap()
                                .push_mixed_item(&branch.variant_name, value);
                        }
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if unknown_depth > 0 {
                        continue;
                    }
                    let raw = e.as_ref();
                    let raw_bytes = raw.as_bytes();
                    if memchr::memchr(b'&', raw_bytes).is_none() {
                        if active_scalar_field.is_some() || active_mixed_scalar.is_some() {
                            text_buf.extend_from_slice(raw_bytes);
                        } else if let Some(frame) = stack.last_mut() {
                            if let Some(ref mut tb) = frame.frame_text_buf {
                                tb.extend_from_slice(raw_bytes);
                            } else {
                                frame.push_mixed_text(raw_bytes)?;
                            }
                        }
                    } else {
                        let unescaped = quick_xml::escape::unescape(raw)?;
                        if active_scalar_field.is_some() || active_mixed_scalar.is_some() {
                            text_buf.extend_from_slice(unescaped.as_bytes());
                        } else if let Some(frame) = stack.last_mut() {
                            if let Some(ref mut tb) = frame.frame_text_buf {
                                tb.extend_from_slice(unescaped.as_bytes());
                            } else {
                                frame.push_mixed_text(unescaped.as_bytes())?;
                            }
                        }
                    }
                }
                Ok(Event::CData(ref e)) => {
                    if unknown_depth > 0 {
                        continue;
                    }
                    if active_scalar_field.is_some() || active_mixed_scalar.is_some() {
                        text_buf.extend_from_slice(e.as_ref().as_bytes());
                    } else if let Some(frame) = stack.last_mut() {
                        if let Some(ref mut tb) = frame.frame_text_buf {
                            tb.extend_from_slice(e.as_ref().as_bytes());
                        } else {
                            frame.push_mixed_text(e.as_ref().as_bytes())?;
                        }
                    }
                }
                Ok(Event::GeneralRef(ref e)) => {
                    if unknown_depth > 0 {
                        continue;
                    }
                    let frame_tb = stack.last_mut().and_then(|f| f.frame_text_buf.as_mut());
                    if active_scalar_field.is_some()
                        || active_mixed_scalar.is_some()
                        || frame_tb.is_some()
                    {
                        append_general_ref(
                            e,
                            active_scalar_field.is_some() || active_mixed_scalar.is_some(),
                            &mut text_buf,
                            frame_tb,
                        )?;
                    } else {
                        let mut resolved = Vec::new();
                        append_general_ref(e, true, &mut resolved, None)?;
                        if let Some(frame) = stack.last_mut() {
                            frame.push_mixed_text(&resolved)?;
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    namespace_stack.pop();
                    if unknown_depth > 0 {
                        unknown_depth -= 1;
                        continue;
                    }
                    if let Some((kind, scalar_type)) = active_mixed_scalar.take() {
                        let parsed = ValueConverter::parse_scalar(&scalar_type, &text_buf, &kind)?;
                        stack.last_mut().unwrap().push_mixed_item(&kind, parsed);
                    } else if let Some((field_idx, ref scalar_type, is_list)) =
                        active_scalar_field.take()
                    {
                        let field_name = &stack.last().unwrap().schema.fields[field_idx].name;
                        let parsed_val =
                            ValueConverter::parse_scalar(scalar_type, &text_buf, field_name)?;
                        let frame = stack.last_mut().unwrap();
                        if is_list {
                            frame.push_list_item(field_idx, parsed_val);
                        } else {
                            frame.values[field_idx] = Some(parsed_val);
                        }
                    } else if stack.len() > 1 {
                        let finished_frame = stack.pop().unwrap();
                        let local_name = e.local_name();
                        let mixed_kind = finished_frame.mixed_parent_kind.clone();
                        let instance = finished_frame.finish()?;
                        let parent = stack.last_mut().unwrap();

                        if let Some(kind) = mixed_kind {
                            parent.push_mixed_item(&kind, instance);
                            continue;
                        }

                        if let Some(&field_idx) = parent
                            .schema
                            .element_map
                            .get(local_name.as_ref().as_bytes())
                        {
                            let field = &parent.schema.fields[field_idx];
                            if matches!(field.val_type, ValueType::List(_)) {
                                parent.push_list_item(field_idx, instance);
                            } else {
                                parent.values[field_idx] = Some(instance);
                            }
                        }
                    } else if stack.len() == 1 {
                        let root_frame = stack.pop().unwrap();
                        return root_frame.finish();
                    }
                }
                Ok(Event::Eof) => break,
                Err(err) => {
                    return Err(PolyXmlError::XmlSyntaxError {
                        position: reader.buffer_position(),
                        source: err,
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        Err(PolyXmlError::SchemaError(
            "Unexpected end of XML stream".to_string(),
        ))
    }

    pub(crate) fn parse_attributes(e: &BytesStart, frame: &mut StackFrame) -> Result<()> {
        for attr in e.attributes().flatten() {
            let raw_key = attr.key.as_ref();
            if raw_key == "xmlns" || raw_key.starts_with("xmlns:") {
                continue;
            }
            let key = attr.key.local_name();
            if let Some(&field_idx) = frame.schema.attribute_map.get(key.as_ref().as_bytes()) {
                let field = &frame.schema.fields[field_idx];
                if let ValueType::Scalar(ref st) = field.val_type {
                    let unescaped = quick_xml::escape::unescape(attr.value.as_ref())?;
                    let val = ValueConverter::parse_scalar(
                        st,
                        unescaped.as_ref().as_bytes(),
                        &field.name,
                    )?;
                    frame.values[field_idx] = Some(val);
                }
            }
        }
        Ok(())
    }
}

pub struct XmlItemStream<R: std::io::BufRead> {
    reader: Reader<R>,
    schema: Arc<ModelSchema>,
    target_tag: Vec<u8>,
    max_depth: usize,
    buf: Vec<u8>,
    namespace_stack: Vec<NamespaceScope>,
}

impl<R: std::io::BufRead> XmlItemStream<R> {
    pub fn new(reader: R, schema: Arc<ModelSchema>, target_tag: &[u8]) -> Self {
        let xml_reader = Reader::from_reader(reader);
        Self {
            reader: xml_reader,
            schema,
            target_tag: target_tag.to_vec(),
            max_depth: DEFAULT_MAX_DEPTH,
            buf: Vec::new(),
            namespace_stack: Vec::new(),
        }
    }

    pub fn next_item(&mut self) -> Result<Option<PolyValue>> {
        let target_local = if let Some(pos) = self.target_tag.iter().position(|&b| b == b':') {
            &self.target_tag[pos + 1..]
        } else {
            self.target_tag.as_slice()
        };

        loop {
            self.buf.clear();
            match self.reader.read_event_into(&mut self.buf) {
                Ok(Event::Start(ref e)) => {
                    let local = e.local_name();
                    if local.as_ref().as_bytes() == target_local
                        || e.name().as_ref().as_bytes() == self.target_tag.as_slice()
                    {
                        let inherited = self
                            .namespace_stack
                            .last()
                            .cloned()
                            .unwrap_or_else(|| Arc::new(HashMap::new()));
                        let item = XmlDeserializer::parse_sub_tree(
                            &mut self.reader,
                            Arc::clone(&self.schema),
                            e,
                            self.max_depth,
                            &inherited,
                        )?;
                        return Ok(Some(item));
                    }
                    let inherited = self
                        .namespace_stack
                        .last()
                        .cloned()
                        .unwrap_or_else(|| Arc::new(HashMap::new()));
                    self.namespace_stack.push(namespace_scope(&inherited, e));
                }
                Ok(Event::Empty(ref e)) => {
                    let local = e.local_name();
                    if local.as_ref().as_bytes() == target_local
                        || e.name().as_ref().as_bytes() == self.target_tag.as_slice()
                    {
                        let inherited = self
                            .namespace_stack
                            .last()
                            .cloned()
                            .unwrap_or_else(|| Arc::new(HashMap::new()));
                        let scope = namespace_scope(&inherited, e);
                        let schema = resolve_record_schema(&self.schema, e, &scope)?;
                        let mut frame = StackFrame::new(schema);
                        XmlDeserializer::parse_attributes(e, &mut frame)?;
                        return Ok(Some(frame.finish()?));
                    }
                }
                Ok(Event::Eof) => return Ok(None),
                Ok(Event::End(_)) => {
                    self.namespace_stack.pop();
                }
                Err(err) => {
                    return Err(PolyXmlError::XmlSyntaxError {
                        position: self.reader.buffer_position(),
                        source: err,
                    });
                }
                _ => {}
            }
        }
    }
}
