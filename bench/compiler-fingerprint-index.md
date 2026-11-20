# Checkpoint fingerprint indexing

The compiler now indexes the repeated searches inside `elaborate::incremental::fingerprint`. The change preserves the checkpoint topology domain, every shape/body preimage and the existing refusal rules. It removes a structural lookup cost; whole-source loading, serialization, hashing and global packaging remain. Performance is not yet measured for this change.

## Cost and scope

The old function searches all input paths for each source file and type node, and scans every function once for every source file. With F source files, I inputs, N functions and T type nodes, those searches cost O(FN + (F+T)I) comparisons, before the existing sorting and byte work.

The replacement constructs a first-path map, a FileId-to-input-index vector and per-file vectors of function indices. Index preparation uses O((I+F) log I + N + F) lookup/grouping work, followed by direct FileId lookup for type nodes. Path comparisons still depend on path length. Temporary storage is O(unique path bytes + I + F + N); raw input payloads remain borrowed. The path map is dropped before the per-file body walk. Existing region sorting, source-path membership, serialization and hashing remain unchanged.

This does not make a one-body edit independent of total program size. The historical 100k edit's 897.889 ms aggregate key/hash interval is not a measurement of this helper alone. The closed conditional-capture comparison used `--no-store` and never exercised this driver path; none of its twelve observations is reused as evidence for this change.

The [frozen implementation and plan](compiler-fingerprint-index-data/preparation-1/PLAN.md) retain both complete function bodies and the brace-delimited replacement. The single production item keeps its signature and the `lkjscript-front-topology-2` domain. No store format, authentication contract, checked-IR rule or optimizer changes.

## Preserved behavior

The first-path map keeps the first matching input, including a truncated first duplicate. Missing source bytes remain an Option entry and refuse at that file's original visit. Grouping retains original function indices and does not inspect blocks early. An out-of-range function FileId remains ignored even if its body index is invalid. Valid-file bodies are inspected at their original per-file/function visit.

Regions still sort by `(start, end, function index)`. Raw gaps and suffixes, body hashes and their original vector slots, extra input order, type spellings, expanded include literals, all arena/scoping coordinates and the local-item refusal retain their exact encoding order. Duplicate unparsed inputs are still encoded individually in their original order.

Index construction adds allocations before the old failure points, so resource behavior is not identical. A malformed type FileId still bounds-traps, now at the input-index vector instead of the source-file vector. The static review does not claim identical trap locations for malformed internal coordinates. These qualifications are recorded in the [root review](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-fingerprint-index-data/root-integration-review-1.json).

## Original implementation as the oracle

Before changing production source, the unchanged `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db` compiler built a small harness that directly calls the original production fingerprint function. The harness source is an exact 1,744-file population: Git `9326dbaa` plus a temporary main and the frozen fixture module. The original incremental module remains SHA-256 `760485a5a12d97ce320ba9113797370df584c154084ee4a1f9689da28c7d1127`.

The [prospective controls](compiler-fingerprint-index-data/golden-preparation-1/control/plan.json) are committed at `042bce95`. The build and dump each ran once, with explicit source/library/runtime selectors, inline0, no store, a 4 GiB address-space limit and a 900 s watchdog/10 s grace. Full source checks were outside the command intervals.

| Phase | GNU wall / user / system s | Peak RSS KiB | Load before → after (1/5/15 min) |
| --- | --- | ---: | --- |
| Original-harness build | 13.77 / 13.36 / 0.13 | 470,112 | 1.55/3.20/2.89 → 1.58/3.13/2.87 |
| Original-harness dump | 0.00 / 0.00 / 0.00 | 3,692 | 1.58/3.13/2.87 → 1.58/3.13/2.87 |

The dump's launch-inclusive interval is 6.771441 ms; GNU's 0.00 is rounded. The actual image is 1,568,827 B, SHA-256 `99a179be5fd5f6cd9c8e7d3e4466468ff4b519a406303e77e935c1d27dcade89`. The complete 4,531-byte dump is SHA-256 `346340880cf8b9bd247c0caf30f75ed534a702c8b0257dc81fcea2cb5849e4cd`.

All seventeen records are retained, with eleven Some values and six None outcomes. Complete strings include each case name, shape, every body slot, empty slots and terminators. The new tests compare those captured records; they do not copy the indexing algorithm to manufacture an oracle.

