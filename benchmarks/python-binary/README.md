# Python Binary Serialization Benchmark Suite

Self-contained benchmark comparing **PolyXML Binary** (`polyxml.dumps_binary`, `loads_binary` based on MessagePack) against standard library **`pickle`** (protocol 5 with C accelerator) and **`cloudpickle`** across:

1. **Direct (uncompressed)** serialization and deserialization.
2. **Compressed (LZ4)** serialization and deserialization (`lz4.frame`).

## Workloads

- **Small**: Single `SensorReading` entity (~118 B MessagePack vs ~185 B Pickle).
- **Nested**: `Order` entity with 10 line items and optional fields (~642 B MessagePack vs ~684 B Pickle).
- **Repeated Batch**: Collection of 100 sensor readings (~11.8 KB MessagePack vs ~7.2 KB Pickle).

## Semantic Verification

Every run verifies:
- `assert loads(dumps(obj)) == obj` for all serializers in both uncompressed and LZ4 compressed modes.
- Encode latency, decode latency, output byte size, and memory overhead are recorded independently.

## How to Run

```bash
# Smoke test (5 iterations):
./benchmarks/python-binary/run.sh --smoke

# Full benchmark (50 iterations, with raw JSON & Markdown output):
./benchmarks/python-binary/run.sh --iterations 50 --output-json docs/benchmarks/data/2026-09-26/python-binary.json --output-md docs/benchmarks/data/2026-09-26/python-binary.md
```
