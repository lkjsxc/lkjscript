# Independent acceptance of separate setup2

The completed setup2 is accepted as preparation evidence. All five recorded phases completed with command status 0 and capture-validation status 0. This does not release or measure an A/B comparison. The original setup1 cohort remains terminally closed after its first preparation failure; all twelve originally scheduled A/B rows and its four later setup phases are not-run.

## Scope and exact controls

Inputs are the unchanged completed roots `target/compiler-reference-capture-measurement-2` and `target/compiler-reference-capture-setup-launch-2`. The audit read their saved records, actual produced files and relevant Git blobs. It performed no compiler, generator, native-program, setup-helper or comparison invocation; no compression, source extraction, key/store access or mutation of original evidence.

The release is Git `5e1b82f3cf23654e04d0904e948c202c96723c0d`. The five copied control files match that commit exactly; the released controls-manifest SHA-256 is `fe8321b36a88864b33dd3c8bda9a71ab954f9dae14926d3ba9e45fbb564af847`. The outer capture records the exact released invocation, status 0, empty diagnostics and identical before/after control pins. The frozen prospective controls retain their pre-release wording; the separately recorded release authorizes this completed setup.

The auditor checked 3,817 distinct regular input files totaling 81,474,072 logical bytes, recorded their SHA-256 identities, and reread them unchanged before acceptance. The byte total counts separate file instances, including duplicate source/factory and recovery representations; it is not a unique-data or disk-allocation metric. The audit copied 140 bounded original metadata records into `raw/`. Large artifacts remain in their original root and are mapped below.

## Result checks

- The 10,741,760-byte compiler source tar has SHA-256 `79aa24a9eb6f8e7940b4424fc20839759a79dd8d480b1b746f7e29b17171d806`. An independent 512-byte tar parser checked headers, checksums, lengths, zero padding, terminator, path safety and the exact Git global PAX comment. All 1,742 regular members and 42 directories match the complete `src`, `library` and `tests` population at Git `c887b7c5aca7bb80d70ca26d7dfdf1a44213a163`; each payload reproduces its exact Git blob identity. The two path-only PAX records correctly name the long `whole_enum_mutation_invalidates_both_saved_predicates_and_proven_tags_0000.lkjir` and `_0001.lkjir` files. Their raw offsets, placeholders, decoded paths and payload hashes are retained in `source-pax-paths.json`.
- Every live compiler-input file and its private extraction mode agrees with the archive. The complete factory-before inventory agrees too. The final factory has the same population and modes, with exactly one content replacement: `src/main.lkj` is the frozen generator source. The other 1,741 files are unchanged. The saved original main matches the input archive.
- The generator source SHA-256 is `557d6b124fe739f17c678c77b92e5f6259d87ebca69c2a56aca411071c5ff1e9`; shape SHA-256 is `0457f8f9424882d2e3160a568f280193d49b9b046a5fae2d6e2d0b6da047072c`. Both match their exact committed Git artifacts.
- There is one recorded generator build, by actual producer A `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc` (32,584,542 bytes), using the explicit factory source/library/runtime grants, `--inline-limit 0 --no-store --stats`. Its single statistics record terminates at `stopped_stage=done`, reports 1,642,167 emitted bytes and zero front/check/emission/image reuse or inline attempts. Producer A byte-compares with the complete in-memory expansion of the exact committed seed gzip.
- The actual generator is `be23723deaf08b25756a497a674a3455a75415851ebf5ae4a746dc16024e882c` (1,642,167 bytes). Its retained gzip expands byte-for-byte to that executable. There is one recorded generation command with seed 20261116 and target 100000 lines, with the frozen shape as stdin.
- The generated tree contains 169 language files, 100,000 lines and 3,923,070 source bytes, plus the 262-byte `workload.txt`; there are no child directories. The complete generated archive has 170 regular members. Every archive member's bytes and mode match the live tree, and the retained gzip expands exactly to the original tar. The workload artifact, live workload and recorded generation stdout are identical. This establishes this new specimen; it does not claim byte identity with previously cleaned historical specimens.

## Captured setup phases

