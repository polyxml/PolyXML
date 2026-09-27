#include "polyxml.hpp"
#include "batch.hpp"

#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <iterator>
#include <stdexcept>
#include <string>
#include <vector>

// Helper to convert polyxml::Value to generated Batch
polyxml::generated::Batch to_batch(const polyxml::Value& val) {
    polyxml::generated::Batch batch;
    const auto sensor_list = val.get("Sensor");
    if (!sensor_list) {
        return batch;
    }
    const std::size_t len = sensor_list->size();
    batch.sensor.reserve(len);
    for (std::size_t i = 0; i < len; ++i) {
        const auto item = sensor_list->get_item(i);
        if (!item) continue;
        polyxml::generated::SensorType s;
        if (const auto id_opt = item->get("Id")) {
            if (const auto str_opt = id_opt->as_string()) {
                s.id = std::string(*str_opt);
            }
        }
        if (const auto val_opt = item->get("Value")) {
            if (const auto int_opt = val_opt->as_int()) {
                s.value = static_cast<std::int32_t>(*int_opt);
            }
        }
        batch.sensor.push_back(std::move(s));
    }
    return batch;
}

// Helper to convert generated Batch to polyxml::Value
polyxml::Value from_batch(
    const polyxml::generated::Batch& batch,
    const polyxml::Schema& batch_schema,
    const polyxml::Schema& sensor_schema
) {
    auto batch_val = polyxml::Value::create_record(batch_schema);
    auto list_val = polyxml::Value::create_list(batch.sensor.size());
    for (const auto& s : batch.sensor) {
        auto item = polyxml::Value::create_record(sensor_schema);
        item.set("Id", polyxml::Value::create_string(s.id));
        item.set("Value", polyxml::Value::create_int(s.value));
        list_val.append(std::move(item));
    }
    batch_val.set("Sensor", std::move(list_val));
    return batch_val;
}

