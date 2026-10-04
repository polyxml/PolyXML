<p align="center">
  <a href="https://github.com/polyxml/PolyXML">
    <img src="https://raw.githubusercontent.com/polyxml/PolyXML/main/docs/assets/brand/logo_polyxml_banner.png" alt="PolyXML" width="800">
  </a>
</p>

# PolyXML Java Bindings

<p align="center">
  <a href="https://central.sonatype.com/artifact/io.github.polyxml/polyxml"><img src="https://img.shields.io/maven-central/v/io.github.polyxml/polyxml.svg?logo=apache-maven&color=C71A36&label=Maven" alt="Maven Central"></a>
  <a href="https://openjdk.org/projects/panama/"><img src="https://img.shields.io/badge/Java-22%2B%20Panama-ED8B00.svg?logo=openjdk&logoColor=white" alt="Java: 22+ Panama"></a>
  <a href="https://polyxml.github.io/PolyXML/languages/java/"><img src="https://img.shields.io/badge/docs-zensical-blue.svg" alt="Documentation"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</p>

High-performance native XML data-binding runtime for Java 22+ built on **Project Panama FFI** (Foreign Function & Memory API, JEP 454).

Bypasses legacy JNI overhead and avoids intermediate DOM allocation, providing direct native Rust parsing speeds for Java `record`s and JavaBeans.

---

## Generated models for Spring Boot

Use the CLI's `--lang java --backend jackson3` for Spring Boot 4 or
`--backend jackson` for Jackson 2 / Spring Boot 3. Generated models and
direct StAX codecs use Java APIs and do not require this native binding.
Java 25 / Spring Boot 4.1.1 integration is verified in a dedicated CI job;
the [Java guide](../../docs/languages/java.md#spring-boot-4-and-jackson-3)
contains dependencies and XML-text record configuration.

## Installation

### Maven

```xml
<dependency>
    <groupId>io.github.polyxml</groupId>
    <artifactId>polyxml</artifactId>
    <version>0.34.7</version>
</dependency>
```

### Gradle

```groovy
implementation 'io.github.polyxml:polyxml:0.34.7'
```

---

## Features

- **⚡ Zero JNI Overhead**: Direct C-ABI invocations via Java 22 Foreign Function & Memory API (`java.lang.foreign`).
- **☕ Modern Java 22+ Records**: Native integration with immutable records, sealed interfaces, and pattern matching.
- **Generated model integration**: The separate CLI emits direct StAX codecs and Jackson 2/3 annotations for Spring applications. See [the Java guide](../../docs/languages/java.md#spring-boot-4-and-jackson-3).
- **🛡️ Memory Safe**: Automatic arena memory management with `Arena.ofConfined()`.

---

## License

MIT
