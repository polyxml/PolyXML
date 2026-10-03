---
title: C++
description: Header-only, zero-overhead C++20 bindings for PolyXML with RAII, move semantics, and zero memory leaks.
---

# C++

PolyXML delivers high-throughput native XML processing for modern C++20 applications. The C++ bindings are header-only (`bindings/cpp/include/polyxml.hpp`), wrapping the pure Rust engine (`polyxml-c`) with idiomatic C++20 types, `std::optional`, `std::string_view`, RAII memory management, and move semantics.

---

## 📦 Build System Integration

### Option A: CMake `FetchContent`

```cmake
include(FetchContent)
FetchContent_Declare(
    polyxml
    GIT_REPOSITORY https://github.com/polyxml/PolyXML.git
    GIT_TAG        main
)
FetchContent_MakeAvailable(polyxml)

target_link_libraries(my_app PRIVATE polyxml_cpp)
```

### Option B: CMake `add_subdirectory`

```cmake
add_subdirectory(path/to/PolyXML/bindings/cpp)
target_link_libraries(my_app PRIVATE polyxml_cpp)
```

### Option C: Conan & vcpkg

- **Conan**: Add `polyxml/0.23.2` to your `conanfile.txt`.
- **vcpkg**: Run `vcpkg install polyxml`.

---

## 1. Schema Construction & Basic Deserialization

Define an XML schema using the fluent `SchemaBuilder` API and parse XML into a type-safe `polyxml::Value`:

```cpp
#include "polyxml.hpp"
#include <iostream>

int main() {
    // 1. Build the schema
    auto schema = polyxml::SchemaBuilder("Telemetry")
        .add_attribute("device_id", "id", POLYXML_SCALAR_INT)
        .add_element("altitude", "altitude", POLYXML_SCALAR_FLOAT)
        .add_element("armed", "armed", POLYXML_SCALAR_BOOL)
        .add_element("status", "status", POLYXML_SCALAR_STRING)
        .build();

    std::string_view xml = R"(
        <Telemetry id="1001">
            <altitude>24500.5</altitude>
            <armed>true</armed>
            <status>STABLE</status>
        </Telemetry>
    )";

    // 2. Deserialize XML string into polyxml::Value
    polyxml::Value val = polyxml::deserialize(xml, schema);

    // 3. Extract values using type-safe std::optional accessors
    int64_t id = val.get("device_id")->as_int().value_or(0);
    double alt = val.get("altitude")->as_float().value_or(0.0);
    bool armed = val.get("armed")->as_bool().value_or(false);
    std::string_view status = val.get("status")->as_string().value_or("UNKNOWN");

    std::cout << "Device " << id << " (" << status << "): " << alt << " ft, Armed: " << std::boolalpha << armed << "\n";

    return 0;
}
```

---

## 2. Formatting & Serialization

Convert native values back into formatted XML with customizable indentation:

```cpp
#include "polyxml.hpp"
#include <iostream>

int main() {
    auto schema = polyxml::SchemaBuilder("SystemConfig")
        .add_attribute("env", "env", POLYXML_SCALAR_STRING)
        .add_element("max_threads", "max_threads", POLYXML_SCALAR_INT)
        .add_element("debug", "debug", POLYXML_SCALAR_BOOL)
        .build();

    std::string xml = R"(<SystemConfig env="production"><max_threads>32</max_threads><debug>false</debug></SystemConfig>)";
    auto val = polyxml::deserialize(xml, schema);

    // Serialize with 2-space indentation
    std::string pretty_xml = polyxml::serialize("SystemConfig", val, schema, 2);
    std::cout << "Formatted XML:\n" << pretty_xml << "\n";

    // Compact serialization (indent = 0)
    std::string compact_xml = polyxml::serialize("SystemConfig", val, schema, 0);
    std::cout << "Compact:\n" << compact_xml << "\n";

    return 0;
}
```

---

## 3. RAII, Move Semantics & Exception Safety

`polyxml::Value` and `polyxml::Schema` are strict RAII objects:
- Destruction automatically releases underlying Rust memory (`polyxml_value_free`, `polyxml_schema_free`).
- Copy construction is explicitly disabled to prevent accidental double-free bugs.
- Move construction (`std::move`) transfers ownership with zero overhead.
- Parsing or serialization errors throw `polyxml::Exception`.

