Dynamic Rust XML runtime investigation, October 3, 2026

Retained engine: 7f845ae043b68cdb3166d04ce74eaa99bc9c2b0d
Control (0.34.6): bbbd087a9a8e9838a32912973cd4029c30f29c08
Historical (0.27.0): cef575c11cc4f05e877ced157036f7ff53b8687b

selected-core, selected-constraints, selected-cache-boundaries and
selected-old-core retain harnesses, locks, metadata, build/run logs, summaries
and samples.tar.gz. Each archive contains every original Criterion result file
for the six measured processes, including estimates and sample arrays. To
reconstruct a summary, extract samples.tar.gz in its comparison directory, then:

python3 benchmarks/rust-runtime-investigation/summarize.py COMPARISON_DIRECTORY

selected-generated retains every CSV, emitted model, lock, the common harness,
build logs and metadata for four rounds per revision/mode. For owned consumers,
the runner changes BatchType<'_> to BatchType in the common harness. Summarize:

python3 benchmarks/rust-runtime-investigation/summarize.py --generated selected-generated

screening.tar.gz retains exp1 through exp7 plus FIFO working-set and replacement
policy screens. These short runs select hypotheses; confirmation numbers are
in the selected-* directories. Exp1 retains writer results only; its unanchored
serialization filter also selected reads, but those read artifacts were not
retained and no read conclusion is drawn from that attempt.

prototype-confirmation.tar.gz retains the earlier FIFO implementation's
comparisons and generated controls. Its final-core folder is excluded from
confirmation tables because a companion uv help invocation rebuilt an editable
binding during its last round. investigation-note.json records the exclusion;
final-core-clean repeats all three rounds, and selected-core subsequently tests
the retained replacement policy. Both generated controls are retained because
an initial apparent JSON batch-write slowdown did not repeat.

archives.json records SHA-256 hashes, file counts and compressed sizes. Published
Criterion summaries were independently reconstructed from these archives and
checked for equality. This data does not contain benchmark binaries or downloaded
Valgrind packages.

diagnostics/old, control, fifo and selected retain counting-allocator sources,
locks, counts, measured-region Callgrind files and annotations. Allocation
instrumentation is diagnostic only. Requested bytes are cumulative requests,
not peak RSS. Callgrind counts simulated instructions, not hardware cycles.
Counting is disabled in instruction-profile mode, so its printed allocation
zeros do not indicate allocation-free operations.

verification retains the quality gate, refreshed companion CType output,
reference corpus output, tooling/docs checks and a minimal reproducer for two
unchanged correctness limitations. The corpus reference log has no source SHA;
its summary and group table match the refreshed run exactly. The failed corpus
preflight found no default release CLI; the actual sample explicitly placed the
fresh debug CLI on PATH. The companion lockfile was restored after refreshing
its editable extension, then the sample ran with uv --no-sync.

All measured comparisons alternate revision order and run serially with systemd
memory caps and swap disabled. Core/constraint/cache confirmation: 3 processes,
100 samples, 3-second warmup, 5-second measurement targets. Generated controls:
4 process rounds per revision/mode, 100 warmup calls, 7 samples of at least 250ms.
Schema setup and correctness checks precede timing. Source, payload sizes,
toolchain and host details are in the accompanying report and metadata.
