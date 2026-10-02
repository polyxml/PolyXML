#![allow(clippy::type_complexity)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::too_many_arguments)]

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyString, PyTuple, PyType};
use pyo3::IntoPyObjectExt;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use polyxml::schema::{
    FieldKind, FieldSchema, MixedBranchSchema, MixedContentSchema, ModelSchema, ScalarType,
    ValueType,
};
use polyxml::value::PolyValue;

use smallvec::SmallVec;

type PyObject = Py<PyAny>;

struct CachedFieldMeta {
    py_string: Py<PyString>,
    is_init: bool,
    #[allow(dead_code)]
    is_kw_only: bool,
    py_type: Option<PyObject>,
}

struct CachedSchemaMeta {
    schema: Arc<ModelSchema>,
    py_cls: PyObject,
    is_dataclass: bool,
    all_init: bool,
    kwnames: Option<Py<PyTuple>>,
    fields: Vec<CachedFieldMeta>,
}

// Global thread-safe schema cache keyed by Python type pointer
static SCHEMA_CACHE: RwLock<Option<HashMap<usize, Arc<CachedSchemaMeta>>>> = RwLock::new(None);
// Cache keyed by schema name to resolve nested Python types during deserialization
static CLASS_BY_SCHEMA: RwLock<Option<HashMap<String, Arc<CachedSchemaMeta>>>> = RwLock::new(None);
thread_local! { static DISCOVERING_VARIANTS: Cell<bool> = const { Cell::new(false) }; }

fn refresh_variants(cls: &Bound<'_, PyType>, meta: &CachedSchemaMeta) {
    DISCOVERING_VARIANTS.with(|busy| {
        if busy.replace(true) {
            return;
        }
        let mut seen = HashSet::new();
        refresh_variant_tree(cls, &meta.schema, &mut seen);
        busy.set(false);
    });
}

fn refresh_variant_tree(
    cls: &Bound<'_, PyType>,
    schema: &Arc<ModelSchema>,
    seen: &mut HashSet<usize>,
) {
    if !seen.insert(Arc::as_ptr(schema) as usize) {
        return;
    }
    schema.set_variants(discover_variants(cls));
    for field in &schema.fields {
        let nested = match &field.val_type {
            ValueType::Nested(nested) => Some(nested),
            ValueType::List(inner) => match inner.as_ref() {
                ValueType::Nested(nested) => Some(nested),
                _ => None,
            },
            _ => None,
        };
        if let Some(nested) = nested {
            if let Some(meta) = lookup_cached_meta(&nested.name) {
                if let Ok(child_cls) = meta.py_cls.bind(cls.py()).cast::<PyType>() {
                    refresh_variant_tree(child_cls, &meta.schema, seen);
                }
            }
        }
    }
}

fn lookup_cached_meta(schema_name: &str) -> Option<Arc<CachedSchemaMeta>> {
    let class_map = CLASS_BY_SCHEMA.read().unwrap_or_else(|p| p.into_inner());
    if let Some(ref map) = *class_map {
        if let Some(meta) = map.get(schema_name) {
            return Some(Arc::clone(meta));
        }
    }
    None
}

fn lookup_py_class<'py>(py: Python<'py>, schema_name: &str) -> Option<Bound<'py, PyAny>> {
    lookup_cached_meta(schema_name).map(|meta| meta.py_cls.bind(py).clone())
}

fn unwrap_optional_type<'py>(type_obj: &Bound<'py, PyAny>) -> Bound<'py, PyAny> {
    if let Ok(val) = type_obj.getattr("__value__") {
        return unwrap_optional_type(&val);
    }
    if type_obj.hasattr("__metadata__").unwrap_or(false) {
        if let Ok(origin) = type_obj.getattr("__origin__") {
            return unwrap_optional_type(&origin);
        }
    }

    let is_union = if let Ok(origin) = type_obj.getattr("__origin__") {
        if let Ok(origin_name) = origin.getattr("__name__") {
            let origin_str: String = origin_name.extract().unwrap_or_default();
            origin_str == "Union" || origin_str == "UnionType"
        } else {
            false
        }
    } else {
        type_obj
            .get_type()
            .name()
            .map(|n| n == "UnionType")
            .unwrap_or(false)
    };

    if is_union {
        if let Ok(args) = type_obj.getattr("__args__") {
            if let Ok(tuple) = args.cast::<PyTuple>() {
                for arg in tuple.iter() {
                    let arg_name: String = arg
                        .getattr("__name__")
                        .and_then(|n| n.extract())
                        .unwrap_or_default();
                    if arg_name != "NoneType" {
                        return unwrap_optional_type(&arg);
                    }
                }
            }
        }
    }
    type_obj.clone()
}

fn is_enum_class<'py>(py: Python<'py>, cls: &Bound<'py, PyAny>) -> bool {
    if let Ok(py_type) = cls.cast::<PyType>() {
        if let Ok(enum_module) = py.import("enum") {
            if let Ok(enum_cls) = enum_module.getattr("Enum") {
                if let Ok(py_enum) = enum_cls.cast::<PyType>() {
                    return py_type.is_subclass(py_enum).unwrap_or(false);
                }
            }
        }
    }
    false
}

fn resolve_scalar_type(py: Python<'_>, type_obj: &Bound<'_, PyAny>) -> PyResult<ScalarType> {
    let unwrapped = unwrap_optional_type(type_obj);
    let type_name: String = unwrapped
        .getattr("__name__")
        .and_then(|n| n.extract())
        .unwrap_or_default();

    match type_name.as_str() {
        "str" => Ok(ScalarType::String),
        "int" => Ok(ScalarType::Int),
        "float" => Ok(ScalarType::Float),
        "bool" => Ok(ScalarType::Bool),
        "Decimal" => Ok(ScalarType::Decimal),
        "XmlDate" => Ok(ScalarType::XmlDate),
        "XmlDateTime" => Ok(ScalarType::XmlDateTime),
        "XmlTime" => Ok(ScalarType::XmlTime),
        "XmlDuration" => Ok(ScalarType::XmlDuration),
        _ => {
            if is_enum_class(py, &unwrapped) {
                if let Ok(members) = unwrapped.getattr("__members__") {
                    if let Ok(values) = members.call_method0("values") {
                        let mut choices = Vec::new();
                        for member in values.try_iter()?.flatten() {
                            if let Ok(value) = member.getattr("value") {
                                if let Ok(value) = value.str().and_then(|text| text.extract()) {
                                    choices.push(value);
                                }
                            }
                            if let Ok(name) = member.getattr("name") {
                                if let Ok(name) = name.extract() {
                                    choices.push(name);
                                }
                            }
                        }
                        return Ok(ScalarType::Enum(choices));
                    }
                }
            }
            Ok(ScalarType::Any)
        }
    }
}

