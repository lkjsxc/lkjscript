# Completed reference-capture roots: recovery and finite cleanup proposal

Preparation only. No deletion is authorized or implemented. The fixed recovery commit is `e68ea0eb29b755cb48df5034c57fb226a1f7c3c6`; existing setup2 recovery remains the committed representation accepted at `1404b4af81b2afa979993b0c426921321a6801f0`.

## Exact coverage

The offline mapper compared all 3,056 regular files, totaling 160,438,247 logical bytes across 30 older literal roots, with named committed byte recoveries. All matched. There were no unresolved files, symlinks, special files, hardlinks or ownership exceptions. No original file was changed. The mapper did not traverse the four protected comparison/dependency roots.

`compiler-reference-capture-recovery-map-1.json` retains the exact file paths, sizes, SHA-256 hashes, modes, owner/link/inode identities, directory population, and recovery-catalog index for every file. The catalog names a committed Git blob, an exact member of a committed archive, or a complete expansion of a committed seed gzip. Shared equal-byte records are explicitly named; the mapping preserves the original path and does not conflate historical events.

| Older root suffix | Files | Logical bytes | State |
| --- | ---: | ---: | --- |
| adoption-acceptance-1 | 37 | 725,828 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| bootstrap-1 | 18 | 426,653 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| cache-cleanup-1 | 20 | 3,059,967 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| cache-cleanup-acceptance-1 | 1 | 5,570 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| ci-1 | 35 | 167,625 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| commit-acceptance-1 | 56 | 503,192 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| core-check-1 | 18 | 3,419 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| core-operation-1 | 19 | 434,465 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| desired-1 | 1743 | 9,381,010 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| gate-1 | 143 | 658,948 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| gate-acceptance-1 | 60 | 1,180,116 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| integration-preparation-1 | 42 | 342,636 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| measurement-1 | 46 | 50,005,332 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| measurement-preparation-1 | 6 | 19,600 | DEPENDENCY/KEEP |
| preliminary-retention-1 | 34 | 10,123,821 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| preparation-1 | 42 | 11,106,447 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| query-1 | 140 | 188,007 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| query-acceptance-1 | 56 | 112,697 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| seed-1 | 25 | 45,694,488 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| semantics-1 | 15 | 215,788 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| semantics-2 | 15 | 215,247 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| semantics-3 | 18 | 428,202 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| semantics-4 | 33 | 21,524,311 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| setup-acceptance-1 | 102 | 137,661 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| setup-acceptance-2 | 205 | 1,478,741 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| setup-launch-1 | 15 | 1,020 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| setup-launch-2 | 15 | 1,006 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| setup-preparation-1 | 30 | 80,367 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| setup-preparation-2 | 43 | 1,356,954 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |
| tests-operation-1 | 24 | 859,129 | RECOVERY-VERIFIED/NO-DELETION-RELEASE |

## Recovery sources

- Direct committed copies under `bench/compiler-reference-capture-data` cover operation/adoption/query/commit/bootstrap/gate/CI/setup/cleanup records. Completed CI is included from the pinned e68 commit.
- The already committed `preliminary/preliminary-roots.tar.gz` covers all seven original preparation/desired/core-check/semantics roots. It was decoded in memory and members compared; no extraction or new archive was written.
- The committed gate `raw-logs.tar.gz` and named direct blobs supply exact capture recoveries, including duplicate raw values where bytes match.
- Closed failed setup1 retains its failure boundary and uses existing committed copies for input artifacts. Its copied A executable and gzip recover through `7d0731e99aeaf241c4334fc5ff065a341fb388c8:seed/lkjscript.gz`.
- Both seed-1 `candidate.gz` and `repeated.gz` byte-compare to `e5294c339d9aefbbafc2e951c997f4fefabcc9ad:seed/lkjscript.gz`, SHA-256 `c7912e43e0bbb26f814008a6616f3e8b2e4dbe09b45166e5b9523ce0461018ae`. Its complete expansion byte-compares to `expanded`, SHA-256 `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db` (32,588,975 bytes). These are the three formerly unmapped extras; no new gzip was produced.

Recovery kinds by original file: committed-git-blob=946, committed-gzip-expansion=2, committed-gzip-tar-member=2108.

## Protected pending roots

`comparison-2`, `comparison-launch-2`, `measurement-2` and `trial-preparation-2` remain ACTIVE/KEEP and were not traversed. Their separate result/archive acceptance must be committed before a later closure. `measurement-preparation-1` has exact recovery but remains DEPENDENCY/KEEP because the trial controls name its plan. No unknown prefix root is admitted automatically.

Accordingly, only 29 of the mapped roots (3,050 files / 160,418,647 logical bytes) are recovery-complete without that dependency hold. This is a proposal boundary, not permission to remove them.

## Required finite guard before any deletion

1. Bind this map, mapper and pinned commit by their retained hashes. Require a separate explicit release naming the exact permitted literal roots; refuse active/dependency/unknown roots.
2. Before the first unlink, enumerate each permitted root without following links. Require the exact recorded file and directory populations, regular-file/owner/link-count and held root identities, all stored metadata, SHA-256 bytes and every named committed recovery. Refuse extra, missing, changed, unsafe or unmapped entries. Recheck the whole set before any mutation.
3. After successful complete preflight, remove only the enumerated regular paths, then the enumerated empty directories deepest-first. No recursive blanket removal, prefix expansion or generic cleanup tool is proposed. Preserve a per-step progress capture and stop on any failure.
4. Require explicit ENOENT for every completed root. Deletion across roots is non-atomic; a partial stop must be reported with the exact removed/remaining set and preserved recovery. Never continue by widening the scope.
5. Canonical `target/lkjscript`, `target/gate`, global/foreign credentials, other target prefixes and worktrees are excluded. The mapper read no credential contents; only the three known test-source basenames containing key words were read as archived source files.

## Execution boundary and evidence

The mapper ran once under 4 GiB / 300 s / 10 s: exit 0, 1.01 s wall, 0.81 s user, 0.12 s system, 236,296 KiB peak RSS; load 0.90 / 1.36 / 1.96 unchanged. The preceding syntax check also passed. There was no workload, compiler, native program, process/FD observer, compression, source extraction or deletion.

Gateway launch `compiler-reference-capture-recovery-run-1443` and completion `compiler-reference-capture-recovery-poll-1444` refer to this one mapping execution. The run JSON preserves argv, cwd, UTC/nanoseconds, load, stdout/stderr, status and helper hash; GNU time is retained separately. The proof manifest binds the mapper, exact map, captures, this proposal and the earlier metadata inventory.
