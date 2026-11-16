# Conditional reference-capture adoption audit

Result: the two successful semantic publications account for **585 / 585 retained .lkj added+deleted lines** from language baseline 0b0b27803d27b932a5b91cde2d766bd3c8bb82b9. No uncovered source delta was found. This is source-publication attribution, not a new compiler/test execution or performance result.

## Exact source population

All **1,742** initial files under src/library/tests were independently checked against their exact baseline Git blobs. The core after manifest equals the test transaction before manifest byte-for-byte. All **1,743** final files match both the publication captures and current bytes. There are nine changed existing files and one new file; **1,733** original files remain unchanged.

| Scope | Baseline | Final | Unchanged originals |
| --- | ---: | ---: | ---: |
| src | 184 | 185 | 175 |
| library | 32 | 32 | 32 |
| tests | 1,526 | 1,526 | 1,526 |
| Total | 1,742 | 1,743 | 1,733 |

The complete desired population also matches its frozen 1,743-record manifest by path and digest. No source file, private key or credential was written by this audit.

## Transactions and retained lines

| Stage | Operations | Source sites | Files | Retained added+deleted lines |
| --- | ---: | ---: | ---: | ---: |
| Core replay | 21 | 20 | 7 | 280 |
| Explicit semantic-test request | 3 | 3 | 3 | 305 |
| Total | 24 | 23 | 10 | 585 |

The final delta is **564 additions and 21 deletions**. The test stage comprises the exact 300-line new module, three registration lines, and the two-line added/deleted count-body replacement. The core total includes the private assertion items as published by the same semantic replay.

Every reported file-before/file-after hash agrees with the captured inventories and actual bytes. The explicit request is exactly 3721803e8ed288cbe6687f35cee818f6bf403310bac9c234adf1eeb650acbd37, with the three declared identities and full module projection. Both actual argv vectors match the prepared commands exactly.

Core replay reports desired=candidate checked meaning 59681227ba0d1ace14608e4363d8901ccf2466c2d6e7e49e19102efe8bb31c16, verified=true, and 12,924 fresh checks. The explicit test transaction reports 10,532 fresh checks and final checked meaning 0b770ac6868af0bd5a3647b4bb003af31ba1bafdb45a0934a98d8b402854dc4d. It used no replay target; no invented desired-equality marker is attributed to it. Both report mode=apply, proof-reused=0 and frontend-resumed=0.

The source/library and all-retained-.lkj denominators are both 585 for this delta. The two host shell-tool edits are separate and add no .lkj lines. No refused/transient desired edits, whole-item source lengths, JSON/report/Rust/archive bytes or repeated validation copies are counted.

## Desired projection discrepancies: complete bounded review

**1,740 files match desired exactly.** All hunks of the remaining three patches were read:

- src/elaborate.lkj: replay omitted two detached documentation-comment lines before infer_resolved_report_with_capture and one detached assertion-probe comment. Its other differences are blank-line placement.
- src/elaborate/definition_types.lkj: replay omitted one detached recording-probe comment and the two detached deferred-subject comments.
- src/semantic_tests.lkj: one added blank line before mod reference_capture.

No code token or item-order difference occurs in these exact hunks. The six desired comment lines were not published and are not credited as retained lines. The three complete raw patches and their hashes are retained; no general whitespace/comment normalization rule is used as a semantic proof. The new module bytes and closed_core count192 match desired exactly.

## Producer and evidence limits

Both operations independently recorded the exact canonical 6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc before and after execution. The audit deliberately did not read the current executable: root bootstrap was authorized to replace that path. No new claim about the later bootstrap producer is made.

The core wrapper used a successful desired-manifest preflight rather than a separate desired-before file. This audit retains that qualification. The frozen preliminary manifest and the test operation's exact desired-before/after records provide the retained checkpoints around core publication; the audit does not invent a missing capture.

The audit verifies working source after publication. Root must compare these same hashes to the eventual committed source blobs; a future source commit is not asserted here. The preliminary desired closed-suite pass and future integrated gates remain separate execution evidence.

## Audit execution

One bounded offline audit completed with exit0 in **0.20 s**, peak **19,724 KiB**, load **3.85/3.81/3.77 unchanged**, under 4 GiB AS / 300 s / 10 s grace. The helper was syntax-checked before that run. No compiler, native program, profiler or official semantic reader ran.

acceptance.json is the immutable machine audit; projection-review.json records the subsequent complete hunk review. input-pins.json, baseline/live manifests, exact command records, raw stdout/stderr/time/status and all discrepancies are retained. One metadata inspection requested the nonexistent generic test numstat filename; its harmless missing-file diagnostic is retained separately. The actual audit independently recomputed numstat and used no missing record.
