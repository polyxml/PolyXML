use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use thiserror::Error;

use crate::ir::{
    Cardinality, ElementDef, EnumDef, EnumValue, FieldDef, FieldKind, OccursLimit, PrimitiveType,
    QName, RestrictionFacets, SchemaIR, SimpleTypeDef, StructDef, TypeDef, TypeRef, UnionBranch,
    UnionDef,
};

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Malformed schema: {0}")]
    Malformed(String),

    #[error("Resolution error: {0}")]
    Resolution(String),
}

/// A parsed `<xs:group>` definition: its flattened element fields plus the
/// positions of any nested `<xs:group ref="...">` particles it contains.
#[derive(Debug, Clone, Default)]
struct GroupDef {
    fields: Vec<FieldDef>,
    /// `(insertion index into `fields`, referenced group)` — ascending.
    group_refs: Vec<(usize, QName)>,
}

/// A recorded `<xs:group ref>` particle awaiting post-parse expansion.
#[derive(Debug, Clone)]
struct PendingGroupRef {
    /// The struct (or group body) the fields must be spliced into.
    owner: QName,
    /// Insertion index into the owner's field list (pre-expansion).
    at: usize,
    group: QName,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompositorKind {
    Sequence,
    All,
    Choice,
}

#[derive(Debug, Clone)]
struct CompositorFrame {
    kind: CompositorKind,
    is_unbounded: bool,
    min_occurs: usize,
    fields_start: usize,
    choice_branches: Vec<UnionBranch>,
    has_sequence_branch: bool,
}

/// A pure-Rust XSD 1.0/1.1 Schema Parser.
pub struct XsdParser {
    /// Raw (pre-merge) IR per canonical file path, keyed so chameleon includes
    /// can be re-namespaced per includer without re-parsing.
    file_cache: HashMap<PathBuf, SchemaIR>,
    /// Groups registered by each cached file, replayed on cache hits.
    file_groups: HashMap<PathBuf, Vec<(QName, GroupDef)>>,
    /// Files currently being parsed (include-cycle guard).
    active_files: HashSet<PathBuf>,
    /// Named model groups visible to the current parse.
    groups: HashMap<QName, GroupDef>,
    /// Group references awaiting expansion at frame end.
    pending_group_refs: Vec<PendingGroupRef>,
    /// Include/import recursion depth; post-passes run in every frame, the
    /// root frame (depth 1 while executing) additionally drops dead refs.
    frame_depth: usize,
}

impl Default for XsdParser {
    fn default() -> Self {
        Self::new()
    }
}

impl XsdParser {
    pub fn new() -> Self {
        Self {
            file_cache: HashMap::new(),
            file_groups: HashMap::new(),
            active_files: HashSet::new(),
            groups: HashMap::new(),
            pending_group_refs: Vec::new(),
            frame_depth: 0,
        }
    }

    /// Parse an XSD schema from a file path, recursively resolving includes and imports.
    pub fn parse_file(&mut self, path: impl AsRef<Path>) -> Result<SchemaIR, SchemaError> {
        let path = path.as_ref();
        let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

        if let Some(cached) = self.file_cache.get(&canonical) {
            // Cache hit: replay this file's group definitions so includers can
            // resolve `<xs:group ref>` against them, then return the raw IR.
            if let Some(registered) = self.file_groups.get(&canonical) {
                for (qname, def) in registered {
                    self.groups.insert(qname.clone(), def.clone());
                }
            }
            return Ok(cached.clone());
        }
        if self.active_files.contains(&canonical) {
            // Include cycle: cut recursion.
            return Ok(SchemaIR::new());
        }

        self.active_files.insert(canonical.clone());
        let groups_before: HashSet<QName> = self.groups.keys().cloned().collect();
        let content = fs::read_to_string(path);
        let parsed = match content {
            Ok(content) => {
                let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
                self.parse_str_internal(&content, Some(base_dir))
            }
            Err(e) => Err(e.into()),
        };
        self.active_files.remove(&canonical);
        let ir = parsed?;

        // Snapshot groups registered while parsing this file for cache replay.
        let file_owned: Vec<(QName, GroupDef)> = self
            .groups
            .iter()
            .filter(|(q, _)| !groups_before.contains(q))
            .map(|(q, d)| (q.clone(), d.clone()))
            .collect();
        if !file_owned.is_empty() {
            self.file_groups
                .entry(canonical.clone())
                .or_default()
                .extend(file_owned);
        }

        self.file_cache.insert(canonical.clone(), ir.clone());
        Ok(ir)
    }

    /// Parse an XSD schema from a string slice.
    pub fn parse_str(&mut self, xml: &str) -> Result<SchemaIR, SchemaError> {
        self.parse_str_internal(xml, None)
    }

    fn parse_str_internal(
        &mut self,
        xml: &str,
        base_dir: Option<&Path>,
    ) -> Result<SchemaIR, SchemaError> {
        self.frame_depth += 1;
        let result = self.parse_str_body(xml, base_dir);
        self.frame_depth -= 1;
        let mut ir = result?;

        // Frame post-passes: inherit pattern facets up derivation chains
        // and expand named model group references. Both are idempotent.
        // Cycle detection runs last so fields spliced in from groups are covered.
        add_gregorian_types(&mut ir);
        inherit_pattern_facets(&mut ir);
        self.expand_group_refs(&mut ir, self.frame_depth == 0);
        if self.frame_depth == 0 {
            resolve_global_field_refs(&mut ir);
            expand_substitution_fields(&mut ir);
            compile_mixed_types(&mut ir);
        }
        ir.resolve_cycles();

        Ok(ir)
    }

    fn parse_str_body(
        &mut self,
        xml: &str,
        base_dir: Option<&Path>,
    ) -> Result<SchemaIR, SchemaError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut ir = SchemaIR::new();
        let mut target_namespace = None;
        // The `xml` prefix is bound by XML itself, even when the schema omits
        // an explicit xmlns:xml declaration (as Dublin Core's dc.xsd does).
        let mut prefixes = HashMap::from([(
            "xml".to_string(),
            "http://www.w3.org/XML/1998/namespace".to_string(),
        )]);
        let mut buf = Vec::new();

        // Pass 1: Parse root schema attributes and build prefix table
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    let name = e.name().into_inner();
                    let local = strip_prefix(name);
                    if local == "schema" {
                        for attr in e.attributes().flatten() {
                            let key = attr.key.as_ref();
                            let val = attr.value.as_ref();

                            if key == "targetNamespace" {
                                target_namespace = Some(val.to_string());
                                ir.target_namespace = Some(val.to_string());
                            } else if key == "xmlns" {
                                prefixes.insert(String::new(), val.to_string());
                            } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                                prefixes.insert(prefix.to_string(), val.to_string());
                            }
                        }
                        break;
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        // Pass 2: Ingest top-level constructs, includes, and imports
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        buf.clear();

