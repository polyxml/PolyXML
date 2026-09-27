#include "sensor.hpp"
#include <charconv>
#include <chrono>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <iterator>
#include <stdexcept>
#include <string>
#include <vector>

struct BaselineSensor { std::string id; int value; };

template<class T> T read_xml(const std::string& xml) {
    const auto start = xml.find("<Id>");
    const auto end = xml.find("</Id>");
    const auto number = xml.find("<Value>");
    const auto number_end = xml.find("</Value>");
    if (start == std::string::npos || end == std::string::npos || number == std::string::npos || number_end == std::string::npos) throw std::runtime_error("Invalid fixture");
    T result{};
    result.id = xml.substr(start + 4, end - start - 4);
    const auto *first = xml.data() + number + 7;
    const auto *last = xml.data() + number_end;
    if (std::from_chars(first, last, result.value).ec != std::errc{}) throw std::runtime_error("Invalid integer");
    return result;
}

template<class T> std::string write_xml(const T& value) {
    return "<Sensor><Id>" + value.id + "</Id><Value>" + std::to_string(value.value) + "</Value></Sensor>";
}

template<class T> std::string write_batch(const std::vector<T>& values) {
    std::string xml = "<Batch>";
    for (const auto& value : values) xml += write_xml(value);
    return xml + "</Batch>";
}

template<class T> std::vector<T> read_batch(const std::string& xml) {
    std::vector<T> values;
    std::size_t pos = 0;
    while ((pos = xml.find("<Sensor>", pos)) != std::string::npos) {
        const auto end = xml.find("</Sensor>", pos);
        if (end == std::string::npos) throw std::runtime_error("Invalid batch");
        values.push_back(read_xml<T>(xml.substr(pos, end + 9 - pos)));
        pos = end + 9;
    }
    return values;
}

template<class T> void measure(const char* name, std::size_t size, int iterations) {
    std::vector<T> values;
    values.reserve(size);
    for (std::size_t i = 0; i < size; ++i) values.push_back(T{"sensor-" + std::to_string(i), static_cast<int>(i)});
    std::ifstream file("../workloads/sensor-batch/sensor-" + std::to_string(size) + ".xml");
    if (!file) throw std::runtime_error("Shared XML fixture not found");
    const std::string xml((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    if (xml != write_batch(values)) throw std::runtime_error("Writer differs from shared fixture");
    const auto roundtrip = read_batch<T>(xml);
    if (roundtrip.size() != size || roundtrip.back().id != values.back().id || roundtrip.back().value != values.back().value) throw std::runtime_error("Round trip failed");
    for (int i = 0; i < 100; ++i) { (void)read_batch<T>(xml); (void)write_batch(values); }
    for (int operation = 0; operation < 2; ++operation) {
        volatile std::size_t sink = 0;
        const auto start = std::chrono::steady_clock::now();
        for (int i = 0; i < iterations; ++i) {
            if (operation == 0) sink = sink + read_batch<T>(xml).size();
            else sink = sink + write_batch(values).size();
        }
        const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now() - start).count();
        std::cout << name << ",size=" << size << ',' << (operation == 0 ? "read" : "write") << ",ns/op=" << elapsed / iterations << ",xml_bytes=" << xml.size() << ",sink=" << sink << '\n';
    }
}

int main() {
    const int iterations = std::getenv("BENCH_ITERATIONS") ? std::stoi(std::getenv("BENCH_ITERATIONS")) : 10000;
    for (auto size : {1u, 1000u}) {
        measure<polyxml::generated::Sensor>("generated", size, iterations);
        measure<BaselineSensor>("baseline", size, iterations);
    }
}
