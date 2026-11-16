# Scalar evidence retention proposal

Status: prospective only. The cold gate owns all four compiler workers. This proposal does not authorize deletion, create an archive, compress a binary, run a compiler, or inspect a cache/key. Root executes retention only after the gate drains.

## Fixed identities and observed scope

Source commit: 5b5ce7759c035ae0437e3512e18efc440aa50d2e.
Its direct parent/base: e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94.
Source tree: 6277f3cec86791e7f297a95ff0a70bed60b951eb.
Library tree: 25bdb8ff8567b941790d3951c78e896b23e26e8e.
Tests tree in both commits: e2b30ac0f2fa8f897f366293189d6af552a41980.

The inventory covers only the following 13 closed directories below
/home/coder/workspace/lkjscript-compiler-scalar-20261116/target.
It records every regular file; no symlink/special file was accepted.
The two bulky archive/native-image members were statted only, not read.

| Closed scope | Files | Logical bytes | Proposed treatment |
| --- | ---: | ---: | --- |
| compiler-scalar-proposals | 94 | 6,332,308 | Keep all 93 records, including original failed drafts and nested syntax/mutation records; preserve the input tar separately |
| compiler-scalar-edit-core-1 | 10 | 1,170 | Keep initial E_TRAIT refusal |
| compiler-scalar-core-check-v2 | 17 | 2,091 | Keep corrected ordinary source-check success |
| compiler-scalar-edit-core-2 | 17 | 74,036 | Keep successful dry-run, distinct from publication |
| compiler-scalar-tests-check-v3 | 21 | 86,242 | Keep incomplete desired-tree setup refusal, including missing-include diagnostics |
| compiler-scalar-edit-core-apply-1 | 20 | 54,985 | Keep successful 15-operation publication |
| compiler-scalar-tests-check-v3-complete | 21 | 466,110 | Keep complete-tree ordinary source check |
| compiler-scalar-edit-tests-apply-1 | 20 | 853,382 | Keep E_REPLAY_FILES refusal and unchanged manifests |
| compiler-scalar-edit-tests-explicit-1 | 19 | 477,975 | Keep successful explicit three-operation publication and exact request |
| compiler-scalar-live-preliminary-1 | 43 | 21,687,858 | Keep 42 records; preserve its native image separately |
| compiler-scalar-edit-mutation-apply-1 | 20 | 857,856 | Keep successful 12-operation replay and full desired/source manifests |
| compiler-scalar-bootstrap-1 | 20 | 41,650 | Keep factory source/seed/argv and producer identity evidence |
| compiler-scalar-focused-1 | 66 | 23,223 | Keep all three focused command captures and aggregate manifest |
| **Total** | **388** | **30,958,886** | **386 records = 4,331,830 B; two bulky members = 26,627,056 B** |

closed-inventory.json is the complete per-file list with size, mode, inode metadata, and hashes for the 386 records. Its SHA-256 is bb18dc804c6591917118d4184c4a87a6f78b9fba5a48899d64709d39443ec8b8.
closed-records.sha256 pins those 386 exact paths/bytes; SHA-256 b985b80cf81d34fa5c8aa7886debea6cfb7d9adad1cc21d4a532de5071f02bca.
argv.nul and similar captures are raw records, not assumed UTF-8 prose.

## Small retained package after the gate

1. Retain all 386 records without filtering successful/failed status. One deterministic compressed archive is sufficient, using only the explicit inventory paths, no recursive target traversal. Preserve original member names, including the duplicate test_generators.lkj proposal. This avoids another loose source tree while retaining exact proposal bytes; it earns no additional adoption credit. Store the per-member SHA-256 inventory next to it and verify every decompressed member against that inventory.
2. Keep this proposal/inventory and both independent adoption-audit JSON files. The audits are already members of the 386-record scope. Preserve source hashes, nonblank-only desired differences, and the distinction between 12 mutation operations and 11 sites.
3. Export the saved gateway-only failed auditor result compiler-scalar-core-audit-1309 (exit 255; Perl compilation error before file reads), plus successful auditor compiler-scalar-publications-audit-1320. Their stdout/stderr/result metadata are small additional records; they must not be silently represented as compiler failures. Recovery paths are in retention.json. The new inventory/planning commands can be kept as documentary records; no workload is involved.
4. Preserve the final desired-tree difference with an overlay of exactly three files: desired src/driver.lkj, src/reuse.lkj, and src/semantic_tests/optimizer.lkj. The complete final desired manifest already survives in the mutation capture. The prior independent audit proved all other 1,741 paths equal actual source bytes. Copy/hash the three overlay files after the gate, and recheck their listed expected hashes before relying on this recovery. Do not claim the overlay is already archived.
5. Retain the exact source commit in the thin Git bundle described below.
6. Keep the current gate evidence separate. No active gate file is in this proposal; root can append a distinct closed-gate package later.

## Exact source recovery through Git

