---
title: WebAssembly Runtime
description: Use PolyXML's Rust XML/JSON core from Node.js, Bun, and browsers through WebAssembly.
---

# WebAssembly Runtime

Install the published package with `npm install @polyxml/wasm`. It uses the
same Rust core as the native bindings and works without a native Node addon.
To build it from source, run `./crates/polyxml-wasm/build.sh`.

```js
import { createPolyXml } from '@polyxml/wasm'

const polyxml = await createPolyXml()
const value = polyxml.xmlToJson('<order><qty>2</qty></order>')
const xml = polyxml.jsonToXml(value)
```

`xmlToJson` returns a JavaScript object; `jsonToXml` returns a string. For a
byte-oriented path, use `xmlToJsonBytes` and `jsonToXmlBytes`, which return
`Uint8Array`. Inputs may be strings, `Uint8Array`, or `ArrayBuffer`.

To apply XSD field types, compile a self-contained schema once:

```js
const schema = polyxml.schemaFromXsd(xsdText, 'order')
try {
  const order = schema.xmlToJson(xmlBytes)
  const output = schema.jsonToXmlBytes(order)
} finally {
  schema.free()
}
```

The inline XSD path rejects `include`, `import`, `redefine`, and `override`
because those need an external schema resolver. Browser bundlers load the
adjacent `.wasm` asset through the package's browser entry point; Node.js and
Bun use the Node entry point. `createPolyXml(source)` also accepts an explicit
Wasm source when the host controls asset loading.

`wasm-bindgen` copies byte inputs into Wasm memory and byte outputs back to
JavaScript memory. Converting JSON to a JavaScript object allocates again. The
complete-document methods hold the input and output in memory. For a document
with repeated children under one root, `parseStream` yields each child when it
is complete:

```js
for await (const record of polyxml.parseStream(xmlReadableStream)) {
  console.log(record)
}
```

The stream accepts a web `ReadableStream` or an async iterable of strings or
bytes. It uses the schema-free converter and yields one object per direct child
of the document root. It keeps only the current child in memory, plus the root
tag and Wasm copies; the default maximum child size is 16 MiB and can be set
with `{ maxRecordBytes: 1048576 }`. The root must contain records rather than
mixed text. DTDs and other declarations are rejected. This API is for large
lists of records; it cannot split a single huge child into smaller records.

See [Wasm benchmark results](../benchmarks/wasm-vs-js.md) for measured tradeoffs.
