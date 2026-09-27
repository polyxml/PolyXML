#![allow(clippy::missing_safety_doc)]

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Arc;

use polyxml::schema::{
    FieldKind, FieldSchema, ModelSchema, ModelSchemaBuilder, ScalarType, ValueType,
};
use polyxml::value::PolyValue;

// Opaque types
pub struct PolyXmlSchemaBuilder(ModelSchemaBuilder);
pub struct PolyXmlSchema(Arc<ModelSchema>);
pub struct PolyXmlValue(PolyValue);

#[repr(C)]
pub enum PolyXmlFieldKind {
    Attribute = 0,
    Element = 1,
    Text = 2,
}

#[repr(C)]
pub enum PolyXmlScalarType {
    String = 0,
    Int = 1,
    Float = 2,
    Bool = 3,
    Decimal = 4,
    XmlDate = 5,
    XmlDateTime = 6,
    Any = 7,
}

#[repr(C)]
pub enum PolyXmlErrorCode {
    Ok = 0,
    ErrSyntax = 1,
    ErrScalar = 2,
    ErrSchema = 3,
    ErrNullPtr = 4,
    ErrUtf8 = 5,
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_builder_create(
    name: *const c_char,
) -> *mut PolyXmlSchemaBuilder {
    if name.is_null() {
        return std::ptr::null_mut();
    }
    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    Box::into_raw(Box::new(PolyXmlSchemaBuilder(ModelSchema::builder(
        name_str,
    ))))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_builder_set_namespace(
    builder: *mut PolyXmlSchemaBuilder,
    namespace_uri: *const c_char,
) {
    if builder.is_null() || namespace_uri.is_null() {
        return;
    }
    let ns_str = match CStr::from_ptr(namespace_uri).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    let b = &mut *builder;
    b.0 = std::mem::replace(&mut b.0, ModelSchema::builder("")).with_namespace(ns_str);
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_builder_add_field(
    builder: *mut PolyXmlSchemaBuilder,
    name: *const c_char,
    xml_name: *const c_char,
    kind: PolyXmlFieldKind,
    scalar_type: PolyXmlScalarType,
) {
    polyxml_schema_builder_add_field_with_namespace(
        builder,
        name,
        xml_name,
        kind,
        scalar_type,
        std::ptr::null(),
    );
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_builder_add_field_with_namespace(
    builder: *mut PolyXmlSchemaBuilder,
    name: *const c_char,
    xml_name: *const c_char,
    kind: PolyXmlFieldKind,
    scalar_type: PolyXmlScalarType,
    namespace_uri: *const c_char,
) {
    if builder.is_null() || name.is_null() || xml_name.is_null() {
        return;
    }
    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    let xml_bytes = CStr::from_ptr(xml_name).to_bytes();

    let field_kind = match kind {
        PolyXmlFieldKind::Attribute => FieldKind::Attribute,
        PolyXmlFieldKind::Element => FieldKind::Element,
        PolyXmlFieldKind::Text => FieldKind::Text,
    };

    let sc_type = match scalar_type {
        PolyXmlScalarType::String => ScalarType::String,
        PolyXmlScalarType::Int => ScalarType::Int,
        PolyXmlScalarType::Float => ScalarType::Float,
        PolyXmlScalarType::Bool => ScalarType::Bool,
        PolyXmlScalarType::Decimal => ScalarType::Decimal,
        PolyXmlScalarType::XmlDate => ScalarType::XmlDate,
        PolyXmlScalarType::XmlDateTime => ScalarType::XmlDateTime,
        PolyXmlScalarType::Any => ScalarType::Any,
    };

    let mut field = FieldSchema::new(name_str, xml_bytes, field_kind, ValueType::Scalar(sc_type));
    if !namespace_uri.is_null() {
        if let Ok(ns_str) = CStr::from_ptr(namespace_uri).to_str() {
            field = field.with_namespace(ns_str);
        }
    }
    let b = &mut *builder;
    b.0 = std::mem::replace(&mut b.0, ModelSchema::builder("")).field(field);
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_builder_add_nested_field(
    builder: *mut PolyXmlSchemaBuilder,
    name: *const c_char,
    xml_name: *const c_char,
    nested_schema: *const PolyXmlSchema,
    namespace_uri: *const c_char,
) {
    if builder.is_null() || name.is_null() || xml_name.is_null() || nested_schema.is_null() {
        return;
    }
    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    let xml_bytes = CStr::from_ptr(xml_name).to_bytes();
    let nested = Arc::clone(&(*nested_schema).0);

    let mut field = FieldSchema::new(
        name_str,
        xml_bytes,
        FieldKind::Element,
        ValueType::Nested(nested),
    );
    if !namespace_uri.is_null() {
        if let Ok(ns_str) = CStr::from_ptr(namespace_uri).to_str() {
            field = field.with_namespace(ns_str);
        }
    }
    let b = &mut *builder;
    b.0 = std::mem::replace(&mut b.0, ModelSchema::builder("")).field(field);
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_builder_add_list_nested_field(
    builder: *mut PolyXmlSchemaBuilder,
    name: *const c_char,
    xml_name: *const c_char,
    item_schema: *const PolyXmlSchema,
    namespace_uri: *const c_char,
) {
    if builder.is_null() || name.is_null() || xml_name.is_null() || item_schema.is_null() {
        return;
    }
    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    let xml_bytes = CStr::from_ptr(xml_name).to_bytes();
    let item = Arc::clone(&(*item_schema).0);

    let mut field = FieldSchema::new(
        name_str,
        xml_bytes,
        FieldKind::Element,
        ValueType::List(Box::new(ValueType::Nested(item))),
    );
    if !namespace_uri.is_null() {
        if let Ok(ns_str) = CStr::from_ptr(namespace_uri).to_str() {
            field = field.with_namespace(ns_str);
        }
    }
    let b = &mut *builder;
    b.0 = std::mem::replace(&mut b.0, ModelSchema::builder("")).field(field);
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_builder_add_list_scalar_field(
    builder: *mut PolyXmlSchemaBuilder,
    name: *const c_char,
    xml_name: *const c_char,
    scalar_type: PolyXmlScalarType,
    namespace_uri: *const c_char,
) {
    if builder.is_null() || name.is_null() || xml_name.is_null() {
        return;
    }
    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    let xml_bytes = CStr::from_ptr(xml_name).to_bytes();
    let sc_type = match scalar_type {
        PolyXmlScalarType::String => ScalarType::String,
        PolyXmlScalarType::Int => ScalarType::Int,
        PolyXmlScalarType::Float => ScalarType::Float,
        PolyXmlScalarType::Bool => ScalarType::Bool,
        PolyXmlScalarType::Decimal => ScalarType::Decimal,
        PolyXmlScalarType::XmlDate => ScalarType::XmlDate,
        PolyXmlScalarType::XmlDateTime => ScalarType::XmlDateTime,
        PolyXmlScalarType::Any => ScalarType::Any,
    };

    let mut field = FieldSchema::new(
        name_str,
        xml_bytes,
        FieldKind::Element,
        ValueType::List(Box::new(ValueType::Scalar(sc_type))),
    );
    if !namespace_uri.is_null() {
        if let Ok(ns_str) = CStr::from_ptr(namespace_uri).to_str() {
            field = field.with_namespace(ns_str);
        }
    }
    let b = &mut *builder;
    b.0 = std::mem::replace(&mut b.0, ModelSchema::builder("")).field(field);
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_builder_build(
    builder: *mut PolyXmlSchemaBuilder,
) -> *mut PolyXmlSchema {
    if builder.is_null() {
        return std::ptr::null_mut();
    }
    let boxed = Box::from_raw(builder);
    let schema = boxed.0.build();
    Box::into_raw(Box::new(PolyXmlSchema(schema)))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_schema_free(schema: *mut PolyXmlSchema) {
    if !schema.is_null() {
        drop(Box::from_raw(schema));
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_deserialize(
    data: *const u8,
    len: usize,
    schema: *const PolyXmlSchema,
    out_value: *mut *mut PolyXmlValue,
) -> PolyXmlErrorCode {
    if data.is_null() || schema.is_null() || out_value.is_null() {
        return PolyXmlErrorCode::ErrNullPtr;
    }

    let slice = std::slice::from_raw_parts(data, len);
    let s = &(*schema).0;

    match polyxml::deserialize(slice, Arc::clone(s)) {
        Ok(val) => {
            *out_value = Box::into_raw(Box::new(PolyXmlValue(val)));
            PolyXmlErrorCode::Ok
        }
        Err(polyxml::error::PolyXmlError::XmlSyntaxError { .. }) => PolyXmlErrorCode::ErrSyntax,
        Err(polyxml::error::PolyXmlError::ScalarParseError { .. }) => PolyXmlErrorCode::ErrScalar,
        Err(_) => PolyXmlErrorCode::ErrSchema,
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_serialize(
    root_name: *const c_char,
    value: *const PolyXmlValue,
    schema: *const PolyXmlSchema,
    indent: i32,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
) -> PolyXmlErrorCode {
    polyxml_serialize_with_options(
        root_name,
        value,
        schema,
        indent,
        -1,
        std::ptr::null(),
        std::ptr::null(),
        0,
        out_bytes,
        out_len,
    )
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_serialize_with_options(
    root_name: *const c_char,
    value: *const PolyXmlValue,
    schema: *const PolyXmlSchema,
    indent: i32,
    enable_namespaces: i32,
    ns_prefixes: *const *const c_char,
    ns_uris: *const *const c_char,
    ns_count: usize,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
) -> PolyXmlErrorCode {
    if root_name.is_null()
        || value.is_null()
        || schema.is_null()
        || out_bytes.is_null()
        || out_len.is_null()
    {
        return PolyXmlErrorCode::ErrNullPtr;
    }

    let root_str = match CStr::from_ptr(root_name).to_str() {
        Ok(s) => s,
        Err(_) => return PolyXmlErrorCode::ErrUtf8,
    };

    let indent_opt = if indent > 0 {
        Some(indent as usize)
    } else {
        None
    };

    let enable_ns_opt = match enable_namespaces {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };

    let ns_map = if ns_count > 0 && !ns_prefixes.is_null() && !ns_uris.is_null() {
        let mut map = std::collections::HashMap::new();
        let prefix_slice = std::slice::from_raw_parts(ns_prefixes, ns_count);
        let uri_slice = std::slice::from_raw_parts(ns_uris, ns_count);
        for i in 0..ns_count {
            let prefix_ptr = prefix_slice[i];
            let uri_ptr = uri_slice[i];
            if !uri_ptr.is_null() {
                if let Ok(uri) = CStr::from_ptr(uri_ptr).to_str() {
                    let prefix = if prefix_ptr.is_null() {
                        String::new()
                    } else if let Ok(p) = CStr::from_ptr(prefix_ptr).to_str() {
                        p.to_string()
                    } else {
                        String::new()
                    };
                    map.insert(prefix, uri.to_string());
                }
            }
        }
        Some(map)
    } else {
        None
    };

    let val = &(*value).0;
    let s = &(*schema).0;

    match polyxml::serialize_with_options(
        root_str,
        val,
        s,
        indent_opt,
        enable_ns_opt,
        ns_map.as_ref(),
    ) {
        Ok(mut bytes) => {
            bytes.shrink_to_fit();
            *out_len = bytes.len();
            *out_bytes = bytes.as_mut_ptr();
            std::mem::forget(bytes);
            PolyXmlErrorCode::Ok
        }
        Err(_) => PolyXmlErrorCode::ErrSchema,
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_bytes_free(bytes: *mut u8, len: usize) {
    if !bytes.is_null() {
        drop(Vec::from_raw_parts(bytes, len, len));
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_create_record(
    schema: *const PolyXmlSchema,
) -> *mut PolyXmlValue {
    if schema.is_null() {
        return std::ptr::null_mut();
    }
    let s = Arc::clone(&(*schema).0);
    let count = s.fields.len();
    let val = PolyValue::Record {
        schema: s,
        values: vec![None; count].into_boxed_slice(),
    };
    Box::into_raw(Box::new(PolyXmlValue(val)))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_create_list() -> *mut PolyXmlValue {
    Box::into_raw(Box::new(PolyXmlValue(PolyValue::List(Vec::new()))))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_create_list_with_capacity(
    capacity: usize,
) -> *mut PolyXmlValue {
    Box::into_raw(Box::new(PolyXmlValue(PolyValue::List(Vec::with_capacity(
        capacity,
    )))))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_create_string(
    str_ptr: *const c_char,
    len: usize,
) -> *mut PolyXmlValue {
    if str_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let slice = std::slice::from_raw_parts(str_ptr as *const u8, len);
    let s = match std::str::from_utf8(slice) {
        Ok(valid) => valid.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };
    Box::into_raw(Box::new(PolyXmlValue(PolyValue::String(s))))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_create_int(val: i64) -> *mut PolyXmlValue {
    Box::into_raw(Box::new(PolyXmlValue(PolyValue::Int(val))))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_create_float(val: f64) -> *mut PolyXmlValue {
    Box::into_raw(Box::new(PolyXmlValue(PolyValue::Float(val))))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_create_bool(val: bool) -> *mut PolyXmlValue {
    Box::into_raw(Box::new(PolyXmlValue(PolyValue::Bool(val))))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_create_null() -> *mut PolyXmlValue {
    Box::into_raw(Box::new(PolyXmlValue(PolyValue::Null)))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_clone(val: *const PolyXmlValue) -> *mut PolyXmlValue {
    if val.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(PolyXmlValue((*val).0.clone())))
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_set_field(
    record: *mut PolyXmlValue,
    key: *const c_char,
    child_val: *mut PolyXmlValue,
) -> PolyXmlErrorCode {
    if record.is_null() || key.is_null() || child_val.is_null() {
        return PolyXmlErrorCode::ErrNullPtr;
    }
    let key_str = match CStr::from_ptr(key).to_str() {
        Ok(s) => s,
        Err(_) => return PolyXmlErrorCode::ErrUtf8,
    };
    let child_boxed = Box::from_raw(child_val);
    let child_inner = child_boxed.0;

    match &mut (*record).0 {
        PolyValue::Record { schema, values } => {
            if let Some(idx) = schema.fields.iter().position(|f| f.name == key_str) {
                values[idx] = Some(child_inner);
                PolyXmlErrorCode::Ok
            } else {
                PolyXmlErrorCode::ErrSchema
            }
        }
        PolyValue::Object(map) => {
            map.insert(key_str.to_string(), child_inner);
            PolyXmlErrorCode::Ok
        }
        _ => PolyXmlErrorCode::ErrSchema,
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_list_append(
    list: *mut PolyXmlValue,
    item: *mut PolyXmlValue,
) -> PolyXmlErrorCode {
    if list.is_null() || item.is_null() {
        return PolyXmlErrorCode::ErrNullPtr;
    }
    let item_boxed = Box::from_raw(item);
    let item_inner = item_boxed.0;

    match &mut (*list).0 {
        PolyValue::List(vec) => {
            vec.push(item_inner);
            PolyXmlErrorCode::Ok
        }
        _ => PolyXmlErrorCode::ErrSchema,
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_get_field(
    val: *const PolyXmlValue,
    key: *const c_char,
) -> *const PolyXmlValue {
    if val.is_null() || key.is_null() {
        return std::ptr::null();
    }
    let key_str = match CStr::from_ptr(key).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null(),
    };
    if let Some(sub_val) = (*val).0.get(key_str) {
        sub_val as *const PolyValue as *const PolyXmlValue
    } else {
        std::ptr::null()
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_get_int(
    val: *const PolyXmlValue,
    out_int: *mut i64,
) -> PolyXmlErrorCode {
    if val.is_null() || out_int.is_null() {
        return PolyXmlErrorCode::ErrNullPtr;
    }
    if let Some(i) = (*val).0.as_i64() {
        *out_int = i;
        PolyXmlErrorCode::Ok
    } else {
        PolyXmlErrorCode::ErrScalar
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_get_float(
    val: *const PolyXmlValue,
    out_float: *mut f64,
) -> PolyXmlErrorCode {
    if val.is_null() || out_float.is_null() {
        return PolyXmlErrorCode::ErrNullPtr;
    }
    if let Some(f) = (*val).0.as_f64() {
        *out_float = f;
        PolyXmlErrorCode::Ok
    } else {
        PolyXmlErrorCode::ErrScalar
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_get_bool(
    val: *const PolyXmlValue,
    out_bool: *mut bool,
) -> PolyXmlErrorCode {
    if val.is_null() || out_bool.is_null() {
        return PolyXmlErrorCode::ErrNullPtr;
    }
    if let Some(b) = (*val).0.as_bool() {
        *out_bool = b;
        PolyXmlErrorCode::Ok
    } else {
        PolyXmlErrorCode::ErrScalar
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_get_string(
    val: *const PolyXmlValue,
    out_str: *mut *const c_char,
    out_len: *mut usize,
) -> PolyXmlErrorCode {
    if val.is_null() || out_str.is_null() || out_len.is_null() {
        return PolyXmlErrorCode::ErrNullPtr;
    }
    if let Some(s) = (*val).0.as_str() {
        *out_str = s.as_ptr() as *const c_char;
        *out_len = s.len();
        PolyXmlErrorCode::Ok
    } else {
        PolyXmlErrorCode::ErrScalar
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_get_list_len(
    val: *const PolyXmlValue,
    out_len: *mut usize,
) -> PolyXmlErrorCode {
    if val.is_null() || out_len.is_null() {
        return PolyXmlErrorCode::ErrNullPtr;
    }
    if let Some(l) = (*val).0.as_list() {
        *out_len = l.len();
        PolyXmlErrorCode::Ok
    } else {
        PolyXmlErrorCode::ErrScalar
    }
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_get_list_item(
    val: *const PolyXmlValue,
    idx: usize,
) -> *const PolyXmlValue {
    if val.is_null() {
        return std::ptr::null();
    }
    if let Some(l) = (*val).0.as_list() {
        if let Some(item) = l.get(idx) {
            return item as *const PolyValue as *const PolyXmlValue;
        }
    }
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_is_null(val: *const PolyXmlValue) -> bool {
    if val.is_null() {
        return true;
    }
    (*val).0.is_null()
}

#[no_mangle]
pub unsafe extern "C" fn polyxml_value_free(val: *mut PolyXmlValue) {
    if !val.is_null() {
        drop(Box::from_raw(val));
    }
}

#[no_mangle]
pub extern "C" fn polyxml_version() -> *const c_char {
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_c_abi_namespaced_roundtrip() {
        unsafe {
            let name = CString::new("Order").unwrap();
            let ns_order = CString::new("https://example.com/orders").unwrap();
            let builder = polyxml_schema_builder_create(name.as_ptr());
            polyxml_schema_builder_set_namespace(builder, ns_order.as_ptr());

            let id_name = CString::new("id").unwrap();
            let id_xml = CString::new("id").unwrap();
            polyxml_schema_builder_add_field(
                builder,
                id_name.as_ptr(),
                id_xml.as_ptr(),
                PolyXmlFieldKind::Attribute,
                PolyXmlScalarType::Int,
            );

            let item_name = CString::new("item").unwrap();
            let item_xml = CString::new("item").unwrap();
            let ns_item = CString::new("https://example.com/items").unwrap();
            polyxml_schema_builder_add_field_with_namespace(
                builder,
                item_name.as_ptr(),
                item_xml.as_ptr(),
                PolyXmlFieldKind::Element,
                PolyXmlScalarType::String,
                ns_item.as_ptr(),
            );

            let schema = polyxml_schema_builder_build(builder);
            assert!(!schema.is_null());

            let xml = br#"<ns0:Order xmlns:ns0="https://example.com/orders" xmlns:ns1="https://example.com/items" id="101"><ns1:item>Gadget</ns1:item></ns0:Order>"#;
            let mut val_ptr: *mut PolyXmlValue = std::ptr::null_mut();
            let err = polyxml_deserialize(xml.as_ptr(), xml.len(), schema, &mut val_ptr);
            assert_eq!(err as i32, PolyXmlErrorCode::Ok as i32);
            assert!(!val_ptr.is_null());

            let mut int_val = 0;
            let id_field = polyxml_value_get_field(val_ptr, id_name.as_ptr());
            assert!(!id_field.is_null());
            polyxml_value_get_int(id_field, &mut int_val);
            assert_eq!(int_val, 101);

            let mut out_bytes: *mut u8 = std::ptr::null_mut();
            let mut out_len = 0;
            let p0 = CString::new("ord").unwrap();
            let u0 = CString::new("https://example.com/orders").unwrap();
            let prefixes = [p0.as_ptr()];
            let uris = [u0.as_ptr()];

            let serr = polyxml_serialize_with_options(
                name.as_ptr(),
                val_ptr,
                schema,
                0,
                1,
                prefixes.as_ptr(),
                uris.as_ptr(),
                1,
                &mut out_bytes,
                &mut out_len,
            );
            assert_eq!(serr as i32, PolyXmlErrorCode::Ok as i32);
            assert!(!out_bytes.is_null());
            let s = std::str::from_utf8(std::slice::from_raw_parts(out_bytes, out_len)).unwrap();
            assert!(s.contains("xmlns:ord=\"https://example.com/orders\""));
            assert!(s.contains("<ord:Order"));
            assert!(s.contains("id=\"101\""));

            polyxml_bytes_free(out_bytes, out_len);
            polyxml_value_free(val_ptr);
            polyxml_schema_free(schema);
        }
    }

    #[test]
    fn test_c_abi_nested_list_batch_roundtrip() {
        unsafe {
            // Build Sensor schema
            let sensor_name = CString::new("Sensor").unwrap();
            let s_builder = polyxml_schema_builder_create(sensor_name.as_ptr());
            let id_str = CString::new("Id").unwrap();
            let val_str = CString::new("Value").unwrap();
            polyxml_schema_builder_add_field(
                s_builder,
                id_str.as_ptr(),
                id_str.as_ptr(),
                PolyXmlFieldKind::Element,
                PolyXmlScalarType::String,
            );
            polyxml_schema_builder_add_field(
                s_builder,
                val_str.as_ptr(),
                val_str.as_ptr(),
                PolyXmlFieldKind::Element,
                PolyXmlScalarType::Int,
            );
            let sensor_schema = polyxml_schema_builder_build(s_builder);
            assert!(!sensor_schema.is_null());

            // Build Batch schema with list of Sensor
            let batch_name = CString::new("Batch").unwrap();
            let b_builder = polyxml_schema_builder_create(batch_name.as_ptr());
            let sensor_field_name = CString::new("Sensor").unwrap();
            let sensor_xml_tag = CString::new("Sensor").unwrap();
            polyxml_schema_builder_add_list_nested_field(
                b_builder,
                sensor_field_name.as_ptr(),
                sensor_xml_tag.as_ptr(),
                sensor_schema,
                std::ptr::null(),
            );
            let batch_schema = polyxml_schema_builder_build(b_builder);
            assert!(!batch_schema.is_null());

            // Deserialize XML
            let xml = br#"<Batch><Sensor><Id>sensor-0</Id><Value>10</Value></Sensor><Sensor><Id>sensor-1</Id><Value>20</Value></Sensor></Batch>"#;
            let mut val_ptr: *mut PolyXmlValue = std::ptr::null_mut();
            let err = polyxml_deserialize(xml.as_ptr(), xml.len(), batch_schema, &mut val_ptr);
            assert_eq!(err as i32, PolyXmlErrorCode::Ok as i32);
            assert!(!val_ptr.is_null());

            // Inspect list length
            let list_val = polyxml_value_get_field(val_ptr, sensor_field_name.as_ptr());
            assert!(!list_val.is_null());
            let mut len = 0;
            let len_err = polyxml_value_get_list_len(list_val, &mut len);
            assert_eq!(len_err as i32, PolyXmlErrorCode::Ok as i32);
            assert_eq!(len, 2);

            // Inspect item 0
            let item0 = polyxml_value_get_list_item(list_val, 0);
            assert!(!item0.is_null());
            let id0 = polyxml_value_get_field(item0, id_str.as_ptr());
            let mut id0_ptr: *const c_char = std::ptr::null();
            let mut id0_len = 0;
            polyxml_value_get_string(id0, &mut id0_ptr, &mut id0_len);
            let id0_str =
                std::str::from_utf8(std::slice::from_raw_parts(id0_ptr as *const u8, id0_len))
                    .unwrap();
            assert_eq!(id0_str, "sensor-0");
            let v0 = polyxml_value_get_field(item0, val_str.as_ptr());
            let mut int0 = 0;
            polyxml_value_get_int(v0, &mut int0);
            assert_eq!(int0, 10);

            // Serialize back
            let mut out_bytes: *mut u8 = std::ptr::null_mut();
            let mut out_len = 0;
            let serr = polyxml_serialize(
                batch_name.as_ptr(),
                val_ptr,
                batch_schema,
                0,
                &mut out_bytes,
                &mut out_len,
            );
            assert_eq!(serr as i32, PolyXmlErrorCode::Ok as i32);
            let out_str =
                std::str::from_utf8(std::slice::from_raw_parts(out_bytes, out_len)).unwrap();
            assert_eq!(out_str, std::str::from_utf8(xml).unwrap());

            polyxml_bytes_free(out_bytes, out_len);
            polyxml_value_free(val_ptr);

            // Construct Batch manually using C-ABI value constructors
            let manual_batch = polyxml_value_create_record(batch_schema);
            let manual_list = polyxml_value_create_list_with_capacity(2);
            for i in 0..2 {
                let s = polyxml_value_create_record(sensor_schema);
                let id_val_str = format!("sensor-{}", i);
                let c_id = polyxml_value_create_string(
                    id_val_str.as_ptr() as *const c_char,
                    id_val_str.len(),
                );
                polyxml_value_set_field(s, id_str.as_ptr(), c_id);
                let c_val = polyxml_value_create_int((i + 1) * 10);
                polyxml_value_set_field(s, val_str.as_ptr(), c_val);
                polyxml_value_list_append(manual_list, s);
            }
            polyxml_value_set_field(manual_batch, sensor_field_name.as_ptr(), manual_list);

            let mut manual_bytes: *mut u8 = std::ptr::null_mut();
            let mut manual_len = 0;
            let m_serr = polyxml_serialize(
                batch_name.as_ptr(),
                manual_batch,
                batch_schema,
                0,
                &mut manual_bytes,
                &mut manual_len,
            );
            assert_eq!(m_serr as i32, PolyXmlErrorCode::Ok as i32);
            let manual_str =
                std::str::from_utf8(std::slice::from_raw_parts(manual_bytes, manual_len)).unwrap();
            assert_eq!(manual_str, std::str::from_utf8(xml).unwrap());

            polyxml_bytes_free(manual_bytes, manual_len);
            polyxml_value_free(manual_batch);
            polyxml_schema_free(sensor_schema);
            polyxml_schema_free(batch_schema);
        }
    }
}
