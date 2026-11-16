# Conditional reference capture: preliminary recovery package

This package retains exactly seven completed, frozen preparation/proof roots. It does not include the concurrent core semantic operation, any later bootstrap/gate result, or unrelated main/worktree files.

## Scope

| Original path below repository root | Regular files | Directories (root included) | Logical bytes |
| --- | ---: | ---: | ---: |
| target/compiler-reference-capture-preparation-1 | 42 | 2 | 11,106,447 |
| target/compiler-reference-capture-desired-1 | 1,743 | 43 | 9,381,010 |
| target/compiler-reference-capture-core-check-1 | 18 | 1 | 3,419 |
| target/compiler-reference-capture-semantics-1 | 15 | 1 | 215,788 |
| target/compiler-reference-capture-semantics-2 | 15 | 1 | 215,247 |
| target/compiler-reference-capture-semantics-3 | 18 | 1 | 428,202 |
| target/compiler-reference-capture-semantics-4 | 33 | 1 | 21,524,311 |
| Total | 1,884 | 50 | 42,874,424 |

The sole archive is `preliminary-roots.tar.gz`, 7,839,018 bytes, SHA-256 `35ced5f584174c20a884dd3feeb988c188bf4350d0276831a6c52ac15e7088bc`. Each original path is the identical archive member name. `verification.json` supplies the complete path/member, byte-size, digest, and full byte-equality mapping; `inventory.json` preserves every original file/directory identity and metadata. `original-files.sha256` lists all 1,884 regular files.

## Verification and bounds

One tar/gzip pipeline created the archive. Tar member order is byte-sorted, uid/gid/mtime are normalized to zero, original modes are retained, and gzip records neither filename nor timestamp. The archive has 1,934 members with no duplicate, missing, extra, symlink, special, or multiply linked original file. Every regular member was compared byte-for-byte against its original exactly once during verification. The original complete path set and metadata were rechecked afterward. No archive member was extracted or executed, no second compression was performed, and no extra image copy or per-image gzip was made.

The one released package command completed with exit 0 in **1.32 s wall / 1.31 s CPU / 137,708 KiB peak RSS**, load **4.02/3.84/3.78 → 4.02/3.84/3.78**, at 2026-11-16 17:38:53–17:38:54 UTC. It ran under a 4 GiB address-space limit, 900 s watchdog and 10 s termination grace, with one compression worker. Exact launch argv, UTC/nanoseconds, GNU time, load, complete output, syntax result and component exits are retained. Tar and gzip both exited 0 with empty stderr. Gateway launch/poll requests are `compiler-reference-capture-retention-once-293` and `compiler-reference-capture-retention-poll-294`.

## Preliminary evidence retained

All three failed preliminary builds remain unchanged, including complete diagnostics, exact replacement/fix provenance and their controls. Their build exits are 1 and no native run followed those failed builds. The fourth build and native run both exited 0; its raw stderr is exactly `closed semantics 728 contracts, 0 failures\n`, with empty stdout. `controls.json` preserves that literal evidence and the timing/load controls; it is a retention summary, not a fresh semantic execution.

The actual fourth-attempt native image is included once at `target/compiler-reference-capture-semantics-4/semantics`: 21,095,338 bytes, SHA-256 `9da231621d686f79a870feee8d1ea04fe59da282783dc446dd527d9d34f7bac9`. Its captured before/after source-manifest bytes are identical and hash to `e34cdb687a9e7195e686f38bf53a6f55c1b38f7f1769f60605ba36361b6119b7`. The original preparation `source.tar` and all 42 preparation files, including `documentation-projection.json` and `host-tools-integration.json`, are retained. The final desired tree is inventoried as it existed at packaging; historical attempt manifests remain historical evidence.

No compiler, native executable, generator, profiler, test or observer ran during packaging. No source, original evidence, bench path, key, service or other worktree was changed or deleted. This recovery package is not a cleanup release.
