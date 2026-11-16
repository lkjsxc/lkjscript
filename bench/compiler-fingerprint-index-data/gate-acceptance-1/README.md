# Fingerprint index: independent gate acceptance

Accepted the completed `gate-1` run through one offline Perl audit, with no compiler or test rerun. The raw gate and copied logs remain in their existing permanent location; this package adds the reader, its capture, derived phase table, log-copy proof and acceptance record.

## Result

The source-declared roster contains exactly 60 phases. Every phase has its retained detail log; each of the 54 parallel summaries exactly matches its top-level stdout record. All 119 listed live logs are byte-identical to their permanent copies. All are bound to this roster or its five structural runners; there are no extra copied logs being counted as fresh evidence.

The 60 JSON statistics records and 26 closed-runner records reconstruct 5,459 fresh case requests and zero skipped requests. The structural runners report 196 + 134 + 134 + 134 + 134 = 732 contracts. The fixed seed-1701 mutation summary reports 128 mutants: 104 rejected, 22 values, one trap, one nontermination, zero resource outcomes and zero findings. Its 12 checked and five emitted internal reuses are distinct from skipped gate case requests.

Gate wall time is 600.087 s internally, 600.09 s by GNU time, and 600.091569160 s from the outer nanosecond capture. GNU user/system CPU sums to 1,527.25 s; peak RSS is 1,267,048 KiB. Load was 1.87/1.77/2.11 → 3.66/3.84/3.09. The overlapping phase durations sum to 875.374 s, which is not total wall or CPU. Their rounded interval union is 600.032 s, leaving 0.055 s outside those printed intervals.

## Identity and completeness

The before/after input manifests are byte-identical, SHA-256 `7182979b5951f113d95217addf0fb9500f10b0ad06e5199401916950e1563baa`. The audit independently matched the complete current Git cached/other path set and every current file hash: 1,940 inputs, 11,128,526 bytes. This includes 186 source, 32 library, 1,526 test, 118 tool, 67 example, ten witness and one workflow file.

The captured working base was `88a06df2b212d5856b62aa9bcb49e9092cad1d10`. The five gated source/tool files were subsequently committed as `31f412a0a7df47d897b6b39e794a9b152b4fa29e` without changing their bytes. The reviewed implementation, test module and gate scripts also match their exact committed blobs. The new module was included in both captured input manifests even though it was untracked at gate start.

The pinned bootstrap script successfully compared its generated images and intentionally replaced canonical b01 with `a92e81e2d2c18fe1bd645a9b5e287c3a1a2af5eaffa1be16e400289590e7625a`, 32,593,071 bytes. The audit hashed the current image and matched the captured after value. The fixed-point claim rests on the retained successful bootstrap comparison, not a new execution by this audit. Seed `c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae`, 6,551,428 bytes, stayed unchanged and matches the exact compressed Git blob at `e5294c339d9aefbbafc2e951c997f4fefabcc9ad`. No decompression was performed.

## Audit capture

The reader and launcher passed syntax and execution on the first attempt. Gateway request `compiler-fingerprint-gate-independent-audit-374` completed with exit 0. Audit GNU wall/CPU were 0.21/0.20 s, peak RSS 24,796 KiB, load 4.32/3.53/3.11 → 4.32/3.53/3.11. Its exact argv, UTC/nanosecond clocks, loads, stdout, stderr and exit are retained here.

- `acceptance.json`: full derived result and qualifications.
- `phases.tsv`: all 60 phase times, loads and reconstructed counts.
- `log-copy-proof.json`: complete 119-file log correspondence.
- `read-pins.json`: exact raw records and reviewed source read by the auditor.
- `manifest.sha256`: every package file except the manifest itself.

No source or raw evidence was modified. No archive, compiler, native executable, test, official reader, process/FD observer, store/cache read, credential read, cleanup or Git write was performed.
