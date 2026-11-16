# Conditional source-reference capture

Ordinary and declaration-only compilation now omit the two source-reference export vectors. Full indexing and reference-dependent edits select recording before inference. This removes work that has no consumer in an ordinary build while preserving inference, lexical binding origins, closure capture and independently checked IR.

The change does not establish a memory saving or explain the earlier 1M R0008 refusal. Its fixed two-input performance comparison is prepared separately and has not run.

## Contract and implementation

`DefinitionTypes.local_paths` and `semantic_bindings` were populated by initial, demanded and fallback inference even when no full source catalog would be exported. Their only production reader is `DefinitionTypes::semantic`. The implementation guards their three recording writes and leaves the surrounding constraints, binding origins, capture paths, method selection and ownership work intact.

The driver chooses `capture_references = memo.capture_catalog && !memo.catalog_items_only` once, before initial inference. The concrete compiler and generic-verification compiler retain the same mode. Incremental ordinary/declaration-only resumption explicitly uses false. Existing direct frontend APIs retain full capture by default.

| Request | Recording | Checkpoint behavior |
| --- | --- | --- |
| Ordinary check, build or IR | Off | Eligible for existing ordinary frontend resumption |
| Declaration-only comparison or item/body operation | Off | Eligible; reference completeness remains false |
| Full index or reference-dependent edit | On | Re-infer unless a separate authenticated full snapshot is reusable |

A late request to export full evidence from capture-off inference, or a concrete/verifier mode mismatch, returns `E_SEMANTIC_CAPTURE` before taking or changing either caller catalog. Empty vectors never prove a complete set of references. The ordinary image and LKJFRONT2 checkpoint alone cannot satisfy a later full index. The existing full-snapshot authentication requirement remains the gate for unchanged indexed image reuse.

No IR field, store wire format, request-key mode bit, runtime counter, observer or optimization policy was added. Source facts remain frontend claims; checked authority, layout, ownership and loan facts retain their existing IR boundary. [IR contracts](../docs/ir.md) and [structural operations](../docs/structure.md) describe the distinction.

## Preliminary validation and retained refusals

The desired projection was checked and its separate native semantic suite was built by the prior canonical compiler `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`. These runs validate the new source and its executed frontend paths; they are separate from the later candidate producer and full gate.

| Phase | GNU wall s | Peak RSS KiB | Load before → after | Result |
| --- | ---: | ---: | --- | --- |
| Core source check | 19.77 | 678,068 | 1.28/1.64/1.87 → 1.40/1.64/1.87 | Pass |
| Semantic build 1 | 16.89 | 596,556 | 1.42/1.27/1.52 → 2.08/1.43/1.56 | Refused new test forms |
| Semantic build 2 | 16.97 | 585,244 | 1.32/1.90/1.90 → 1.52/1.91/1.90 | Refused new predicate form |
| Semantic build 3 | 18.68 | 583,112 | 1.17/1.64/1.80 → 2.95/1.99/1.91 | Refused inferred unsized test binding |
| Semantic build 4 | 18.22 | 583,164 | 8.02/6.50/3.82 → 6.77/6.30/3.80 | Pass |
| Native closed semantic execution | 61.26 | 89,456 | 6.77/6.30/3.80 → 12.60/8.18/4.62 | 728 contracts, zero failures |

The first refusal found borrowed string views in heap-held test vectors and missing closure context. The second found an unsupported declared predicate signature; the third found an inferred unsized match result in the new assertion helper. The test source was rewritten to use local views, an ordinary borrowed loop, explicit owned binding names and direct Shape equality. No unrelated inference rule or heap-view capability was added. All failed source forms, exact corrections, diagnostics and manifests remain in the [preliminary package](compiler-reference-capture-data/preliminary/README.md).

The seven additional closed-core groups compare exact independently checked binary IR with capture off/on; interpret the fixed result; exercise demanded generic trait/closure selection and all-library validation; preserve diagnostic order and locations; reject late full export before publication; preserve legacy full exports; and force the real deferred-pattern and missing-definition fallback paths. Private assertions inspect both recording vectors without exposing a production counter. The core count is 192 and the unchanged four source partitions contribute 536, totaling 728.

