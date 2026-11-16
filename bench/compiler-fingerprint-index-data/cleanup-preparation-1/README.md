# Fingerprint preparation closure — prospective helper

The helper has not run. Deletion is not released.

## Fixed scope

| Root beneath /home/coder/workspace/lkjscript/target | Files | Logical bytes |
| --- | ---: | ---: |
| compiler-fingerprint-index-preparation-1 | 9 | 39,730 |
| compiler-fingerprint-index-golden-1 | 1,816 | 45,498,446 |
| compiler-fingerprint-index-integration-1 | 49 | 1,382,925 |
| Total | 1,874 | 46,921,101 |

The reviewed map contains 57 directories including these roots. No other target name is accepted. Canonical target/lkjscript (now a92e81…), gate stores, measurement roots, credentials and other worktrees are outside traversal/removal.

## Release and invocation

Root must first commit recovery-1, the separate baseline-seed-recovery-1 records and wrapper, this helper and seed-proof.sha256. Root must establish that the new comparison owns producer A, then fill release.json from release.template.json: status RELEASED_AFTER_COMPARISON_OWNS_A, comparison_owns_producer_a true, the full retention commit and a retained comparison-copy evidence reference. That is a cooperative ownership attestation; the helper does not inspect another comparison root or process state.

Run once from the repository under root's retained 4 GiB / 900 s / 10 s-grace timing wrapper:

```sh
/usr/bin/perl /home/coder/workspace/lkjscript/bench/compiler-fingerprint-index-data/cleanup-preparation-1/guard.pl /home/coder/workspace/lkjscript/bench/compiler-fingerprint-index-data/cleanup-preparation-1/release.json
```

Fixed output bench/compiler-fingerprint-index-data/cleanup-execution-1 must be absent. Exclusive creation prevents reuse of that attempt. The wrapper retains argv, cwd, status, stdout/stderr, GNU time, UTC/nanosecond timestamps and load. No automatic retry.

## Checks and progress

Before the first unlink, the helper byte-compares all 32 recovery-manifest records and all 18 seed-proof records with their exact Git blobs at the release commit. It independently rechecks the 1,872 Git-backed original providers and committed gzip/proof pins for the two binaries. It checks the complete live population, metadata and full SHA-256 bytes of all 1,874 regular single-link files, plus all 57 directory identities, rejecting extra/missing paths, links, specials or drift. All directory descriptors are held. Three root-preflight-complete records precede one all-preflight-complete record.

Only then does it recheck each file and held parent path identity, record unlink-intent, remove the exact enumerated file and record unlink-done. Only listed empty directories are removed, deepest first, with intent/completion records. All three roots must end with ENOENT. Autoflushed progress.jsonl and result.json retain phase, counts and any partial stop.

Metadata coverage is exactly mode, uid/gid, device/inode, link count and file size/hash. The map has no historical nanosecond timestamp fields; no such comparison is claimed. Directory link counts are exact before removal but may change as known children are removed. Each file's identity and full hash are checked again before unlink.

## Recovery and limits

Frozen recovery-1 remains unchanged, including its historical pending baseline-seed wording. Separate root1636 proof resolves that relation: exact e529 Git gzip equals pinned c791 seed; one streaming expansion compared every byte with owned b01. The helper verifies the committed proof and current owned b01 hash. The golden-image gzip has its separate full expansion proof. The helper performs no archive expansion or compression.

Cleanup is cooperative, finite and non-atomic. Failure stops with partial progress and no retry or rollback. Held descriptors and repeated path checks do not guarantee protection against a hostile concurrent rename between the final check and unlink. Exact local Git blobs and accepted recovery hashes establish retention, not new compiler admission. No workload, observer, key read or source edit is involved.

## Preparation record

The first unexecuted draft was c062f3da…. Its sole subsequent normalization explicitly returns an empty string for an empty read. Final helper: d1e6d59747e145a4827bdd148c7ec4dee70168e9e4919f0dec0814ffca7efebe. Perl syntax checking is the only helper-related execution in preparation; it does not run the helper.

One tool-schema refusal occurred before any control-file write: request compiler-fingerprint-cleanup-controls-1500 passed null where before_sha256 required a string. The exact response is preserved in preparation-refusal.json. The corrected request uses the documented absent sentinel; no cleanup execution was attempted.
