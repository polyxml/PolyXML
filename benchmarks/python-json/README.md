# Python JSON Benchmark Suite: PolyXML vs xsdata

Self-contained benchmark comparing **PolyXML** native JSON APIs (`polyxml.loads_json`, `dumps_json`, `JsonParser`, `JsonSerializer`) against the **`xsdata`** JSON data-binding runtime on the **exact same Python dataclass models** and **JSON bytes**.

## Workloads

1. **Small Sensor Telemetry (`SensorReading`)**: Flat entity with 6 scalar fields (~128 bytes JSON).
2. **Nested Order (`Order`)**: Nested domain entity with 10 line items, optional fields, and attribute mappings (~793 bytes JSON).
3. **Repeated Batch (`SensorBatch`)**: Collection of 100 sensor readings (~12.8 KB JSON).

## Semantic Verification

Every run verifies:
- `assert poly_parsed == expected == xs_parsed` for all payloads.
- JSON emitted by `polyxml` is validated by parsing it back into typed models.
- Deserialization and serialization are timed separately.

## How to Run

From the repository root with virtual environment activated:

```bash
# Smoke test (5 iterations):
./benchmarks/python-json/run.sh --smoke

# Full benchmark (50 iterations, with raw JSON & Markdown output):
./benchmarks/python-json/run.sh --iterations 50 --output-json docs/benchmarks/data/2026-09-26/python-json.json --output-md docs/benchmarks/data/2026-09-26/python-json.md
```