5b5ce775 is one commit directly above e0b21e66. A thin bundle containing its new commit/tree/blob objects is the smallest exact source recovery, provided the full e0 base objects remain available.

After the gate, root can create a uniquely named temporary refs/retention/compiler-scalar-5b5ce775 ref using update-ref's expected-absent check, pointing to the exact full source commit. Create a bundle containing that ref and excluding ^e0b21e66659dc1830a5bebddf0c1f30fd2d8fa94. The worktree is detached, so do not assume a branch name.

Retain:
- the bundle's SHA-256 and byte count;
- git bundle verify and list-heads outputs;
- exact advertised ref/commit and the explicit e0 prerequisite;
- source/library/tests/tools/seed object pins and the final source inventory.

Remove only that temporary ref using expected-value compare-and-delete, after bundle verification. This is future root work, not performed here. Restoring requires a repository with the e0 prerequisite; this is not described as a standalone all-history backup. If the prerequisite cannot be guaranteed, root must retain a separate base bundle or exact base archive before any source-copy disposal.

The commit already contains all nine changed .lkj paths, the integrated tool hooks and tests. Its tracked source remains the truth; operation requests/logs establish how those bytes were published. The source bundle does not need to duplicate desired source except the three blank-line overlay files.

## Later binary/archive choices

Use gzip -n with a fixed level (recommend -9 after gate), record compressed and original SHA-256/byte counts, and verify decompression matches the original before disposal. No compressed-size estimate is asserted.

| Object | Original identity / bytes | Choice |
| --- | --- | --- |
| target/lkjscript | f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64 / 32,761,734 B | Preserve one content-addressed gzip after gate; this is the focused/gate producer. Path is excluded from the current inventory/read and must be revalidated then. Do not replace the active executable or main seed. |
| compiler-scalar-live-preliminary-1/live | fff89090526d3c9f0ed6081f57bd54121e51773912b833bff62db31e531a1b85 / 21,251,056 B | Preserve one gzip; label as preliminary old-emitter semantic suite, not final candidate validation. Hash is from retained before/after records, not a new binary read. |
| compiler-scalar-proposals/desired-tests-input.tar | 556b02937776b789adc51e04127ba4ff919400c8be047509970f28418f47e156 / 5,376,000 B | Preserve gzip of the exact tar bytes. e0 tests tree recovers extracted source, but gzip also retains the original preparation archive without inventing a byte-identical regeneration. |
| Existing bootstrap seed | Git blob bdd99cabbb0a2154257c9a536eb002be8fa32a1f; gzip SHA-256 505be947525d53b4db1bff6c85cb77622cb2f5dc7e7d7fe92b772ee3b8ffc3e2 | Reuse exact committed base/seed artifact; no second decompressed producer copy needed. Do not read or mutate the main executable/key. |

The thin bundle is already packed; it needs no additional gzip. Duplicate canonical executables elsewhere should be retained once by content identity only after their equality is actually checked. No equality between the current active gate's private images and target/lkjscript is inferred here.

## Recovery and disposal boundaries

After root has committed and independently verified the retained records, archive members, source bundle, three-file desired overlay, and chosen gzips, the 13 closed scopes can be considered for a later explicit guarded cleanup. Re-list each exact scope, reject changed/new/nonregular leaves, verify recovery for every retained member, then remove only that enumerated scope. This proposal performs no cleanup and is not a deletion release.

The complete compiler-scalar-desired copy is a future disposable duplicate only after its final manifest and three-file overlay match the recovered 5b source. It was not traversed in this inventory. The signed compiler-scalar-bootstrap-store-1 cache is optional regeneration data, not unique source evidence; its deletion requires a separate root-owned scope/closed-state check. Neither directory is included in the 388-file cleanup candidate.

Explicitly excluded from reads, archives, and any proposed cleanup here:
target/gate; target/compiler-scalar-cold-store-1; target/compiler-scalar-cold-gate-1; target/bootstrap.nG4W90; all keys/credentials and durable .lkjscript-edit.* metadata; main worktree; unrelated worktrees. target/lkjscript is only a later binary-retention candidate and remains untouched while gate runs.

No failed attempt is overwritten. Source-check success, preliminary native success, focused success, current gate, and any future benchmark remain distinct evidence levels. The frozen benchmark example/policy in proposals is not a launch authorization.

## Work performed for this proposal

Only directory listings, the explicit closed-record inventory/hashes, small capture reads and Git object metadata were read. Inventory gateway compiler-scalar-retention-inventory-1323 completed in 32 ms; Git metadata request compiler-scalar-retention-git-1324 completed in 19 ms. No compiler, native image, archive/compression command, process scan, key read, cleanup, or active-gate read ran. The probe git symbolic-ref HEAD reported detached HEAD; that expected planning observation is retained in gateway compiler-scalar-retention-records-1325 and is why the bundle plan uses an explicit temporary ref.
