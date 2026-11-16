# Prepared cleanup of seven old root-owned scopes

This bundle is a read-only inventory and a proposed guarded cleanup. **No deletion has been executed.** Root must review and commit this bundle, then explicitly release the exact guard and bundle revision. The existing gate, current copy-site work, shared target directory, worktrees and credentials are outside the seven literal scopes.

## Scope and exact inventory

Every scope is an immediate child of `/home/coder/workspace/lkjscript/target`.

| Scope | Regular files | Directories | Logical file bytes | Allocated file bytes |
|---|---:|---:|---:|---:|
| compiler-origin-container-gate.uQZd6X | 16,173 | 7 | 364,308,624 | 415,346,688 |
| compiler-container-root-audit | 10 | 1 | 26,555 | 57,344 |
| compiler-generated-source-root-audit | 6 | 1 | 2,607 | 24,576 |
| compiler-strict-scoped-root-audit | 124 | 2 | 1,264,482 | 1,671,168 |
| compiler-current50-root-audit | 10 | 1 | 49,028 | 77,824 |
| compiler-strict-derived-root-audit | 34 | 2 | 221,789 | 335,872 |
| compiler-scale-refusal-root-audit | 8 | 1 | 3,674 | 28,672 |
| **Total** | **16,365** | **15** | **365,876,759** | **417,542,144** |

`full-inventory.tsv` records every exact relative path, type, mode, UID/GID, link count, size, allocated blocks, device/inode, modification/change time and regular-file SHA-256. Its SHA-256 is `bda3ce9d26ec494ba97779315bd1d731cb35ecf97c326f80ce6e58d542d69b8e`. All discovered entries are regular files or directories; every regular file has one link. The complete hash pass matched the initial path/metadata census. No process or FD scan was performed.

Allocated bytes are the observed regular-file block counts, not a promise about eventual free-space change. Directory metadata is inventoried separately.

## Recovery and deliberate disposal

The retained evidence mapping is pinned to main `f9e4581ae815ba041510d31421e6b588d66b4059`, even though later main preparation commits exist. The four disjoint file classes account for the whole inventory:

| Class | Files | Bytes | Recovery |
|---|---:|---:|---|
| Existing committed audit/log/control bytes | 309 | 1,882,030 | Exact Git blob contents, independently checked by SHA-256 |
| Old canonical and two gzip seeds | 3 | 45,128,972 | Exact historical Git seed, with decompression for the canonical |
| Previously uncommitted evidence/control bytes | 4 | 46,607 | Newly copied into this bundle; deletion remains blocked until committed |
| Disposable gate store | 16,049 | 318,819,150 | **Payload bytes are not archived and are not claimed recoverable** |

`retained-counterparts-verified.json` names the exact Git reference for all 309 matches. The principal counterpart families are `compiler-origin-container-integration-data/local`, `compiler-query-container-data/root-acceptance`, `compiler-generated-source-data/root-acceptance`, the original strict-scoped/strict-derived result tables and logs, `compiler-query-current-root-audit-data`, and `compiler-scale-refusal-root-audit-data`. Some identical short/empty values have more than one committed copy. The verification compares bytes, not merely a similar filename. The family-ranking hints in the preparation summary are not an additional proof claim.

The old canonical is 32,196,802 bytes with SHA-256 `34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377`. Reading and decompressing `72a259e8889cff6e7c4ac712d7940b77343a9f5b:seed/lkjscript.gz` reproduced exactly that length and digest without executing it. The compressed Git blob is 6,466,085 bytes, SHA-256 `0f2b0b0f0d62dc31e625eea80cfb37e317f007f6fcb4a7e8ecf6ef5249d51522`, matching both live seed copies.

The four formerly uncommitted files are the eight-byte old gate PID marker and three current50 landing transcripts. All 533 transcript lines are Git commit headers, dates, change summaries or `bench/` create-mode paths. They contain no private key contents, owner inbox or unrelated payload. The exact transcript hashes and classification are in `transcript-review-v2.json`. The new retention set also includes the 32-byte repository-path marker and the empty start marker, for coherent ownership context: six copied files, 46,639 bytes, with the two already recoverable markers counted only once in the table above.

The store consists only of known `front`, `check`, `native`, `image`, input/test/gate proof namespaces and budget metadata; its namespace counts are in `full-inventory-summary.json`. Its hashes identify the discarded state. They do not retain that state or claim HMAC validation. The pinned direction explicitly makes `--store` a disposable Directory and grants its private key separately. Rebuilding can repopulate useful cache state; that is not an exact-byte recovery promise. An execution release of this proposal therefore authorizes intentional loss of those cache payloads, while preserving all gate/result evidence.

## Credentials and excluded state

The retained old gate metadata names the global store key only as a path. The credential itself was not opened. Neither `/home/coder/.local/state/lkjscript/store-key` nor `/tmp/compiler-container-key.m4u6F6` is inside any cleanup scope. The inventory contains no source-edit pending/staging metadata or credential-like filenames. The repository marker was checked as the literal repository path, and the PID marker as decimal digits plus a newline; no PID lookup was performed.

Everything outside the seven literal roots remains excluded, including `target/gate`, every current copy-sites directory, the shared target and all worktrees. Root's known completed commands and explicit ownership establish the later release; this inventory makes no independent process-liveness assertion.

## Guard and release protocol

The proposed committed bundle path is `bench/compiler-legacy-root-cleanup-data`. After root verifies, commits and explicitly releases it, the single invocation is:

```sh
perl guard.pl --execute-after-root-release EXACT_COMMITTED_BUNDLE_REVISION
```

Run it inside a wrapper retaining stdout/stderr/status, UTC, elapsed time and before/after load. The guard itself has only been parsed with `perl -c`; it has **not** been invoked.

The guard verifies its own committed bytes and every bundle manifest entry, all 309 original counterparts, the six retention copies and exact historical seed recovery. It then checks the complete live path set, types, metadata and hashes before any unlink, checks paths again, and rechecks each file immediately before removing it. Extra entries, missing files, changed metadata or hashes refuse. Removal uses only explicit inventoried regular-file unlinks followed by empty-directory removals. There is no recursive sweep, broadened exclusion, alternate route or automatic retry. New entries appearing after preflight are never added to the deletion set and prevent the relevant directory removal.

This cleanup is not a transaction against concurrent external writers. Root must release it only after the known owned commands have finished. A late refusal can leave already-removed approved files absent; the result records the completed count and error, and the guard stops.

The guard records its single result outside the deleted scopes at `/home/coder/workspace/lkjscript-compiler-copy-sites-native-20261116/target/compiler-legacy-root-inventory-20261116/cleanup-results/result.json`. An existing result directory prevents a repeat. No process observer, private-key input, compiler invocation, repository mutation or shared-document update is part of the guard.

## Preparation qualifications

`preparation-attempts.json` retains the preliminary truncated listing, a Perl verifier syntax refusal, and an invalid preliminary transcript-summary helper result. The corrected verification and transcript classification are explicitly identified; none of those preparation issues modified the seven source scopes. Packaging emitted one warning while copying an empty stderr capture; subsequent bundle-copy verification checks the exact empty bytes. These are preparation records, not compiler tests or measurement results.
