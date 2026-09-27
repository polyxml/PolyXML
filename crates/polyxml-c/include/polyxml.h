#ifndef POLYXML_H
#define POLYXML_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handles */
typedef struct polyxml_schema_builder polyxml_schema_builder_t;
typedef struct polyxml_schema polyxml_schema_t;
typedef struct polyxml_value polyxml_value_t;

/* Field kinds */
typedef enum polyxml_field_kind {
    POLYXML_FIELD_ATTRIBUTE = 0,
    POLYXML_FIELD_ELEMENT   = 1,
    POLYXML_FIELD_TEXT      = 2
} polyxml_field_kind_t;

/* Scalar types */
typedef enum polyxml_scalar_type {
    POLYXML_SCALAR_STRING       = 0,
    POLYXML_SCALAR_INT          = 1,
    POLYXML_SCALAR_FLOAT        = 2,
    POLYXML_SCALAR_BOOL         = 3,
    POLYXML_SCALAR_DECIMAL      = 4,
    POLYXML_SCALAR_XML_DATE     = 5,
    POLYXML_SCALAR_XML_DATETIME = 6,
    POLYXML_SCALAR_ANY          = 7
} polyxml_scalar_type_t;

/* Error codes */
typedef enum polyxml_error_code {
    POLYXML_OK           = 0,
    POLYXML_ERR_SYNTAX   = 1,
    POLYXML_ERR_SCALAR   = 2,
    POLYXML_ERR_SCHEMA   = 3,
    POLYXML_ERR_NULL_PTR = 4,
    POLYXML_ERR_UTF8     = 5
} polyxml_error_code_t;

/* Schema Builder API */
polyxml_schema_builder_t* polyxml_schema_builder_create(const char* name);
void polyxml_schema_builder_set_namespace(
    polyxml_schema_builder_t* builder,
    const char* namespace_uri
);
void polyxml_schema_builder_add_field(
    polyxml_schema_builder_t* builder,
    const char* name,
    const char* xml_name,
    polyxml_field_kind_t kind,
    polyxml_scalar_type_t scalar_type
);
void polyxml_schema_builder_add_field_with_namespace(
    polyxml_schema_builder_t* builder,
    const char* name,
    const char* xml_name,
    polyxml_field_kind_t kind,
    polyxml_scalar_type_t scalar_type,
    const char* namespace_uri
);
void polyxml_schema_builder_add_nested_field(
    polyxml_schema_builder_t* builder,
    const char* name,
    const char* xml_name,
    const polyxml_schema_t* nested_schema,
    const char* namespace_uri
);
void polyxml_schema_builder_add_list_nested_field(
    polyxml_schema_builder_t* builder,
    const char* name,
    const char* xml_name,
    const polyxml_schema_t* item_schema,
    const char* namespace_uri
);
void polyxml_schema_builder_add_list_scalar_field(
    polyxml_schema_builder_t* builder,
    const char* name,
    const char* xml_name,
    polyxml_scalar_type_t scalar_type,
    const char* namespace_uri
);
polyxml_schema_t* polyxml_schema_builder_build(polyxml_schema_builder_t* builder);
void polyxml_schema_free(polyxml_schema_t* schema);

/* Deserialization & Serialization */
polyxml_error_code_t polyxml_deserialize(
    const uint8_t* data,
    size_t len,
    const polyxml_schema_t* schema,
    polyxml_value_t** out_value
);

polyxml_error_code_t polyxml_serialize(
    const char* root_name,
    const polyxml_value_t* value,
    const polyxml_schema_t* schema,
    int indent,
    uint8_t** out_bytes,
    size_t* out_len
);

polyxml_error_code_t polyxml_serialize_with_options(
    const char* root_name,
    const polyxml_value_t* value,
    const polyxml_schema_t* schema,
    int indent,
    int enable_namespaces,
    const char* const* ns_prefixes,
    const char* const* ns_uris,
    size_t ns_count,
    uint8_t** out_bytes,
    size_t* out_len
);

void polyxml_bytes_free(uint8_t* bytes, size_t len);

/* Value Construction API */
polyxml_value_t* polyxml_value_create_record(const polyxml_schema_t* schema);
polyxml_value_t* polyxml_value_create_list(void);
polyxml_value_t* polyxml_value_create_list_with_capacity(size_t capacity);
polyxml_value_t* polyxml_value_create_string(const char* str, size_t len);
polyxml_value_t* polyxml_value_create_int(int64_t val);
polyxml_value_t* polyxml_value_create_float(double val);
polyxml_value_t* polyxml_value_create_bool(bool val);
polyxml_value_t* polyxml_value_create_null(void);
polyxml_value_t* polyxml_value_clone(const polyxml_value_t* val);

/* Field & List Modification API (transfers ownership of child/item) */
polyxml_error_code_t polyxml_value_set_field(
    polyxml_value_t* record,
    const char* key,
    polyxml_value_t* child_val
);
polyxml_error_code_t polyxml_value_list_append(
    polyxml_value_t* list,
    polyxml_value_t* item
);

/* Value Inspection API */
const polyxml_value_t* polyxml_value_get_field(const polyxml_value_t* val, const char* key);
polyxml_error_code_t polyxml_value_get_int(const polyxml_value_t* val, int64_t* out_int);
polyxml_error_code_t polyxml_value_get_float(const polyxml_value_t* val, double* out_float);
polyxml_error_code_t polyxml_value_get_bool(const polyxml_value_t* val, bool* out_bool);
polyxml_error_code_t polyxml_value_get_string(const polyxml_value_t* val, const char** out_str, size_t* out_len);
polyxml_error_code_t polyxml_value_get_list_len(const polyxml_value_t* val, size_t* out_len);
const polyxml_value_t* polyxml_value_get_list_item(const polyxml_value_t* val, size_t idx);
bool polyxml_value_is_null(const polyxml_value_t* val);
void polyxml_value_free(polyxml_value_t* val);

/* Version info */
const char* polyxml_version(void);

#ifdef __cplusplus
}
#endif

#endif /* POLYXML_H */
