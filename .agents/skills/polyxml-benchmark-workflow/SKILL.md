---
name: polyxml-benchmark-workflow
description: Use when running, adding, or publishing PolyXML language and runtime benchmarks.
---

# PolyXML benchmark workflow

1. Read `benchmarks/README.md` and the target suite README before running.
   Identify whether the suite measures generated models, a native binding,
   a standard serializer, or a fixture-specific adapter. Do not combine their
   throughput into a language ranking without equivalent outputs and work.
2. Build the local `polyxml` CLI before generating benchmark models. Use
   `scripts/memcap.sh` for heavy release builds, Java JMH runs, and ad-hoc
   Criterion runs on shared or small hosts.
3. Check decoded values or round-trip correctness before timing. The shared
   sensor reader fixtures are in `benchmarks/workloads/sensor-batch/`; do not
   change input bytes independently in one suite. Its README maps all seven
   language entry points and the different values they return.
4. Use short iterations only as smoke checks. For publishable data, run on an
   otherwise idle host, repeat measurements, and retain every raw sample.
   Record the source commit, OS/CPU, toolchain versions, iteration/warmup
   settings, payload byte lengths, and what the operation returns.
5. For Java, use JDK 22+ and JMH warmup plus multiple forks. `-prof gc` reports
   Java heap allocations only; Panama native allocations need separate tools.
   The Java suite's 80-field workloads differ from the shared sensor fixture.
6. For C++, keep `run_runtime.sh` (PolyXML C ABI scalar binding) separate from
   `run.sh` (fixed-fixture adapter into generated models). Generated C++ models
   currently have no XML codec.
7. Put published summaries under `docs/benchmarks/` and raw output in its
   `data/` subdirectory. Link the page from `docs/benchmarks/index.md` and run
   `uvx zensical build --clean --strict` before pushing. Docs-site Markdown
   must link to repo-only benchmark READMEs with GitHub URLs.
8. Place a specific result or benchmark-source link next to every numerical
   performance claim in the README and docs. For tables, introduce the table
   with a source link. Remove or qualify old figures when no matching result
   or reproducible runner can be found; a general benchmark index link does
   not establish an individual measurement.
