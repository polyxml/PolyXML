cmake_minimum_required(VERSION 3.20)
project(polyxml_cpp VERSION 0.33.9 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(polyxml_cpp INTERFACE)

target_include_directories(polyxml_cpp INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/../../crates/polyxml-c/include>
    $<INSTALL_INTERFACE:include>
)

# Test target
enable_testing()
add_executable(test_polyxml tests/test_polyxml.cpp)
target_link_libraries(test_polyxml PRIVATE polyxml_cpp)

# Link native polyxml-c library
find_library(POLYXML_LIB NAMES polyxml polyxml.dll.a libpolyxml.so libpolyxml.dylib
    PATHS
    ${CMAKE_CURRENT_SOURCE_DIR}/../../target/debug
    ${CMAKE_CURRENT_SOURCE_DIR}/../../target/release
)

if(POLYXML_LIB)
    target_link_libraries(test_polyxml PRIVATE ${POLYXML_LIB})
endif()

add_test(NAME test_polyxml COMMAND test_polyxml)

# Export target
install(TARGETS polyxml_cpp EXPORT polyxml_cppTargets)
install(DIRECTORY include/ DESTINATION include)
