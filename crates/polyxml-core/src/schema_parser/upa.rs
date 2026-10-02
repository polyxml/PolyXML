//! XSD 1.0 UPA checks on source particles, before field flattening loses identity.
//! The position-labelled epsilon automaton follows XML Schema Part 1, appendix H.
use super::{strip_prefix, SchemaError};
use crate::ir::{QName, SchemaIR};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    namespace: Option<String>,
    groups: BTreeMap<QName, Model>,
    named: BTreeMap<QName, Model>,
    anonymous: Vec<(String, Model)>,
    #[serde(default)]
    unsupported_attribute_types: BTreeSet<QName>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum Model {
    Element(QName, bool),
    Reference(QName),
    Wildcard(NamespaceSet),
    Group(QName),
    Extension(QName, Box<Model>),
    Sequence(Vec<Model>),
    Choice(Vec<Model>),
    All(Vec<Model>),
    Repeat(Box<Model>, usize, Option<usize>),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum NamespaceSet {
    Any,
    Other(Option<String>),
    Names {
        names: BTreeSet<Option<String>>,
        target: Option<Option<String>>,
    },
}
impl NamespaceSet {
    fn allows(&self, ns: &Option<String>) -> bool {
        match self {
            Self::Any => true,
            Self::Other(excluded) => ns.is_some() && ns != excluded,
            Self::Names { names, target } => {
                names.contains(ns) || target.as_ref().is_some_and(|target| target == ns)
            }
        }
    }
    fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Any, _) | (_, Self::Any) | (Self::Other(_), Self::Other(_)) => true,
            (Self::Names { names, target }, other) | (other, Self::Names { names, target }) => {
                names.iter().chain(target.iter()).any(|ns| other.allows(ns))
            }
        }
    }
}
struct Node {
    tag: String,
    attrs: HashMap<String, String>,
    prefixes: HashMap<String, String>,
    children: Vec<Node>,
    position: u64,
}
impl Node {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }
    fn qname(&self, value: &str, namespace: Option<&str>) -> QName {
        if let Some((prefix, local)) = value.split_once(':') {
            QName::new(self.prefixes.get(prefix).map(String::as_str), local)
        } else {
            QName::new(
                self.prefixes.get("").map(String::as_str).or(namespace),
                value,
            )
        }
    }
}
fn node(
    reader: &mut Reader<&[u8]>,
    start: BytesStart<'_>,
    prefixes: &HashMap<String, String>,
    position: u64,
    empty: bool,
) -> Result<Node, SchemaError> {
    let mut scoped = prefixes.clone();
    let mut attrs = HashMap::new();
    for attr in start.attributes() {
        let attr = attr.map_err(|error| SchemaError::Malformed(error.to_string()))?;
        let key = attr.key.as_ref();
        let value = attr
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)?
            .into_owned();
        if key == "xmlns" {
            scoped.insert(String::new(), value);
        } else if let Some(prefix) = key.strip_prefix("xmlns:") {
            scoped.insert(prefix.into(), value);
        } else {
            attrs.insert(key.into(), value);
        }
    }
    let mut result = Node {
        tag: strip_prefix(start.name().into_inner()).into(),
        attrs,
        prefixes: scoped,
        children: Vec::new(),
        position,
    };
    if !empty {
        loop {
            let position = reader.buffer_position();
            match reader.read_event()? {
                Event::Start(child)
                    if matches!(
                        strip_prefix(child.name().into_inner()),
                        "annotation" | "simpleType" | "unique" | "key" | "keyref"
                    ) =>
                {
                    reader.read_to_end(child.name())?;
                }
                Event::Empty(child)
                    if matches!(
                        strip_prefix(child.name().into_inner()),
                        "annotation" | "simpleType" | "unique" | "key" | "keyref"
                    ) => {}
                Event::Start(child) => {
                    result
                        .children
                        .push(node(reader, child, &result.prefixes, position, false)?)
                }
                Event::Empty(child) => {
                    result
                        .children
                        .push(node(reader, child, &result.prefixes, position, true)?)
                }
                Event::End(_) => break,
                Event::Eof => {
                    return Err(SchemaError::Malformed(
                        "Unexpected end while capturing UPA particles".into(),
                    ))
                }
                _ => {}
            }
        }
    }
    Ok(result)
}
fn repeat(node: &Node, model: Model) -> Model {
    let min = node
        .attr("minOccurs")
        .and_then(|n| n.parse().ok())
        .unwrap_or(1);
    let max = match node.attr("maxOccurs") {
        Some("unbounded") => None,
        Some(n) => n.parse().ok(),
        None => Some(1),
    };
    if min == 1 && max == Some(1) {
        model
    } else {
        Model::Repeat(Box::new(model), min, max)
    }
}
fn content(node: &Node, ns: Option<&str>, qualified: bool) -> Model {
    Model::Sequence(
        node.children
            .iter()
            .filter_map(|child| particle(child, ns, qualified))
            .collect(),
    )
}
fn particle(node: &Node, ns: Option<&str>, qualified: bool) -> Option<Model> {
    let model = match node.tag.as_str() {
        "element" => {
            if let Some(reference) = node.attr("ref") {
                Model::Reference(node.qname(reference, ns))
            } else {
                let qualified = match node.attr("form") {
                    Some("qualified") => true,
                    Some("unqualified") => false,
                    _ => qualified,
                };
                Model::Element(
                    QName::new(if qualified { ns } else { None }, node.attr("name")?),
                    qualified,
                )
            }
        }
        "any" => {
            let namespace = match node.attr("namespace").unwrap_or("##any") {
                "##any" => NamespaceSet::Any,
                "##other" => NamespaceSet::Other(ns.map(str::to_string)),
                names => NamespaceSet::Names {
                    names: names
                        .split_whitespace()
                        .filter(|n| *n != "##targetNamespace")
                        .map(|n| {
                            if n == "##local" {
                                None
                            } else {
                                Some(n.to_string())
                            }
                        })
                        .collect(),
                    target: names
                        .split_whitespace()
                        .any(|n| n == "##targetNamespace")
                        .then(|| ns.map(str::to_string)),
                },
            };
            Model::Wildcard(namespace)
        }
        "group" => {
            if let Some(reference) = node.attr("ref") {
                Model::Group(node.qname(reference, ns))
            } else {
                return Some(content(node, ns, qualified));
            }
        }
        "sequence" | "choice" | "all" => {
            let children = node
                .children
                .iter()
                .filter_map(|child| particle(child, ns, qualified))
                .collect();
            match node.tag.as_str() {
                "choice" => Model::Choice(children),
                "all" => Model::All(children),
                _ => Model::Sequence(children),
            }
        }
        "complexContent" => return Some(content(node, ns, qualified)),
        "extension" => Model::Extension(
            node.qname(node.attr("base")?, ns),
            Box::new(content(node, ns, qualified)),
        ),
        "restriction" => return Some(content(node, ns, qualified)),
        _ => return None,
    };
    Some(repeat(node, model))
}
impl Document {
    /// Whether a named type has plain sequence content in one element namespace.
    /// Source QNames retain local `form` and `elementFormDefault` distinctions.
    pub(crate) fn uniform_sequence(
        &self,
        name: &QName,
        namespace: &Option<String>,
    ) -> Option<bool> {
        fn uniform(model: &Model, namespace: &Option<String>) -> bool {
            match model {
                Model::Element(q, _) => &q.namespace == namespace,
                Model::Sequence(parts) => parts.iter().all(|p| uniform(p, namespace)),
                Model::Repeat(part, _, _) if matches!(part.as_ref(), Model::Element(..)) => {
                    uniform(part, namespace)
                }
                _ => false,
            }
        }
        self.named.get(name).map(|model| {
            !self.unsupported_attribute_types.contains(name) && uniform(model, namespace)
        })
    }

