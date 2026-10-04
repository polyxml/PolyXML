# Matched serializer comparison

Compare serializers within one runtime, on identical inputs and equivalent
values. Rust generated XML codecs compete with `quick-xml` Serde and
`serde-xml-rs`; generated Rust JSON uses `serde_json`, so those rows compare
models. Python PolyXML and xsdata bind the same generated dataclasses;
ElementTree/lxml adapters also construct those models. Pydantic XML has a
separate validating model graph. The Rust dynamic value graph is separate.

## Prepare and run

From the repository root, with systemd cgroups, Rust, uv and Python 3.12:

```bash
CARGO_BUILD_JOBS=1 scripts/memcap.sh cargo build -p polyxml-cli
python3 benchmarks/serializer-comparison/prepare.py
uv venv --python 3.12 benchmarks/serializer-comparison/target/venv
CARGO_BUILD_JOBS=1 scripts/memcap.sh uv pip install \
  --python benchmarks/serializer-comparison/target/venv/bin/python \
  -e './crates/polyxml-python[dev]' xsdata lxml pydantic-xml
python3 benchmarks/serializer-comparison/run.py \
  --output benchmarks/serializer-comparison/target/results
python3 benchmarks/serializer-comparison/summarize.py \
  benchmarks/serializer-comparison/target/results
```

The editable native extension must be a **release** build matching the local
source and CLI (uv's maturin backend builds release by default). Updating the
CLI after codegen requires regenerating the models. For a published rerun,
use the retained dependency versions: install the recorded Python packages and
copy `source/rust-consumer.lock` to `target/rust-consumer/Cargo.lock` before
building. `run.py` refuses to overwrite an existing output directory.

Defaults: six fresh processes per runtime, five 150 ms samples per operation,
64 warmup calls and ten pilot calls to select a small timing chunk. The run
rotates case/lane order, alternates runtime order, and runs builds and measured
processes serially. Cyclic GC stays enabled in Python. Input is preloaded;
schema/model setup is outside timing. Reads include complete materialization
and result destruction. Writes allocate fresh UTF-8 output bytes, including
str-to-bytes conversion where needed. No output buffer reuse. XML formatting
need not be byte-identical; semantic values must match. JSON inputs use the
same field aliases; every writer must return the same JSON values.

Rust allocation calls and total **requested** bytes are recorded in a separate
feature build, never the timing binary. Reallocations count as calls and their
full new sizes count as bytes; this is neither retained nor peak memory.
Python has no comparable total allocation measure here: tracing Python alone
misses native allocations. Do not put the two in one memory ranking.

Every field is checked before timing. Python XML outputs pass lxml XSD
validation; Rust outputs are decoded through handwritten Serde models and
checked again with Python's independent ElementTree implementation. Plain
Rust XML input must actually borrow strings in both Cow lanes. Escaped text
may require owned strings. The generated Cow **JSON** lane is a model choice,
not a claim that its Serde implementation borrows input strings.

The retained artifact directory contains source, locks, generated models,
fixtures, metadata/hashes, output samples, all timing CSVs and build logs.
The summary uses the median of each process's sample median and reports the
minimum/maximum process medians. These are descriptive ranges, not confidence
intervals. Ratios divide a specified baseline's median latency by the row's
median latency; above 1 means faster. Near-parity results need more evidence
before calling a win.

One/1,000 sensor workloads and escaped 1,000-sensor text exercise list sizes,
integer conversion, UTF-8 and entity handling. They are not a conformance suite,
large-schema benchmark, streaming-memory test or universal XML ranking. Java,
Go and C# have dedicated harnesses; the comparison page labels older results
separately until refreshed. C++ and JS need matched-return-value comparisons
before they can join a typed serializer ranking.

## Native models and Java

After the Rust/Python suite finishes, refresh the other matched comparisons:

```bash
python3 benchmarks/serializer-comparison/run_models.py \
  --maven /path/to/apache-maven/bin/mvn \
  --output benchmarks/serializer-comparison/target/model-results
```

Requires Go, the .NET SDK/runtime, Maven and JDK 22+. Go/C# use five fresh
processes with model order alternating. Java uses two JMH forks per case,
three 2-second warmups and five 2-second measurements with GC profiling.
It returns complete equivalent POJO/JAXB graphs; Panama is excluded. Its
80-field synthetic projections differ from the sensor batch.

The default-tiering .NET 10 runs showed a first-model timing effect even
after the longer warmup. Retain them as diagnostics and run this separate
controlled comparison after every other measured suite has finished:

```bash
python3 benchmarks/serializer-comparison/control_csharp.py \
  benchmarks/serializer-comparison/target/model-results
```

This launches six balanced fresh processes with `DOTNET_TieredCompilation=0`.
It measures warmed execution under that explicit JIT configuration, not default
startup or tiered PGO behavior. No build or other benchmark may run concurrently.

Summarize the native runs after the control finishes:

```bash
python3 benchmarks/serializer-comparison/summarize_models.py \
  benchmarks/serializer-comparison/target/model-results
```

`render.py` builds the dated documentation page from summaries retained under
`docs/benchmarks/data/2026-10-04/{serializer-comparison,native-models}/`. It keeps
the displayed values tied to the raw measurements. Java records its actual
StAX provider: the shaded benchmark uses Woodstox, not the JDK default provider.