        while let Ok(event) = reader.read_event_into(&mut buf) {
            match event {
                Event::Start(ref e) => {
                    let local = strip_prefix(e.name().into_inner());

                    match local {
                        "include" | "redefine" => {
                            if let Some(schema_location) = get_attr_value(e, "schemaLocation") {
                                if let Some(dir) = base_dir {
                                    let inc_path = dir.join(&schema_location);
                                    if inc_path.exists() {
                                        let pend_before = self.pending_group_refs.len();
                                        let groups_before: HashSet<QName> =
                                            self.groups.keys().cloned().collect();
                                        let mut sub_ir = self.parse_file(&inc_path)?;
                                        // Chameleon include (no targetNamespace in the
                                        // included file): attribute its components to
                                        // this schema's namespace.
                                        if sub_ir.target_namespace.is_none() {
                                            if let Some(ns) = target_namespace.as_deref() {
                                                rekey_to_namespace(&mut sub_ir, ns);
                                                self.rekey_new_state(
                                                    &groups_before,
                                                    pend_before,
                                                    ns,
                                                );
                                            }
                                        }
                                        merge_ir(&mut ir, sub_ir);
                                    } else {
                                        return Err(SchemaError::Resolution(format!(
                                            "included schema not found: {}",
                                            inc_path.display()
                                        )));
                                    }
                                }
                            }
                        }
                        "import" => {
                            if let Some(schema_location) = get_attr_value(e, "schemaLocation") {
                                if let Some(dir) = base_dir {
                                    let imp_path = dir.join(&schema_location);
                                    if imp_path.exists() {
                                        let sub_ir = self.parse_file(&imp_path)?;
                                        merge_ir(&mut ir, sub_ir);
                                    }
                                }
                            }
                        }
                        "complexType" => {
                            if let Some(type_def) = self.parse_complex_type(
                                &mut reader,
                                e,
                                target_namespace.as_deref(),
                                &prefixes,
                                None,
                                &mut ir,
                            )? {
                                ir.add_type(type_def);
                            }
                        }
                        "simpleType" => {
                            if let Some(type_def) = self.parse_simple_type(
                                &mut reader,
                                e,
                                target_namespace.as_deref(),
                                &prefixes,
                                None,
                                &mut ir,
                            )? {
                                ir.add_type(type_def);
                            }
                        }
                        "element" => {
                            if let Some(elem_def) = self.parse_global_element(
                                &mut reader,
                                e,
                                target_namespace.as_deref(),
                                &prefixes,
                                &mut ir,
                            )? {
                                ir.add_element(elem_def);
                            }
                        }
                        "attribute" => {
                            if let (Some(name), Some(ty)) =
                                (get_attr_value(e, "name"), get_attr_value(e, "type"))
                            {
                                ir.attributes
                                    .entry(QName::new(target_namespace.as_deref(), name))
                                    .or_insert_with(|| {
                                        resolve_type_ref(
                                            &ty,
                                            target_namespace.as_deref(),
                                            &prefixes,
                                        )
                                    });
                            }
                            skip_subtree(&mut reader)?;
                        }
                        "group" => {
                            // Named model group definition.
                            if let Some(gname) = get_attr_value(e, "name") {
                                let gq = QName::new(target_namespace.as_deref(), gname);
                                let def = self.parse_group_body(
                                    &mut reader,
                                    target_namespace.as_deref(),
                                    &prefixes,
                                    &mut ir,
                                )?;
                                self.groups.insert(gq, def);
                            } else {
                                skip_subtree(&mut reader)?;
                            }
                        }
                        _ => {}
                    }
                }
                Event::Empty(ref e) => {
                    let local = strip_prefix(e.name().into_inner());

                    match local {
                        "include" | "redefine" => {
                            if let Some(schema_location) = get_attr_value(e, "schemaLocation") {
                                if let Some(dir) = base_dir {
                                    let inc_path = dir.join(&schema_location);
                                    if inc_path.exists() {
                                        let pend_before = self.pending_group_refs.len();
                                        let groups_before: HashSet<QName> =
                                            self.groups.keys().cloned().collect();
                                        let mut sub_ir = self.parse_file(&inc_path)?;
                                        // Chameleon include (no targetNamespace in the
                                        // included file): attribute its components to
                                        // this schema's namespace.
                                        if sub_ir.target_namespace.is_none() {
                                            if let Some(ns) = target_namespace.as_deref() {
                                                rekey_to_namespace(&mut sub_ir, ns);
                                                self.rekey_new_state(
                                                    &groups_before,
                                                    pend_before,
                                                    ns,
                                                );
                                            }
                                        }
                                        merge_ir(&mut ir, sub_ir);
                                    } else {
                                        return Err(SchemaError::Resolution(format!(
                                            "included schema not found: {}",
                                            inc_path.display()
                                        )));
                                    }
                                }
                            }
                        }
                        "import" => {
                            if let Some(schema_location) = get_attr_value(e, "schemaLocation") {
                                if let Some(dir) = base_dir {
                                    let imp_path = dir.join(&schema_location);
                                    if imp_path.exists() {
                                        let sub_ir = self.parse_file(&imp_path)?;
                                        merge_ir(&mut ir, sub_ir);
                                    }
                                }
                            }
                        }
                        "element" => {
                            if let Some(elem_def) = parse_empty_global_element(
                                e,
                                target_namespace.as_deref(),
                                &prefixes,
                            ) {
                                ir.add_element(elem_def);
                            }
                        }
                        "attribute" => {
                            if let (Some(name), Some(ty)) =
                                (get_attr_value(e, "name"), get_attr_value(e, "type"))
                            {
                                ir.attributes
                                    .entry(QName::new(target_namespace.as_deref(), name))
                                    .or_insert_with(|| {
                                        resolve_type_ref(
                                            &ty,
                                            target_namespace.as_deref(),
                                            &prefixes,
                                        )
                                    });
                            }
                        }
                        "group" => {
                            // Empty named model group (no particles).
                            if let Some(gname) = get_attr_value(e, "name") {
                                let gq = QName::new(target_namespace.as_deref(), gname);
                                self.groups.insert(gq, GroupDef::default());
                            }
                        }
                        "complexType" => {
                            if let Some(name) = get_attr_value(e, "name") {
                                let is_abstract = get_attr_value(e, "abstract")
                                    .map(|v| v == "true" || v == "1")
                                    .unwrap_or(false);
                                let qname = QName::new(target_namespace.as_deref(), name);
                                ir.add_type(TypeDef::Struct(StructDef {
                                    qname,
                                    base_type: None,
                                    is_abstract,
                                    is_mixed: get_attr_value(e, "mixed")
                                        .is_some_and(|value| value == "true" || value == "1"),
                                    fields: Vec::new(),
                                    documentation: None,
                                }));
                            }
                        }
                        "simpleType" => {
                            if let Some(name) = get_attr_value(e, "name") {
                                let qname = QName::new(target_namespace.as_deref(), name);
                                ir.add_type(TypeDef::Simple(Box::new(SimpleTypeDef {
                                    qname,
                                    base_type: TypeRef::string(),
                                    facets: RestrictionFacets::default(),
                                    documentation: None,
                                })));
                            }
                        }
                        _ => {}
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        // Cycle detection runs in `parse_str_internal` after the group
        // expansion and pattern inheritance post-passes.

        Ok(ir)
    }

    fn parse_complex_type(
        &mut self,
        reader: &mut Reader<&[u8]>,
        start: &BytesStart,
        target_ns: Option<&str>,
        prefixes: &HashMap<String, String>,
        name_override: Option<String>,
        ir: &mut SchemaIR,
    ) -> Result<Option<TypeDef>, SchemaError> {
        let name = match get_attr_value(start, "name").or(name_override) {
            Some(n) => n,
            None => return Ok(None), // Anonymous type handled in place
        };

        let is_abstract = get_attr_value(start, "abstract")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let is_mixed =
            get_attr_value(start, "mixed").is_some_and(|value| value == "true" || value == "1");

        let qname = QName::new(target_ns, name.clone());
        if !is_mixed {
            if let Some(model) = capture_content_model(reader, target_ns, prefixes)? {
                if model.has_choice() || model.has_repeated_sequence() {
                    regex::Regex::new(&format!("^(?:{})$", model.pattern())).map_err(|error| {
                        SchemaError::Malformed(format!("Unsupported content model: {error}"))
                    })?;
                    ir.content_models.insert(qname.clone(), model);
                }
            }
        }
        let mut fields = Vec::new();
        let mut base_type = None;
        let mut documentation = None;
        let mut in_simple_content = false;
        let mut value_field_pushed = false;
        let mut group_refs: Vec<(usize, QName)> = Vec::new();
        let mut buf = Vec::new();

        let mut is_top_level_choice = false;
        let mut top_level_choice_branches: Vec<UnionBranch> = Vec::new();
        let mut top_level_choice_fields_start = 0;
        let mut top_level_choice_fields_count = 0;
        let mut compositor_stack: Vec<CompositorFrame> = Vec::new();
        let mut depth = 1;
        while depth > 0 {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    depth += 1;
                    let local = strip_prefix(e.name().into_inner());

                    match local {
                        "documentation" => {
                            let text = reader.read_text(e.name())?.to_string();
                            documentation = Some(text);
                            depth -= 1;
                        }
                        "simpleContent" => {
                            in_simple_content = true;
                        }
                        "restriction" if in_simple_content => {
                            if let Some(base) = get_attr_value(e, "base") {
                                let resolved = resolve_qname(&base, target_ns, prefixes);
                                if resolved != qname {
                                    base_type = Some(resolved);
                                }
                                if !value_field_pushed {
                                    value_field_pushed = true;
                                    fields.push(value_field(&base, target_ns, prefixes));
                                }
                            }
                        }
                        "extension" => {
                            if let Some(base) = get_attr_value(e, "base") {
                                let resolved = resolve_qname(&base, target_ns, prefixes);
                                if resolved != qname {
                                    base_type = Some(resolved);
                                }
                                // simpleContent extension: the text content is the
                                // value of the restricted base type.
                                if in_simple_content && !value_field_pushed {
                                    value_field_pushed = true;
                                    fields.push(value_field(&base, target_ns, prefixes));
                                }
                            }
                        }
                        "group" => {
                            // Named model group particle; expanded post-parse.
                            if let Some(r) = get_attr_value(e, "ref") {
                                group_refs
                                    .push((fields.len(), resolve_qname(&r, target_ns, prefixes)));
                            }
                            skip_subtree(reader)?;
                            depth -= 1;
                        }
                        "sequence" | "all" => {
                            let is_unbounded = get_attr_value(e, "maxOccurs")
                                .map(|v| {
                                    v == "unbounded"
                                        || v.parse::<u32>().map(|n| n > 1).unwrap_or(false)
                                })
                                .unwrap_or(false);
                            let min_occurs = get_attr_value(e, "minOccurs")
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(1);
                            compositor_stack.push(CompositorFrame {
                                kind: if local == "sequence" {
                                    CompositorKind::Sequence
                                } else {
                                    CompositorKind::All
                                },
                                is_unbounded,
                                min_occurs,
                                fields_start: fields.len(),
                                choice_branches: Vec::new(),
                                has_sequence_branch: false,
                            });
                        }
                        "choice" => {
                            let is_unbounded = get_attr_value(e, "maxOccurs")
                                .map(|v| {
                                    v == "unbounded"
                                        || v.parse::<u32>().map(|n| n > 1).unwrap_or(false)
                                })
                                .unwrap_or(false);
                            let min_occurs = get_attr_value(e, "minOccurs")
                                .and_then(|v| v.parse::<usize>().ok())
                                .unwrap_or(1);
                            compositor_stack.push(CompositorFrame {
                                kind: CompositorKind::Choice,
                                is_unbounded,
                                min_occurs,
                                fields_start: fields.len(),
                                choice_branches: Vec::new(),
                                has_sequence_branch: false,
                            });
                        }
                        "element" => {
                            let in_unbounded = compositor_stack.iter().any(|c| c.is_unbounded);
                            let in_choice = compositor_stack
                                .last()
                                .map(|c| c.kind == CompositorKind::Choice)
                                .unwrap_or(false);
                            let in_any_choice = compositor_stack
                                .iter()
                                .any(|c| c.kind == CompositorKind::Choice);
                            if let Some(mut field) = parse_element_field(
                                e,
                                target_ns,
                                prefixes,
                                in_any_choice,
                                in_unbounded,
                            ) {
                                // Consume inline type definitions so nested
                                // fields cannot leak into the parent struct;
                                // extracted types are registered in `ir`.
                                self.consume_inline_element_type(
                                    reader, e, target_ns, prefixes, &name, &mut field, ir,
                                )?;
                                depth -= 1;
                                if in_choice {
                                    if let Some(frame) = compositor_stack.last_mut() {
                                        if frame.kind == CompositorKind::Choice {
                                            frame.choice_branches.push(UnionBranch {
                                                variant_name: field.name.clone(),
                                                xml_name: field.xml_name.clone(),
                                                namespace: field.namespace.clone(),
                                                type_ref: field.type_ref.clone(),
                                                documentation: field.documentation.clone(),
                                            });
                                        }
                                    }
                                }
                                fields.push(field);
                            }
                        }
                        "attribute" => {
                            if let Some(field) = parse_attribute_field(e, target_ns, prefixes) {
                                fields.push(field);
                            }
                        }
                        "any" => {
                            let in_unbounded = compositor_stack.iter().any(|c| c.is_unbounded);
                            let in_choice = compositor_stack
                                .last()
                                .map(|c| c.kind == CompositorKind::Choice)
                                .unwrap_or(false);
                            let field = parse_any_field(e, in_choice, in_unbounded);
                            if in_choice {
                                if let Some(frame) = compositor_stack.last_mut() {
                                    if frame.kind == CompositorKind::Choice {
                                        frame.choice_branches.push(UnionBranch {
                                            variant_name: field.name.clone(),
                                            xml_name: field.xml_name.clone(),
                                            namespace: field.namespace.clone(),
                                            type_ref: field.type_ref.clone(),
                                            documentation: field.documentation.clone(),
                                        });
                                    }
                                }
                            }
                            fields.push(field);
                        }
                        "anyAttribute"
                            if !fields.iter().any(|f| f.kind == FieldKind::AnyAttribute) =>
                        {
                            fields.push(parse_any_attribute_field(e));
                        }
                        _ => {}
                    }
                }
                Event::Empty(ref e) => {
                    let local = strip_prefix(e.name().into_inner());

                    match local {
                        "extension" => {
                            if let Some(base) = get_attr_value(e, "base") {
                                let resolved = resolve_qname(&base, target_ns, prefixes);
                                if resolved != qname {
                                    base_type = Some(resolved);
                                }
                                // simpleContent extension without children.
                                if in_simple_content && !value_field_pushed {
                                    value_field_pushed = true;
                                    fields.push(value_field(&base, target_ns, prefixes));
                                }
                            }
                        }
                        "group" => {
                            // Self-closing group reference; expanded post-parse.
                            if let Some(r) = get_attr_value(e, "ref") {
                                group_refs
                                    .push((fields.len(), resolve_qname(&r, target_ns, prefixes)));
                            }
                        }
                        "element" => {
                            let in_unbounded = compositor_stack.iter().any(|c| c.is_unbounded);
                            let in_choice = compositor_stack
                                .last()
                                .map(|c| c.kind == CompositorKind::Choice)
                                .unwrap_or(false);
                            let in_any_choice = compositor_stack
                                .iter()
                                .any(|c| c.kind == CompositorKind::Choice);
                            if let Some(field) = parse_element_field(
                                e,
                                target_ns,
                                prefixes,
                                in_any_choice,
                                in_unbounded,
                            ) {
                                if in_choice {
                                    if let Some(frame) = compositor_stack.last_mut() {
                                        if frame.kind == CompositorKind::Choice {
                                            frame.choice_branches.push(UnionBranch {
                                                variant_name: field.name.clone(),
                                                xml_name: field.xml_name.clone(),
                                                namespace: field.namespace.clone(),
                                                type_ref: field.type_ref.clone(),
                                                documentation: field.documentation.clone(),
                                            });
                                        }
                                    }
                                }
                                fields.push(field);
                            }
                        }
                        "attribute" => {
                            if let Some(field) = parse_attribute_field(e, target_ns, prefixes) {
                                fields.push(field);
                            }
                        }
                        "any" => {
                            let in_unbounded = compositor_stack.iter().any(|c| c.is_unbounded);
                            let in_choice = compositor_stack
                                .last()
                                .map(|c| c.kind == CompositorKind::Choice)
                                .unwrap_or(false);
                            let field = parse_any_field(e, in_choice, in_unbounded);
                            if in_choice {
                                if let Some(frame) = compositor_stack.last_mut() {
                                    if frame.kind == CompositorKind::Choice {
                                        frame.choice_branches.push(UnionBranch {
                                            variant_name: field.name.clone(),
                                            xml_name: field.xml_name.clone(),
                                            namespace: field.namespace.clone(),
                                            type_ref: field.type_ref.clone(),
                                            documentation: field.documentation.clone(),
                                        });
                                    }
                                }
                            }
                            fields.push(field);
                        }
                        "anyAttribute"
                            if !fields.iter().any(|f| f.kind == FieldKind::AnyAttribute) =>
                        {
                            fields.push(parse_any_attribute_field(e));
                        }
                        _ => {}
                    }
                }
                Event::End(ref e) => {
                    let local = strip_prefix(e.name().into_inner());
                    if local == "sequence" || local == "choice" || local == "all" {
                        if let Some(frame) = compositor_stack.pop() {
                            if frame.kind == CompositorKind::Sequence {
                                if frame.is_unbounded
                                    && fields[frame.fields_start..]
                                        .iter()
                                        .filter(|f| f.kind == FieldKind::Element)
                                        .count()
                                        > 1
                                {
                                    ir.ordered_types.insert(qname.clone());
                                }
                                if let Some(parent) = compositor_stack.last_mut() {
                                    if parent.kind == CompositorKind::Choice {
                                        parent.has_sequence_branch = true;
                                        let seq_fields = fields[frame.fields_start..].to_vec();
                                        if !seq_fields.is_empty() {
                                            let seq_name = unique_type_name(
                                                ir,
                                                target_ns,
                                                &format!("{}Sequence", name),
                                            );
                                            let seq_qname = QName::new(target_ns, seq_name);
                                            ir.add_type(TypeDef::Struct(StructDef {
                                                qname: seq_qname.clone(),
                                                base_type: None,
                                                is_abstract: false,
                                                is_mixed: false,
                                                fields: seq_fields,
                                                documentation: None,
                                            }));
                                            let branch_variant = if let Some(first_field) =
                                                fields.get(frame.fields_start)
                                            {
                                                format!(
                                                    "{}Sequence",
                                                    sanitize_field_name(&first_field.name)
                                                )
                                            } else {
                                                "Sequence".to_string()
                                            };
                                            let branch_xml = if let Some(first_field) =
                                                fields.get(frame.fields_start)
                                            {
                                                first_field.xml_name.clone()
                                            } else {
                                                String::new()
                                            };
                                            parent.choice_branches.push(UnionBranch {
                                                variant_name: branch_variant,
                                                xml_name: branch_xml,
                                                namespace: target_ns.map(Into::into),
                                                type_ref: TypeRef::Named(seq_qname),
                                                documentation: None,
                                            });
                                        }
                                    }
                                }
                            } else if frame.kind == CompositorKind::Choice {
                                let choice_is_unbounded = frame.is_unbounded
                                    || compositor_stack.iter().any(|c| c.is_unbounded);
                                let is_nested_choice = !compositor_stack.is_empty();
                                let needs_fields = fields[frame.fields_start..]
                                    .iter()
                                    .any(|f| f.cardinality.is_list())
                                    || frame.has_sequence_branch
                                    || (!choice_is_unbounded
                                        && is_nested_choice
                                        && ir.content_models.contains_key(&qname));
                                if (choice_is_unbounded
                                    || is_nested_choice
                                    || (compositor_stack.is_empty() && base_type.is_some()))
                                    && !frame.choice_branches.is_empty()
                                    && (!needs_fields || choice_is_unbounded)
                                {
                                    let choice_name =
                                        unique_type_name(ir, target_ns, &format!("{}Choice", name));
                                    let choice_qname = QName::new(target_ns, choice_name);
                                    let choice_def = UnionDef {
                                        qname: choice_qname.clone(),
                                        branches: frame.choice_branches,
                                        documentation: None,
                                    };
                                    ir.add_type(TypeDef::Union(choice_def));

                                    fields.truncate(frame.fields_start);
                                    let field_base = if choice_is_unbounded {
                                        "items"
                                    } else {
                                        "choice"
                                    };
                                    let mut item_field_name = field_base.to_string();
                                    let mut counter = 2;
                                    while fields.iter().any(|f| f.name == item_field_name) {
                                        item_field_name = format!("{field_base}_{counter}");
                                        counter += 1;
                                    }
                                    fields.push(FieldDef {
                                        name: item_field_name,
                                        xml_name: String::new(),
                                        namespace: None,
                                        kind: FieldKind::Element,
                                        type_ref: TypeRef::Named(choice_qname),
                                        cardinality: if choice_is_unbounded {
                                            Cardinality::unbounded(frame.min_occurs)
                                        } else if frame.min_occurs == 0 {
                                            Cardinality::optional_one()
                                        } else {
                                            Cardinality::required_one()
                                        },
                                        nillable: false,
                                        default_value: None,
                                        fixed_value: None,
                                        documentation: None,
                                        facets: None,
                                        is_cycle_cut: false,
                                    });
                                } else if !choice_is_unbounded && compositor_stack.is_empty() {
                                    top_level_choice_branches = if needs_fields {
                                        Vec::new()
                                    } else {
                                        frame.choice_branches
                                    };
                                    top_level_choice_fields_start = frame.fields_start;
                                    top_level_choice_fields_count =
                                        fields.len() - frame.fields_start;
                                    is_top_level_choice = true;
                                }
                            }
                        }
                    }
                    depth -= 1;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        // Group references inside a bounded choice are not spliceable into a
        // union's branch list, so keep such types as structs.
        let is_union = !is_mixed
            && is_top_level_choice
            && !top_level_choice_branches.is_empty()
            && group_refs.is_empty()
            && top_level_choice_fields_start == 0
            && fields.len() == top_level_choice_fields_count;

        // Record group refs for post-parse expansion.
        if !is_union {
            for (at, group) in group_refs {
                self.pending_group_refs.push(PendingGroupRef {
                    owner: qname.clone(),
                    at,
                    group,
                });
            }
        }

        if is_union {
            Ok(Some(TypeDef::Union(UnionDef {
                qname,
                branches: top_level_choice_branches,
                documentation,
            })))
        } else {
            // The same expanded element name may occur in exclusive sequence
            // branches. A single optional field binds either occurrence.
            let mut merged: Vec<FieldDef> = Vec::new();
            for field in fields {
                if let Some(existing) = merged.iter_mut().find(|f| {
                    f.cardinality.is_optional()
                        && field.cardinality.is_optional()
                        && f.kind == FieldKind::Element
                        && field.kind == FieldKind::Element
                        && f.xml_name == field.xml_name
                        && f.namespace == field.namespace
                        && f.type_ref == field.type_ref
                }) {
                    existing.cardinality.min_occurs = existing
                        .cardinality
                        .min_occurs
                        .min(field.cardinality.min_occurs);
                } else {
                    merged.push(field);
                }
            }
            Ok(Some(TypeDef::Struct(StructDef {
                qname,
                base_type,
                is_abstract,
                is_mixed,
                fields: merged,
                documentation,
            })))
        }
    }

    /// Consume an `<xs:element>` start tag's subtree. If it carries an inline
    /// `complexType`/`simpleType` definition (and no `type`/`ref` attribute),
    /// extract that definition as a uniquely named type and point `field` at
    /// it. Otherwise the subtree is skipped so its content can never leak into
    /// the enclosing struct.
    #[allow(clippy::too_many_arguments)]
    fn consume_inline_element_type(
        &mut self,
        reader: &mut Reader<&[u8]>,
        element_start: &BytesStart,
        target_ns: Option<&str>,
        prefixes: &HashMap<String, String>,
        parent_local: &str,
        field: &mut FieldDef,
        ir: &mut SchemaIR,
    ) -> Result<(), SchemaError> {
        if get_attr_value(element_start, "type").is_some()
            || get_attr_value(element_start, "ref").is_some()
        {
            // Type already specified — discard (illegal) inline content.
            skip_subtree(reader)?;
            return Ok(());
        }

        let elem_local = get_attr_value(element_start, "name")
            .or_else(|| get_attr_value(element_start, "ref").map(|r| strip_prefix(&r).to_string()))
            .unwrap_or_else(|| field.name.clone());

        let mut extracted = false;
        let mut depth = 1usize;
        let mut buf = Vec::new();
        while depth > 0 {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    let local = strip_prefix(e.name().into_inner());
                    if !extracted && local == "complexType" {
                        let unique = unique_type_name(
                            ir,
                            target_ns,
                            &format!("{}{}Type", parent_local, elem_local),
                        );
                        if let Some(type_def) = self.parse_complex_type(
                            reader,
                            e,
                            target_ns,
                            prefixes,
                            Some(unique),
                            ir,
                        )? {
                            let q = type_def.qname().clone();
                            ir.add_type(type_def);
                            field.type_ref = TypeRef::Named(q);
                            extracted = true;
                        }
                    } else if !extracted && local == "simpleType" {
                        let unique = unique_type_name(
                            ir,
                            target_ns,
                            &format!("{}{}SimpleType", parent_local, elem_local),
                        );
                        if let Some(type_def) = self.parse_simple_type(
                            reader,
                            e,
                            target_ns,
                            prefixes,
                            Some(unique),
                            ir,
                        )? {
                            let q = type_def.qname().clone();
                            ir.add_type(type_def);
                            field.type_ref = TypeRef::Named(q);
                            extracted = true;
                        }
                    } else {
                        depth += 1;
                    }
                }
                Event::End(_) => depth -= 1,
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        Ok(())
    }

    /// Parse the body of a named `<xs:group>` definition: its element
    /// particles plus nested group references.
    fn parse_group_body(
        &mut self,
        reader: &mut Reader<&[u8]>,
        target_ns: Option<&str>,
        prefixes: &HashMap<String, String>,
        ir: &mut SchemaIR,
    ) -> Result<GroupDef, SchemaError> {
        let mut def = GroupDef::default();
        let mut compositor_stack: Vec<bool> = Vec::new();
        let mut in_choice = false;
        let mut depth = 1usize;
        let mut buf = Vec::new();

        while depth > 0 {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    depth += 1;
                    let local = strip_prefix(e.name().into_inner());
                    match local {
                        "documentation" => {
                            let _text = reader.read_text(e.name())?.to_string();
                            depth -= 1;
                        }
                        "sequence" | "all" | "choice" => {
                            if local == "choice" {
                                in_choice = true;
                            }
                            let is_unbounded = get_attr_value(e, "maxOccurs")
                                .map(|v| {
                                    v == "unbounded"
                                        || v.parse::<u32>().map(|n| n > 1).unwrap_or(false)
                                })
                                .unwrap_or(false);
                            compositor_stack.push(is_unbounded);
                        }
                        "element" => {
                            let in_unbounded = compositor_stack.iter().any(|&b| b);
                            if let Some(mut field) =
                                parse_element_field(e, target_ns, prefixes, in_choice, in_unbounded)
                            {
                                self.consume_inline_element_type(
                                    reader, e, target_ns, prefixes, "", &mut field, ir,
                                )?;
                                depth -= 1;
                                def.fields.push(field);
                            }
                        }
                        "group" => {
                            if let Some(r) = get_attr_value(e, "ref") {
                                def.group_refs.push((
                                    def.fields.len(),
                                    resolve_qname(&r, target_ns, prefixes),
                                ));
                            }
                            skip_subtree(reader)?;
                            depth -= 1;
                        }
                        "any" => {
                            let in_unbounded = compositor_stack.iter().any(|&b| b);
                            def.fields.push(parse_any_field(e, in_choice, in_unbounded));
                        }
                        _ => {}
                    }
                }
                Event::Empty(ref e) => {
                    let local = strip_prefix(e.name().into_inner());
                    match local {
                        "element" => {
                            let in_unbounded = compositor_stack.iter().any(|&b| b);
                            if let Some(field) =
                                parse_element_field(e, target_ns, prefixes, in_choice, in_unbounded)
                            {
                                def.fields.push(field);
                            }
                        }
                        "group" => {
                            if let Some(r) = get_attr_value(e, "ref") {
                                def.group_refs.push((
                                    def.fields.len(),
                                    resolve_qname(&r, target_ns, prefixes),
                                ));
                            }
                        }
                        "any" => {
                            let in_unbounded = compositor_stack.iter().any(|&b| b);
                            def.fields.push(parse_any_field(e, in_choice, in_unbounded));
                        }
                        _ => {}
                    }
                }
                Event::End(ref e) => {
                    let local = strip_prefix(e.name().into_inner());
                    if local == "sequence" || local == "choice" || local == "all" {
                        compositor_stack.pop();
                    }
                    depth -= 1;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }
        Ok(def)
    }

    fn parse_simple_type(
        &self,
        reader: &mut Reader<&[u8]>,
        start: &BytesStart,
        target_ns: Option<&str>,
        prefixes: &HashMap<String, String>,
        name_override: Option<String>,
        ir: &mut SchemaIR,
    ) -> Result<Option<TypeDef>, SchemaError> {
        let name = match get_attr_value(start, "name").or(name_override) {
            Some(n) => n,
            None => return Ok(None),
        };

        let qname = QName::new(target_ns, name);
        let mut base_type = TypeRef::string();
        let mut facets = RestrictionFacets::default();
        let mut enum_values = Vec::new();
        let mut union_branches: Option<Vec<UnionBranch>> = None;
        let mut documentation = None;
        let mut buf = Vec::new();

        let mut depth = 1;
        while depth > 0 {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    depth += 1;
                    let local = strip_prefix(e.name().into_inner());

                    match local {
                        "documentation" => {
                            let text = reader.read_text(e.name())?.to_string();
                            documentation = Some(text);
                            depth -= 1;
                        }
                        "restriction" => {
                            if let Some(base) = get_attr_value(e, "base") {
                                base_type = resolve_type_ref(&base, target_ns, prefixes);
                            }
                        }
                        "union" => {
                            union_branches = Some(parse_union_members(e, target_ns, prefixes));
                        }
                        "simpleType" if union_branches.is_some() => {
                            let branch_index = union_branches.as_ref().unwrap().len() + 1;
                            let inline_name = unique_type_name(
                                ir,
                                target_ns,
                                &format!("{}Member{}", qname.local, branch_index),
                            );
                            if let Some(inner) = self.parse_simple_type(
                                reader,
                                e,
                                target_ns,
                                prefixes,
                                Some(inline_name),
                                ir,
                            )? {
                                let inner_qname = inner.qname().clone();
                                ir.add_type(inner);
                                union_branches.as_mut().unwrap().push(UnionBranch {
                                    variant_name: format!("{}Value", inner_qname.local),
                                    xml_name: String::new(),
                                    namespace: None,
                                    type_ref: TypeRef::Named(inner_qname),
                                    documentation: None,
                                });
                            }
                            depth -= 1;
                        }
                        "pattern" => {
                            if let Some(val) = get_attr_value(e, "value") {
                                facets.patterns.push(val);
                            }
                        }
                        "enumeration" => {
                            if let Some(val) = get_attr_value(e, "value") {
                                enum_values.push(EnumValue {
                                    name: sanitize_variant_name(&val),
                                    value: val.clone(),
                                    documentation: None,
                                });
                                facets.enumerations.push(val);
                            }
                        }
                        _ => {}
                    }
                }
                Event::Empty(ref e) => {
                    let local = strip_prefix(e.name().into_inner());

                    match local {
                        "restriction" => {
                            if let Some(base) = get_attr_value(e, "base") {
                                base_type = resolve_type_ref(&base, target_ns, prefixes);
                            }
                        }
                        "union" => {
                            union_branches = Some(parse_union_members(e, target_ns, prefixes));
                        }
                        "enumeration" => {
                            if let Some(val) = get_attr_value(e, "value") {
                                enum_values.push(EnumValue {
                                    name: sanitize_variant_name(&val),
                                    value: val.clone(),
                                    documentation: None,
                                });
                                facets.enumerations.push(val);
                            }
                        }
                        "pattern" => {
                            if let Some(val) = get_attr_value(e, "value") {
                                facets.patterns.push(val);
                            }
                        }
                        "minInclusive" => {
                            facets.min_inclusive = get_attr_value(e, "value");
                        }
                        "maxInclusive" => {
                            facets.max_inclusive = get_attr_value(e, "value");
                        }
                        "minExclusive" => {
                            facets.min_exclusive = get_attr_value(e, "value");
                        }
                        "maxExclusive" => {
                            facets.max_exclusive = get_attr_value(e, "value");
                        }
                        "minLength" => {
                            facets.min_length =
                                get_attr_value(e, "value").and_then(|v| v.parse().ok());
                        }
                        "maxLength" => {
                            facets.max_length =
                                get_attr_value(e, "value").and_then(|v| v.parse().ok());
                        }
                        "length" => {
                            facets.length = get_attr_value(e, "value").and_then(|v| v.parse().ok());
                        }
                        "totalDigits" => {
                            facets.total_digits =
                                get_attr_value(e, "value").and_then(|v| v.parse().ok());
                        }
                        "fractionDigits" => {
                            facets.fraction_digits =
                                get_attr_value(e, "value").and_then(|v| v.parse().ok());
                        }
                        "whiteSpace" => {
                            facets.white_space = get_attr_value(e, "value");
                        }
                        _ => {}
                    }
                }
                Event::End(_) => {
                    depth -= 1;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        // XSD same-restriction alternatives form ONE pattern facet. Preserve
        // each derivation boundary as a separate entry during inheritance.
        if facets.patterns.len() > 1 {
            let alternatives = facets
                .patterns
                .iter()
                .map(|p| format!("({p})"))
                .collect::<Vec<_>>()
                .join("|");
            facets.patterns = vec![alternatives];
        }

        // Drop duplicate enumeration values (keep the first
        // occurrence) and disambiguate variant names that collide after
        // sanitization, so generated enums always compile.
        if !enum_values.is_empty() {
            let mut seen_values = HashSet::new();
            enum_values.retain(|v| seen_values.insert(v.value.clone()));

            let mut seen_names = HashSet::new();
            for v in &mut enum_values {
                if !seen_names.insert(v.name.clone()) {
                    let base = v.name.clone();
                    let mut n = 2u32;
                    let mut candidate = format!("{base}{n}");
                    while !seen_names.insert(candidate.clone()) {
                        n += 1;
                        candidate = format!("{base}{n}");
                    }
                    v.name = candidate;
                }
            }
        }

        if let Some(mut branches) = union_branches {
            if branches.is_empty() {
                return Err(SchemaError::Malformed(format!(
                    "xs:union '{}' has no member types",
                    qname.local
                )));
            }
            let mut seen = HashSet::new();
            for branch in &mut branches {
                let base = branch.variant_name.clone();
                let mut name = base.clone();
                let mut suffix = 2;
                while !seen.insert(name.clone()) {
                    name = format!("{base}{suffix}");
                    suffix += 1;
                }
                branch.variant_name = name;
            }
            return Ok(Some(TypeDef::Union(UnionDef {
                qname,
                branches,
                documentation,
            })));
        }

        if !enum_values.is_empty() {
            Ok(Some(TypeDef::Enum(EnumDef {
                qname,
                base_type,
                variants: enum_values,
                documentation,
            })))
        } else {
            Ok(Some(TypeDef::Simple(Box::new(SimpleTypeDef {
                qname,
                base_type,
                facets,
                documentation,
            }))))
        }
    }

    fn parse_global_element(
        &mut self,
        reader: &mut Reader<&[u8]>,
        start: &BytesStart,
        target_ns: Option<&str>,
        prefixes: &HashMap<String, String>,
        ir: &mut SchemaIR,
    ) -> Result<Option<ElementDef>, SchemaError> {
        let name = match get_attr_value(start, "name") {
            Some(n) => n,
            None => return Ok(None),
        };

        let qname = QName::new(target_ns, name.clone());
        let substitution_group = get_attr_value(start, "substitutionGroup")
            .map(|s| resolve_qname(&s, target_ns, prefixes));
        let nillable = get_attr_value(start, "nillable")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let mut type_ref = get_attr_value(start, "type")
            .map(|t| resolve_type_ref(&t, target_ns, prefixes))
            .unwrap_or(TypeRef::Primitive(PrimitiveType::AnyType));

        let mut documentation = None;
        let mut buf = Vec::new();

        let mut depth = 1;
        while depth > 0 {
            match reader.read_event_into(&mut buf)? {
                Event::Start(ref e) => {
                    depth += 1;
                    let local = strip_prefix(e.name().into_inner());

                    match local {
                        "documentation" => {
                            let text = reader.read_text(e.name())?.to_string();
                            documentation = Some(text);
                            depth -= 1;
                        }
                        "complexType" => {
                            // Unique naming guards against `{name}Type`
                            // colliding with an existing type.
                            let anon_name =
                                unique_type_name(ir, target_ns, &format!("{}Type", name));
                            let anon_qname = QName::new(target_ns, anon_name.clone());
                            if let Some(type_def) = self.parse_complex_type(
                                reader,
                                e,
                                target_ns,
                                prefixes,
                                Some(anon_name),
                                ir,
                            )? {
                                match type_def {
                                    TypeDef::Struct(mut s) => {
                                        s.qname = anon_qname.clone();
                                        ir.add_type(TypeDef::Struct(s));
                                    }
                                    TypeDef::Union(mut u) => {
                                        u.qname = anon_qname.clone();
                                        ir.add_type(TypeDef::Union(u));
                                    }
                                    _ => {}
                                }
                                type_ref = TypeRef::Named(anon_qname);
                            }
                            depth -= 1;
                        }
                        "simpleType" => {
                            let anon_name =
                                unique_type_name(ir, target_ns, &format!("{}SimpleType", name));
                            let anon_qname = QName::new(target_ns, anon_name.clone());
                            if let Some(type_def) = self.parse_simple_type(
                                reader,
                                e,
                                target_ns,
                                prefixes,
                                Some(anon_name),
                                ir,
                            )? {
                                match type_def {
                                    TypeDef::Enum(mut ed) => {
                                        ed.qname = anon_qname.clone();
                                        ir.add_type(TypeDef::Enum(ed));
                                    }
                                    TypeDef::Simple(mut sd) => {
                                        sd.qname = anon_qname.clone();
                                        ir.add_type(TypeDef::Simple(sd));
                                    }
                                    TypeDef::Union(mut union) => {
                                        union.qname = anon_qname.clone();
                                        ir.add_type(TypeDef::Union(union));
                                    }
                                    _ => {}
                                }
                                type_ref = TypeRef::Named(anon_qname);
                            }
                            depth -= 1;
                        }
                        _ => {}
                    }
                }
                Event::End(_) => {
                    depth -= 1;
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        Ok(Some(ElementDef {
            qname,
            type_ref,
            substitution_group,
            nillable,
            documentation,
        }))
    }

    /// Splice referenced group fields into their owner structs. Runs at the
    /// end of every parse frame; entries that cannot be resolved yet are
    /// carried to the parent frame (dropped at the root).
    fn expand_group_refs(&mut self, ir: &mut SchemaIR, root: bool) {
        let pending = std::mem::take(&mut self.pending_group_refs);
        let mut carried: Vec<PendingGroupRef> = Vec::new();
        let mut deltas: HashMap<QName, usize> = HashMap::new();

        for p in pending {
            let resolved = {
                let mut visiting = HashSet::new();
                self.group_fields(&p.group, &mut visiting)
            };
            let Some(gfields) = resolved else {
                if !root {
                    carried.push(p);
                }
                continue;
            };

            let Some(TypeDef::Struct(owner)) = ir.types.get_mut(&p.owner) else {
                if !root {
                    carried.push(p);
                }
                continue;
            };
            let delta = deltas.get(&p.owner).copied().unwrap_or(0);
            let at = (p.at + delta).min(owner.fields.len());
            let before = owner.fields.len();
            for (i, f) in gfields.into_iter().enumerate() {
                owner.fields.insert(at + i, f);
            }
            let added = owner.fields.len() - before;
            *deltas.entry(p.owner.clone()).or_insert(0) += added;
        }

        self.pending_group_refs = carried;
    }

    /// Expand a group's fields, recursively splicing nested group refs.
    /// Returns `None` if the group (or a nested group) is not yet defined.
    fn group_fields(&self, gq: &QName, visiting: &mut HashSet<QName>) -> Option<Vec<FieldDef>> {
        if !visiting.insert(gq.clone()) {
            // Illegal group cycle — cut rather than recurse forever.
            return Some(Vec::new());
        }
        let def = self.groups.get(gq)?;
        let mut fields = def.fields.clone();
        let refs = def.group_refs.clone();
        let mut delta = 0usize;
        for (at, sub) in refs {
            let sub_fields = self.group_fields(&sub, visiting)?;
            let at = (at + delta).min(fields.len());
            let before = fields.len();
            for (i, f) in sub_fields.into_iter().enumerate() {
                fields.insert(at + i, f);
            }
            delta += fields.len() - before;
        }
        Some(fields)
    }

    /// Re-namespace parser state created during a chameleon include: newly
    /// registered groups and carried group refs that still carry no namespace
    /// adopt the includer's namespace.
    fn rekey_new_state(&mut self, groups_before: &HashSet<QName>, pend_before: usize, ns: &str) {
        let stale: Vec<QName> = self
            .groups
            .keys()
            .filter(|q| q.namespace.is_none() && !groups_before.contains(*q))
            .cloned()
            .collect();
        for old in stale {
            if let Some(mut def) = self.groups.remove(&old) {
                rekey_group_def(&mut def, ns);
                self.groups
                    .insert(QName::new(Some(ns.to_string()), old.local), def);
            }
        }
        for p in self.pending_group_refs.iter_mut().skip(pend_before) {
            if p.owner.namespace.is_none() {
                p.owner = QName::new(Some(ns.to_string()), p.owner.local.clone());
            }
            if p.group.namespace.is_none() {
                p.group = QName::new(Some(ns.to_string()), p.group.local.clone());
            }
        }
    }
}

/// Replace a mixed complex type's child fields with one ordered item stream.
/// The union branches retain the element names and types for all generators.
fn expand_substitution_fields(ir: &mut SchemaIR) {
    let types = ir.types.keys().cloned().collect::<Vec<_>>();
    for qname in types {
        let Some(TypeDef::Struct(snapshot)) = ir.types.get(&qname).cloned() else {
            continue;
        };
        let mut fields = Vec::new();
        let mut expanded = false;
        for field in snapshot.fields {
            let head = QName::new(field.namespace.clone(), strip_prefix(&field.xml_name));
            fields.push(field.clone());
            if field.kind != FieldKind::Element {
                continue;
            }
            let mut pending = ir
                .substitution_groups
                .get(&head)
                .cloned()
                .unwrap_or_default();
            let mut seen = HashSet::new();
            while let Some(member) = pending.pop() {
                if !seen.insert(member.clone()) {
                    continue;
                }
                if let Some(children) = ir.substitution_groups.get(&member) {
                    pending.extend(children.iter().cloned());
                }
                if let Some(element) = ir.elements.get(&member) {
                    let mut replacement = field.clone();
                    replacement.name = sanitize_field_name(&member.local);
                    replacement.xml_name = member.local.clone();
                    replacement.namespace = member.namespace.clone();
                    replacement.type_ref = element.type_ref.clone();
                    replacement.nillable = element.nillable;
                    fields.push(replacement);
                    expanded = true;
                }
            }
        }
        if expanded {
            ir.ordered_types.insert(qname.clone());
            if let Some(TypeDef::Struct(structure)) = ir.types.get_mut(&qname) {
                structure.fields = fields;
            }
        }
    }
}

fn compile_mixed_types(ir: &mut SchemaIR) {
    let mixed_types = ir
        .types
        .values()
        .filter_map(|def| match def {
            TypeDef::Struct(s) if ir.has_ordered_content(s) => Some(s.qname.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for qname in mixed_types {
        let Some(TypeDef::Struct(snapshot)) = ir.types.get(&qname).cloned() else {
            continue;
        };
        if snapshot
            .fields
            .iter()
            .any(|field| field.xml_name.is_empty() && matches!(&field.type_ref, TypeRef::Named(item_type) if matches!(ir.types.get(item_type), Some(TypeDef::Union(union)) if union.is_mixed_content())))
        {
            continue;
        }
        let mut branches = vec![UnionBranch {
            variant_name: "Text".into(),
            xml_name: "#text".into(),
            namespace: None,
            type_ref: TypeRef::string(),
            documentation: None,
        }];
        for field in snapshot
            .fields
            .iter()
            .filter(|field| field.kind == FieldKind::Element)
        {
            if field.xml_name.is_empty() {
                if let TypeRef::Named(choice) = &field.type_ref {
                    if let Some(TypeDef::Union(union)) = ir.types.get(choice) {
                        branches.extend(union.branches.iter().cloned());
                    }
                }
            } else {
                branches.push(UnionBranch {
                    variant_name: field.name.clone(),
                    xml_name: field.xml_name.clone(),
                    namespace: field.namespace.clone(),
                    type_ref: field.type_ref.clone(),
                    documentation: field.documentation.clone(),
                });
            }
        }
        let union_name = unique_type_name(
            ir,
            qname.namespace.as_deref(),
            &format!("{}Item", qname.local),
        );
        let union_qname = QName::new(qname.namespace.as_deref(), union_name);
        ir.add_type(TypeDef::Union(UnionDef {
            qname: union_qname.clone(),
            branches,
            documentation: Some(format!("Ordered mixed content for {}", qname.local)),
        }));
        let mut items_name = "items".to_string();
        let mut suffix = 2;
        while snapshot.fields.iter().any(|field| field.name == items_name) {
            items_name = format!("items_{suffix}");
            suffix += 1;
        }
        if let Some(TypeDef::Struct(structure)) = ir.types.get_mut(&qname) {
            structure
                .fields
                .retain(|field| field.kind != FieldKind::Element);
            structure.fields.push(FieldDef {
                name: items_name,
                xml_name: String::new(),
                namespace: None,
                kind: FieldKind::Element,
                type_ref: TypeRef::Named(union_qname),
                cardinality: Cardinality::unbounded(0),
                nillable: false,
                default_value: None,
                fixed_value: None,
                documentation: Some("Text and child elements in document order".into()),
                facets: None,
                is_cycle_cut: false,
            });
        }
    }
}

fn parse_union_members(
    union: &BytesStart,
    target_ns: Option<&str>,
    prefixes: &HashMap<String, String>,
) -> Vec<UnionBranch> {
    let mut names = HashMap::<String, usize>::new();
    get_attr_value(union, "memberTypes")
        .unwrap_or_default()
        .split_whitespace()
        .map(|member| {
            let type_ref = resolve_type_ref(member, target_ns, prefixes);
            let base = match &type_ref {
                TypeRef::Primitive(primitive) => format!("{primitive:?}Value"),
                TypeRef::Named(qname) => format!("{}Value", qname.local),
                _ => "Member".into(),
            };
            let count = names.entry(base.clone()).or_insert(0);
            *count += 1;
            UnionBranch {
                variant_name: if *count == 1 {
                    base
                } else {
                    format!("{base}{count}")
                },
                xml_name: String::new(),
                namespace: None,
                type_ref,
                documentation: None,
            }
        })
        .collect()
}

/// Replace element-reference placeholders with their global element's declared type.
/// Referenced elements may be defined in an included file parsed later.
fn resolve_global_field_refs(ir: &mut SchemaIR) {
    let elements = &ir.elements;
    let attributes = &ir.attributes;
    let type_names: HashSet<QName> = ir.types.keys().cloned().collect();
    fn resolve(
        reference: &mut TypeRef,
        elements: &BTreeMap<QName, ElementDef>,
        types: &HashSet<QName>,
    ) {
        let TypeRef::Named(qname) = reference else {
            return;
        };
        if types.contains(qname) {
            return;
        }
        let mut current = qname.clone();
        let mut seen = HashSet::new();
        while seen.insert(current.clone()) {
            let Some(element) = elements.get(&current) else {
                break;
            };
            if let TypeRef::Named(next) = &element.type_ref {
                if !types.contains(next) && elements.contains_key(next) {
                    current = next.clone();
                    continue;
                }
            }
            *reference = element.type_ref.clone();
            break;
        }
    }
    for def in ir.types.values_mut() {
        match def {
            TypeDef::Struct(structure) => {
                for field in &mut structure.fields {
                    match field.kind {
                        FieldKind::Element => resolve(&mut field.type_ref, elements, &type_names),
                        FieldKind::Attribute => {
                            if let TypeRef::Named(qname) = &field.type_ref {
                                if !type_names.contains(qname) {
                                    if let Some(type_ref) = attributes.get(qname) {
                                        field.type_ref = type_ref.clone();
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            TypeDef::Union(union) if !union.is_lexical() => {
                for branch in &mut union.branches {
                    resolve(&mut branch.type_ref, elements, &type_names);
                }
            }
            _ => {}
        }
    }
}

/// Give XSD Gregorian primitives named, validated types in every target.
/// The original primitive remains the base of each synthesized simple type.
fn add_gregorian_types(ir: &mut SchemaIR) {
    const NS: &str = "urn:polyxml:builtins:gregorian";
    const TZ: &str = r"(?:Z|[+-](?:(?:0[0-9]|1[0-3]):[0-5][0-9]|14:00))?";
    fn pattern(kind: PrimitiveType) -> Option<String> {
        let body = match kind {
            PrimitiveType::GDay => r"---(?:0[1-9]|[12][0-9]|3[01])",
            PrimitiveType::GMonth => r"--(?:0[1-9]|1[0-2])",
            PrimitiveType::GYear => r"-?(?:[0-9]{4}|[1-9][0-9]{4,})",
            PrimitiveType::GYearMonth => r"-?(?:[0-9]{4}|[1-9][0-9]{4,})-(?:0[1-9]|1[0-2])",
            PrimitiveType::GMonthDay => {
                r"--(?:02-(?:0[1-9]|1[0-9]|2[0-9])|(?:04|06|09|11)-(?:0[1-9]|[12][0-9]|30)|(?:01|03|05|07|08|10|12)-(?:0[1-9]|[12][0-9]|3[01]))"
            }
            _ => return None,
        };
        Some(format!("{body}{TZ}"))
    }
    fn name(kind: PrimitiveType) -> &'static str {
        match kind {
            PrimitiveType::GDay => "GDay",
            PrimitiveType::GMonth => "GMonth",
            PrimitiveType::GYear => "GYear",
            PrimitiveType::GYearMonth => "GYearMonth",
            PrimitiveType::GMonthDay => "GMonthDay",
            _ => unreachable!(),
        }
    }
    fn rewrite(ty: &mut TypeRef, used: &mut HashSet<PrimitiveType>) {
        match ty {
            TypeRef::Primitive(kind) if pattern(*kind).is_some() => {
                used.insert(*kind);
                *ty = TypeRef::Named(QName::new(Some(NS), name(*kind)));
            }
            TypeRef::Boxed(inner) | TypeRef::List(inner) => rewrite(inner, used),
            _ => {}
        }
    }
    let mut used = HashSet::new();
    for element in ir.elements.values_mut() {
        rewrite(&mut element.type_ref, &mut used);
    }
    for def in ir.types.values_mut() {
        if def.qname().namespace.as_deref() == Some(NS) {
            continue;
        }
        match def {
            TypeDef::Struct(s) => {
                for field in &mut s.fields {
                    rewrite(&mut field.type_ref, &mut used);
                }
            }
            TypeDef::Enum(e) => rewrite(&mut e.base_type, &mut used),
            TypeDef::Union(u) => {
                for branch in &mut u.branches {
                    rewrite(&mut branch.type_ref, &mut used);
                }
            }
            TypeDef::Simple(s) => rewrite(&mut s.base_type, &mut used),
        }
    }
    for kind in used {
        let qname = QName::new(Some(NS), name(kind));
        ir.types.entry(qname.clone()).or_insert_with(|| {
            TypeDef::Simple(Box::new(SimpleTypeDef {
                qname,
                base_type: TypeRef::Primitive(kind),
                facets: RestrictionFacets {
                    patterns: vec![pattern(kind).unwrap()],
                    ..Default::default()
                },
                documentation: Some("Validated W3C Gregorian partial date".into()),
            }))
        });
    }
}

// Helpers

fn strip_prefix(s: &str) -> &str {
    s.split_once(':').map(|(_, local)| local).unwrap_or(s)
}

fn get_attr_value(e: &BytesStart, name: &str) -> Option<String> {
    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref();
        if key == name || strip_prefix(key) == name {
            return Some(attr.value.as_ref().to_string());
        }
    }
    None
}

fn resolve_qname(name: &str, target_ns: Option<&str>, prefixes: &HashMap<String, String>) -> QName {
    if let Some((prefix, local)) = name.split_once(':') {
        let ns = prefixes.get(prefix).cloned();
        QName::new(ns, local)
    } else {
        QName::new(target_ns, name)
    }
}

fn resolve_type_ref(
    name: &str,
    target_ns: Option<&str>,
    prefixes: &HashMap<String, String>,
) -> TypeRef {
    if let Some(prim) = PrimitiveType::from_xsd_name(name) {
        return TypeRef::Primitive(prim);
    }

    if let Some((prefix, local)) = name.split_once(':') {
        if prefix == "xs" || prefix == "xsd" {
            if let Some(prim) = PrimitiveType::from_xsd_name(local) {
                return TypeRef::Primitive(prim);
            }
        }
        let ns = prefixes.get(prefix).cloned();
        TypeRef::Named(QName::new(ns, local))
    } else {
        TypeRef::Named(QName::new(target_ns, name))
    }
}

fn parse_element_field(
    e: &BytesStart,
    target_ns: Option<&str>,
    prefixes: &HashMap<String, String>,
    in_choice: bool,
    in_unbounded_compositor: bool,
) -> Option<FieldDef> {
    let ref_attr = get_attr_value(e, "ref");
    let name_attr = get_attr_value(e, "name");

    let (name, xml_name, namespace) = if let Some(ref r) = ref_attr {
        let qname = resolve_qname(r, target_ns, prefixes);
        let field_name = name_attr.unwrap_or_else(|| qname.local.clone());
        let xml_name = qname.local;
        let namespace = qname.namespace;
        (field_name, xml_name, namespace)
    } else {
        let n = name_attr?;
        let xml_name = n.clone();
        let namespace = target_ns.map(Into::into);
        (n, xml_name, namespace)
    };

    let type_ref = get_attr_value(e, "type")
        .map(|t| resolve_type_ref(&t, target_ns, prefixes))
        .or_else(|| {
            ref_attr
                .as_ref()
                .map(|r| resolve_type_ref(r, target_ns, prefixes))
        })
        .unwrap_or(TypeRef::Primitive(PrimitiveType::String));

    let min_occurs = if in_choice {
        0
    } else {
        get_attr_value(e, "minOccurs")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1)
    };

    let max_occurs = if in_unbounded_compositor {
        OccursLimit::Unbounded
    } else {
        match get_attr_value(e, "maxOccurs").as_deref() {
            Some("unbounded") => OccursLimit::Unbounded,
            Some(v) => OccursLimit::Count(v.parse().unwrap_or(1)),
            None => OccursLimit::Count(1),
        }
    };

    let nillable = get_attr_value(e, "nillable")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let default_value = get_attr_value(e, "default");
    let fixed_value = get_attr_value(e, "fixed");

    Some(FieldDef {
        name: sanitize_field_name(&name),
        xml_name,
        namespace,
        kind: FieldKind::Element,
        type_ref,
        cardinality: Cardinality {
            min_occurs,
            max_occurs,
        },
        nillable,
        default_value,
        fixed_value,
        documentation: None,
        facets: None,
        is_cycle_cut: false,
    })
}

fn parse_attribute_field(
    e: &BytesStart,
    target_ns: Option<&str>,
    prefixes: &HashMap<String, String>,
) -> Option<FieldDef> {
    let ref_attr = get_attr_value(e, "ref");
    let name_attr = get_attr_value(e, "name");

    let (name, xml_name, namespace) = if let Some(ref r) = ref_attr {
        let qname = resolve_qname(r, target_ns, prefixes);
        let field_name = name_attr.unwrap_or_else(|| qname.local.clone());
        let xml_name = qname.local;
        let namespace = qname.namespace;
        (field_name, xml_name, namespace)
    } else {
        let n = name_attr?;
        let xml_name = n.clone();
        (n, xml_name, None)
    };

    let type_ref = get_attr_value(e, "type")
        .map(|t| resolve_type_ref(&t, target_ns, prefixes))
        .or_else(|| {
            ref_attr
                .as_ref()
                .map(|r| resolve_type_ref(r, target_ns, prefixes))
        })
        .unwrap_or(TypeRef::Primitive(PrimitiveType::String));

    let is_required = get_attr_value(e, "use")
        .map(|u| u == "required")
        .unwrap_or(false);

    let cardinality = if is_required {
        Cardinality::required_one()
    } else {
        Cardinality::optional_one()
    };

    let default_value = get_attr_value(e, "default");
    let fixed_value = get_attr_value(e, "fixed");

    Some(FieldDef {
        name: sanitize_field_name(&name),
        xml_name,
        namespace,
        kind: FieldKind::Attribute,
        type_ref,
        cardinality,
        nillable: false,
        default_value,
        fixed_value,
        documentation: None,
        facets: None,
        is_cycle_cut: false,
    })
}

fn parse_any_field(e: &BytesStart, in_choice: bool, in_unbounded_compositor: bool) -> FieldDef {
    let min_occurs = if in_choice {
        0
    } else {
        get_attr_value(e, "minOccurs")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1)
    };
    let max_occurs = if in_unbounded_compositor {
        OccursLimit::Unbounded
    } else {
        match get_attr_value(e, "maxOccurs").as_deref() {
            Some("unbounded") => OccursLimit::Unbounded,
            Some(v) => OccursLimit::Count(v.parse::<usize>().unwrap_or(1)),
            None => OccursLimit::Count(1),
        }
    };
    let namespace = get_attr_value(e, "namespace");
    FieldDef {
        name: "any".to_string(),
        xml_name: "*".to_string(),
        namespace,
        kind: FieldKind::Any,
        type_ref: TypeRef::Primitive(PrimitiveType::AnyType),
        cardinality: Cardinality {
            min_occurs,
            max_occurs,
        },
        nillable: false,
        default_value: None,
        fixed_value: None,
        documentation: None,
        facets: None,
        is_cycle_cut: false,
    }
}

fn parse_any_attribute_field(_e: &BytesStart) -> FieldDef {
    FieldDef {
        name: "any_attribute".to_string(),
        xml_name: "*".to_string(),
        namespace: None,
        kind: FieldKind::AnyAttribute,
        type_ref: TypeRef::Primitive(PrimitiveType::AnyType),
        cardinality: Cardinality::optional_one(),
        nillable: false,
        default_value: None,
        fixed_value: None,
        documentation: None,
        facets: None,
        is_cycle_cut: false,
    }
}

fn parse_empty_global_element(
    e: &BytesStart,
    target_ns: Option<&str>,
    prefixes: &HashMap<String, String>,
) -> Option<ElementDef> {
    let name = get_attr_value(e, "name")?;
    let qname = QName::new(target_ns, name);
    let substitution_group =
        get_attr_value(e, "substitutionGroup").map(|s| resolve_qname(&s, target_ns, prefixes));
    let nillable = get_attr_value(e, "nillable")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let type_ref = get_attr_value(e, "type")
        .map(|t| resolve_type_ref(&t, target_ns, prefixes))
        .unwrap_or(TypeRef::Primitive(PrimitiveType::AnyType));

    Some(ElementDef {
        qname,
        type_ref,
        substitution_group,
        nillable,
        documentation: None,
    })
}

fn sanitize_field_name(name: &str) -> String {
    let s = heck::AsSnakeCase(name).to_string();
    if s.is_empty() {
        "field".to_string()
    } else if s.chars().next().unwrap().is_ascii_digit() {
        format!("_{}", s)
    } else {
        s
    }
}

fn sanitize_variant_name(name: &str) -> String {
    let normalized = crate::codegen::normalize_symbol_name(name);
    let s = heck::AsPascalCase(&normalized).to_string();
    if s.is_empty() {
        "Variant".to_string()
    } else if s.chars().next().unwrap().is_ascii_digit() {
        format!("V{}", s)
    } else {
        s
    }
}

/// Consume events until the closing tag of the current element (whose
/// `Start` was already read). Discards content without interpreting it.
fn skip_subtree(reader: &mut Reader<&[u8]>) -> Result<(), SchemaError> {
    let mut depth = 1usize;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

/// The synthetic `<Text>` field representing a `simpleContent` value.
fn value_field(
    base: &str,
    target_ns: Option<&str>,
    prefixes: &HashMap<String, String>,
) -> FieldDef {
    FieldDef {
        name: "value".to_string(),
        xml_name: "value".to_string(),
        namespace: None,
        kind: FieldKind::Text,
        type_ref: resolve_type_ref(base, target_ns, prefixes),
        cardinality: Cardinality::required_one(),
        nillable: false,
        default_value: None,
        fixed_value: None,
        documentation: None,
        facets: None,
        is_cycle_cut: false,
    }
}

/// Pick a type name that does not collide with an existing type in `ir`:
/// `{base}`, `{base}2`, `{base}3`, ...
fn unique_type_name(ir: &SchemaIR, target_ns: Option<&str>, base: &str) -> String {
    let mut name = base.to_string();
    let mut n = 2u32;
    while ir.types.contains_key(&QName::new(target_ns, name.as_str())) {
        name = format!("{base}{n}");
        n += 1;
    }
    name
}

/// Rewrite every namespace-less QName in the IR to `ns`. Used for chameleon
/// includes, whose components adopt the including schema's target namespace.
fn rekey_to_namespace(ir: &mut SchemaIR, ns: &str) {
    ir.target_namespace = Some(ns.to_string());
    ir.content_models = std::mem::take(&mut ir.content_models)
        .into_iter()
        .map(|(mut name, mut model)| {
            if name.namespace.is_none() {
                name.namespace = Some(ns.into());
            }
            model.adopt_namespace(ns);
            (name, model)
        })
        .collect();
    ir.ordered_types = std::mem::take(&mut ir.ordered_types)
        .into_iter()
        .map(|mut q| {
            if q.namespace.is_none() {
                q.namespace = Some(ns.into());
            }
            q
        })
        .collect();

    let old_types = std::mem::take(&mut ir.types);
    let mut types = BTreeMap::new();
    for (mut k, mut v) in old_types {
        if k.namespace.is_none() {
            k = QName::new(Some(ns.to_string()), k.local);
        }
        rekey_type_def(&mut v, ns);
        types.insert(k, v);
    }
    ir.types = types;

    let old_elements = std::mem::take(&mut ir.elements);
    let mut elements = BTreeMap::new();
    for (mut k, mut v) in old_elements {
        if k.namespace.is_none() {
            k = QName::new(Some(ns.to_string()), k.local);
        }
        if v.qname.namespace.is_none() {
            v.qname = QName::new(Some(ns.to_string()), v.qname.local.clone());
        }
        rekey_type_ref(&mut v.type_ref, ns);
        if let Some(sg) = &mut v.substitution_group {
            if sg.namespace.is_none() {
                *sg = QName::new(Some(ns.to_string()), sg.local.clone());
            }
        }
        elements.insert(k, v);
    }
    ir.elements = elements;

    let old_attributes = std::mem::take(&mut ir.attributes);
    for (mut qname, mut type_ref) in old_attributes {
        if qname.namespace.is_none() {
            qname = QName::new(Some(ns.to_string()), qname.local);
        }
        rekey_type_ref(&mut type_ref, ns);
        ir.attributes.insert(qname, type_ref);
    }

    let old_subs = std::mem::take(&mut ir.substitution_groups);
    let mut subs = HashMap::new();
    for (k, v) in old_subs {
        let k = if k.namespace.is_none() {
            QName::new(Some(ns.to_string()), k.local)
        } else {
            k
        };
        let v = v
            .into_iter()
            .map(|q| {
                if q.namespace.is_none() {
                    QName::new(Some(ns.to_string()), q.local)
                } else {
                    q
                }
            })
            .collect();
        subs.insert(k, v);
    }
    ir.substitution_groups = subs;
}

fn rekey_type_ref(tr: &mut TypeRef, ns: &str) {
    match tr {
        TypeRef::Named(q) => {
            if q.namespace.is_none() {
                *q = QName::new(Some(ns.to_string()), q.local.clone());
            }
        }
        TypeRef::Boxed(inner) | TypeRef::List(inner) => rekey_type_ref(inner, ns),
        TypeRef::Primitive(_) => {}
    }
}

fn rekey_type_def(td: &mut TypeDef, ns: &str) {
    match td {
        TypeDef::Struct(s) => {
            if s.qname.namespace.is_none() {
                s.qname = QName::new(Some(ns.to_string()), s.qname.local.clone());
            }
            if let Some(b) = &mut s.base_type {
                if b.namespace.is_none() {
                    *b = QName::new(Some(ns.to_string()), b.local.clone());
                }
            }
            for f in &mut s.fields {
                rekey_field(f, ns);
            }
        }
        TypeDef::Enum(e) => {
            if e.qname.namespace.is_none() {
                e.qname = QName::new(Some(ns.to_string()), e.qname.local.clone());
            }
            rekey_type_ref(&mut e.base_type, ns);
        }
        TypeDef::Simple(st) => {
            if st.qname.namespace.is_none() {
                st.qname = QName::new(Some(ns.to_string()), st.qname.local.clone());
            }
            rekey_type_ref(&mut st.base_type, ns);
        }
        TypeDef::Union(u) => {
            if u.qname.namespace.is_none() {
                u.qname = QName::new(Some(ns.to_string()), u.qname.local.clone());
            }
            for b in &mut u.branches {
                // Branches are element particles: they carry the schema
                // target namespace, same as element fields.
                if b.namespace.is_none() {
                    b.namespace = Some(ns.to_string());
                }
                rekey_type_ref(&mut b.type_ref, ns);
            }
        }
    }
}

fn rekey_field(f: &mut FieldDef, ns: &str) {
    rekey_type_ref(&mut f.type_ref, ns);
    // Element fields normalize to the target namespace; attributes are
    // unqualified and wildcards/content stay namespace-less.
    if matches!(f.kind, FieldKind::Element) && f.namespace.is_none() {
        f.namespace = Some(ns.to_string());
    }
}

fn rekey_group_def(def: &mut GroupDef, ns: &str) {
    for f in &mut def.fields {
        rekey_field(f, ns);
    }
    for (_, g) in &mut def.group_refs {
        if g.namespace.is_none() {
            *g = QName::new(Some(ns.to_string()), g.local.clone());
        }
    }
}

/// Inherit pattern facets from base simple types down their derivation
/// chains: every derivation step's patterns must be
/// enforced together. Idempotent.
fn inherit_pattern_facets(ir: &mut SchemaIR) {
    let derived: Vec<(QName, QName)> = ir
        .types
        .iter()
        .filter_map(|(q, t)| match t {
            TypeDef::Simple(s) => match &s.base_type {
                TypeRef::Named(b) => Some((q.clone(), b.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect();

    for (q, base) in derived {
        let mut visited = HashSet::new();
        let inherited = collect_chain_patterns(ir, &base, &mut visited);
        if inherited.is_empty() {
            continue;
        }
        if let Some(TypeDef::Simple(s)) = ir.types.get_mut(&q) {
            for p in inherited {
                if !s.facets.patterns.contains(&p) {
                    s.facets.patterns.push(p);
                }
            }
        }
    }
}

/// Collect every pattern facet along a simple type's base chain
/// (base-first), with cycle protection.
fn collect_chain_patterns(ir: &SchemaIR, q: &QName, visited: &mut HashSet<QName>) -> Vec<String> {
    if !visited.insert(q.clone()) {
        return Vec::new();
    }
    match ir.types.get(q) {
        Some(TypeDef::Simple(s)) => {
            let mut out = match &s.base_type {
                TypeRef::Named(b) => collect_chain_patterns(ir, b, visited),
                _ => Vec::new(),
            };
            out.extend(s.facets.patterns.iter().cloned());
            out
        }
        _ => Vec::new(),
    }
}

fn merge_ir(dest: &mut SchemaIR, src: SchemaIR) {
    dest.ordered_types.extend(src.ordered_types);
    dest.content_models.extend(src.content_models);
    for (k, v) in src.types {
        dest.types.insert(k, v);
    }
    for (k, v) in src.elements {
        dest.elements.insert(k, v);
    }
    for (k, v) in src.attributes {
        dest.attributes.insert(k, v);
    }
    for (k, v) in src.substitution_groups {
        dest.substitution_groups.entry(k).or_default().extend(v);
    }
}

fn capture_content_model(
    reader: &Reader<&[u8]>,
    target_ns: Option<&str>,
    prefixes: &HashMap<String, String>,
) -> Result<Option<crate::ir::Particle>, SchemaError> {
    use crate::ir::Particle;
    fn children(
        reader: &mut Reader<&[u8]>,
        end: &str,
        target_ns: Option<&str>,
        prefixes: &HashMap<String, String>,
    ) -> Result<Option<Vec<Particle>>, SchemaError> {
        let mut result = Vec::new();
        loop {
            match reader.read_event()? {
                Event::Start(e) | Event::Empty(e)
                    if matches!(
                        strip_prefix(e.name().into_inner()),
                        "group" | "any" | "extension" | "restriction" | "all"
                    ) =>
                {
                    return Ok(None)
                }
                Event::Start(e) => {
                    let local = strip_prefix(e.name().into_inner());
                    if matches!(local, "sequence" | "choice") {
                        let Some(items) = children(reader, local, target_ns, prefixes)? else {
                            return Ok(None);
                        };
                        let model = if local == "choice" {
                            Particle::Choice(items)
                        } else {
                            Particle::Sequence(items)
                        };
                        result.push(repeat(model, &e));
                    } else if local == "element" {
                        if let Some(field) =
                            parse_element_field(&e, target_ns, prefixes, false, false)
                        {
                            result.push(repeat(
                                Particle::Element(QName::new(field.namespace, field.xml_name)),
                                &e,
                            ));
                        }
                        reader.read_to_end(e.name())?;
                    }
                }
                Event::Empty(e) if strip_prefix(e.name().into_inner()) == "element" => {
                    if let Some(field) = parse_element_field(&e, target_ns, prefixes, false, false)
                    {
                        result.push(repeat(
                            Particle::Element(QName::new(field.namespace, field.xml_name)),
                            &e,
                        ));
                    }
                }
                Event::End(e) if strip_prefix(e.name().into_inner()) == end => {
                    return Ok(Some(result))
                }
                Event::Eof => return Ok(None),
                _ => {}
            }
        }
    }
    fn repeat(model: Particle, e: &BytesStart) -> Particle {
        let min = get_attr_value(e, "minOccurs")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let max = match get_attr_value(e, "maxOccurs").as_deref() {
            Some("unbounded") => None,
            Some(value) => value.parse().ok(),
            None => Some(1),
        };
        if min == 1 && max == Some(1) {
            model
        } else {
            Particle::Repeat {
                particle: Box::new(model),
                min,
                max,
            }
        }
    }
    let mut preview = Reader::from_reader(*reader.get_ref());
    preview.config_mut().allow_unmatched_ends = true;
    Ok(children(&mut preview, "complexType", target_ns, prefixes)?.map(Particle::Sequence))
}
