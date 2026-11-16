<p align="center">
  <a href="https://github.com/polyxml/PolyXML">
    <img src="https://raw.githubusercontent.com/polyxml/PolyXML/main/docs/assets/brand/logo_polyxml_banner.png" alt="PolyXML" width="800">
  </a>
</p>

# PolyXML Node.js & TypeScript Bindings

<p align="center">
  <a href="https://www.npmjs.com/package/@polyxml/node"><img src="https://img.shields.io/npm/v/@polyxml/node.svg?logo=npm&color=CB3837&label=npm" alt="npm"></a>
  <a href="https://nodejs.org"><img src="https://img.shields.io/badge/Node.js-20%20%7C%2022-339933.svg?logo=node.js&logoColor=white" alt="Node.js: 20 | 22"></a>
  <a href="https://www.typescriptlang.org"><img src="https://img.shields.io/badge/TypeScript-5.0%2B-3178C6.svg?logo=typescript&logoColor=white" alt="TypeScript: 5.0+"></a>
  <a href="https://polyxml.github.io/PolyXML/languages/node/"><img src="https://img.shields.io/badge/docs-zensical-blue.svg" alt="Documentation"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</p>

High-performance, polyglot streaming XML data-binding engine for Node.js and TypeScript.

- Precompiled native binaries for Linux (x86_64), macOS (Apple Silicon & Intel), and Windows (x64).
- Zero external native toolchain dependencies needed at runtime.
- Powered by the ultra-fast Rust `polyxml-core` engine and NAPI-RS.

---

## Installation


```bash
npm install @polyxml/node
```

## Usage

```javascript
const polyxml = require('@polyxml/node');

const schema = {
  name: 'Book',
  fields: [
    { name: 'id', xmlName: 'id', kind: 'attribute', scalarType: 'int' },
    { name: 'title', xmlName: 'title', kind: 'element', scalarType: 'string' },
    { name: 'price', xmlName: 'price', kind: 'element', scalarType: 'float' }
  ]
};

const xml = '<Book id="42"><title>Rust in Action</title><price>39.99</price></Book>';
const book = polyxml.deserialize(xml, schema);

console.log(book);
// { id: 42, title: 'Rust in Action', price: 39.99 }
```

### Nested Schemas, Lists & `xsi:type` Polymorphism

`FieldDefinition` supports `nestedSchema`, `isList`, and `required`, while
`ModelSchema` supports `xmlName`, `isAbstract`, and `variants` for full
complex-type hierarchies and `xsi:type` round-trips:

```javascript
const shipmentSchema = {
  name: 'Shipment',
  fields: [
    {
      name: 'destination',
      xmlName: 'destination',
      kind: 'element',
      required: true,
      nestedSchema: {
        name: 'Address',
        fields: [
          { name: 'city', xmlName: 'city', kind: 'element', scalarType: 'string', required: true },
        ],
      },
    },
    {
      name: 'vehicles',
      xmlName: 'vehicle',
      kind: 'element',
      isList: true,
      nestedSchema: {
        name: 'Vehicle',
        isAbstract: true,
        variants: [
          {
            name: 'Car',
            fields: [
              { name: 'id', xmlName: 'id', kind: 'element', scalarType: 'string' },
              { name: 'doors', xmlName: 'doors', kind: 'element', scalarType: 'int' },
            ],
          },
        ],
        fields: [{ name: 'id', xmlName: 'id', kind: 'element', scalarType: 'string' }],
      },
    },
  ],
};
```

## License

MIT