fn resolve_value_type(py: Python<'_>, type_obj: &Bound<'_, PyAny>) -> PyResult<ValueType> {
    if let Ok(val) = type_obj.getattr("__value__") {
        return resolve_value_type(py, &val);
    }
    if type_obj.hasattr("__metadata__").unwrap_or(false) {
        if let Ok(origin) = type_obj.getattr("__origin__") {
            let base = resolve_value_type(py, &origin)?;
            if let ValueType::Scalar(scalar) = base {
                if let Ok(metadata) = type_obj.getattr("__metadata__") {
                    if let Ok(items) = metadata.cast::<PyTuple>() {
                        for item in items.iter() {
                            if let Ok(marker) = item.cast::<PyTuple>() {
                                if marker.len() == 2
                                    && marker.get_item(0)?.extract::<String>().ok().as_deref()
                                        == Some("polyxml_patterns")
                                {
                                    let patterns = marker.get_item(1)?.extract::<Vec<String>>()?;
                                    return Ok(ValueType::Scalar(ScalarType::Pattern(
                                        Box::new(scalar),
                                        patterns,
                                    )));
                                }
                            }
                        }
                    }
                }
                return Ok(ValueType::Scalar(scalar));
            }
            return Ok(base);
        }
    }

    // Handle typing.Optional / Union and PEP 604 UnionType
    let is_union = if let Ok(origin) = type_obj.getattr("__origin__") {
        if let Ok(origin_name) = origin.getattr("__name__") {
            let origin_str: String = origin_name.extract().unwrap_or_default();
            origin_str == "Union" || origin_str == "UnionType"
        } else {
            false
        }
    } else {
        type_obj
            .get_type()
            .name()
            .map(|n| n == "UnionType")
            .unwrap_or(false)
    };

    if is_union {
        if let Ok(args) = type_obj.getattr("__args__") {
            if let Ok(tuple) = args.cast_into::<PyTuple>() {
                let mut scalars = Vec::new();
                let mut first_non_scalar = None;
                for arg in tuple.iter() {
                    let arg_name: String = arg
                        .getattr("__name__")
                        .and_then(|n| n.extract())
                        .unwrap_or_default();
                    if arg_name != "NoneType" {
                        match resolve_value_type(py, &arg)? {
                            ValueType::Scalar(scalar) => scalars.push(scalar),
                            other => {
                                if first_non_scalar.is_none() {
                                    first_non_scalar = Some(other);
                                }
                            }
                        }
                    }
                }
                if let Some(non_scalar) = first_non_scalar {
                    return Ok(non_scalar);
                }
                if scalars.len() > 1 {
                    return Ok(ValueType::Scalar(ScalarType::Union(scalars)));
                }
                if let Some(scalar) = scalars.into_iter().next() {
                    return Ok(ValueType::Scalar(scalar));
                }
            }
        }
    }

    if let Ok(origin) = type_obj.getattr("__origin__") {
        if let Ok(origin_name) = origin.getattr("__name__") {
            let origin_str: String = origin_name.extract().unwrap_or_default();
            if origin_str == "list" || origin_str == "List" {
                if let Ok(args) = type_obj.getattr("__args__") {
                    let tuple: Bound<'_, pyo3::types::PyTuple> = args.cast_into()?;
                    if let Some(first) = tuple.iter().next() {
                        let inner = resolve_value_type(py, &first)?;
                        return Ok(ValueType::List(Box::new(inner)));
                    }
                }
                return Ok(ValueType::List(Box::new(ValueType::Scalar(
                    ScalarType::String,
                ))));
            }
        }
    }

    // Check if target is a nested dataclass or Pydantic model
    if type_obj.hasattr("__dataclass_fields__")? || type_obj.hasattr("model_fields")? {
        if let Ok(py_type) = type_obj.cast::<PyType>() {
            let (nested_schema, _) = get_or_create_schema(py_type)?;
            return Ok(ValueType::Nested(nested_schema));
        }
    }

    let scalar = resolve_scalar_type(py, type_obj)?;
    Ok(ValueType::Scalar(scalar))
}