```cpp
#include "polyxml.hpp"
#include <iostream>
#include <vector>

void process_batch(const std::vector<std::string>& payloads) {
    auto schema = polyxml::SchemaBuilder("Sensor")
        .add_attribute("id", "id", POLYXML_SCALAR_INT)
        .add_element("val", "val", POLYXML_SCALAR_FLOAT)
        .build();

    for (const auto& payload : payloads) {
        try {
            // Deserializes and manages lifetime via RAII
            polyxml::Value record = polyxml::deserialize(payload, schema);
            
            // Move ownership into another container or function
            polyxml::Value moved_record = std::move(record);
            
            std::cout << "Processed sensor: " << moved_record.get("id")->as_int().value_or(-1) << "\n";
        } catch (const polyxml::Exception& ex) {
            std::cerr << "Malformed XML payload: " << ex.what() << "\n";
        }
    }
}
```

---

## 4. Struct Mapping Pattern

For production codebases, wrap `polyxml::Value` extraction into a typed C++ struct:

```cpp
#include "polyxml.hpp"
#include <iostream>
#include <string>

struct DroneTelemetry {
    int64_t drone_id;
    double battery_pct;
    double velocity_mps;
    std::string flight_mode;

    static DroneTelemetry from_xml(std::string_view xml, const polyxml::Schema& schema) {
        auto val = polyxml::deserialize(xml, schema);
        return DroneTelemetry{
            .drone_id = val.get("id")->as_int().value_or(0),
            .battery_pct = val.get("battery")->as_float().value_or(0.0),
            .velocity_mps = val.get("velocity")->as_float().value_or(0.0),
            .flight_mode = std::string(val.get("mode")->as_string().value_or("MANUAL")),
        };
    }
};

int main() {
    auto schema = polyxml::SchemaBuilder("Drone")
        .add_attribute("id", "id", POLYXML_SCALAR_INT)
        .add_element("battery", "battery", POLYXML_SCALAR_FLOAT)
        .add_element("velocity", "velocity", POLYXML_SCALAR_FLOAT)
        .add_element("mode", "mode", POLYXML_SCALAR_STRING)
        .build();

    std::string xml = R"(<Drone id="707"><battery>94.5</battery><velocity>18.2</velocity><mode>AUTONOMOUS</mode></Drone>)";
    
    DroneTelemetry telem = DroneTelemetry::from_xml(xml, schema);
    std::cout << "Drone " << telem.drone_id << " in " << telem.flight_mode 
              << " mode at " << telem.velocity_mps << " m/s (" << telem.battery_pct << "% battery)\n";

    return 0;
}
```

---

## 5. XML Namespaces & Prefix Mapping

PolyXML C++20 bindings support full W3C XML namespace declarations and custom prefix maps:

```cpp
#include "polyxml.hpp"
#include <iostream>
#include <map>

int main() {
    auto schema = polyxml::SchemaBuilder("Order")
        .set_namespace("https://example.com/orders")
        .add_attribute("id", "id", POLYXML_SCALAR_INT)
        .add_element("item", "item", POLYXML_SCALAR_STRING, "https://example.com/items")
        .build();

    std::string_view xml = R"(
        <ns0:Order xmlns:ns0="https://example.com/orders" xmlns:ns1="https://example.com/items" id="505">
            <ns1:item>Industrial Sensor</ns1:item>
        </ns0:Order>
    )";

    polyxml::Value val = polyxml::deserialize(xml, schema);
    std::cout << "Order ID: " << val.get("id")->as_int().value_or(0) << "\n";
    std::cout << "Item: " << val.get("item")->as_string().value_or("") << "\n";

    // Serialize with custom prefix mapping
    std::map<std::string, std::string> ns_map = {
        {"ord", "https://example.com/orders"},
        {"itm", "https://example.com/items"}
    };

    std::string serialized = polyxml::serialize_with_options("Order", val, schema, 2, true, ns_map);
    std::cout << "Namespaced XML:\n" << serialized << "\n";

    return 0;
}
```

---

## 6. Performance Guidelines for C++20

1. **Keep `polyxml::Schema` Instances Long-Lived**: Creating a schema involves heap allocation and string parsing. Create schemas once (e.g. as `static const` or class members) and reuse them across all requests.
2. **Use `std::string_view`**: The `polyxml::deserialize` function accepts `std::string_view`, allowing zero-copy parsing from network buffers, memory-mapped files (`mmap`), or string literals.
3. **Avoid Copying String Values**: `val.get("field")->as_string()` returns `std::optional<std::string_view>`, pointing directly into the parsed token memory without heap string allocations.

---

## 7. C++20 Modules (`--mode modules`)

PolyXML can emit standard **C++20 Module Interface Units (`.cppm`)** instead of traditional header files, eliminating redundant header parsing and reducing incremental build times by up to 80% across large enterprise codebases.