| Fixed cases | Required relation or outcome |
| --- | --- |
| Base and reversed function arena | Complete six-slot vector reverses, including the empty slot; shape changes. This is a helper-coordinate fixture, not admission of a permuted AST. |
| Later duplicate parsed input | Same shape and every body as the base. |
| First duplicate parsed input | Same shape; only the alpha function's body slot changes. |
| Missing bodyless source; truncated first duplicate | None for both. |
| Ignored invalid function FileId and body index | The ignored function's slot becomes empty; other body slots remain exact. |
| Reversed bounds; body past EOF; overlapping bodies; type past EOF | None for all four. |
| Reordered or duplicate unparsed inputs | Shape changes; complete body vectors stay equal. |
| Parsed-source include with a child body edit | Shape and child body change; the root body remains equal. |
| Binary or UTF-8 string include payload edit | Shape changes; both body slots remain equal. |

The [independent acceptance](compiler-fingerprint-index-data/golden-acceptance-1/README.md) verifies all 45 committed preparation records, all source and producer bytes, exact commands, complete captures and ten value relations. Its first audit-only attempt rejected differently formatted but field-identical release JSON; the retained second reader checks both byte identities and complete decoded-field equality. No compiler or image reran. The corrected audit takes 0.51 s, load 2.44/1.96/2.33 unchanged.

## One structural transaction

The [four-operation request](compiler-fingerprint-index-data/integration-preparation-1/request.json), SHA-256 `47ceb4ed8e8e7fdc3746537d325ae48da9520142f3845b0527baecfe61d3927c`, is rooted at `src/semantic_main.lkj`. It replaces the fingerprint body, adds the explicit external fixture module beneath `semantic_tests`, registers its four closed groups, and changes `closed_core` from 192 to 196. The source declaration and projected file are one checked transaction.

Root independently decoded all seventeen expected strings and rejoined the complete original dump. All fixture bytes outside `expected(case)` match the frozen source, including the binary and UTF-8 include cases. The fingerprint body payload differs from its frozen brace-delimited source only by the removed terminal LF. A full 1,221-file tracked-language guard and 1,743-file source/library/test guard precede the compiler's own staleness, candidate and publication checks; no item or snapshot digest is guessed.

The actual transaction succeeds once in 36.98 s wall, 35.85 s user /0.33 s system CPU, peak RSS 1,017,712 KiB, load 1.63/1.67/2.11 → 2.03/1.79/2.13. It changes four sites in four files, with five operation targets, and freshly checks all 10,559 instances. Canonical checked meaning changes from `0b770ac6868af0bd5a3647b4bb003af31ba1bafdb45a0934a98d8b402854dc4d` to `cd70ead245f91bf3d1a89e7b4496b8a0fe110272ac337068c5815d280def4b71`. The added fixture entries have empty authority; no existing entry gains authority. [Raw execution](compiler-fingerprint-index-data/integration-execution-1/stdout) and full before/after manifests are retained.

The resulting fixture file is exactly 15,640 B and 301 lines, SHA-256 `ccaad8644eddaab9f4ae344648922ef7e3df032751a732e8923dcdbe1df7d956`. A separate guarded shell edit changes only the closed-runner count from 192 to 196. It earns no language-operation credit. The [independent adoption acceptance](compiler-fingerprint-index-data/integration-acceptance-1/README.md) verifies every operation payload, all 1,743→1,744 source/library/test paths and the broader 1,831→1,832 source/language union. All 1,828 untouched originals are byte-identical. The retained language delta is 331 added plus six deleted lines: **337/337 through operations**. The existing incremental file accounts for 26 added/five deleted lines, semantic_main for one/one, semantic_tests for three added lines, and the new module for 301 added lines.

The final adoption reader passes in 0.47 s, CPU 0.45 s, peak RSS 33,712 KiB, load 4.83/3.78/2.90 unchanged. Four earlier audit-only stops are retained: a Perl declaration error, an after-manifest count that omitted the new file, Git pack mapping under the audit's 1 GiB address-space envelope, and an explicitly documented proposal-file terminal LF. Corrections affect only the offline reader and per-command Git pack windows; no semantic operation, compiler, native image or source edit reruns.

## Gate and prospective measurement

The full empty-store, four-worker gate passes once under the [reviewed wrapper](compiler-fingerprint-index-data/gate-run-1.sh): 600.087 s internal /600.09 s GNU wall, 1,503.92 s user /23.33 s system CPU, peak RSS 1,267,048 KiB, load 1.87/1.77/2.11 → 3.66/3.84/3.09. It executes 5,459 fresh case requests with zero skips, including 196 + 4×134 = 732 structural contracts. The 128-mutant slice reports 104 rejections, 22 values, one trap, one nontermination and zero findings; its 12 checked/five emitted internal reuses are separate from case reuse.

