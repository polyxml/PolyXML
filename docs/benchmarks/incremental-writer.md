---
title: Incremental Rust writer memory
description: Peak resident memory of generated Rust iterator producers with independent XSD validation.
---

# Incremental Rust writer memory

The generated producer consumes items lazily and writes directly to a buffered
file. It does not construct a root model, retain a child `Vec`, or return XML
bytes. Memory depends on the current item and the caller's sink buffering.

The [runner](https://github.com/polyxml/PolyXML/blob/main/scripts/verify_incremental_writer.py)
compiles and executes both borrowed (`Cow`) and owned (`String`) models from
[the fixture](https://github.com/polyxml/PolyXML/blob/main/tests/fixtures/incremental/items.xsd).
Each sample launches a fresh debug executable under GNU `time` (`%M`, KiB),
then validates its entire output with libxml2 through lxml's streaming XSD
validator. Validation runs in the parent process and is excluded from producer
RSS. Runtime assertions exercise minimum/maximum occurrences, empty optional
collections, sink failures, lazy iterator consumption, and unqualified output.

Reproduce from the repository root with lxml installed in the local environment:

```bash
cargo build -p polyxml-cli
.venv/bin/python scripts/verify_incremental_writer.py \
  --output target/incremental-rss.json
```

The default run measures three fresh processes per model representation at
1,000, 100,000, and 1,000,000 items. This measures peak RSS, not throughput.
A sink that retains the complete output, or an iterator that retains its
inputs, will still grow with document size.

## Recorded run: October 2, 2026

[Raw samples](data/incremental-writer-2026-10-02.json) record source commit,
CPU/OS, compiler version, byte lengths, and all measurements. The run used
`rustc 1.98.1 (48a229cea 2026-09-01)` on AMD Ryzen 5 4500 6-Core Processor, with no concurrent repository test/build
jobs. Three fresh processes were measured for each row; all 18 output documents
passed independent XSD validation.

| Item strings | Items | Median peak RSS (KiB) | Range (KiB) | XML bytes |
| :--- | ---: | ---: | ---: | ---: |
| Cow | 1,000 | 2,604 | 2,576–2,628 | 73,923 |
| Cow | 100,000 | 2,564 | 2,528–2,604 | 7,588,923 |
| Cow | 1,000,000 | 2,436 | 2,436–2,564 | 76,888,923 |
| String | 1,000 | 2,564 | 2,552–2,592 | 73,923 |
| String | 100,000 | 2,564 | 2,560–2,612 | 7,588,923 |
| String | 1,000,000 | 2,552 | 2,500–2,692 | 76,888,923 |

The output grew from 73,923 to 76,888,923 bytes while median producer RSS
remained around 2.5 MiB in both representations. This fixture supports bounded
producer memory as the item count increases; it does not establish a universal
memory ceiling or a throughput result. Per-item payloads and sink buffering
still determine the working set. The runner retains tiny fixed-size regression
outputs in memory, independently of the measured collection size.