Each command and its validator have separate retained argv, cwd, stdin, status, stdout/stderr, time, UTC/ns, free-space and load records. All wrappers specify 4 GiB address-space limit, 900 seconds and 10 seconds termination grace. Each recorded free-space preflight exceeds the fixed 3 GiB minimum.

| Phase | Wall s | User s | System s | Peak RSS KiB | Load before → after |
| --- | ---: | ---: | ---: | ---: | --- |
| recover-inputs | 0.59 | 0.51 | 0.07 | 128,740 | 2.54 / 1.86 / 2.30 → same |
| prepare-factory | 0.38 | 0.32 | 0.05 | 63,868 | 2.54 / 1.86 / 2.30 → same |
| generator-build | 2.07 | 2.00 | 0.00 | 118,424 | 2.54 / 1.86 / 2.30 → 2.66 / 1.89 / 2.31 |
| generate-100k | 1.10 | 1.07 | 0.00 | 2,108 | 2.66 / 1.89 / 2.31 → same |
| freeze-artifacts | 0.79 | 0.72 | 0.04 | 68,764 | 2.66 / 1.89 / 2.31 → same |

The outer setup capture is 5.25 s wall, 4.88 s user, 0.25 s system and 128,740 KiB peak RSS; load 2.54 / 1.86 / 2.30 → 2.66 / 1.89 / 2.31. These are preparation observations, not A/B performance data.

## Existing artifact recovery

No duplicate binary or archive was written by this audit. SHA-256 pins and exact existing recovery paths are in `acceptance.json`, `input-pins.json` and the copied `artifacts.sha256`.

| Object | Existing recovery | Verified identity |
| --- | --- | --- |
| Producer A | Git `7d0731e99aeaf241c4334fc5ff065a341fb388c8:seed/lkjscript.gz`; run `artifacts/baseline-A.gz` | gzip `505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2`; full expansion matches A |
| Compiler input | run `artifacts/compiler-source.tar` plus exact Git member identities | tar `79aa24a9eb6f8e7940b4424fc20839759a79dd8d480b1b746f7e29b17171d806` |
| Generator | run `artifacts/generator.gz` | gzip `3b7f282a8fd1c0d373741fe9c81b5a2afcae365888b0c45ea4736fa123015f91`; full expansion matches generator |
| Generated specimen | run `artifacts/generated-100k.tar.gz` and `generated-100k.tar` | gzip `57ead08d82ea53ba4d833b2689cbf1158550931b8cf288aac3363644d2dab07c`; tar `ae6b1704d0796e9c2437c7e4ebefe25995c493fedca7308fe0a2fc6c42857cb4` |

Root retains and commits these original artifacts separately before any later comparison or cleanup release.

## Auditor execution and preserved preparation failure

The first helper, SHA-256 `7745521bcfa7720d258c905aa5a10a02cac50bc1a12859fd48fb4bbc24426b47`, failed the compile-only Perl syntax check because whitespace was missing before a statement-modifier `if`. No input audit executed in that attempt. The exact original is retained as `audit-v1-syntax-failure.pl`, with the original launcher and `captures/syntax-1/`. The syntax failure was 0.02 s, exit 255, RSS 12,116 KiB, load 1.18 / 1.54 / 2.01 unchanged.

The corrected auditor SHA-256 is `172bf51490a9f47b59b96c55a35f25bd5a314b8ee5e324853cd13527d8fcf024`. Its only source change inserts the missing whitespace; no acceptance rule changed. The distinct syntax-2 capture passed in 0.02 s, RSS 12,692 KiB, load 1.46 / 1.42 / 1.88 unchanged. The data audit ran once, status 0, in 1.77 s wall, 1.64 s user and 0.09 s system, RSS 163,228 KiB, load 1.46 / 1.42 / 1.88 → 1.50 / 1.43 / 1.88, under 4 GiB / 300 s / 10 s. Exact argv/cwd/helper-before-and-after/status/stdout/stderr/time/load records are in `captures/audit-2/`.

Gateway run `compiler-reference-capture-setup2-audit-run-1429` and completion `compiler-reference-capture-setup2-audit-poll-1430` refer to the same single audit process. The earlier `compiler-reference-capture-setup2-audit-run-1427` is the preserved syntax-only failure. No original capture was repaired or overwritten.