    pub fn parse(xml: &str) -> Result<Self, SchemaError> {
        let mut reader = Reader::from_str(xml);
        let root = loop {
            match reader.read_event()? {
                Event::Start(start) => break node(&mut reader, start, &HashMap::new(), 0, false)?,
                Event::Empty(start) => break node(&mut reader, start, &HashMap::new(), 0, true)?,
                Event::Eof => return Err(SchemaError::Malformed("Missing schema".into())),
                _ => {}
            }
        };
        let namespace = root.attr("targetNamespace").map(str::to_string);
        let qualified = root.attr("elementFormDefault") == Some("qualified");
        let mut document = Self {
            namespace: namespace.clone(),
            groups: BTreeMap::new(),
            named: BTreeMap::new(),
            anonymous: Vec::new(),
            unsupported_attribute_types: BTreeSet::new(),
        };
        fn qualified_attributes(node: &Node, default_qualified: bool) -> bool {
            matches!(node.tag.as_str(), "attributeGroup" | "anyAttribute")
                || (node.tag == "attribute"
                    && (node.attr("ref").is_some()
                        || match node.attr("form") {
                            Some("qualified") => true,
                            Some("unqualified") => false,
                            _ => default_qualified,
                        }))
                || node
                    .children
                    .iter()
                    .any(|child| qualified_attributes(child, default_qualified))
        }
        fn collect(
            node: &Node,
            document: &mut Document,
            qualified: bool,
            attributes_qualified: bool,
        ) {
            let ns = document.namespace.as_deref();
            if node.tag == "complexType" {
                let model = content(node, ns, qualified);
                if let Some(name) = node.attr("name") {
                    document.named.insert(QName::new(ns, name), model);
                    if qualified_attributes(node, attributes_qualified) {
                        document
                            .unsupported_attribute_types
                            .insert(QName::new(ns, name));
                    }
                } else {
                    document.anonymous.push((
                        format!("anonymous complexType at byte {}", node.position),
                        model,
                    ));
                }
            }
            for child in &node.children {
                collect(child, document, qualified, attributes_qualified);
            }
        }
        for child in &root.children {
            if child.tag == "group" {
                if let Some(name) = child.attr("name") {
                    document.groups.insert(
                        QName::new(namespace.as_deref(), name),
                        content(child, namespace.as_deref(), qualified),
                    );
                }
            }
        }
        collect(
            &root,
            &mut document,
            qualified,
            root.attr("attributeFormDefault") == Some("qualified"),
        );
        Ok(document)
    }
    pub fn adopt_namespace(&mut self, namespace: &str) {
        if self.namespace.is_some() {
            return;
        }
        self.namespace = Some(namespace.into());
        fn name(q: &mut QName, ns: &str) {
            if q.namespace.is_none() {
                q.namespace = Some(ns.into());
            }
        }
        fn model(p: &mut Model, ns: &str) {
            match p {
                Model::Element(q, qualified) => {
                    if *qualified {
                        name(q, ns);
                    }
                }
                Model::Reference(q) | Model::Group(q) => name(q, ns),
                Model::Extension(q, p) => {
                    name(q, ns);
                    model(p, ns);
                }
                Model::Sequence(items) | Model::Choice(items) | Model::All(items) => {
                    for p in items {
                        model(p, ns);
                    }
                }
                Model::Repeat(p, ..) => model(p, ns),
                Model::Wildcard(NamespaceSet::Other(excluded)) if excluded.is_none() => {
                    *excluded = Some(ns.into());
                }
                Model::Wildcard(NamespaceSet::Names {
                    target: Some(target),
                    ..
                }) if target.is_none() => {
                    *target = Some(ns.into());
                }
                _ => {}
            }
        }
        self.unsupported_attribute_types = std::mem::take(&mut self.unsupported_attribute_types)
            .into_iter()
            .map(|mut q| {
                name(&mut q, namespace);
                q
            })
            .collect();
        for collection in [&mut self.groups, &mut self.named] {
            *collection = std::mem::take(collection)
                .into_iter()
                .map(|(mut q, mut p)| {
                    name(&mut q, namespace);
                    model(&mut p, namespace);
                    (q, p)
                })
                .collect();
        }
        for (_, p) in &mut self.anonymous {
            model(p, namespace);
        }
    }
}
#[derive(Clone)]
enum Label {
    Elements(BTreeSet<QName>),
    Wildcard(NamespaceSet),
}
impl Label {
    fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Elements(a), Self::Elements(b)) => !a.is_disjoint(b),
            (Self::Wildcard(a), Self::Wildcard(b)) => a.overlaps(b),
            (Self::Elements(names), Self::Wildcard(ns))
            | (Self::Wildcard(ns), Self::Elements(names)) => {
                names.iter().any(|q| ns.allows(&q.namespace))
            }
        }
    }
    fn describe(&self) -> String {
        match self {
            Self::Elements(names) => names
                .iter()
                .map(|q| format!("{q:?}"))
                .collect::<Vec<_>>()
                .join(", "),
            Self::Wildcard(ns) => format!("wildcard {ns:?}"),
        }
    }
}
#[derive(Clone, Default)]
struct State {
    epsilon: Vec<usize>,
    edges: Vec<(usize, usize)>,
}
#[derive(Default)]
struct Automaton {
    states: Vec<State>,
    labels: Vec<Label>,
}
impl Automaton {
    fn state(&mut self) -> Result<usize, SchemaError> {
        if self.states.len() >= 250_000 {
            return Err(SchemaError::Malformed(
                "UPA content model exceeds automaton state budget".into(),
            ));
        }
        let state = self.states.len();
        self.states.push(State::default());
        Ok(state)
    }
    fn empty(&mut self) -> Result<(usize, usize), SchemaError> {
        let a = self.state()?;
        let b = self.state()?;
        self.states[a].epsilon.push(b);
        Ok((a, b))
    }
    fn leaf(&mut self, label: Label) -> Result<(usize, usize), SchemaError> {
        let a = self.state()?;
        let b = self.state()?;
        let id = self.labels.len();
        self.labels.push(label);
        self.states[a].edges.push((id, b));
        Ok((a, b))
    }
    fn copy(
        &mut self,
        template: &[State],
        first: usize,
        fragment: (usize, usize),
    ) -> Result<(usize, usize), SchemaError> {
        let offset = self.states.len();
        for _ in template {
            self.state()?;
        }
        for (i, original) in template.iter().enumerate() {
            let old = first + i;
            let mut state = original.clone();
            for to in &mut state.epsilon {
                *to = offset + *to - first;
            }
            for (_, to) in &mut state.edges {
                *to = offset + *to - first;
            }
            self.states[offset + old - first] = state;
        }
        Ok((offset + fragment.0 - first, offset + fragment.1 - first))
    }
    fn build(
        &mut self,
        p: &Model,
        groups: &BTreeMap<QName, Model>,
        types: &BTreeMap<QName, Model>,
        ir: &SchemaIR,
        visiting: &mut HashSet<QName>,
    ) -> Result<(usize, usize), SchemaError> {
        match p {
            Model::Element(q, _) => self.leaf(Label::Elements(BTreeSet::from([q.clone()]))),
            Model::Reference(head) => {
                fn members(
                    head: &QName,
                    ir: &SchemaIR,
                    names: &mut BTreeSet<QName>,
                    seen: &mut HashSet<QName>,
                ) {
                    if !seen.insert(head.clone()) {
                        return;
                    }
                    if !ir.abstract_elements.contains(head) {
                        names.insert(head.clone());
                    }
                    if let Some(items) = ir.substitution_groups.get(head) {
                        for member in items {
                            members(member, ir, names, seen);
                        }
                    }
                }
                let mut names = BTreeSet::new();
                members(head, ir, &mut names, &mut HashSet::new());
                self.leaf(Label::Elements(names))
            }
            Model::Wildcard(ns) => self.leaf(Label::Wildcard(ns.clone())),
            Model::Group(q) => {
                if !visiting.insert(q.clone()) {
                    return Err(SchemaError::Malformed(format!(
                        "Circular model group in UPA validation: {q:?}"
                    )));
                }
                let model = groups.get(q).ok_or_else(|| {
                    SchemaError::Resolution(format!("Missing model group in UPA validation: {q:?}"))
                })?;
                let result = self.build(model, groups, types, ir, visiting);
                visiting.remove(q);
                result
            }
            Model::Extension(q, own) => {
                if !visiting.insert(q.clone()) {
                    return Err(SchemaError::Malformed(format!(
                        "Circular complex type in UPA validation: {q:?}"
                    )));
                }
                let base = if let Some(base) = types.get(q) {
                    self.build(base, groups, types, ir, visiting)?
                } else {
                    self.empty()?
                };
                visiting.remove(q);
                let own = self.build(own, groups, types, ir, visiting)?;
                self.states[base.1].epsilon.push(own.0);
                Ok((base.0, own.1))
            }
            Model::Sequence(items) => {
                let (start, mut end) = self.empty()?;
                for item in items {
                    let fragment = self.build(item, groups, types, ir, visiting)?;
                    self.states[end].epsilon.push(fragment.0);
                    end = fragment.1;
                }
                Ok((start, end))
            }
            Model::Choice(items) => {
                let start = self.state()?;
                let end = self.state()?;
                for item in items {
                    let fragment = self.build(item, groups, types, ir, visiting)?;
                    self.states[start].epsilon.push(fragment.0);
                    self.states[fragment.1].epsilon.push(end);
                }
                Ok((start, end))
            }
            Model::All(items) => {
                // XSD 1.0 all children are single element particles. Track which
                // children have been consumed without copying their identities.
                if items.len() >= 18 {
                    return Err(SchemaError::Malformed(
                        "UPA all-group exceeds automaton state budget".into(),
                    ));
                }
                let mut fragments = Vec::new();
                for item in items {
                    fragments.push(self.build(item, groups, types, ir, visiting)?);
                }
                let mut states = Vec::new();
                for _ in 0..(1usize << items.len()) {
                    states.push(self.state()?);
                }
                let end = self.state()?;
                for (mask, &state) in states.iter().enumerate() {
                    let mut complete = true;
                    for (i, (a, b)) in fragments.iter().copied().enumerate() {
                        if mask & (1 << i) == 0 {
                            if !nullable(&items[i]) {
                                complete = false;
                            }
                            // Copy state wiring per mask, preserving leaf IDs.
                            let label = self.states[a].edges.clone();
                            let outgoing = if label.is_empty() {
                                self.closure(&BTreeSet::from([a]))
                                    .into_iter()
                                    .flat_map(|s| self.states[s].edges.clone())
                                    .collect::<Vec<_>>()
                            } else {
                                label
                            };
                            for (id, _) in outgoing {
                                self.states[state].edges.push((id, states[mask | (1 << i)]));
                            }
                            let _ = b;
                        }
                    }
                    if complete {
                        self.states[state].epsilon.push(end);
                    }
                }
                Ok((states[0], end))
            }
            Model::Repeat(body, min, max) => {
                if *max == Some(0) {
                    return self.empty();
                }
                let start = self.state()?;
                let end = self.state()?;
                let first = self.states.len();
                let template = self.build(body, groups, types, ir, visiting)?;
                let snapshot = self.states[first..].to_vec();
                let mut tail = start;
                let copies = match max {
                    Some(max) => *max,
                    None => min.checked_add(1).ok_or_else(|| {
                        SchemaError::Malformed("UPA occurrence range overflows".into())
                    })?,
                };
                for i in 0..copies {
                    let fragment = if i == 0 {
                        template
                    } else {
                        self.copy(&snapshot, first, template)?
                    };
                    if i >= *min {
                        self.states[tail].epsilon.push(end);
                    }
                    self.states[tail].epsilon.push(fragment.0);
                    tail = fragment.1;
                    if max.is_none() && i == *min {
                        self.states[tail].epsilon.push(fragment.0);
                    }
                }
                self.states[tail].epsilon.push(end);
                Ok((start, end))
            }
        }
    }
    fn closure(&self, seed: &BTreeSet<usize>) -> BTreeSet<usize> {
        let mut result = seed.clone();
        let mut stack: Vec<_> = seed.iter().copied().collect();
        while let Some(s) = stack.pop() {
            for &to in &self.states[s].epsilon {
                if result.insert(to) {
                    stack.push(to);
                }
            }
        }
        result
    }
    fn check(&self, start: usize, owner: &str) -> Result<(), SchemaError> {
        let initial = self.closure(&BTreeSet::from([start]));
        let mut queue = VecDeque::from([initial.clone()]);
        let mut seen = HashSet::from([initial]);
        while let Some(states) = queue.pop_front() {
            let mut transitions: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
            for state in states {
                for &(id, to) in &self.states[state].edges {
                    transitions.entry(id).or_default().insert(to);
                }
            }
            let ids: Vec<_> = transitions.keys().copied().collect();
            for (i, &a) in ids.iter().enumerate() {
                for &b in &ids[i + 1..] {
                    if self.labels[a].overlaps(&self.labels[b]) {
                        return Err(SchemaError::Malformed(format!("Unique Particle Attribution (UPA) violation in {owner}: {} overlaps {}",self.labels[a].describe(),self.labels[b].describe())));
                    }
                }
            }
            for targets in transitions.values() {
                let next = self.closure(targets);
                if seen.insert(next.clone()) {
                    if seen.len() > 100_000 {
                        return Err(SchemaError::Malformed(
                            "UPA determinization state budget exceeded".into(),
                        ));
                    }
                    queue.push_back(next);
                }
            }
        }
        Ok(())
    }
}
fn nullable(model: &Model) -> bool {
    match model {
        Model::Sequence(items) | Model::All(items) => items.iter().all(nullable),
        Model::Choice(items) => items.iter().any(nullable),
        Model::Repeat(body, min, _) => *min == 0 || nullable(body),
        _ => false,
    }
}