### Generating Modules

**CLI:**

```bash
polyxml generate \
  --lang cpp \
  --mode modules \
  --package enterprise::telemetry \
  --out src/modules \
  schemas/telemetry.xsd
```

**`polyxml.toml`:**

```toml
[[generate]]
target = "cpp"
output = "src/modules"
package = "enterprise::telemetry"
mode = "modules" # or modules = true
```

### Module Structure

The generated `telemetry.cppm` exports the module and its namespace directly:

```cpp
// Target: Modern C++20/C++23 Module Interface Unit
module;

#include <concepts>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <variant>
#include <vector>

export module telemetry;

export namespace enterprise::telemetry {
    struct DroneTelemetry {
        std::int64_t drone_id;
        double altitude_m;
        bool armed;
        bool operator==(const DroneTelemetry&) const = default;
    };
}
```

### CMake 3.28+ Integration

```cmake
cmake_minimum_required(VERSION 3.28)
project(my_telemetry_app LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(telemetry_models)
target_sources(telemetry_models
    PUBLIC
        FILE_SET CXX_MODULES FILES
            src/modules/telemetry.cppm
)

add_executable(my_app src/main.cpp)
target_link_libraries(my_app PRIVATE telemetry_models)
```

### Compiler Support

- **GCC 14+**: `g++ -std=c++20 -fmodules-ts -c telemetry.cppm`
- **Clang 17+**: `clang++ -std=c++20 --precompile telemetry.cppm -o telemetry.pcm`
- **MSVC 2022 (17.4+)**: `cl /std:c++20 /interface /TP /c telemetry.cppm`

---

## 8. High-Throughput Glaze Serialization (`--backend glaze`)

PolyXML provides compile-time reflection metadata for [Glaze](https://github.com/stephenberry/glaze), a C++ JSON/XML serialization library that works without runtime reflection or macros. Benchmark this backend with your own models before making a throughput claim.

### Generating Glaze Metadata

**CLI:**

```bash
polyxml generate \
  --lang cpp \
  --backend glaze \
  --package enterprise::crm \
  --out src/generated \
  schemas/crm.xsd
```

**`polyxml.toml`:**

```toml
[[generate]]
target = "cpp"
output = "src/generated"
package = "enterprise::crm"
backend = "glaze"
```

### Emitted `glz::meta` Specializations

PolyXML automatically emits `#include <glaze/glaze.hpp>` and compile-time `glz::meta` specializations for all structs and enums:

```cpp
#include <glaze/glaze.hpp>

namespace enterprise::crm {
    enum class Status { Active, Suspended };
    struct Customer {
        std::string name;
        Status status;
        std::int32_t id;
    };
}

// Glaze compile-time reflection metadata
template <>
struct glz::meta<enterprise::crm::Status> {
    using T = enterprise::crm::Status;
    static constexpr auto value = enumerate(
        "active", T::Active,
        "suspended", T::Suspended
    );
};

template <>
struct glz::meta<enterprise::crm::Customer> {
    using T = enterprise::crm::Customer;
    static constexpr auto value = object(
        "name", &T::name,
        "status", &T::status,
        "id", &T::id
    );
};
```

### Consuming with Glaze

```cpp
#include "crm.hpp"
#include <glaze/glaze.hpp>
#include <iostream>

int main() {
    enterprise::crm::Customer customer{
        .name = "Global Logistics Ltd",
        .status = enterprise::crm::Status::Active,
        .id = 42
    };

    // Fast serialize to JSON string (multi-GB/s)
    std::string json;
    glz::write_json(customer, json);
    std::cout << "Serialized: " << json << "\n";

    // Fast deserialize from JSON buffer
    enterprise::crm::Customer parsed;
    auto ec = glz::read_json(parsed, json);
    if (!ec) {
        std::cout << "Successfully parsed Customer ID: " << parsed.id << "\n";
    }

    return 0;
}
```

## Generated models and serialization

```bash
polyxml generate customer.xsd --lang cpp --out generated
```

Default generated C++ headers contain model types, not standalone XML codecs.
Use the native `polyxml::Schema`/`polyxml::Value` API shown above and explicitly
map its values to your generated types when XML is required. Generating a header
does not automatically create that mapping or connect it to the native binding.

For JSON metadata, select `--backend glaze` and follow the Glaze example above
with your generated header and the matching Glaze dependency. This is a C++
serialization integration; it does not expose Rust Serde traits. Do not assume
the generated JSON metadata establishes XML round-trip support.