fn extract_schema_from_class<'py>(
    cls: &Bound<'py, PyType>,
) -> PyResult<(Arc<ModelSchema>, Vec<CachedFieldMeta>)> {
    let py = cls.py();
    let class_name: String = cls.getattr("__name__")?.extract()?;
    let mut builder = ModelSchema::builder(class_name);

    if let Ok(meta_cls) = cls.getattr("Meta") {
        if let Ok(pattern) = meta_cls.getattr("content_pattern") {
            let pattern: String = pattern.extract()?;
            builder = builder.content_pattern(
                polyxml::schema::compile_content_pattern(&pattern)
                    .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))?,
            );
        }
        if let Ok(strict_root) = meta_cls.getattr("strict_root") {
            builder = builder.strict_root(strict_root.extract::<bool>().unwrap_or(false));
        }
        if let Ok(name_val) = meta_cls.getattr("name") {
            if let Ok(name_str) = name_val.extract::<String>() {
                if !name_str.is_empty() {
                    builder = builder.xml_name(name_str.as_bytes());
                }
            }
        }
        if let Ok(ns_val) = meta_cls.getattr("namespace") {
            if let Ok(ns_str) = ns_val.extract::<String>() {
                if !ns_str.is_empty() {
                    builder = builder.namespace(ns_str);
                }
            }
        }
        // Abstract types raise a clear error when xsi:type names
        // a derivation the runtime does not know.
        if let Ok(abstract_val) = meta_cls.getattr("abstract") {
            if abstract_val.extract::<bool>().unwrap_or(false) {
                builder = builder.is_abstract(true);
            }
        }
    }

    let mut cached_fields = Vec::new();

    let type_hints: Option<Bound<'py, PyDict>> = py
        .import("typing")
        .ok()
        .and_then(|m| m.getattr("get_type_hints").ok())
        .and_then(|f| f.call1((cls,)).ok())
        .and_then(|h| h.cast_into::<PyDict>().ok());

    if cls.hasattr("model_fields")? {
        let model_fields: Bound<'py, PyDict> = cls.getattr("model_fields")?.cast_into()?;
        for (name_obj, field_obj) in model_fields.iter() {
            let py_name: String = name_obj.extract()?;
            let py_string = PyString::new(py, &py_name).unbind();
            let mut xml_name = py_name.clone();
            let mut kind = FieldKind::Element;
            let mut field_ns: Option<String> = None;

            let meta = field_obj
                .getattr("xsdata_metadata")
                .ok()
                .or_else(|| field_obj.getattr("json_schema_extra").ok());

            if let Some(ref m) = meta {
                if let Ok(t) = m.get_item("type") {
                    let t_str: String = t.extract().unwrap_or_default();
                    match t_str.as_str() {
                        "Attribute" => kind = FieldKind::Attribute,
                        "Attributes" => kind = FieldKind::AnyAttribute,
                        "Text" => kind = FieldKind::Text,
                        "Wildcard" => {
                            if let Ok(m_mixed) = m.get_item("mixed") {
                                if m_mixed.extract::<bool>().unwrap_or(false) {
                                    kind = FieldKind::Text;
                                } else {
                                    kind = FieldKind::Any;
                                }
                            } else {
                                kind = FieldKind::Any;
                            }
                        }
                        _ => kind = FieldKind::Element,
                    }
                }
                if let Ok(n) = m.get_item("name") {
                    xml_name = n.extract().unwrap_or(py_name.clone());
                }
                if let Ok(ns) = m.get_item("namespace") {
                    if let Ok(ns_str) = ns.extract::<String>() {
                        if !ns_str.is_empty() {
                            field_ns = Some(ns_str);
                        }
                    }
                }
            }

            let field_type = if let Some(ref hints) = type_hints {
                if let Ok(Some(hint)) = hints.get_item(&py_name) {
                    hint
                } else {
                    field_obj.getattr("annotation")?
                }
            } else {
                field_obj.getattr("annotation")?
            };
            let unwrapped_type = unwrap_optional_type(&field_type);
            let val_type = resolve_value_type(py, &field_type)?;

            cached_fields.push(CachedFieldMeta {
                py_string,
                is_init: true,
                is_kw_only: false,
                py_type: Some(unwrapped_type.into_any().unbind()),
            });

            let mut field_schema = FieldSchema::new(py_name, xml_name.as_bytes(), kind, val_type);
            field_schema.fixed_value = meta
                .as_ref()
                .and_then(|m| m.get_item("fixed").ok())
                .and_then(|v| v.extract::<String>().ok());
            field_schema.default_value = meta
                .as_ref()
                .and_then(|m| m.get_item("default").ok())
                .and_then(|v| v.extract::<String>().ok());
            if let Some(ns) = field_ns {
                field_schema = field_schema.namespace(ns);
            }
            builder = builder.field(field_schema);
        }
    } else if cls.hasattr("__dataclass_fields__")? {
        let fields: Bound<'py, PyDict> = cls.getattr("__dataclass_fields__")?.cast_into()?;
        for (name_obj, field_obj) in fields.iter() {
            let py_name: String = name_obj.extract()?;
            let py_string = PyString::new(py, &py_name).unbind();
            let mut xml_name = py_name.clone();
            let mut kind = FieldKind::Element;
            let mut field_ns: Option<String> = None;

            if let Ok(meta) = field_obj.getattr("metadata") {
                if let Ok(t) = meta.get_item("type") {
                    let t_str: String = t.extract().unwrap_or_default();
                    match t_str.as_str() {
                        "Attribute" => kind = FieldKind::Attribute,
                        "Attributes" => kind = FieldKind::AnyAttribute,
                        "Text" => kind = FieldKind::Text,
                        "Wildcard" => {
                            if let Ok(m_mixed) = meta.get_item("mixed") {
                                if m_mixed.extract::<bool>().unwrap_or(false) {
                                    kind = FieldKind::Text;
                                } else {
                                    kind = FieldKind::Any;
                                }
                            } else {
                                kind = FieldKind::Any;
                            }
                        }
                        _ => kind = FieldKind::Element,
                    }
                }
                if let Ok(n) = meta.get_item("name") {
                    xml_name = n.extract().unwrap_or(py_name.clone());
                }
                if let Ok(ns) = meta.get_item("namespace") {
                    if let Ok(ns_str) = ns.extract::<String>() {
                        if !ns_str.is_empty() {
                            field_ns = Some(ns_str);
                        }
                    }
                }
            }

            let is_init = field_obj
                .getattr("init")
                .and_then(|i| i.extract::<bool>())
                .unwrap_or(true);

            let is_kw_only = field_obj
                .getattr("kw_only")
                .and_then(|k| k.extract::<bool>())
                .unwrap_or(false);

            let field_type = if let Some(ref hints) = type_hints {
                if let Ok(Some(hint)) = hints.get_item(&py_name) {
                    hint
                } else {
                    field_obj.getattr("type")?
                }
            } else {
                field_obj.getattr("type")?
            };
            let unwrapped_type = unwrap_optional_type(&field_type);
            let val_type = resolve_value_type(py, &field_type)?;

            cached_fields.push(CachedFieldMeta {
                py_string,
                is_init,
                is_kw_only,
                py_type: Some(unwrapped_type.into_any().unbind()),
            });

            let mut field_schema = FieldSchema::new(py_name, xml_name.as_bytes(), kind, val_type);
            field_schema.fixed_value = field_obj
                .getattr("metadata")
                .ok()
                .and_then(|m| m.get_item("fixed").ok())
                .and_then(|v| v.extract::<String>().ok());
            field_schema.default_value = field_obj
                .getattr("metadata")
                .ok()
                .and_then(|m| m.get_item("default").ok())
                .and_then(|v| v.extract::<String>().ok());
            if let Some(ns) = field_ns {
                field_schema = field_schema.namespace(ns);
            }
            builder = builder.field(field_schema);
        }
    }

    let mut schema = builder.build();
    if let Ok(branches_obj) = cls
        .getattr("Meta")
        .and_then(|meta| meta.getattr("mixed_branches"))
    {
        let item_name: String = cls
            .getattr("Meta")?
            .getattr("mixed_items")
            .and_then(|value| value.extract())
            .unwrap_or_else(|_| "items".into());
        let item_index = schema
            .fields
            .iter()
            .position(|field| field.name == item_name)
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("Mixed content model has no items field")
            })?;
        let module_name: String = cls.getattr("__module__")?.extract()?;
        let module = py.import(&module_name)?;
        let builtins = py.import("builtins")?;
        let mut branches = Vec::new();
        for branch_obj in branches_obj.try_iter()? {
            let (variant_name, xml_name, namespace, type_name): (
                String,
                String,
                Option<String>,
                String,
            ) = branch_obj?.extract()?;
            let branch_type = module
                .getattr(type_name.as_str())
                .or_else(|_| builtins.getattr(type_name.as_str()))?;
            branches.push(MixedBranchSchema {
                variant_name,
                xml_name: xml_name.into_bytes(),
                namespace,
                val_type: resolve_value_type(py, &branch_type)?,
            });
        }
        Arc::get_mut(&mut schema)
            .expect("newly built schema must be unique")
            .mixed_content = Some(MixedContentSchema {
            items_index: item_index,
            branches,
        });
    }
    Ok((schema, cached_fields))
}

fn get_or_create_schema_meta<'py>(cls: &Bound<'py, PyType>) -> PyResult<Arc<CachedSchemaMeta>> {
    let type_key = cls.as_ptr() as usize;

    {
        let cache = SCHEMA_CACHE.read().unwrap_or_else(|p| p.into_inner());
        if let Some(ref map) = *cache {
            if let Some(meta) = map.get(&type_key) {
                let meta = Arc::clone(meta);
                drop(cache);
                refresh_variants(cls, &meta);
                return Ok(meta);
            }
        }
    }

    let is_dataclass = cls.hasattr("__dataclass_fields__")? && !cls.hasattr("model_fields")?;
    let (schema, cached_fields) = extract_schema_from_class(cls)?;
    let py_cls_obj: PyObject = cls.clone().into_any().unbind();

    let all_init =
        is_dataclass && !cached_fields.is_empty() && cached_fields.iter().all(|f| f.is_init);
    let kwnames = if all_init {
        let py_strings: Vec<Bound<'py, PyString>> = cached_fields
            .iter()
            .map(|f| f.py_string.bind(cls.py()).clone())
            .collect();
        PyTuple::new(cls.py(), &py_strings).ok().map(|t| t.unbind())
    } else {
        None
    };

    let meta = Arc::new(CachedSchemaMeta {
        schema: Arc::clone(&schema),
        py_cls: py_cls_obj,
        is_dataclass,
        all_init,
        kwnames,
        fields: cached_fields,
    });

    {
        let mut cache = SCHEMA_CACHE.write().unwrap_or_else(|p| p.into_inner());
        let map = cache.get_or_insert_with(HashMap::new);
        map.insert(type_key, Arc::clone(&meta));
    }

    {
        let mut class_map = CLASS_BY_SCHEMA.write().unwrap_or_else(|p| p.into_inner());
        let map = class_map.get_or_insert_with(HashMap::new);
        map.insert(schema.name.clone(), Arc::clone(&meta));
    }

    // xsi:type dispatch: register concrete subclasses as
    // derivations of this type. Deliberately runs AFTER the cache inserts so
    // a subclass field typed as this class resolves from cache instead of
    // re-entering extraction.
    refresh_variants(cls, &meta);

    Ok(meta)
}

