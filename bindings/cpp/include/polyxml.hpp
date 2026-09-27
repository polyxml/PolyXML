#pragma once

#include <cstdint>
#include <map>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

#include "polyxml.h"

namespace polyxml {

class Exception : public std::runtime_error {
public:
    explicit Exception(const std::string& msg) : std::runtime_error(msg) {}
};

class Schema;

class Value {
public:
    Value(polyxml_value_t* raw, bool owns) : raw_(raw), owns_(owns) {}

    ~Value() {
        if (owns_ && raw_) {
            polyxml_value_free(raw_);
        }
    }

    Value(const Value&) = delete;
    Value& operator=(const Value&) = delete;

    Value(Value&& other) noexcept : raw_(other.raw_), owns_(other.owns_) {
        other.raw_ = nullptr;
        other.owns_ = false;
    }

    Value& operator=(Value&& other) noexcept {
        if (this != &other) {
            if (owns_ && raw_) {
                polyxml_value_free(raw_);
            }
            raw_ = other.raw_;
            owns_ = other.owns_;
            other.raw_ = nullptr;
            other.owns_ = false;
        }
        return *this;
    }

    [[nodiscard]] bool is_null() const {
        return polyxml_value_is_null(raw_);
    }

    [[nodiscard]] std::optional<int64_t> as_int() const {
        int64_t v = 0;
        if (polyxml_value_get_int(raw_, &v) == POLYXML_OK) {
            return v;
        }
        return std::nullopt;
    }

    [[nodiscard]] std::optional<double> as_float() const {
        double v = 0.0;
        if (polyxml_value_get_float(raw_, &v) == POLYXML_OK) {
            return v;
        }
        return std::nullopt;
    }

    [[nodiscard]] std::optional<bool> as_bool() const {
        bool v = false;
        if (polyxml_value_get_bool(raw_, &v) == POLYXML_OK) {
            return v;
        }
        return std::nullopt;
    }

    [[nodiscard]] std::optional<std::string_view> as_string() const {
        const char* str = nullptr;
        size_t len = 0;
        if (polyxml_value_get_string(raw_, &str, &len) == POLYXML_OK && str != nullptr) {
            return std::string_view(str, len);
        }
        return std::nullopt;
    }

    [[nodiscard]] std::size_t size() const {
        size_t len = 0;
        if (polyxml_value_get_list_len(raw_, &len) == POLYXML_OK) {
            return len;
        }
        return 0;
    }

    [[nodiscard]] std::optional<Value> get_item(std::size_t idx) const {
        const polyxml_value_t* item = polyxml_value_get_list_item(raw_, idx);
        if (item) {
            return Value(const_cast<polyxml_value_t*>(item), false);
        }
        return std::nullopt;
    }

    [[nodiscard]] std::optional<Value> operator[](std::size_t idx) const {
        return get_item(idx);
    }

    [[nodiscard]] std::optional<Value> get(const std::string& key) const {
        const polyxml_value_t* field = polyxml_value_get_field(raw_, key.c_str());
        if (field) {
            return Value(const_cast<polyxml_value_t*>(field), false);
        }
        return std::nullopt;
    }

    [[nodiscard]] std::optional<Value> operator[](const std::string& key) const {
        return get(key);
    }

    static Value create_record(const Schema& schema);

    static Value create_list(std::size_t capacity = 0) {
        polyxml_value_t* raw = capacity > 0
            ? polyxml_value_create_list_with_capacity(capacity)
            : polyxml_value_create_list();
        if (!raw) throw Exception("Failed to create list value");
        return Value(raw, true);
    }

    static Value create_string(std::string_view s) {
        polyxml_value_t* raw = polyxml_value_create_string(s.data(), s.size());
        if (!raw) throw Exception("Failed to create string value");
        return Value(raw, true);
    }

    static Value create_int(int64_t val) {
        polyxml_value_t* raw = polyxml_value_create_int(val);
        if (!raw) throw Exception("Failed to create int value");
        return Value(raw, true);
    }

    static Value create_float(double val) {
        polyxml_value_t* raw = polyxml_value_create_float(val);
        if (!raw) throw Exception("Failed to create float value");
        return Value(raw, true);
    }

    static Value create_bool(bool val) {
        polyxml_value_t* raw = polyxml_value_create_bool(val);
        if (!raw) throw Exception("Failed to create bool value");
        return Value(raw, true);
    }

    static Value create_null() {
        polyxml_value_t* raw = polyxml_value_create_null();
        if (!raw) throw Exception("Failed to create null value");
        return Value(raw, true);
    }

    [[nodiscard]] Value clone() const {
        polyxml_value_t* raw = polyxml_value_clone(raw_);
        if (!raw) throw Exception("Failed to clone value");
        return Value(raw, true);
    }

    void set(const std::string& key, Value&& child) {
        if (!child.raw_ || !child.owns_) {
            throw Exception("Child value must be owned");
        }
        polyxml_value_t* child_raw = child.raw_;
        child.raw_ = nullptr;
        child.owns_ = false;
        auto code = polyxml_value_set_field(raw_, key.c_str(), child_raw);
        if (code != POLYXML_OK) {
            polyxml_value_free(child_raw);
            throw Exception("Failed to set field '" + key + "' (code: " + std::to_string(code) + ")");
        }
    }

    void append(Value&& item) {
        if (!item.raw_ || !item.owns_) {
            throw Exception("Item value must be owned");
        }
        polyxml_value_t* item_raw = item.raw_;
        item.raw_ = nullptr;
        item.owns_ = false;
        auto code = polyxml_value_list_append(raw_, item_raw);
        if (code != POLYXML_OK) {
            polyxml_value_free(item_raw);
            throw Exception("Failed to append item to list (code: " + std::to_string(code) + ")");
        }
    }

