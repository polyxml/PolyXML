<p align="center">
  <a href="https://github.com/polyxml/PolyXML">
    <img src="https://raw.githubusercontent.com/polyxml/PolyXML/main/docs/assets/brand/logo_polyxml_banner.png" alt="PolyXML" width="800">
  </a>
</p>

# `@polyxml/wasm`

<p align="center">
  <a href="https://www.npmjs.com/package/@polyxml/wasm"><img src="https://img.shields.io/npm/v/@polyxml/wasm.svg?logo=npm&color=CB3837&label=npm" alt="npm"></a>
  <a href="https://webassembly.org"><img src="https://img.shields.io/badge/WebAssembly-Wasm-654FF0.svg?logo=webassembly&logoColor=white" alt="WebAssembly"></a>
  <a href="https://polyxml.github.io/PolyXML/languages/wasm/"><img src="https://img.shields.io/badge/docs-zensical-blue.svg" alt="Documentation"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</p>

WebAssembly XML/JSON conversion for Node.js, Bun, and browsers. This package
uses PolyXML's Rust core without a native Node addon.

---

## Installation

```bash
npm install @polyxml/wasm
```

---

## Quickstart

```js
import { createPolyXml } from '@polyxml/wasm'

const polyxml = await createPolyXml()
const value = polyxml.xmlToJson('<order><qty>2</qty></order>')
const xml = polyxml.jsonToXml(value)
```

The browser entry point loads the adjacent `.wasm` asset with `fetch`; Node.js
and Bun load it from the package. An application can pass Wasm bytes or a URL
to `createPolyXml(source)` to control loading. `xmlToJsonBytes` and
`jsonToXmlBytes` return `Uint8Array` instead of converting to JS values/text.

For typed conversion, compile a self-contained XSD once:

```js
const schema = polyxml.schemaFromXsd(xsdText, 'order')
const order = schema.xmlToJson(xmlBytes)
const xmlBytes = schema.jsonToXmlBytes(order)
schema.free()
```

Inline XSDs with `include`, `import`, `redefine`, or `override` are rejected;
browser runtimes cannot read referenced schema files. Schema-free conversion
is dynamic and does not validate against an XSD.

Input `Uint8Array` values are copied into Wasm memory by `wasm-bindgen`, and
output bytes are copied back into JS memory. Converting to JS objects incurs
further allocations. The API does not promise zero-copy behavior or a speedup
over native JS parsers.

For documents containing repeated children under one root, stream records as
they complete:

```js
for await (const record of polyxml.parseStream(readableStream)) {
  console.log(record)
}
```

`parseStream` accepts a web `ReadableStream` or async iterable of byte/string
chunks. It uses schema-free conversion and retains one record at a time, plus
the root tag and transient Wasm copies; the default per-record limit is 16 MiB.
DTDs and mixed root text are not supported.

## Build from source

Install the `wasm32-unknown-unknown` Rust target and `wasm-pack`, then run:

```bash
./crates/polyxml-wasm/build.sh
node --test crates/polyxml-wasm/test/*.test.mjs
```

The publishable package is generated in `crates/polyxml-wasm/pkg/`.