/// Collect runtime schemas for every dataclass/Pydantic subclass of `cls`,
/// transitively, to power `xsi:type` dispatch.
fn discover_variants(cls: &Bound<'_, PyType>) -> Vec<Arc<ModelSchema>> {
    let mut out = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();
    let root_type = cls
        .getattr("Meta")
        .and_then(|meta| meta.getattr("root_type"))
        .ok();
    let dispatch_type = root_type
        .as_ref()
        .and_then(|value| value.cast::<PyType>().ok())
        .unwrap_or(cls);
    collect_subclass_schemas(dispatch_type, &mut out, &mut seen);
    out
}

fn collect_subclass_schemas(
    cls: &Bound<'_, PyType>,
    out: &mut Vec<Arc<ModelSchema>>,
    seen: &mut HashSet<usize>,
) {
    let Ok(subclasses) = cls.call_method0("__subclasses__") else {
        return;
    };
    let Ok(items) = subclasses.try_iter() else {
        return;
    };
    for item in items.flatten() {
        let Ok(sub) = item.cast::<PyType>() else {
            continue;
        };
        if !seen.insert(sub.as_ptr() as usize) {
            continue;
        }
        // Global element models bind a wire root; they are not xsi:type
        // derivations of the schema type they wrap.
        if sub
            .getattr("Meta")
            .and_then(|meta| meta.getattr("root_type"))
            .is_ok()
        {
            continue;
        }
        let usable = sub.hasattr("__dataclass_fields__").unwrap_or(false)
            || sub.hasattr("model_fields").unwrap_or(false);
        if usable {
            if let Ok(meta) = get_or_create_schema_meta(sub) {
                out.push(Arc::clone(&meta.schema));
            }
        }
        collect_subclass_schemas(sub, out, seen);
    }
}

fn get_or_create_schema<'py>(cls: &Bound<'py, PyType>) -> PyResult<(Arc<ModelSchema>, PyObject)> {
    let meta = get_or_create_schema_meta(cls)?;
    Ok((Arc::clone(&meta.schema), meta.py_cls.clone_ref(cls.py())))
}

