# Conditional reference-capture query lifecycle audit

The bounded raw-record audit passed. It verified seven captured build logs, five indexed snapshot records and all 25 retained query replies. The audit consumed 49 named input files and did not invoke a compiler, query command, native executable or official reader. It did not reconstruct the deleted live fixture, read a key/store, or compress anything.

## Producer and capture boundary

The suite's identical before/after input records pin producer `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db` and semantic-query test script `b36bf7c4eed7906d573bd769d26de99a572d6f8f78f0a14bf54c3cec3c930162`. All 25 reply headers independently name that producer.

The outer suite has exit status 0. Each of the seven combined build logs contains exactly one `LKJSCRIPT_STATS` row with `stopped_stage=done`. Separate per-build exit records were not captured. This audit preserves that distinction: the terminal records and the successful suite's declared fail-fast helper support completion; no individual exit code is manufactured. Individual query argv and exit files likewise are absent from this capture.

## Frontend and image lifecycle

| Captured build | Fresh frontend definitions | Reused frontend definitions | Fresh frontend functions | Image reused | Checked proofs reused | Store writes |
|---|---:|---:|---:|---:|---:|---:|
| ordinary | 4 | 0 | 3 | 0 | 0 | 5 |
| from ordinary | 4 | 0 | 3 | 0 | 3 | 7 |
| cold before | 4 | 0 | 3 | 0 | 0 | 9 |
| unchanged full | 0 | 0 | 0 | 1 | 0 | 0 |
| ordinary after body edit | 1 | 3 | 1 | 0 | 2 | 5 |
| full after body edit | 4 | 0 | 3 | 0 | 3 | 7 |
| cold after | 4 | 0 | 3 | 0 | 0 | 9 |

All four rebuilt full indexes report four fresh frontend definitions and no frontend/image reuse. Their semantic catalog/index phase fields are present. This does not imply fresh checking of every function: the two warm full rebuilds legitimately reuse three checked proofs. The ordinary body edit reuses three frontend definitions and freshly elaborates one. The unchanged indexed build returns its image with no fresh frontend definitions or store writes; absent phase fields remain absent.

## Exact replies and source facts

The three before families are byte-identical for each of the five questions; the two after families are also byte-identical for each question. These are 15 complete-reply comparisons, not normalized or selected-line comparisons. Before and after snapshots differ, as do their source and checked-IR identities.

Before snapshot: `4324232149a7e4c86185e161a3eef9730a4b9b4fa514b515d18bced18c432b94`.

After snapshot: `a247c0ba1a1112e55b2889936c5f1ab787ee072be53334e2f7292d5fb4bf6b9c`.

The retained 258-byte source fixtures differ only by `scalar(1)` becoming `scalar(2)`. The before fixture hashes to `89a34bb34ada01c6037427ad869b594861e8a49a0b24ad0a81ebf75cf3d0fd7d`; after hashes to `36492152cbe083c9ed6ff2f943c1f4fb476f8394a0d529a087cdee599d680183`. Token offsets are unchanged.

For every family, the audit checked two global `scalar` references, three local references within `shadowed`, the shadowing binding's `i64` type, two generic local/parameter references, the held generic binding's `T` type, and three complete generic outline signatures. All 35 reference spans select the expected token bytes in the saved fixture and name the same captured file. The edited shadowed-body binding identity changes; the untouched generic held-binding identity stays equal.

The 25 replies contain 60 fact rows and 28,865 bytes in total. The largest reply has 5 facts and 1,680 bytes. All have depth 1, explicit selection and `cursor=none`, within the declared 80-fact / 32,768-byte limits. These are verified output bounds, not reconstructed per-query command lines.

## Proof limits and audit capture

The replies retain their own distinctions: authenticated-producing-compiler validation, current-check not performed, freshness not asserted, recorded-site reference coverage, and demanded-instance coverage. This offline audit checks captured bytes and metadata; it does not authenticate store HMACs or claim current filesystem freshness.

The audit ran once after syntax checking under 4 GiB AS / 300 seconds / 10-second termination grace. It exited 0 in 0.03 seconds, used 9,112 KiB maximum RSS, and observed load 1.18 / 2.44 / 3.25 both before and after. The enclosing original query suite took 8.33 seconds; that is the entire suite and is not an isolated lifecycle latency result.

The original unexecuted audit candidate is retained as `audit-v1-unexecuted.pl`. A self-comparison bug in its final input-stability check was corrected before execution, with the correction retained separately. The final executed helper is SHA-256 `22a296640b98aa8cbded4c7683fa7a59e7ced0d0b87a751420daa7d4d67990ec`.

`acceptance.json` contains all seven parsed stats rows, all reply hashes, exact snapshot identities and scope qualifications. `input-pins.json` binds the 49 original records. `raw-controls/` contains exact copies of the 15 outer controls; the original seven logs, 25 replies and two source fixtures remain at the input path for the parent's full raw retention. `captures/` retains both syntax and audit commands, status, timing, load, diagnostics and helper hashes.
