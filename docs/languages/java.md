---
title: Java
description: Generate Java records and POJOs for Spring Boot 4 and Jackson 3, use direct StAX codecs, or integrate the Java 22+ native binding.
---

# Java

PolyXML supports Java 22 or newer for both generated Java applications and the
published Java binding. Java 25 is a good LTS choice. Generated models and direct
StAX codecs use only Java APIs; they do not automatically call the Rust runtime.
Use the Panama binding below when your application needs native PolyXML parsing.

## Java 22 models: records, POJOs, and builders

Records remain the default. Select mutable JavaBeans for setter-based frameworks:

```bash
polyxml generate schema.xsd --lang java --style pojo --feature builder --out generated
polyxml generate schema.xsd --lang java --style pojo --backend jackson3 --feature builder --out generated
polyxml generate schema.xsd --lang java --style record --feature builder --out generated
```

`--style class` is an alias for `pojo`. Mutable complex types have a public no-argument
constructor, private fields, `getX`/`setX` accessors (`isX` for primitive booleans),
`Serializable`, and value-based `equals`, `hashCode`, and `toString`. XSD extensions
use Java inheritance. Optional scalar properties are nullable boxed values; repeated
properties are mutable lists. A list getter initializes the list if it was set to null,
so `order.getItems().add(item)` works. Simple-value wrappers and choice branches also
use mutable classes in this mode.

```java
var entity = EntityMt.builder().id("UUID-1234").build();
entity.setStatus("PROCESSED");
```

Builders include inherited fields and produce a fresh object on each `build()`.
POJO builders copy their list containers; nested objects remain shared. Record builders
use the canonical constructor and its facet checks. When builders or direct codecs
are enabled, records flatten inherited fields into their components. POJO setters enforce
supported facets, but a no-argument constructor permits a partially populated object.