fn convert_scalar_to_py<'py>(
    py: Python<'py>,
    val: &PolyValue,
    _scalar_type: &ScalarType,
    py_type_opt: Option<&Bound<'py, PyAny>>,
) -> PyResult<PyObject> {
    match val {
        PolyValue::Null => Ok(py.None()),
        PolyValue::Bool(b) => b.into_py_any(py),
        PolyValue::Int(i) => {
            if let Some(target_type) = py_type_opt {
                if is_enum_class(py, target_type) {
                    if let Ok(enum_val) = target_type.call1((*i,)) {
                        return Ok(enum_val.unbind());
                    }
                }
            }
            i.into_py_any(py)
        }
        PolyValue::Float(f) => f.into_py_any(py),
        PolyValue::String(s) => {
            if let Some(target_type) = py_type_opt {
                if is_enum_class(py, target_type) {
                    if let Ok(enum_val) = target_type.call1((s.as_str(),)) {
                        return Ok(enum_val.unbind());
                    }
                    if let Ok(int_val) = s.parse::<i64>() {
                        if let Ok(enum_val) = target_type.call1((int_val,)) {
                            return Ok(enum_val.unbind());
                        }
                    }
                    if let Ok(enum_val) = target_type.get_item(s.as_str()) {
                        return Ok(enum_val.unbind());
                    }
                }

                let type_name: String = target_type
                    .getattr("__name__")
                    .and_then(|n| n.extract())
                    .unwrap_or_default();

                match type_name.as_str() {
                    "Decimal" => {
                        let decimal_cls = py.import("decimal")?.getattr("Decimal")?;
                        let dec = decimal_cls.call1((s.as_str(),))?;
                        return Ok(dec.unbind());
                    }
                    "XmlDate" | "XmlDateTime" | "XmlTime" => {
                        if let Ok(val) = target_type.call_method1("from_string", (s.as_str(),)) {
                            return Ok(val.unbind());
                        }
                        if let Ok(datatype_mod) = py.import("pyxsdata.models.datatype") {
                            if let Ok(cls) = datatype_mod.getattr(type_name.as_str()) {
                                if let Ok(val) = cls.call_method1("from_string", (s.as_str(),)) {
                                    return Ok(val.unbind());
                                }
                            }
                        }
                        if let Ok(val) = target_type.call1((s.as_str(),)) {
                            return Ok(val.unbind());
                        }
                        let mod_name = match type_name.as_str() {
                            "XmlDate" => "date",
                            "XmlDateTime" => "datetime",
                            "XmlTime" => "time",
                            _ => "",
                        };
                        if let Ok(datetime_mod) = py.import("datetime") {
                            if let Ok(cls) = datetime_mod.getattr(mod_name) {
                                if let Ok(val) = cls.call_method1("fromisoformat", (s.as_str(),)) {
                                    return Ok(val.unbind());
                                }
                            }
                        }
                    }
                    "XmlDuration" => {
                        if let Ok(val) = target_type.call1((s.as_str(),)) {
                            return Ok(val.unbind());
                        }
                        if let Ok(datatype_mod) = py.import("pyxsdata.models.datatype") {
                            if let Ok(cls) = datatype_mod.getattr("XmlDuration") {
                                if let Ok(val) = cls.call1((s.as_str(),)) {
                                    return Ok(val.unbind());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(PyString::new(py, s).into_any().unbind())
        }
        _ => Ok(py.None()),
    }
}

fn poly_value_to_py<'py>(
    py: Python<'py>,
    val: &PolyValue,
    val_type: &ValueType,
    cls: Option<&Bound<'py, PyAny>>,
    field_meta: Option<&CachedFieldMeta>,
) -> PyResult<PyObject> {
    match val {
        PolyValue::Null => Ok(py.None()),
        PolyValue::Bool(b) => b.into_py_any(py),
        PolyValue::Int(_) | PolyValue::Float(_) | PolyValue::String(_) => {
            let st = match val_type {
                ValueType::Scalar(ref s) => s,
                _ => &ScalarType::Any,
            };
            convert_scalar_to_py(
                py,
                val,
                st,
                field_meta.and_then(|f| f.py_type.as_ref().map(|o| o.bind(py))),
            )
        }
        PolyValue::List(items) => {
            if let ValueType::List(inner_type) = val_type {
                let inner_cls = if let ValueType::Nested(ref s) = inner_type.as_ref() {
                    lookup_py_class(py, &s.name)
                } else {
                    None
                };
                let mut py_items = Vec::with_capacity(items.len());
                for item in items {
                    let py_item =
                        poly_value_to_py(py, item, inner_type, inner_cls.as_ref(), field_meta)?;
                    py_items.push(py_item);
                }
                let py_list = PyList::new(py, &py_items)?;
                Ok(py_list.into_any().unbind())
            } else {
                let py_list = PyList::empty(py);
                Ok(py_list.into_any().unbind())
            }
        }
        PolyValue::Record {
            schema: rec_schema,
            values,
        } => {
            // xsi:type dispatch: the record was parsed as a
            // concrete derivation of the declared type, so construct it with
            // the derivation's Python class.
            if let ValueType::Nested(ref declared) = val_type {
                if declared.matches_variant(rec_schema) {
                    if let Some(variant_meta) = lookup_cached_meta(&rec_schema.name) {
                        let variant_cls = variant_meta.py_cls.bind(py).clone();
                        let variant_vt = ValueType::Nested(Arc::clone(rec_schema));
                        return poly_value_to_py(py, val, &variant_vt, Some(&variant_cls), None);
                    }
                }
            }

            let meta_opt = if let ValueType::Nested(ref s) = val_type {
                lookup_cached_meta(&s.name)
            } else {
                None
            };

            let effective_cls = match cls {
                Some(c) => Some(c.clone()),
                None => meta_opt.as_ref().map(|m| m.py_cls.bind(py).clone()),
            };

            if let Some(ref target_cls) = effective_cls {
                if let ValueType::Nested(ref schema) = val_type {
                    // Method 1: Python 3.12+ Vectorcall for Dataclasses
                    if let Some(ref meta) = meta_opt {
                        if meta.is_dataclass && meta.all_init && !meta.fields.is_empty() {
                            if let Some(ref kwnames) = meta.kwnames {
                                let mut args_ptrs: SmallVec<[*mut pyo3::ffi::PyObject; 16]> =
                                    SmallVec::with_capacity(schema.fields.len());
                                let mut py_vals: SmallVec<[PyObject; 16]> =
                                    SmallVec::with_capacity(schema.fields.len());
                                let mut all_found = true;

                                for (i, field) in schema.fields.iter().enumerate() {
                                    if let Some(Some(field_val)) = values.get(i) {
                                        let field_cls =
                                            if let ValueType::Nested(ref s) = field.val_type {
                                                lookup_py_class(py, &s.name)
                                            } else {
                                                None
                                            };
                                        let f_meta = meta.fields.get(i);
                                        let py_val = poly_value_to_py(
                                            py,
                                            field_val,
                                            &field.val_type,
                                            field_cls.as_ref(),
                                            f_meta,
                                        )?;
                                        args_ptrs.push(py_val.as_ptr());
                                        py_vals.push(py_val);
                                    } else {
                                        all_found = false;
                                        break;
                                    }
                                }

                                if all_found && args_ptrs.len() == schema.fields.len() {
                                    unsafe {
                                        let res_ptr = pyo3::ffi::PyObject_Vectorcall(
                                            target_cls.as_ptr(),
                                            args_ptrs.as_ptr(),
                                            0,
                                            kwnames.as_ptr(),
                                        );
                                        if !res_ptr.is_null() {
                                            return Ok(Bound::from_owned_ptr(py, res_ptr).unbind());
                                        } else {
                                            return Err(PyErr::fetch(py));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Method 2: Keyword arguments with init=False post-init support
                    let kwargs = PyDict::new(py);
                    let mut post_init_fields: Vec<(Py<PyString>, PyObject)> = Vec::new();

                    for (i, field) in schema.fields.iter().enumerate() {
                        if let Some(Some(field_val)) = values.get(i) {
                            let field_cls = if let ValueType::Nested(ref s) = field.val_type {
                                lookup_py_class(py, &s.name)
                            } else {
                                None
                            };
                            let f_meta = meta_opt.as_ref().and_then(|m| m.fields.get(i));
                            let is_init = f_meta.map(|f| f.is_init).unwrap_or(true);
                            let py_val = poly_value_to_py(
                                py,
                                field_val,
                                &field.val_type,
                                field_cls.as_ref(),
                                f_meta,
                            )?;
                            if let Some(meta) = f_meta {
                                if is_init {
                                    kwargs.set_item(meta.py_string.bind(py), py_val)?;
                                } else {
                                    post_init_fields.push((meta.py_string.clone_ref(py), py_val));
                                }
                            } else if is_init {
                                kwargs.set_item(&field.name, py_val)?;
                            } else {
                                post_init_fields
                                    .push((PyString::new(py, &field.name).unbind(), py_val));
                            }
                        }
                    }
                    let instance = target_cls.call((), Some(&kwargs))?;
                    for (attr_name, val) in post_init_fields {
                        instance.setattr(&attr_name, val)?;
                    }
                    return Ok(instance.unbind());
                }

                let dict = PyDict::new(py);
                for (i, field) in rec_schema.fields.iter().enumerate() {
                    if let Some(Some(v)) = values.get(i) {
                        let py_v = poly_value_to_py(
                            py,
                            v,
                            &ValueType::Scalar(ScalarType::Any),
                            None,
                            None,
                        )?;
                        dict.set_item(&field.name, py_v)?;
                    }
                }
                let instance = target_cls.call((), Some(&dict))?;
                Ok(instance.unbind())
            } else {
                let dict = PyDict::new(py);
                for (i, field) in rec_schema.fields.iter().enumerate() {
                    if let Some(Some(v)) = values.get(i) {
                        let py_v = poly_value_to_py(
                            py,
                            v,
                            &ValueType::Scalar(ScalarType::Any),
                            None,
                            None,
                        )?;
                        dict.set_item(&field.name, py_v)?;
                    }
                }
                Ok(dict.into_any().unbind())
            }
        }
        PolyValue::Object(map) => {
            if map.contains_key("qname") && !matches!(val_type, ValueType::Nested(_)) {
                if let Ok(polyxml_mod) = py.import("polyxml") {
                    if let Ok(any_cls) = polyxml_mod.getattr("AnyElement") {
                        let kwargs = PyDict::new(py);
                        if let Some(PolyValue::String(qname)) = map.get("qname") {
                            kwargs.set_item("qname", qname)?;
                        }
                        if let Some(PolyValue::String(text)) = map.get("text") {
                            if !text.is_empty() {
                                kwargs.set_item("text", text)?;
                            }
                        }
                        if let Some(PolyValue::Object(attrs)) = map.get("attributes") {
                            let py_attrs = PyDict::new(py);
                            for (k, v) in attrs {
                                if let PolyValue::String(s) = v {
                                    py_attrs.set_item(k, s)?;
                                }
                            }
                            kwargs.set_item("attributes", py_attrs)?;
                        }
                        if let Some(PolyValue::List(children)) = map.get("children") {
                            let py_children = PyList::empty(py);
                            for child in children {
                                let py_child = poly_value_to_py(
                                    py,
                                    child,
                                    &ValueType::Scalar(ScalarType::Any),
                                    None,
                                    None,
                                )?;
                                py_children.append(py_child)?;
                            }
                            kwargs.set_item("children", py_children)?;
                        }
                        let instance = any_cls.call((), Some(&kwargs))?;
                        return Ok(instance.unbind());
                    }
                }
            }

            let meta_opt = if let ValueType::Nested(ref s) = val_type {
                lookup_cached_meta(&s.name)
            } else {
                None
            };

            let effective_cls = match cls {
                Some(c) => Some(c.clone()),
                None => meta_opt.as_ref().map(|m| m.py_cls.bind(py).clone()),
            };

            if let Some(ref target_cls) = effective_cls {
                if let ValueType::Nested(ref schema) = val_type {
                    // Method 1: Python 3.12+ Vectorcall for Dataclasses
                    if let Some(ref meta) = meta_opt {
                        if meta.is_dataclass && meta.all_init && !meta.fields.is_empty() {
                            if let Some(ref kwnames) = meta.kwnames {
                                let mut args_ptrs: SmallVec<[*mut pyo3::ffi::PyObject; 16]> =
                                    SmallVec::with_capacity(schema.fields.len());
                                let mut py_vals: SmallVec<[PyObject; 16]> =
                                    SmallVec::with_capacity(schema.fields.len());
                                let mut all_found = true;

                                for (i, field) in schema.fields.iter().enumerate() {
                                    if let Some(field_val) = map.get(&field.name) {
                                        let field_cls =
                                            if let ValueType::Nested(ref s) = field.val_type {
                                                lookup_py_class(py, &s.name)
                                            } else {
                                                None
                                            };
                                        let f_meta = meta.fields.get(i);
                                        let py_val = poly_value_to_py(
                                            py,
                                            field_val,
                                            &field.val_type,
                                            field_cls.as_ref(),
                                            f_meta,
                                        )?;
                                        args_ptrs.push(py_val.as_ptr());
                                        py_vals.push(py_val);
                                    } else {
                                        all_found = false;
                                        break;
                                    }
                                }

                                if all_found && args_ptrs.len() == schema.fields.len() {
                                    unsafe {
                                        let res_ptr = pyo3::ffi::PyObject_Vectorcall(
                                            target_cls.as_ptr(),
                                            args_ptrs.as_ptr(),
                                            0,
                                            kwnames.as_ptr(),
                                        );
                                        if !res_ptr.is_null() {
                                            return Ok(Bound::from_owned_ptr(py, res_ptr).unbind());
                                        } else {
                                            return Err(PyErr::fetch(py));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Method 2: Keyword arguments with init=False post-init support
                    let kwargs = PyDict::new(py);
                    let mut post_init_fields: Vec<(Py<PyString>, PyObject)> = Vec::new();

                    for (i, field) in schema.fields.iter().enumerate() {
                        if let Some(field_val) = map.get(&field.name) {
                            let field_cls = if let ValueType::Nested(ref s) = field.val_type {
                                lookup_py_class(py, &s.name)
                            } else {
                                None
                            };
                            let f_meta = meta_opt.as_ref().and_then(|m| m.fields.get(i));
                            let is_init = f_meta.map(|f| f.is_init).unwrap_or(true);
                            let py_val = poly_value_to_py(
                                py,
                                field_val,
                                &field.val_type,
                                field_cls.as_ref(),
                                f_meta,
                            )?;
                            if let Some(meta) = f_meta {
                                if is_init {
                                    kwargs.set_item(meta.py_string.bind(py), py_val)?;
                                } else {
                                    post_init_fields.push((meta.py_string.clone_ref(py), py_val));
                                }
                            } else if is_init {
                                kwargs.set_item(&field.name, py_val)?;
                            } else {
                                post_init_fields
                                    .push((PyString::new(py, &field.name).unbind(), py_val));
                            }
                        }
                    }
                    let instance = target_cls.call((), Some(&kwargs))?;
                    for (attr_name, val) in post_init_fields {
                        instance.setattr(&attr_name, val)?;
                    }
                    return Ok(instance.unbind());
                }
                let dict = PyDict::new(py);
                for (k, v) in map {
                    let py_v =
                        poly_value_to_py(py, v, &ValueType::Scalar(ScalarType::Any), None, None)?;
                    dict.set_item(k, py_v)?;
                }
                let instance = target_cls.call((), Some(&dict))?;
                Ok(instance.unbind())
            } else {
                let dict = PyDict::new(py);
                for (k, v) in map {
                    let py_v =
                        poly_value_to_py(py, v, &ValueType::Scalar(ScalarType::Any), None, None)?;
                    dict.set_item(k, py_v)?;
                }
                Ok(dict.into_any().unbind())
            }
        }
    }
}

fn py_any_element_to_poly_value<'py>(
    py: Python<'py>,
    item: &Bound<'py, PyAny>,
) -> PyResult<PolyValue> {
    if let Ok(qname_obj) = item.getattr("qname") {
        if !qname_obj.is_none() {
            let qname: String = qname_obj.extract()?;
            let mut map = HashMap::new();
            map.insert("qname".to_string(), PolyValue::String(qname));
            if let Ok(text_obj) = item.getattr("text") {
                if !text_obj.is_none() {
                    let text: String = text_obj.extract()?;
                    map.insert("text".to_string(), PolyValue::String(text));
                }
            }
            if let Ok(attrs_obj) = item.getattr("attributes") {
                if let Ok(dict) = attrs_obj.cast::<PyDict>() {
                    let mut attr_map = HashMap::new();
                    for (k, v) in dict.iter() {
                        let k_str: String = k.extract()?;
                        let v_str: String = v.extract()?;
                        attr_map.insert(k_str, PolyValue::String(v_str));
                    }
                    map.insert("attributes".to_string(), PolyValue::Object(attr_map));
                }
            }
            if let Ok(children_obj) = item.getattr("children") {
                if let Ok(list) = children_obj.cast::<PyList>() {
                    let mut child_vec = Vec::new();
                    for child in list.iter() {
                        child_vec.push(py_any_element_to_poly_value(py, &child)?);
                    }
                    map.insert("children".to_string(), PolyValue::List(child_vec));
                }
            }
            return Ok(PolyValue::Object(map));
        }
    }
    if let Ok(tag_obj) = item.getattr("tag") {
        if !tag_obj.is_none() {
            let tag: String = tag_obj.extract()?;
            let mut map = HashMap::new();
            map.insert("qname".to_string(), PolyValue::String(tag));
            if let Ok(text_obj) = item.getattr("text") {
                if !text_obj.is_none() {
                    let text: String = text_obj.extract()?;
                    map.insert("text".to_string(), PolyValue::String(text));
                }
            }
            if let Ok(attrib_obj) = item.getattr("attrib") {
                if let Ok(dict) = attrib_obj.cast::<PyDict>() {
                    let mut attr_map = HashMap::new();
                    for (k, v) in dict.iter() {
                        let k_str: String = k.extract()?;
                        let v_str: String = v.extract()?;
                        attr_map.insert(k_str, PolyValue::String(v_str));
                    }
                    map.insert("attributes".to_string(), PolyValue::Object(attr_map));
                }
            }
            return Ok(PolyValue::Object(map));
        }
    }
    if let Ok(s) = item.extract::<String>() {
        return Ok(PolyValue::String(s));
    }
    Ok(PolyValue::Null)
}

fn py_to_poly_value<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    schema: &ModelSchema,
    meta_opt: Option<&CachedSchemaMeta>,
) -> PyResult<PolyValue> {
    if let Some(meta) = meta_opt {
        // xsi:type dispatch: a concrete subclass instance in a
        // field declared as the base type builds the derivation's record so
        // the serializer re-emits xsi:type and keeps every concrete field.
        let obj_type = obj.get_type();
        if obj_type.as_ptr() != meta.py_cls.as_ptr() {
            if let Ok(actual_meta) = get_or_create_schema_meta(&obj_type) {
                if meta.schema.matches_variant(&actual_meta.schema) {
                    return py_to_poly_value(py, obj, &actual_meta.schema, Some(&actual_meta));
                }
            }
        }

        let mut values: Vec<Option<PolyValue>> = vec![None; schema.fields.len()];

        for (i, field) in schema.fields.iter().enumerate() {
            let val_res = if let Some(f_meta) = meta.fields.get(i) {
                obj.getattr(f_meta.py_string.bind(py))
            } else {
                obj.getattr(field.name.as_str())
            };

            if let Ok(val) = val_res {
                if val.is_none() {
                    values[i] = Some(PolyValue::Null);
                    continue;
                }

                if let Some(mixed) = &schema.mixed_content {
                    if i == mixed.items_index {
                        let list = val.cast::<PyList>()?;
                        let items = list
                            .iter()
                            .map(|item| py_mixed_item_to_poly(py, &item, mixed))
                            .collect::<PyResult<Vec<_>>>()?;
                        values[i] = Some(PolyValue::List(items));
                        continue;
                    }
                }

                match &field.val_type {
                    ValueType::Scalar(st) => match st {
                        ScalarType::Int => {
                            let int_val: i64 = if let Ok(enum_val) = val.getattr("value") {
                                enum_val.extract()?
                            } else {
                                val.extract()?
                            };
                            values[i] = Some(PolyValue::Int(int_val));
                        }
                        ScalarType::Float => {
                            let f: f64 = val.extract()?;
                            values[i] = Some(PolyValue::Float(f));
                        }
                        ScalarType::Bool => {
                            let b: bool = val.extract()?;
                            values[i] = Some(PolyValue::Bool(b));
                        }
                        _ => {
                            if field.kind == FieldKind::Any {
                                values[i] = Some(py_any_element_to_poly_value(py, &val)?);
                            } else {
                                let s: String = if let Ok(enum_val) = val.getattr("value") {
                                    enum_val.str()?.extract()?
                                } else {
                                    val.str()?.extract()?
                                };
                                values[i] = Some(PolyValue::String(s));
                            }
                        }
                    },
                    ValueType::List(inner) => {
                        if let Ok(list) = val.cast::<PyList>() {
                            let mut poly_items = Vec::with_capacity(list.len());
                            let child_meta = if let ValueType::Nested(sub_schema) = inner.as_ref() {
                                lookup_cached_meta(&sub_schema.name)
                            } else {
                                None
                            };
                            for item in list.iter() {
                                if let ValueType::Nested(sub_schema) = inner.as_ref() {
                                    poly_items.push(py_to_poly_value(
                                        py,
                                        &item,
                                        sub_schema,
                                        child_meta.as_deref(),
                                    )?);
                                } else if field.kind == FieldKind::Any {
                                    poly_items.push(py_any_element_to_poly_value(py, &item)?);
                                } else {
                                    let s: String = if let Ok(enum_val) = item.getattr("value") {
                                        enum_val.str()?.extract()?
                                    } else {
                                        item.str()?.extract()?
                                    };
                                    poly_items.push(PolyValue::String(s));
                                }
                            }
                            values[i] = Some(PolyValue::List(poly_items));
                        }
                    }
                    ValueType::Nested(sub_schema) => {
                        let child_meta = lookup_cached_meta(&sub_schema.name);
                        values[i] = Some(py_to_poly_value(
                            py,
                            &val,
                            sub_schema,
                            child_meta.as_deref(),
                        )?);
                    }
                }
            }
        }
        return Ok(PolyValue::Record {
            schema: Arc::clone(&meta.schema),
            values: values.into_boxed_slice(),
        });
    }

    let mut map = HashMap::with_capacity(schema.fields.len());

    for field in &schema.fields {
        let val_res = obj.getattr(field.name.as_str());

        if let Ok(val) = val_res {
            if val.is_none() {
                map.insert(field.name.clone(), PolyValue::Null);
                continue;
            }

            match &field.val_type {
                ValueType::Scalar(st) => match st {
                    ScalarType::Int => {
                        let i: i64 = if let Ok(enum_val) = val.getattr("value") {
                            enum_val.extract()?
                        } else {
                            val.extract()?
                        };
                        map.insert(field.name.clone(), PolyValue::Int(i));
                    }
                    ScalarType::Float => {
                        let f: f64 = val.extract()?;
                        map.insert(field.name.clone(), PolyValue::Float(f));
                    }
                    ScalarType::Bool => {
                        let b: bool = val.extract()?;
                        map.insert(field.name.clone(), PolyValue::Bool(b));
                    }
                    _ => {
                        if field.kind == FieldKind::Any {
                            map.insert(field.name.clone(), py_any_element_to_poly_value(py, &val)?);
                        } else {
                            let s: String = if let Ok(enum_val) = val.getattr("value") {
                                enum_val.str()?.extract()?
                            } else {
                                val.str()?.extract()?
                            };
                            map.insert(field.name.clone(), PolyValue::String(s));
                        }
                    }
                },
                ValueType::List(inner) => {
                    if let Ok(list) = val.cast::<PyList>() {
                        let mut poly_items = Vec::with_capacity(list.len());
                        let child_meta = if let ValueType::Nested(sub_schema) = inner.as_ref() {
                            lookup_cached_meta(&sub_schema.name)
                        } else {
                            None
                        };
                        for item in list.iter() {
                            if let ValueType::Nested(sub_schema) = inner.as_ref() {
                                poly_items.push(py_to_poly_value(
                                    py,
                                    &item,
                                    sub_schema,
                                    child_meta.as_deref(),
                                )?);
                            } else if field.kind == FieldKind::Any {
                                poly_items.push(py_any_element_to_poly_value(py, &item)?);
                            } else {
                                let s: String = if let Ok(enum_val) = item.getattr("value") {
                                    enum_val.str()?.extract()?
                                } else {
                                    item.str()?.extract()?
                                };
                                poly_items.push(PolyValue::String(s));
                            }
                        }
                        map.insert(field.name.clone(), PolyValue::List(poly_items));
                    }
                }
                ValueType::Nested(sub_schema) => {
                    let child_meta = lookup_cached_meta(&sub_schema.name);
                    map.insert(
                        field.name.clone(),
                        py_to_poly_value(py, &val, sub_schema, child_meta.as_deref())?,
                    );
                }
            }
        }
    }

    Ok(PolyValue::Object(map))
}

fn py_mixed_item_to_poly(
    py: Python<'_>,
    item: &Bound<'_, PyAny>,
    mixed: &MixedContentSchema,
) -> PyResult<PolyValue> {
    let kind: String = item.getattr("kind")?.extract()?;
    let branch = mixed
        .branches
        .iter()
        .find(|branch| branch.variant_name == kind || branch.xml_name == kind.as_bytes())
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!("Unknown mixed content kind: {kind}"))
        })?;
    let value = item.getattr("value")?;
    let content = match &branch.val_type {
        ValueType::Scalar(ScalarType::Int) => PolyValue::Int(value.extract()?),
        ValueType::Scalar(ScalarType::Float) => PolyValue::Float(value.extract()?),
        ValueType::Scalar(ScalarType::Bool) => PolyValue::Bool(value.extract()?),
        ValueType::Scalar(_) => PolyValue::String(value.str()?.extract()?),
        ValueType::Nested(schema) => {
            let meta = lookup_cached_meta(&schema.name);
            py_to_poly_value(py, &value, schema, meta.as_deref())?
        }
        ValueType::List(_) => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Mixed content branch cannot be a list",
            ))
        }
    };
    let mut tagged = HashMap::new();
    tagged.insert("kind".into(), PolyValue::String(kind));
    tagged.insert("value".into(), content);
    Ok(PolyValue::Object(tagged))
}

#[pyfunction]
fn deserialize<'py>(
    py: Python<'py>,
    source: &[u8],
    target_type: Bound<'py, PyType>,
) -> PyResult<PyObject> {
    let meta = get_or_create_schema_meta(&target_type)?;
    let poly_val = polyxml::deserialize(source, Arc::clone(&meta.schema))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let bound_cls = meta.py_cls.bind(py);
    poly_value_to_py(
        py,
        &poly_val,
        &ValueType::Nested(Arc::clone(&meta.schema)),
        Some(bound_cls),
        None,
    )
}

#[pyfunction]
#[pyo3(signature = (source, target_type, tag=None))]
fn iterparse<'py>(
    py: Python<'py>,
    source: &[u8],
    target_type: Bound<'py, PyType>,
    tag: Option<String>,
) -> PyResult<XmlIterator> {
    let meta = get_or_create_schema_meta(&target_type)?;
    let target_tag = tag.unwrap_or_else(|| meta.schema.name.clone());
    let stream = polyxml::XmlItemStream::new(
        std::io::Cursor::new(source.to_vec()),
        Arc::clone(&meta.schema),
        target_tag.as_bytes(),
    );
    Ok(XmlIterator {
        stream,
        schema: Arc::clone(&meta.schema),
        py_cls: meta.py_cls.clone_ref(py),
    })
}

#[pyclass]
struct XmlIterator {
    stream: polyxml::XmlItemStream<std::io::Cursor<Vec<u8>>>,
    schema: Arc<ModelSchema>,
    py_cls: PyObject,
}

#[pymethods]
impl XmlIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<Option<PyObject>> {
        match slf.stream.next_item() {
            Ok(Some(poly_val)) => {
                let schema = Arc::clone(&slf.schema);
                let target_cls = slf.py_cls.clone_ref(py);
                let bound_cls = target_cls.bind(py);
                let py_obj = poly_value_to_py(
                    py,
                    &poly_val,
                    &ValueType::Nested(schema),
                    Some(bound_cls),
                    None,
                )?;
                Ok(Some(py_obj))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
        }
    }
}

#[pyfunction]
#[pyo3(signature = (obj, target_type=None, indent=None, namespaces=None, ns_map=None))]
fn serialize<'py>(
    py: Python<'py>,
    obj: Bound<'py, PyAny>,
    target_type: Option<Bound<'py, PyType>>,
    indent: Option<usize>,
    namespaces: Option<bool>,
    ns_map: Option<Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyBytes>> {
    // With `target_type`, serialize against the declared base type so
    // xsi:type dispatch round-trips the element name; without
    // it, the instance's own concrete type is used.
    let meta = match &target_type {
        Some(t) => get_or_create_schema_meta(t)?,
        None => get_or_create_schema_meta(&obj.get_type())?,
    };

    let poly_val = py_to_poly_value(py, &obj, &meta.schema, Some(&meta))?;
    let root_name = std::str::from_utf8(&meta.schema.xml_name).unwrap_or(meta.schema.name.as_str());

    let rust_ns_map = if let Some(dict) = ns_map {
        let mut map = std::collections::HashMap::new();
        for (k, v) in dict.iter() {
            let k_str = if k.is_none() {
                String::new()
            } else {
                k.extract::<String>().unwrap_or_default()
            };
            let v_str = if v.is_none() {
                String::new()
            } else {
                v.extract::<String>().unwrap_or_default()
            };
            map.insert(k_str, v_str);
        }
        Some(map)
    } else {
        None
    };

    let bytes = polyxml::serialize_with_options(
        root_name,
        &poly_val,
        &meta.schema,
        indent,
        namespaces,
        rust_ns_map.as_ref(),
    )
    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
fn deserialize_json<'py>(
    py: Python<'py>,
    source: &[u8],
    target_type: Bound<'py, PyType>,
) -> PyResult<PyObject> {
    let meta = get_or_create_schema_meta(&target_type)?;
    let poly_val = polyxml::deserialize_json(source, Arc::clone(&meta.schema))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let bound_cls = meta.py_cls.bind(py);
    poly_value_to_py(
        py,
        &poly_val,
        &ValueType::Nested(Arc::clone(&meta.schema)),
        Some(bound_cls),
        None,
    )
}

#[pyfunction]
#[pyo3(signature = (obj, indent=None, by_alias=None))]
fn serialize_json<'py>(
    py: Python<'py>,
    obj: Bound<'py, PyAny>,
    indent: Option<usize>,
    by_alias: Option<bool>,
) -> PyResult<Bound<'py, PyBytes>> {
    let cls = obj.get_type();
    let meta = get_or_create_schema_meta(&cls)?;

    let poly_val = py_to_poly_value(py, &obj, &meta.schema, Some(&meta))?;
    let bytes = polyxml::serialize_json(&poly_val, &meta.schema, indent, by_alias.unwrap_or(true))
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
#[pyo3(signature = (source, target_type=None, schema_path=None, root=None, indent=None, by_alias=None))]
fn xml_to_json<'py>(
    py: Python<'py>,
    source: &[u8],
    target_type: Option<Bound<'py, PyType>>,
    schema_path: Option<&str>,
    root: Option<&str>,
    indent: Option<usize>,
    by_alias: Option<bool>,
) -> PyResult<Bound<'py, PyBytes>> {
    let bytes = if let Some(ref target_type) = target_type {
        let meta = get_or_create_schema_meta(target_type)?;
        polyxml::xml_to_json(
            source,
            Some(Arc::clone(&meta.schema)),
            indent,
            by_alias.unwrap_or(true),
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
    } else if let Some(path) = schema_path {
        let xsd_str = std::fs::read_to_string(path).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to read schema file: {}", e))
        })?;
        let mut parser = polyxml::schema_parser::XsdParser::new();
        let ir = parser.parse_str(&xsd_str).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to parse schema: {}", e))
        })?;
        let model_schema = ModelSchema::from_ir(&ir, root).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Failed to build schema from IR: {}",
                e
            ))
        })?;
        polyxml::xml_to_json(source, Some(model_schema), indent, by_alias.unwrap_or(true))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
    } else {
        polyxml::transcoder::xml_to_json_dynamic(source, indent)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
    };

    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
