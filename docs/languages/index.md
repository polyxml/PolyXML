---
title: Languages
description: Use PolyXML and generated models in each supported language ecosystem.
---

# Languages

Choose the ecosystem you are using:

- [Rust](rust.md)
- [Python](python.md)
- [C++20](cpp.md)
- [Go](go.md)
- [TypeScript and Node.js](node.md)
- [WebAssembly](wasm.md)
- [Java](java.md)
- [C#](csharp.md)

For shared CLI options and code generation, see the [schema compiler guide](../guides/compiler.md).

## Choose a starting point

All seven targets can generate models from XSD with `polyxml generate`. The default backend is the recommended starting point; enable the extras only when your project needs them. Use a separate `[[generate]]` entry per target in `polyxml.toml` when options differ between languages.

| Target | Default generated models | Optional code generation |
| --- | --- | --- |
| [Rust](rust.md) | Native Rust types with streaming XML codecs | `phf` dispatch; owned strings with `--zero-copy=false`; topological chunking with `--split-units` |
| [Python](python.md) | Dataclasses with slots and keyword-only fields | Pydantic v2 backend; `slots` and `kw-only` for dataclasses |
| [C++20](cpp.md) | Header-based types | C++ modules (`--mode modules`); Glaze metadata |
| [Go](go.md) | Go structs with XML and JSON tags | EasyJSON annotations; Sonic tags (see the [current limitation](go.md)) |
| [TypeScript / Node.js](node.md) | TypeScript interfaces | Zod, Valibot, or TypeBox schemas |
| [WebAssembly](wasm.md) | Browser, Edge & Node runtime (`@polyxml/wasm`) | Streaming `parseStream`, schema validation, zero-dependency Wasm |
| [Java](java.md) | Java records | POJOs, builders, direct XML codecs, Jackson 2/3, Spring Boot 4 |
| [C#](csharp.md) | Record classes | Mutable classes, record structs, source generation |

The [compiler guide](../guides/compiler.md) lists the exact backend, style, and feature values accepted by the CLI. For parsing and serialization from application code, start with your language guide. For command-line generation, validation, builds, and transcoding, start with the [compiler guide](../guides/compiler.md).