## Actual semantic publication

| Stage | Operations | Sites | Files | Retained .lkj lines | Wall s; load before → after |
| --- | ---: | ---: | ---: | ---: | --- |
| Compiler-root replay | 21 | 20 | 7 | 280 | 66.16; 2.13/4.78/4.02 → 4.05/4.73/4.05 |
| Explicit test-root transaction | 3 | 3 | 3 | 305 | 37.10; 2.58/4.05/3.88 → 2.38/3.85/3.82 |
| Total | 24 | 23 | 10 | 585 | Two sequential transactions |

Core replay independently checks 12,924 instances and reports exact desired/candidate canonical checked-meaning identity `59681227ba0d1ace14608e4363d8901ccf2466c2d6e7e49e19102efe8bb31c16`. This is the established canonical comparison, not a claim that arbitrary serialized IR or all source bytes are equal.

The second transaction uses the real semantic-test root and three explicit operations: create the external module projection, replace the core body, and replace its expected-count body. It independently checks 10,532 instances and reports final checked meaning `0b770ac6868af0bd5a3647b4bb003af31ba1bafdb45a0934a98d8b402854dc4d`. No desired-replay equality is invented for this request.

The [independent adoption audit](compiler-reference-capture-data/adoption/report.md) verifies every baseline Git blob and publication hash chain: 1,742 initial files, 1,743 final files, and 1,733 untouched originals, including all 32 library and 1,526 test files. All 585 retained added/deleted language lines are attributed to the successful operations: 564 additions and 21 deletions. The new module contributes 300 lines. Shell scripts and archives are outside that language-line measure.

Three actual projections differ from desired only in the completely inspected detached comments and blank lines; 1,740 files match exactly. Six desired comment lines were not published and earn no retained-line credit. The raw patches are retained. Core had a successful desired-manifest preflight rather than a separate desired-before copy; the later exact before/after checkpoints remain identified instead of inventing that missing capture. The offline audit takes 0.20 s (load 3.85/3.81/3.77 unchanged), with no compiler execution.

The [committed-source closure](compiler-reference-capture-data/commit-acceptance/report.md) binds all 1,743 adopted hashes to exact Git blobs in source commit `bd8a9050a3786de9f227c6c4037f5acd43205f71`: 9,380,526 bytes, the same ten changed language paths, and 585/585 retained lines. No extra `.lkj` evidence file enters the denominator. The complete 4,146-byte Programs section matches both the baseline and immediate parent, SHA-256 `235aa6c283d334688289ab73e472cae920d25b1e8cbad76cb4461f30d18cc9f4`.

## Candidate producer and query lifecycle

Bootstrap succeeds with two equal current generations and all-library validation. The candidate is 32,588,975 B, SHA-256 `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db`, built at the established producer inline64 policy. Bootstrap takes 79.97 s, peak RSS 1,232,572 KiB, load 1.58/3.39/3.66 → 2.78/3.37/3.63. Ordinary emitted programs retain inline0.

The separate seed-only commit `e5294c339d9aefbbafc2e951c997f4fefabcc9ad` stores deterministic gzip `c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae`, 6,551,428 B. Two independently written gzip outputs compare byte-for-byte; both the prepared expansion and the committed Git blob expand exactly to the validated producer. The [refresh capture](compiler-reference-capture-data/seed/outputs.sha256) takes 4.31 s (load 2.06/2.28/3.04 → 2.29/2.32/3.05). It occurs after the gate, whose before/after seed remains the old `505be947`.

The complete focused query suite passes in 8.33 s, peak RSS 88,668 KiB, load 2.42/3.21/3.56 → 5.64/3.89/3.78. Its new lifecycle case uses one four-definition source, seven builds and 25 bounded pinned replies. Ordinary and one-body resumed builds are followed by full index regeneration; each full result matches its separate cold indexed control exactly. The ordinary body edit resumes three definitions and freshly infers one. Each full regeneration infers all four definitions with no frontend reuse. The unchanged full index reuses its stored image with zero new definitions.