#[pyo3(signature = (source, target_type=None, schema_path=None, root=None, indent=None, namespaces=None, ns_map=None))]
fn json_to_xml<'py>(
    py: Python<'py>,
    source: &[u8],
    target_type: Option<Bound<'py, PyType>>,
    schema_path: Option<&str>,
    root: Option<&str>,
    indent: Option<usize>,
    namespaces: Option<bool>,
    ns_map: Option<HashMap<String, String>>,
) -> PyResult<Bound<'py, PyBytes>> {
    let bytes = if let Some(ref target_type) = target_type {
        let meta = get_or_create_schema_meta(target_type)?;
        let root_name = root.unwrap_or(&meta.schema.name);
        polyxml::json_to_xml(
            source,
            Some(Arc::clone(&meta.schema)),
            Some(root_name),
            indent,
            namespaces,
            ns_map.as_ref(),
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
    } else if let Some(path) = schema_path {
        let xsd_str = std::fs::read_to_string(path).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to read schema file: {}", e))
        })?;
        let mut parser = polyxml::schema_parser::XsdParser::new();
        let ir = parser.parse_str(&xsd_str).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to parse schema: {}", e))
        })?;
        let model_schema = ModelSchema::from_ir(&ir, root).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Failed to build schema from IR: {}",
                e
            ))
        })?;
        let root_name = root
            .map(|s| s.to_string())
            .unwrap_or_else(|| model_schema.name.clone());
        polyxml::json_to_xml(
            source,
            Some(model_schema),
            Some(&root_name),
            indent,
            namespaces,
            ns_map.as_ref(),
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
    } else {
        polyxml::transcoder::json_to_xml_dynamic(source, root, indent)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
    };

    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _polyxml(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<XmlIterator>()?;
    m.add_function(wrap_pyfunction!(deserialize, m)?)?;
    m.add_function(wrap_pyfunction!(deserialize_json, m)?)?;
    m.add_function(wrap_pyfunction!(iterparse, m)?)?;
    m.add_function(wrap_pyfunction!(serialize, m)?)?;
    m.add_function(wrap_pyfunction!(serialize_json, m)?)?;
    m.add_function(wrap_pyfunction!(xml_to_json, m)?)?;
    m.add_function(wrap_pyfunction!(json_to_xml, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
