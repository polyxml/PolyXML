---
title: WebAssembly vs JavaScript XML parsing
description: Reproducible Node, Bun, and Chromium measurements for PolyXML's Wasm runtime.
---

# WebAssembly vs JavaScript XML parsing

These are exploratory measurements from 23 September 2026 on an AMD Ryzen 5
4500 (6 cores), using Node 26.8.2, Bun 1.4.2, and headless Chromium 145.
The fixture repeats `<item id="42"><name>Ada &amp; Bob</name><active>true</active></item>`
inside `<items>`. Its three sizes are 3,075 bytes, 1,048,643 bytes, and
52,428,831 bytes. The scripts and exact run commands are in
[`benchmarks/typescript-wasm`](https://github.com/polyxml/PolyXML/tree/main/benchmarks/typescript-wasm).

The table reports median end-to-end milliseconds for parsing into JavaScript
objects. Lower is better. PolyXML includes Wasm transfer, XML to JSON
transcoding, JSON transfer, and `JSON.parse`. `fast-xml-parser` 5.11.1 uses
numeric scalar parsing and attributes. The cases are similar but do not have
identical output conventions.

| Runtime | Fixture | PolyXML object | fast-xml-parser object | Ratio |
| --- | ---: | ---: | ---: | ---: |
| Node | 3 KB | 0.244 ms | 1.578 ms | 6.5× |
| Node | 1 MB | 36.37 ms | 181.54 ms | 5.0× |
| Node | 50 MB | 1,947.6 ms | 8,629.1 ms | 4.4× |
| Bun | 3 KB | 0.177 ms | 0.456 ms | 2.6× |
| Bun | 1 MB | 31.93 ms | 145.66 ms | 4.6× |
| Bun | 50 MB | 1,716.5 ms | 7,620.6 ms | 4.4× |
| Chromium | 3 KB | 0.2 ms | 0.5 ms | 2.5× |
| Chromium | 1 MB | 40.6 ms | 134.7 ms | 3.3× |
| Chromium | 50 MB | 2,136.5 ms | 6,867.3 ms | 3.2× |

The small and medium cases used 100 and 15 iterations. The 50 MB cases used
separate processes with 3 to 5 iterations. Those large runs and the browser
sub-millisecond timer resolution limit the precision of the ratios. The
benchmark also includes `xml2js` 0.6.2 and browser `DOMParser`, but their
result shapes differ substantially; compare those using the raw suite output.

The generated Wasm binary is 460,190 bytes (167,949 bytes gzip). The generated
Wasm JS glue, browser wrapper, and stream helper total another 20,253 bytes
uncompressed and 5,397 bytes gzipped separately. These are file sizes,
not a minified application bundle. Browser bundlers may change the delivered
size. `wasm-bindgen` copies bytes across the JS/Wasm boundary, and object
conversion allocates in JavaScript. The stream API also buffers one complete
record at a time; it is most useful when a document has many separate records.

The Node runner samples heap deltas and observes GC entries, but this local run
did not yield usable per-case GC pause data. Bun heap deltas were frequently
zero and are not comparable to Node heap deltas. We therefore make no GC
reduction claim. This repeated-record workload does not establish performance
for deeply nested XML, attributes-heavy documents, schemas, or tiny webhook
payloads in production. Run the suite on representative inputs before
choosing a parser solely for speed.

For installation, schema compilation, and streaming usage examples, see the [WebAssembly Runtime guide](../languages/wasm.md).