The existing successive-body checkpoint and include invalidation suite passes in 45.084 s (load 6.03→4.79); the ordinary/resumed/full-index query suite passes in 7.963 s (2.73→2.93). These are gate observations, not a new query-latency or comparative speed campaign. Bootstrap passes in 76.788 s (1.87→1.88) and reproduces canonical `a92e81e2d2c18fe1bd645a9b5e287c3a1a2af5eaffa1be16e400289590e7625a`, 32,593,071 B, with the same producer-inline64 policy. The old seed `c7912e43` stays unchanged throughout the gate.

The [independent gate acceptance](compiler-fingerprint-index-data/gate-acceptance-1/README.md) verifies every current byte in the 1,940-input/11,128,526-byte manifest, all 60 source-declared phases, all 119 live/permanent log pairs and all case/statistic totals. The new file was included even while untracked at gate start. The gated source/tool files are subsequently committed, without byte changes, at `31f412a0a7df47d897b6b39e794a9b152b4fa29e`; source tree `59fcc8447761f00edaae5dacefbf5edb9b23df36`. The auditor passes first time in 0.21 s, peak RSS 24,796 KiB, load 4.32/3.53/3.11 unchanged. Phase wall times overlap and are not summed as elapsed wall or CPU.

A separate [seed refresh](compiler-fingerprint-index-data/seed-refresh-1.sh) compares two deterministic gzip streams and one complete expansion, then atomically publishes the already-compared compressed bytes. It takes 4.29 s wall, 4.18 s user /0.02 s system CPU, peak RSS 3,316 KiB, load 3.94/3.64/3.18 → 3.87/3.63/3.18. Seed-only commit `51ea83ac7d439bcc0efbfca84eaf7c39211b43c7` contains the exact 6,552,328-byte gzip, SHA-256 `af0942f672277466f8276e0b673fcb4470e9a61ed32450d51abfb2fc4826d96a`; its committed bytes compare in full with the published file. No temporary seed remains. Actual publication CI is pending.

The [separate prospective stored comparison](compiler-fingerprint-index-data/measurement-preparation-1/README.md) fixes 36 builds on the retained compiler and generated 100k inputs. For each input, A1/B1/B2/A2/A3/B3 each build cold, unchanged and one-body states in a private row store; A is b01, B is a92, and outputs use inline0. The same source path is used by both producers. Every cold/unchanged image must agree in full, each changed image must agree across producers and differ from its original, and the changed state must actually resume frontend definitions. Twelve stores have an explicit 2 GiB cap each, with a conservative 28 GiB initial disk guard. No extra build, warmup, query, profile, specimen execution or larger rung is permitted.

The initial compiler perturbation changed a help String; static review rejected it before setup because String/Bytes payloads participate in topology. The retained replacement changes only the decimal-parser accumulator in the temporary compiler specimen's `driver::inline_limit`, from 0usize to 1usize. The generated specimen changes `edit::target` from xor1 to xor2. Both are same-width integer-expression changes; neither executing producer changes. Complete source pre/post hashes, byte coordinates, include inspection and the rejected proposal are retained. Static eligibility does not replace the required observed resumption. Setup, the concrete harness, CI observation and the execution release remain pending. No workload has run, and no speed, memory, exponent, one-million-line relief or Rust-O milestone follows from the implementation or golden capture alone.

## Exact recovery before cleanup

The [three-root recovery map](compiler-fingerprint-index-data/recovery-1/README.md) covers 1,874 files /46,921,101 B and 57 directories. All 1,872 text/source files directly match immutable Git providers, with no duplicate raw-record retention. One deterministic 315,511-byte gzip retains the actual original golden image and passes a full expansion comparison. Retention takes 0.41 s (load 4.83/3.78/2.90 unchanged), including its sole 0.10 s gzip at the same load.

The map initially leaves the owned baseline producer's seed expansion explicitly pending. A [separate direct comparison](compiler-fingerprint-index-data/baseline-seed-recovery-1.sh) subsequently proves the exact Git `e5294c33` compressed bytes equal the then-current `c7912e43` seed and its complete expansion equals owned b01. It passes in 0.11 s, peak RSS 3,076 KiB, load 2.67/3.42/3.00 unchanged. Both inputs remain byte-identical across that check. No original root has been removed; baseline A remains available until the new comparison owns its producer. Credentials, canonical bytes, gate stores and other roots are outside this recovery scope.