    [[nodiscard]] const polyxml_value_t* raw() const { return raw_; }

private:
    polyxml_value_t* raw_ = nullptr;
    bool owns_ = false;
};

class Schema {
public:
    explicit Schema(polyxml_schema_t* raw) : raw_(raw, polyxml_schema_free) {}

    [[nodiscard]] const polyxml_schema_t* raw() const { return raw_.get(); }

private:
    std::shared_ptr<polyxml_schema_t> raw_;
};

inline Value Value::create_record(const Schema& schema) {
    polyxml_value_t* raw = polyxml_value_create_record(schema.raw());
    if (!raw) throw Exception("Failed to create record value");
    return Value(raw, true);
}

class SchemaBuilder {
public:
    explicit SchemaBuilder(const std::string& name)
        : raw_(polyxml_schema_builder_create(name.c_str())) {
        if (!raw_) {
            throw Exception("Failed to create schema builder");
        }
    }

    SchemaBuilder& set_namespace(const std::string& namespace_uri) {
        polyxml_schema_builder_set_namespace(raw_, namespace_uri.c_str());
        return *this;
    }

    SchemaBuilder& add_attribute(const std::string& name, const std::string& xml_name, polyxml_scalar_type_t scalar_type, const std::string& namespace_uri = "") {
        const char* ns = namespace_uri.empty() ? nullptr : namespace_uri.c_str();
        polyxml_schema_builder_add_field_with_namespace(raw_, name.c_str(), xml_name.c_str(), POLYXML_FIELD_ATTRIBUTE, scalar_type, ns);
        return *this;
    }

    SchemaBuilder& add_element(const std::string& name, const std::string& xml_name, polyxml_scalar_type_t scalar_type, const std::string& namespace_uri = "") {
        const char* ns = namespace_uri.empty() ? nullptr : namespace_uri.c_str();
        polyxml_schema_builder_add_field_with_namespace(raw_, name.c_str(), xml_name.c_str(), POLYXML_FIELD_ELEMENT, scalar_type, ns);
        return *this;
    }

    SchemaBuilder& add_nested(const std::string& name, const std::string& xml_name, const Schema& nested_schema, const std::string& namespace_uri = "") {
        const char* ns = namespace_uri.empty() ? nullptr : namespace_uri.c_str();
        polyxml_schema_builder_add_nested_field(raw_, name.c_str(), xml_name.c_str(), nested_schema.raw(), ns);
        return *this;
    }

    SchemaBuilder& add_list_nested(const std::string& name, const std::string& xml_name, const Schema& item_schema, const std::string& namespace_uri = "") {
        const char* ns = namespace_uri.empty() ? nullptr : namespace_uri.c_str();
        polyxml_schema_builder_add_list_nested_field(raw_, name.c_str(), xml_name.c_str(), item_schema.raw(), ns);
        return *this;
    }

    SchemaBuilder& add_list_scalar(const std::string& name, const std::string& xml_name, polyxml_scalar_type_t scalar_type, const std::string& namespace_uri = "") {
        const char* ns = namespace_uri.empty() ? nullptr : namespace_uri.c_str();
        polyxml_schema_builder_add_list_scalar_field(raw_, name.c_str(), xml_name.c_str(), scalar_type, ns);
        return *this;
    }

    Schema build() {
        polyxml_schema_t* schema = polyxml_schema_builder_build(raw_);
        raw_ = nullptr;
        if (!schema) {
            throw Exception("Failed to build schema");
        }
        return Schema(schema);
    }

private:
    polyxml_schema_builder_t* raw_ = nullptr;
};

inline Value deserialize(std::string_view xml, const Schema& schema) {
    polyxml_value_t* out_val = nullptr;
    auto code = polyxml_deserialize(
        reinterpret_cast<const uint8_t*>(xml.data()),
        xml.size(),
        schema.raw(),
        &out_val
    );

    if (code != POLYXML_OK || !out_val) {
        throw Exception("Deserialization error (code: " + std::to_string(code) + ")");
    }
    return Value(out_val, true);
}

inline std::string serialize_with_options(
    std::string_view root_name,
    const Value& val,
    const Schema& schema,
    int indent = 0,
    std::optional<bool> enable_namespaces = std::nullopt,
    const std::map<std::string, std::string>& ns_map = {}
) {
    uint8_t* out_bytes = nullptr;
    size_t out_len = 0;
    std::string root_str(root_name);

    int enable_ns_int = -1;
    if (enable_namespaces.has_value()) {
        enable_ns_int = enable_namespaces.value() ? 1 : 0;
    }

    std::vector<const char*> prefixes;
    std::vector<const char*> uris;
    prefixes.reserve(ns_map.size());
    uris.reserve(ns_map.size());

    for (const auto& [prefix, uri] : ns_map) {
        prefixes.push_back(prefix.c_str());
        uris.push_back(uri.c_str());
    }

    auto code = polyxml_serialize_with_options(
        root_str.c_str(),
        val.raw(),
        schema.raw(),
        indent,
        enable_ns_int,
        prefixes.empty() ? nullptr : prefixes.data(),
        uris.empty() ? nullptr : uris.data(),
        ns_map.size(),
        &out_bytes,
        &out_len
    );

    if (code != POLYXML_OK || !out_bytes) {
        throw Exception("Serialization error (code: " + std::to_string(code) + ")");
    }

    std::string result(reinterpret_cast<char*>(out_bytes), out_len);
    polyxml_bytes_free(out_bytes, out_len);
    return result;
}

inline std::string serialize(std::string_view root_name, const Value& val, const Schema& schema, int indent = 0) {
    return serialize_with_options(root_name, val, schema, indent);
}

} // namespace polyxml
