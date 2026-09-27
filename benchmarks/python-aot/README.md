# Python AOT Native Extension vs Standard Dataclass Benchmark Suite

Benchmark comparing PolyXML's standard Python `@dataclass` runtime (`polyxml.deserialize`, `serialize`) against **Ahead-of-Time (AOT) compiled PyO3 native C-extensions** (`--backend aot`).

## Workloads

1. **Real-World Defense & Aerospace Telemetry (`USAF UCI v2.5 Entity`)**:
   - Schema: USAF Universal Command and Control Interface (UCI v2.5).
   - Fixture: `data/uci_entity.xml` (1,510 bytes).
   - Models: Generated standard Python `@dataclass` (`uci_entity_core.EntityMt`) vs Ahead-of-Time PyO3 cdylib (`uci_aot.EntityMt`).
2. **Synthetic Telemetry (`SensorReading`)**:
   - Target schema: `SensorReading` with 5 scalar fields (226 bytes XML).
   - Models: Standard Python `@dataclass(slots=True)` vs `sensor_aot.SensorReadingType`.

## Semantic & Mathematical Verification

Every benchmark execution asserts:
- Semantic field equality: `UUID`, `callsign`, coordinates (`latitude`, `longitude`, `altitude`), and kinematics match identically.
- Mathematical consistency: $\text{ops/sec} \times \text{median latency (s)} == 1.0$.
- Separate measurements for deserialization (read) and serialization (write).

## How to Run

```bash
# Smoke test (10 iterations):
./benchmarks/python-aot/run.sh --smoke

# Full benchmark (2,000 iterations, saving raw JSON & Markdown output):
./benchmarks/python-aot/run.sh --iterations 2000 --output-json docs/benchmarks/data/2026-09-26/python-aot.json --output-md docs/benchmarks/data/2026-09-26/python-aot.md
```