The five repeated question forms cover shadowed call/local references, local i64 type, generic references, generic outline and generic binding type. Before/after cold controls and unchanged reuse agree on complete reply bytes and snapshot identities. The existing authentication, provenance, corruption, missing-source and edit refusal cases remain part of the passing suite. This correctness run is not a new compiler-scale query-latency campaign. The [independent query audit](compiler-reference-capture-data/query-acceptance/report.md) verifies all seven terminal build records, 25 complete replies and 15 byte-equality comparisons. Sixty fact rows occupy 28,865 bytes, with at most five facts/1,680 bytes per reply and no continuation. Two warm full regenerations reuse three checked-IR proofs; fresh frontend inference is not a claim of zero IR proof reuse. Individual build-exit and per-query argv/status files were not separately captured; the retained suite exit and terminal records define that evidence boundary.

The four-worker empty-store gate passes in 611.802 s internally /611.80 s GNU, with 1,543.62 s user and 21.81 s system CPU, peak RSS 1,354,700 KiB, and load 3.45/3.72/3.74 → 3.68/4.14/3.94. All 5,455 case requests are fresh and zero are skipped; the closed structural route executes all 728 contracts. The fixed 128-mutant slice reports 104 rejections, 22 values, one trap, one nontermination and zero findings. Its internal 18 checked/three emitted reuses are separate from case-request reuse. The full source/tool/example/witness input manifest, canonical producer and old seed remain byte-identical across the gate. Candidate CI has not been triggered.

The [independent gate audit](compiler-reference-capture-data/gate-acceptance/README.md) verifies all 60 phases and 1,939 captured input files. One raw archive retains 424 regular files/seven directories/1,201,273 logical bytes, including the complete gate and focused-query captures, every existing gate log and seed-transition metadata; every member matches its original bytes. The audit preserves its initial syntax stop and its subsequent comparison against the intentionally refreshed seed, and then distinguishes the historical gate seed from the later seed commit. Those audit stops did not rerun a gate or a workload. The sum of phase walls overlaps and is not total elapsed time.

## Separate measurement and retention

The [frozen prospective protocol](compiler-reference-capture-data/measurement-preparation/plan.md) compares baseline 6cd with the validated candidate on a new deterministic 100k specimen and exact archived compiler source. Each input has a fixed A1/B1/B2/A2/A3/B3 order, explicit identical source/library/runtime selectors, no store, inline0 and statistics. Setup and measurement remain separate releases. The historical 100k bytes were cleaned; its old manifest is not treated as an archive or substituted for the new specimen.

The first separately released setup stopped before any compiler or generator command: source-archive validation rejected two unresolved Git PAX long-path placeholders. Recover-inputs returned 255 and all four subsequent phases are recorded as not-run. The outer setup took 0.40 s (load 2.07/2.24/2.84 unchanged). The original archive, controls and failure remain unchanged. A corrected preparation must interpret the two preceding per-file PAX path records and retain the same Git content/mode/complete-set checks; no A/B sample exists to repeat or select.

The comparison has no 10% generated-code adoption rule. It will report all observed wall/CPU/RSS values with load and exact output equality, without assigning each byte to the two vectors, claiming statistical significance or diagnosing the 1M refusal. No new rung, exponent or native/Rust-O milestone has been established by this change.

The preliminary recovery archive contains all seven specified preparation/desired/check roots: 1,884 regular files, 50 directories and 42,874,424 logical bytes, including all three failed builds, original source.tar and the actual successful 21,095,338-byte semantic image. Every member was byte-compared once. Packaging takes 1.32 s (load 4.02/3.84/3.78 unchanged); the archive is 7,839,018 B. Its ancillary seal display repeated a consumed digest finalizer, but the stored manifest is correct and root reverified every copied file. No archive rerun or artifact correction was needed.

All source, publication and preparation evidence is retained under compiler-reference-capture-data. Global keys, unrelated operation credentials and live services remain outside these captures.
