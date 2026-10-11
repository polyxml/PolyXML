# Java binding benchmarks

This standalone JMH project compares JAXB (`xjc` output), Jackson XML (generated
POJOs), direct StAX codecs (POJOs and records), and the existing Panama native binding.
It checks XML/JSON round trips before measurement. Maven also runs interoperability
tests covering inherited fields, abstract bases, renamed properties, recursive models,
lists, and simple-value wrappers.

## Build and verify

From the repository root, with JDK 22 and Maven:

```bash
cargo build -p polyxml-cli
mvn -f benchmarks/java/pom.xml clean package
java -jar benchmarks/java/target/benchmarks.jar -l
```

`generate.py` builds an 80-field schema and generates both POJO and record models
using the local CLI. Set `POLYXML_BIN` to use another binary. Rebuild the CLI after
changing the generator; an existing binary is deliberately not rebuilt by Maven.
Generated sources and all results belong in the ignored `target/` directory.

For all four runtimes, use a JDK 22+ installation (set `JAVA_HOME` and `PATH`), then:

```bash
cargo build --release -p polyxml-c
mvn -f benchmarks/java/pom.xml -Ppanama clean package
java --enable-native-access=ALL-UNNAMED \
  -jar benchmarks/java/target/benchmarks.jar \
  -jvmArgsAppend "--enable-native-access=ALL-UNNAMED -Djava.library.path=$PWD/target/release" \
  -p workload=settlement -p batchSize=10000 -prof gc \
  -rf json -rff benchmarks/java/target/settlement.json
java --enable-native-access=ALL-UNNAMED \
  -jar benchmarks/java/target/benchmarks.jar \
  -jvmArgsAppend "--enable-native-access=ALL-UNNAMED -Djava.library.path=$PWD/target/release" \
  -p workload=telemetry -p batchSize=50000 -prof gc \
  -rf json -rff benchmarks/java/target/telemetry.json
```

The explicit annotation processor path in the POM is necessary on newer JDKs that
do not discover processors implicitly. A missing `META-INF/BenchmarkList` indicates
annotation processing did not run. Use a clean build when switching the Panama profile.

## Workloads and interpretation

- **Settlement projection:** 10,000 messages, about 25 MB per batch.
- **Telemetry projection:** 50,000 messages, about 35 MB per batch.
- Each message has seven populated scalar fields and 73 optional fields. IDs vary.
  These are reproducible synthetic scalar projections inspired by financial and
  telemetry pipelines, **not official ISO 20022 pacs.008 or USAF UCI documents**.
- All backends receive the identical XML. The current Panama schema builder exposes
  scalar fields, so comparisons of full nested ISO/UCI object graphs require a
  separate benchmark and an expanded native schema API. The Panama path also returns
  native values rather than populating Java POJOs and includes a sampled ID read.
- Each JMH operation processes one batch. Multiply `ops/s` by `batchSize` for
  messages/s, or by total input bytes / 1,000,000 for input MB/s. Divide allocation
  bytes/op by `batchSize` for allocation per message. Output sizes can differ.
- Read benchmarks include parsing and model construction. Write benchmarks use
  preconstructed equivalent values and include output-buffer allocation. The
  generated stream overloads cache their StAX factories per thread, so
  `directRead`/`directWrite` do not repeat provider lookup; `directReadReuse`
  and `directWriteReuse` additionally share one factory across calls and create
  only the reader/writer, which is the pattern for embedding a document in a
  larger stream or reusing configured factories.
- Mutation benchmarks parse, change status, and serialize; POJOs use one setter,
  records reconstruct all 80 components. They include the whole pipeline, not only
  the assignment. Native mutation is excluded because the binding has no setter API.
- JAXB contexts, Jackson mappers, and native schemas are created outside timing.
  Native values are closed deterministically. GC profiling measures Java heap
  allocations, not Rust/native allocations, so Panama allocation numbers are not
  total process allocation figures.

The defaults use three warmup iterations, five measurement iterations, and two forks.
For a quick executable smoke check, use `-p batchSize=10 -wi 0 -i 1 -r 100ms -f 1
-foe true`. Such output is **not performance evidence**. Run on an otherwise idle host,
record JDK/OS/CPU and CLI revision, inspect confidence intervals, and repeat before
making a throughput claim.

## Allocation, JFR, and startup investigations

Use `-prof gc` for heap allocation and `-prof jfr:dir=target/jfr` for Java Flight
Recorder captures (run from this directory or use an absolute output path). List
available profilers with `-lprof`. Use separate runs for profilers to avoid comparing
unequal instrumentation overhead. The warmed setup excludes JAXB/mapper/schema
initialization; `-bm ss -wi 0 -i 1 -f 10` measures the first binding call after setup,
not full JVM/application startup.

GraalVM native-image startup, executable size, and RSS need a separate application
harness; these JMH results do not measure them. Generated direct codecs have static
model calls, but StAX provider selection and application dependencies still determine
native-image configuration. Do not infer universal zero-configuration AOT support
from the absence of model reflection.

The matched serializer refresh checks every input against all three typed
readers before measurement. JMH logs include aggregate input bytes and
per-message writer output lengths. Reads use varying IDs; writes repeatedly
serialize the same preconstructed ID-0 model to fresh output bytes.