On schemas where most fields are optional and empty, POJO construction is cheaper
than record construction: every empty optional still allocates an `Optional`
component in the record's canonical constructor. The JMH suite measured POJO reads
at roughly twice the record read throughput in the [published JMH runs](../benchmarks/language-results-2026-09.md#java-jmh-binding-comparison);
dense messages narrow the gap. Choose the representation for its ergonomics and
benchmark your own schema before optimizing for this.
This is not full XSD validation. The Jackson backend annotates fields explicitly and
disables automatic bean-property discovery to avoid duplicate properties after XML
names are converted to Java identifiers. The `jackson` backend requires Jackson 2.x XML/annotations; `jackson3` targets Jackson 3.
Standard POJOs and direct codecs require only the JDK. Jakarta Validation is
optional and enabled with `--feature validation`.

## Spring Boot 4 and Jackson 3

Use `--backend jackson3` for Spring Boot 4. The `jackson`, `spring`, and
`spring-boot` backends retain Jackson 2 behavior for Spring Boot 3 applications;
`jackson-3`, `jackson_3`, and `spring-boot-4` select Jackson 3.

```bash
polyxml generate schema.xsd --lang java --backend jackson3 --style pojo \
  --feature builder,validation --package com.example.models --out generated
```

The same options work in a manifest:

```toml
[[generate]]
target = "java"
backend = "jackson3"
style = "pojo"
features = ["builder", "validation"]
package = "com.example.models"
output = "generated/java"
```

For Maven, inherit the Spring Boot parent (the integration fixture pins 4.1.1)
so Boot manages compatible dependency versions. Add:

```xml
<dependency>
  <groupId>org.springframework.boot</groupId>
  <artifactId>spring-boot-starter-webmvc</artifactId>
</dependency>
<dependency>
  <groupId>tools.jackson.dataformat</groupId>
  <artifactId>jackson-dataformat-xml</artifactId>
</dependency>
<dependency>
  <groupId>org.springframework.boot</groupId>
  <artifactId>spring-boot-starter-validation</artifactId>
</dependency>
```

Jackson 3 XML annotations live in `tools.jackson.dataformat.xml.annotation`;
core annotations remain in `com.fasterxml.jackson.annotation`. The new backend
emits schema property order explicitly, overriding Jackson 3's default alphabetical
sort so ordinary XSD sequences retain their element order. Generated models
need no native PolyXML library. Spring's HTTP converters handle them as XML or
JSON according to content negotiation. Use `@Valid @RequestBody` on controller
parameters to activate generated Jakarta constraints. This validates supported
model constraints, not the complete XSD content model.

For **records with simple content and attributes**, configure the XML text
property name to match the generated `value` component:

```java
@Bean
XmlMapperBuilderCustomizer polyxmlXmlText() {
    return builder -> builder.nameForTextElement("value");
}
```

Import `org.springframework.context.annotation.Bean` and
`org.springframework.boot.jackson.autoconfigure.XmlMapperBuilderCustomizer`.
For standalone XML mapping, use
`tools.jackson.dataformat.xml.XmlMapper.builder().nameForTextElement("value").build()`.
POJOs also work with this setting. Without it, Jackson's default empty text
property name cannot bind the record constructor's `value` parameter.

The reproducible integration test uses Java 25, Spring Boot 4.1.1, and its
managed Jackson 3.1.5 dependencies. It compiles generated records and POJOs with
builders, validation, and direct codecs; tests JSON/XML mapper and real HTTP
round trips, namespaces, attributes, text, enum and temporal values, repeated
items, absent optional values, invalid requests, and XSD validation of returned XML:

```bash
# Set JAVA_HOME and PATH to a Java 25 JDK first.
./scripts/verify_spring_boot.sh
```

A dedicated CI job runs this integration fixture on Java 25 with Spring Boot 4.1.1.

See `tests/java-spring/` for the fixture. The test requires Maven and dependency
downloads on its first run, so it is separate from the fast Rust test suites
and runs in a dedicated Java 25 CI job.
The existing direct-codec limits below still apply; this does not establish
GraalVM native-image compatibility or general Jackson support for every XSD.

## Direct streaming XML codecs

```bash
polyxml generate schema.xsd --lang java --style pojo --feature builder,direct-codec --out generated
```

Annotation-based codecs are the default and emit only models.
`--feature direct-codec` adds a companion
`TypeNameCodec.java` for every generated type, with statically linked getters/setters,
constructors, and nested codecs. It works with either record or POJO models and can
be combined with Jackson annotations. It uses StAX from the JDK, without reflection
or a native library:

```java
try (var input = java.nio.file.Files.newInputStream(path)) {
    EntityMt entity = EntityMtCodec.readXml(input);
    entity.setStatus("PROCESSED");
    try (var output = java.nio.file.Files.newOutputStream(destination)) {
        EntityMtCodec.writeXml(entity, output);
    }
}
```

The stream overloads create and close their StAX reader/writer and leave ownership of
the supplied stream with the caller. Their `XMLInputFactory`/`XMLOutputFactory` are
cached in a per-thread field, so repeated calls do not repeat StAX provider lookup or
security-property setup. For reuse inside a larger stream, pass an
`XMLStreamReader` positioned at a start element or an `XMLStreamWriter`. Reading leaves
the cursor at the matching end element. The writer overload accepting `local` and `ns`
selects a particular root element when a type has multiple XML roots. Otherwise, the
first matching root declaration is used, or the type name if no root is declared.

Supported bindings include attributes, text, nested and recursive models, enums,
simple restrictions, nullable values, repeated elements, XML lexical lists in the IR,
choice wrappers, namespaces, XML booleans, and binary values. Unknown elements are
skipped as complete subtrees. Stream-based readers disable DTDs and external entities;
callers supplying a StAX reader configure their own parser. The codecs follow the
normalized IR: choice wrappers retain their generated wrapper representation, and
this is not a general XSD validator or a replacement for missing schema-parser features.
Wildcard fields and `xs:anyType` are rejected by the CLI in direct mode. Runtime
`xsi:type` dispatch is rejected; use the concrete type's codec and XML without
runtime type overrides. Passing a derived POJO to a base codec is also rejected
instead of silently dropping derived fields. Rust API callers
should call `validate_direct_codecs(&ir)` before generation. Models containing cycles
are representable, but serializing an actual cyclic object graph is not supported.

```toml
[[generate]]
target = "java"
output = "generated/java"
package = "com.enterprise.models"
style = "pojo"
features = ["builder", "direct-codec"]
backend = "standard" # Optional: jackson3 for Boot 4 or jackson for Jackson 2
```

The same options work under `[codegen.java]`. See the
[Java benchmark harness](https://github.com/polyxml/PolyXML/tree/main/benchmarks/java)
for JAXB, Jackson, direct POJO/record, and Panama comparisons. No throughput advantage
is assumed; benchmark the relevant schema and workload.

## Java 22+ native bindings

The Panama binding requires a native PolyXML library in addition to Java 22+.

PolyXML provides native C/Rust XML data-binding for the modern Java Virtual Machine using **Java 22+ Project Panama (Foreign Function & Memory API - JEP 454)**. It completely eliminates legacy JNI glue code, GC object pinning, and JNI transition overheads by leveraging native off-heap memory and downcall method handles.

---

## 📦 Build Configuration

### Maven (`pom.xml`)

```xml
<dependencies>
    <dependency>
        <groupId>io.github.polyxml</groupId>
        <artifactId>polyxml</artifactId>
        <version>0.23.2</version>
    </dependency>
</dependencies>

<build>
    <plugins>
        <plugin>
            <groupId>org.apache.maven.plugins</groupId>
            <artifactId>maven-compiler-plugin</artifactId>
            <version>3.13.0</version>
            <configuration>
                <release>22</release>
            </configuration>
        </plugin>
    </plugins>
</build>
```

### JVM Runtime Flags

Because Project Panama accesses native off-heap memory, your application requires the `--enable-native-access` JVM flag at runtime:

```bash
java --enable-native-access=ALL-UNNAMED -jar target/my-app.jar
```

---

## 1. Schema Construction & Resource Management

PolyXML schemas are native off-heap objects. `PolyXML.Schema` implements `AutoCloseable`, enabling clean deterministic cleanup with Java's standard `try-with-resources`:

```java
package com.example;

import io.polyxml.PolyXML;

public class Application {
    public static void main(String[] args) {
        System.out.println("PolyXML Native Core Version: " + PolyXML.version());

        // Build an off-heap native schema using try-with-resources
        try (PolyXML.Schema schema = new PolyXML.SchemaBuilder("ServerMetrics")
                .addField("serverId", "id", PolyXML.FieldKind.ATTRIBUTE, PolyXML.ScalarType.INT)
                .addField("hostname", "host", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.STRING)
                .addField("cpuUtilization", "cpu", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.FLOAT)
                .addField("isHealthy", "healthy", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.BOOL)
                .build()) {

            System.out.println("Schema created successfully in off-heap memory.");
        }
    }
}
```

---

## 2. End-to-End Deserialization & Serialization

Deserialize XML strings or byte buffers into native `PolyXML.Value` objects and serialize back to XML:

```java
package com.example;

import io.polyxml.PolyXML;

public class SerializationExample {
    public static void main(String[] args) {
        try (PolyXML.Schema schema = new PolyXML.SchemaBuilder("Sensor")
                .addField("id", "id", PolyXML.FieldKind.ATTRIBUTE, PolyXML.ScalarType.INT)
                .addField("name", "name", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.STRING)
                .addField("reading", "reading", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.FLOAT)
                .addField("calibrated", "calibrated", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.BOOL)
                .build()) {

            String xml = "<Sensor id=\"101\"><name>Barometer</name><reading>1013.25</reading><calibrated>true</calibrated></Sensor>";

            // 1. Deserialize off-heap
            try (PolyXML.Value val = PolyXML.deserialize(xml, schema)) {
                long id = val.getField("id").getInt().orElse(0L);
                String name = val.getField("name").getString().orElse("");
                double reading = val.getField("reading").getFloat().orElse(0.0);
                boolean calibrated = val.getField("calibrated").getBool().orElse(false);

                System.out.printf("Sensor %d [%s]: %.2f (Calibrated: %b)%n", id, name, reading, calibrated);

                // 2. Serialize back to XML string with 2-space indentation
                String outputXml = PolyXML.serializeToString("Sensor", val, schema, 2);
                System.out.println("Output XML:\n" + outputXml);
            }
        }
    }
}
```

---

## 3. XML Namespaces & Prefix Mapping

Declare model and field namespaces, and serialize with custom prefix mappings:

```java
package com.example;

import io.polyxml.PolyXML;
import java.util.Map;

public class NamespaceExample {
    public static void main(String[] args) {
        try (PolyXML.Schema schema = new PolyXML.SchemaBuilder("Order")
                .setNamespace("https://example.com/orders")
                .addField("id", "id", PolyXML.FieldKind.ATTRIBUTE, PolyXML.ScalarType.INT)
                .addField("item", "item", PolyXML.FieldKind.ELEMENT, PolyXML.ScalarType.STRING, "https://example.com/items")
                .build()) {

            String xml = "<ns0:Order xmlns:ns0=\"https://example.com/orders\" xmlns:ns1=\"https://example.com/items\" id=\"888\"><ns1:item>JavaGadget</ns1:item></ns0:Order>";

            try (PolyXML.Value val = PolyXML.deserialize(xml, schema)) {
                long id = val.getField("id").getInt().orElse(0L);
                String item = val.getField("item").getString().orElse("");
                System.out.printf("Order #%d: %s%n", id, item);

                // Serialize with custom prefix mapping
                Map<String, String> nsMap = Map.of(
                    "ord", "https://example.com/orders",
                    "itm", "https://example.com/items"
                );

                byte[] bytes = PolyXML.serializeWithOptions("Order", val, schema, 2, true, nsMap);
                System.out.println(new String(bytes, java.nio.charset.StandardCharsets.UTF_8));
            }
        }
    }
}
```

---

## 4. Off-Heap Confined Arenas & Zero-GC Pressure

When passing large XML strings or byte streams from Java into PolyXML, Java 22's `Arena.ofConfined()` allocates off-heap memory segments that bypass the JVM garbage collector entirely:

```java
import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.nio.charset.StandardCharsets;

public class OffHeapBufferExample {
    public static void allocateAndPassXml(String xmlData) {
        // Arena confines allocation to current thread and frees immediately upon exit
        try (Arena arena = Arena.ofConfined()) {
            byte[] xmlBytes = xmlData.getBytes(StandardCharsets.UTF_8);
            MemorySegment nativeBuffer = arena.allocate(xmlBytes.length);
            nativeBuffer.copyFrom(MemorySegment.ofArray(xmlBytes));

            System.out.printf("Allocated %d bytes off-heap with zero GC pressure%n", nativeBuffer.byteSize());
            // Pass nativeBuffer.address() to PolyXML native routines...
        } // Instant off-heap deallocation occurs here
    }
}
```

---

## 5. Enterprise Architecture: ISO 20022 Batch Processing

In high-throughput enterprise architectures (e.g. processing millions of ISO 20022 XML financial payment messages or HL7 clinical records):

1. **Singleton Native Schemas**: Store `PolyXML.Schema` instances in `static final` fields or Spring Singleton beans so they are created once at application startup.
2. **Eliminate Garbage Collection Pauses**: By streaming raw socket or file bytes into off-heap `MemorySegment` buffers, you prevent millions of short-lived XML DOM strings from exhausting the JVM Young Generation heap.
3. **Thread Safety**: PolyXML's native schema handles are immutable and read-only after construction, making them safe to share concurrently across all JVM virtual threads (Project Loom).

---

## 6. Jackson 2 Backend (`--backend jackson`)

PolyXML's code generator supports an opt-in **Jackson backend** that annotates generated Java 22+ `record`s with [Jackson](https://github.com/FasterXML/jackson) annotations for seamless integration with **Spring Boot 3**, **Quarkus**, **Micronaut**, and any framework using `ObjectMapper` or `XmlMapper`.

This section covers Jackson 2. For Spring Boot 4, use the
[Jackson 3 setup above](#spring-boot-4-and-jackson-3).

### Quick Start

**CLI:**

```bash
polyxml generate \
  --lang java \
  --backend jackson \
  --package com.enterprise.banking \
  --out src/main/java/com/enterprise/banking \
  schemas/pacs_008_core.xsd
```

**`polyxml.toml`:**

```toml
[[generate]]
target = "java"
output = "src/main/java/com/enterprise/banking"
package = "com.enterprise.banking"
backend = "jackson"
```

### What Gets Generated

When `--backend jackson` is active, PolyXML emits the following annotations:

| Annotation | Applied To | Purpose |
|---|---|---|
| `@JsonIgnoreProperties(ignoreUnknown = true)` | Record class | Forward-compatible deserialization |
| `@JsonInclude(NON_EMPTY)` | Record class + optional/list fields | Skip empty values during serialization |
| `@JacksonXmlRootElement(localName, namespace)` | Record class | XML root element binding |
| `@JsonProperty("...")` | Record components | JSON field name mapping |
| `@JacksonXmlProperty(localName, isAttribute, namespace)` | Record components | XML attribute vs. element discrimination |
| `@JacksonXmlElementWrapper(useWrapping = false)` | List components | Unboxed XML sequences |
| `@JsonValue` / `@JsonCreator` | Enum `getValue()` / `fromValue()` | Enum string serialization |
| `@JsonTypeInfo` / `@JsonSubTypes` / `@JsonTypeName` | Sealed interfaces (choice types) | Polymorphic type discrimination |
| `@JsonValue` / `@JacksonXmlText` / `@JsonCreator` | Simple type wrappers | Transparent value serialization |

### Maven Dependencies

Add Jackson XML to your project:

```xml
<dependencies>
    <dependency>
        <groupId>com.fasterxml.jackson.dataformat</groupId>
        <artifactId>jackson-dataformat-xml</artifactId>
        <version>2.18.3</version>
    </dependency>
    <dependency>
        <groupId>com.fasterxml.jackson.datatype</groupId>
        <artifactId>jackson-datatype-jdk8</artifactId>
        <version>2.18.3</version>
    </dependency>
</dependencies>
```

### Spring Boot 3 Usage Example

```java
package com.enterprise.banking;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.xml.XmlMapper;

public class PaymentProcessor {
    private static final XmlMapper XML = new XmlMapper();
    private static final ObjectMapper JSON = new ObjectMapper();

    public CreditTransfer parseXml(String xml) throws Exception {
        return XML.readValue(xml, CreditTransfer.class);
    }

    public String toJson(CreditTransfer transfer) throws Exception {
        return JSON.writeValueAsString(transfer);
    }
}
```

> **Note:** The default `--backend standard` (or no `--backend`) continues to emit pure, zero-dependency Java 22+ records with no Jackson imports.

## Generate once for XML and JSON

```bash
polyxml generate customer.xsd --lang java --style pojo --backend jackson3 --out generated
```

Compile the generated sources with Jackson 3's databind and XML modules (or the
managed dependencies in the Spring Boot 4 example above). Use your generated
root model in both mapper calls:

```java
var xmlMapper = tools.jackson.dataformat.xml.XmlMapper.builder().build();
var jsonMapper = tools.jackson.databind.json.JsonMapper.builder().build();
Customer customer = xmlMapper.readValue(xml, Customer.class);
String json = jsonMapper.writeValueAsString(customer);
Customer restored = jsonMapper.readValue(json, Customer.class);
String outputXml = xmlMapper.writeValueAsString(restored);
```

Replace `Customer` with the emitted class for your schema. Use `--backend jackson`
with Jackson 2's `com.fasterxml.jackson` mapper packages. For direct StAX XML
without Jackson, see [direct streaming XML codecs](#direct-streaming-xml-codecs).
Java mapping annotations and direct codecs are separate from the native binding
and Rust Serde. The direct-codec support limits described above still apply.
