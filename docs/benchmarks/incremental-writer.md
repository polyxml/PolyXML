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
