<p align="center">
  <a href="https://github.com/polyxml/PolyXML">
    <img src="https://raw.githubusercontent.com/polyxml/PolyXML/main/docs/assets/brand/logo_polyxml_banner.png" alt="PolyXML" width="800">
  </a>
</p>

# PolyXML Python Bindings

<p align="center">
  <a href="https://pypi.org/project/polyxml/"><img src="https://img.shields.io/pypi/v/polyxml.svg?logo=pypi&label=PyPI" alt="PyPI"></a>
  <a href="https://www.python.org"><img src="https://img.shields.io/badge/Python-3.12%20%7C%203.13%20%7C%203.14%20%7C%203.15-3776AB.svg?logo=python&logoColor=white" alt="Python: 3.12+"></a>
  <a href="https://polyxml.github.io/PolyXML/languages/python/"><img src="https://img.shields.io/badge/docs-zensical-blue.svg" alt="Documentation"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</p>

High-performance native XML data-binding engine for Python dataclasses and Pydantic models.

Powered by `polyxml-core` written in Rust and PyO3 (`abi3-py312`).

---

## Installation

```bash
pip install polyxml
```

---

## Features

- **⚡ Fast typed XML binding**: 10.0x faster deserialization and 23.5x faster serialization than `xsdata` on the [10,000-item catalog benchmark](../../benchmarks/python/results.md).
- **🌊 Streaming `iterparse()`**: Process records incrementally; measure memory on your own document structure.
- **📦 Binary Serialization**: Native `dumps_binary()` and `loads_binary()` using MessagePack (`msgspec`).
- **🎯 Full Type Support**: Dataclasses and Pydantic v2 models with zero boilerplate.

---

## Quickstart

```python
from dataclasses import dataclass, field
import polyxml


@dataclass
class Item:
    id: int = field(metadata={"type": "Attribute"})
    name: str = field(metadata={"type": "Element"})


# 1. XML Deserialization
item = polyxml.deserialize(b'<Item id="1"><name>Gadget</name></Item>', Item)

# 2. XML Serialization
xml = polyxml.serialize(item, indent=2)

# 3. High-Speed Binary Serialization (Key-Value databases / IPC)
blob = polyxml.dumps_binary(item)
restored = polyxml.loads_binary(blob, Item)
```