static std::string read_fixture(const std::string& path) {
    std::ifstream file(path);
    if (!file) throw std::runtime_error("Shared fixture not found: " + path);
    return std::string((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
}

static void run_correctness_tests(
    const polyxml::Schema& batch_schema,
    const polyxml::Schema& sensor_schema
) {
    // 1. Single sensor batch
    {
        const auto xml = read_fixture("../workloads/sensor-batch/sensor-1.xml");
        const auto val = polyxml::deserialize(xml, batch_schema);
        const auto batch = to_batch(val);
        if (batch.sensor.size() != 1 ||
            batch.sensor[0].id != "sensor-0" ||
            batch.sensor[0].value != 0) {
            throw std::runtime_error("Single-sensor correctness check failed");
        }
        const auto re_val = from_batch(batch, batch_schema, sensor_schema);
        const auto re_xml = polyxml::serialize("Batch", re_val, batch_schema);
        if (re_xml != xml) {
            throw std::runtime_error("Single-sensor round-trip mismatch");
        }
    }

    // 2. 1,000-sensor batch (first, middle, last assertions)
    {
        const auto xml = read_fixture("../workloads/sensor-batch/sensor-1000.xml");
        const auto val = polyxml::deserialize(xml, batch_schema);
        const auto batch = to_batch(val);
        if (batch.sensor.size() != 1000) {
            throw std::runtime_error("1000-sensor batch size mismatch: " + std::to_string(batch.sensor.size()));
        }
        if (batch.sensor.front().id != "sensor-0" || batch.sensor.front().value != 0) {
            throw std::runtime_error("1000-sensor first element mismatch");
        }
        if (batch.sensor[500].id != "sensor-500" || batch.sensor[500].value != 500) {
            throw std::runtime_error("1000-sensor middle element mismatch");
        }
        if (batch.sensor.back().id != "sensor-999" || batch.sensor.back().value != 999) {
            throw std::runtime_error("1000-sensor last element mismatch");
        }
        const auto re_val = from_batch(batch, batch_schema, sensor_schema);
        const auto re_xml = polyxml::serialize("Batch", re_val, batch_schema);
        if (re_xml != xml) {
            throw std::runtime_error("1000-sensor round-trip mismatch");
        }
    }

    // 3. Escaped text test (&amp;, &lt;, &gt;)
    {
        const std::string esc_xml = "<Batch><Sensor><Id>temp &amp; humidity &lt;sensor&gt;</Id><Value>42</Value></Sensor></Batch>";
        const auto val = polyxml::deserialize(esc_xml, batch_schema);
        const auto batch = to_batch(val);
        if (batch.sensor.size() != 1 ||
            batch.sensor[0].id != "temp & humidity <sensor>" ||
            batch.sensor[0].value != 42) {
            throw std::runtime_error("Escaped text decoding check failed");
        }
        const auto re_val = from_batch(batch, batch_schema, sensor_schema);
        const auto re_xml = polyxml::serialize("Batch", re_val, batch_schema);
        if (re_xml.find("&amp;") == std::string::npos || re_xml.find("&lt;") == std::string::npos) {
            throw std::runtime_error("Escaped text re-serialization failed to escape entities");
        }
    }

    // 4. Malformed XML error test
    {
        const std::string broken_xml = "<Batch><Sensor><Id>unterminated";
        bool caught = false;
        try {
            (void)polyxml::deserialize(broken_xml, batch_schema);
        } catch (const polyxml::Exception&) {
            caught = true;
        }
        if (!caught) {
            throw std::runtime_error("Malformed XML did not throw polyxml::Exception");
        }
    }
}

int main() {
    const auto sensor_schema = polyxml::SchemaBuilder("Sensor")
        .add_element("Id", "Id", POLYXML_SCALAR_STRING)
        .add_element("Value", "Value", POLYXML_SCALAR_INT)
        .build();

    const auto batch_schema = polyxml::SchemaBuilder("Batch")
        .add_list_nested("Sensor", "Sensor", sensor_schema)
        .build();

    run_correctness_tests(batch_schema, sensor_schema);

    const int default_iter_1 = 50000;
    const int default_iter_1000 = 1000;

    const int iter_1 = std::getenv("BENCH_ITERATIONS_1")
        ? std::stoi(std::getenv("BENCH_ITERATIONS_1"))
        : (std::getenv("BENCH_ITERATIONS") ? std::stoi(std::getenv("BENCH_ITERATIONS")) : default_iter_1);

    const int iter_1000 = std::getenv("BENCH_ITERATIONS_1000")
        ? std::stoi(std::getenv("BENCH_ITERATIONS_1000"))
        : (std::getenv("BENCH_ITERATIONS") ? std::max(1, std::stoi(std::getenv("BENCH_ITERATIONS")) / 50) : default_iter_1000);

    for (const std::size_t size : {1ul, 1000ul}) {
        const std::string fixture_path = "../workloads/sensor-batch/sensor-" + std::to_string(size) + ".xml";
        const std::string xml = read_fixture(fixture_path);
        const auto initial_val = polyxml::deserialize(xml, batch_schema);
        const auto initial_batch = to_batch(initial_val);
        const int iterations = (size == 1) ? iter_1 : iter_1000;
        const int warmups = std::min(100, std::max(5, iterations / 10));

        // Warmup
        for (int i = 0; i < warmups; ++i) {
            auto v = polyxml::deserialize(xml, batch_schema);
            auto b = to_batch(v);
            auto rv = from_batch(b, batch_schema, sensor_schema);
            (void)polyxml::serialize("Batch", rv, batch_schema);
        }

        // Measure read and write
        for (int op = 0; op < 2; ++op) {
            const std::string op_name = (op == 0) ? "read" : "write";
            for (int repeat = 0; repeat < 5; ++repeat) {
                volatile std::size_t sink = 0;
                const auto start = std::chrono::steady_clock::now();
                if (op == 0) {
                    for (int i = 0; i < iterations; ++i) {
                        auto v = polyxml::deserialize(xml, batch_schema);
                        auto b = to_batch(v);
                        sink = sink + b.sensor.size();
                    }
                } else {
                    for (int i = 0; i < iterations; ++i) {
                        auto rv = from_batch(initial_batch, batch_schema, sensor_schema);
                        auto out_str = polyxml::serialize("Batch", rv, batch_schema);
                        sink = sink + out_str.size();
                    }
                }
                const auto elapsed_ns = std::chrono::duration_cast<std::chrono::nanoseconds>(
                    std::chrono::steady_clock::now() - start).count();
                const auto ns_per_op = elapsed_ns / iterations;
                std::cout << "native_cpp,size=" << size
                          << ",operation=" << op_name
                          << ",repeat=" << repeat
                          << ",ns/op=" << ns_per_op
                          << ",xml_bytes=" << xml.size()
                          << ",sink=" << sink << '\n';
            }
        }
    }

    return 0;
}
