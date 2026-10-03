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
9. For issue #55 compile-time/size work, use
   `benchmarks/rust-phf-compile/run.py` with a freshly built CLI. It requires
   systemd cgroups, disables swap, uses one Cargo worker and caps each build
   separately (40% available RAM by default). Run the 16-field smoke first.
   Use `encode_xml(..., Some("Record"))` for the round trip: root aliases
   otherwise serialize with the type name. The October 2, 2026 attempt at
   600 fields exceeded 5,597 MiB on rustc 1.99.0; a 16-GiB host alone is
   insufficient assurance. After a capped failure, inspect current headroom
   before deliberately raising the budget; keep a substantial host reserve
   and never disable the cap. On WSL, check both Linux MemAvailable and
   Windows physical/free RAM: this machine has 32 GiB physically but WSL
   sees 15.5 GiB. A 70% retry with about 13.7 GiB available leaves roughly
   4 GiB outside the scope; the 600-field build passed its initial peak at
   about 6.1 GiB. The numeric 600/1500 study completed at 70%, peaking at
   about 6.7 GiB RSS; all consumers passed full XML round trips. Retain `journalctl --user -u <scope>` and the
   matching kernel OOM evidence: a group OOM can kill GNU time before it
   writes peak RSS. A failed build duration is not a compile-time result.
   The detailed checklist is in
   [polyxml-phf-compile-benchmark](../polyxml-phf-compile-benchmark/SKILL.md).

10. For runtime regressions, use `benchmarks/rust-xml-regression/run.py` and
    `run_core.py` against an explicit detached baseline worktree. Keep both
    revisions on the same host/toolchain and shared dependency resolutions;
    retain locks, source revisions, generated sources, samples and confidence
    intervals. Generated XML/Serde consumers test owned/borrowed strings and
    all sensor fields before timing; XML writes specify the declared Batch
    root rather than the type name. Core Criterion checks stay separate from
    generated-model timings. Run builds and measured suites serially, alternate
    revision order and repeat uncertain results before calling a regression.
    Check per-process medians as well as pooled samples; do not treat correlated
    samples as independent process repetitions or call noise a speedup.