// If every distinct source position has a disjoint label, no automaton is
// needed, regardless of occurrence counts or all-group width.
fn skeleton(model: &Model) -> Model {
    match model {
        Model::Repeat(_, _, Some(0)) => Model::Sequence(vec![]),
        Model::Repeat(p, ..) => skeleton(p),
        Model::Sequence(items) => Model::Sequence(items.iter().map(skeleton).collect()),
        Model::Choice(items) | Model::All(items) => {
            Model::Choice(items.iter().map(skeleton).collect())
        }
        Model::Extension(q, p) => Model::Extension(q.clone(), Box::new(skeleton(p))),
        _ => model.clone(),
    }
}
fn uniform(model: &Model, blocks: &mut Vec<(QName, usize, Option<usize>)>) -> bool {
    match model {
        Model::Element(q, _) => {
            blocks.push((q.clone(), 1, Some(1)));
            true
        }
        Model::Sequence(items) => items.iter().all(|p| uniform(p, blocks)),
        Model::Repeat(_, _, Some(0)) => true,
        Model::Repeat(p, min, max) => {
            let mut inner = Vec::new();
            if !uniform(p, &mut inner) || inner.len() != 1 {
                return false;
            }
            let (q, a, b) = inner.pop().unwrap();
            let Some(minimum) = a.checked_mul(*min) else {
                return false;
            };
            let maximum = match (b, max) {
                (Some(b), Some(max)) => match b.checked_mul(*max) {
                    Some(n) => Some(n),
                    None => return false,
                },
                _ => None,
            };
            blocks.push((q, minimum, maximum));
            true
        }
        _ => false,
    }
}

