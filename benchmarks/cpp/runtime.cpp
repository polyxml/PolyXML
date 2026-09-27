#include "polyxml.hpp"
#include <chrono>
#include <cstdlib>
#include <iostream>
#include <stdexcept>
#include <string>

int main() {
    const std::string xml = "<Sensor><Id>sensor-0</Id><Value>0</Value></Sensor>";
    const auto schema = polyxml::SchemaBuilder("Sensor")
        .add_element("Id", "Id", POLYXML_SCALAR_STRING)
        .add_element("Value", "Value", POLYXML_SCALAR_INT)
        .build();
    const auto value = polyxml::deserialize(xml, schema);
    if (value.get("Id")->as_string().value() != "sensor-0" ||
        value.get("Value")->as_int().value() != 0) {
        throw std::runtime_error("Native C++ binding decoded an unexpected value");
    }
    const auto output = polyxml::serialize("Sensor", value, schema);
    const auto decoded_again = polyxml::deserialize(output, schema);
    if (decoded_again.get("Id")->as_string().value() != "sensor-0") {
        throw std::runtime_error("Native C++ binding round trip failed");
    }
    const int iterations = std::getenv("BENCH_ITERATIONS")
        ? std::stoi(std::getenv("BENCH_ITERATIONS")) : 100000;
    for (int i = 0; i < 1000; ++i) {
        (void)polyxml::deserialize(xml, schema);
        (void)polyxml::serialize("Sensor", value, schema);
    }
    for (int operation = 0; operation < 2; ++operation) {
        for (int repeat = 0; repeat < 5; ++repeat) {
            volatile std::size_t sink = 0;
            const auto start = std::chrono::steady_clock::now();
            for (int i = 0; i < iterations; ++i) {
                if (operation == 0) {
                    auto parsed = polyxml::deserialize(xml, schema);
                    sink = sink + parsed.get("Id")->as_string()->size();
                } else {
                    sink = sink + polyxml::serialize("Sensor", value, schema).size();
                }
            }
            const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                std::chrono::steady_clock::now() - start).count();
            std::cout << "native," << (operation == 0 ? "read" : "write")
                << ",repeat=" << repeat << ",ns/op=" << elapsed / iterations
                << ",input_bytes=" << xml.size() << ",output_bytes=" << output.size()
                << ",sink=" << sink << '\n';
        }
    }
}