pub fn validate(ir: &SchemaIR) -> Result<(), SchemaError> {
    let mut groups = BTreeMap::new();
    let mut types = BTreeMap::new();
    for document in &ir.upa_documents {
        groups.extend(document.groups.clone());
        types.extend(document.named.clone());
    }
    let preview_groups = groups
        .iter()
        .map(|(q, p)| (q.clone(), skeleton(p)))
        .collect();
    let preview_types = types
        .iter()
        .map(|(q, p)| (q.clone(), skeleton(p)))
        .collect();
    let mut checked = HashSet::new();
    for document in &ir.upa_documents {
        for (owner, model) in document
            .named
            .iter()
            .map(|(q, p)| (format!("{q:?}"), p))
            .chain(
                document
                    .groups
                    .iter()
                    .map(|(q, p)| (format!("model group {q:?}"), p)),
            )
            .chain(document.anonymous.iter().map(|(name, p)| (name.clone(), p)))
        {
            if !checked.insert(model) {
                continue;
            }
            let mut blocks = Vec::new();
            if uniform(model, &mut blocks) {
                blocks.retain(|(_, _, max)| *max != Some(0));
                if blocks
                    .first()
                    .is_none_or(|(q, ..)| blocks.iter().all(|(name, ..)| name == q))
                {
                    for (_, min, max) in blocks.iter().take(blocks.len().saturating_sub(1)) {
                        if *max != Some(*min) {
                            return Err(SchemaError::Malformed(format!("Unique Particle Attribution (UPA) violation in {owner}: adjacent element occurrence ranges overlap")));
                        }
                    }
                    continue;
                }
            }
            let mut preview = Automaton::default();
            preview.build(
                &skeleton(model),
                &preview_groups,
                &preview_types,
                ir,
                &mut HashSet::new(),
            )?;
            if preview
                .labels
                .iter()
                .enumerate()
                .all(|(i, a)| preview.labels[i + 1..].iter().all(|b| !a.overlaps(b)))
            {
                continue;
            }
            let mut automaton = Automaton::default();
            let (start, _) = automaton.build(model, &groups, &types, ir, &mut HashSet::new())?;
            automaton.check(start, &owner)?;
        }
    }
    Ok(())
}
