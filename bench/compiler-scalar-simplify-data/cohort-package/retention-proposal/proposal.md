# Scalar cohort retention and later closure proposal

Prospective only. The measured scalar gain was 0.702679%, below the fixed 10% rule; the experimental production candidate is discarded with no main adoption. Retention preserves this miss and every original capture. No compression, cleanup, compiler/native run, benchmark or source change occurs in this preparation.

## Exact scopes

Only the following two frozen main-worktree directories were inventoried. Every regular file has a named recovery and disposition in recovery-map.tsv; no symlink or special file was accepted.

| Scope | Files | Logical bytes |
| --- | ---: | ---: |
| target/compiler-scalar-simplify.20261116-1 | 322 | 207024988 |
| target/compiler-scalar-trial-capture.20261116-1 | 15 | 2151 |
| Total | 337 | 207027139 |

## Retention classes

| Class | Files | Bytes |
| --- | ---: | ---: |
| existing-emitter-duplicate | 1 | 32761734 |
| retain-producer-A | 1 | 22937783 |
| retain-producer-B | 1 | 22929591 |
| retain-raw-record | 154 | 87098 |
| retain-source-archive | 1 | 7434240 |
| seed-equal-output-duplicate | 7 | 113596196 |
| source-tree-duplicate | 172 | 7280497 |

Retain all raw cohort and outer capture records with original member paths in one deterministic raw-cohort-records.tar.gz made only from raw-members.nul. Keep the exact per-member hashes, full argv/environment metadata, time/load/status/stdout/stderr, inventories, original pin and result records. Do not rewrite a record because the threshold was missed.

Retain the two actual A/B native binaries as separate content-addressed gzip -n -9 artifacts. Retain exact fixture/source.tar (cf3cd4568e4b7581aefff46cc92f93a6f145d861f46481df76d1e861fd0d0c64) as artifacts/cf3cd4568e4b7581aefff46cc92f93a6f145d861f46481df76d1e861fd0d0c64.source.tar.gz, without regenerating it from a different checkout. The archive's 172 extracted files are duplicate recovery targets, not a second source archive to retain. Verify every original archive member against this inventory before closure.

The copied E emitter maps to the already byte-verified existing preparation artifact bench/compiler-scalar-simplify-data/preparation/artifacts/f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64.compiler.gz (gzip SHA ec4d686eb3a296c97757c479ce308a7c6388f3da8d30dca0079af9cb5684e7c2). Do not make another E gzip or remove that committed artifact. This proposal references its existing verification; it did not read another scope to reverify it.

All six outputs and fixture/expected are 8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132, 16228028 bytes each. Their one shared recoverable representation is the exact frozen seed/lkjscript.gz member (85b4e70de34dfa759aaf3cfb7e1009ba6b49b57499e01e4a3d49ae9182a6193c) already inside the retained source archive. There is no need for seven output gzips or a second seed gzip. The independent acceptance proves their complete byte equality to the exact Git seed; this inventory additionally checks each current size/hash.

## Acceptance and boundaries

Independent acceptance is frozen at target/compiler-scalar-cohort-acceptance-1: acceptance.json 87667ea276c690cd43bc985ac0d439b6ae6aa3d8ceadcbd170213dcfff457f59; source-comparisons.json 9e033a650b89560633204a16d7f0025a324aadc79235d181c567f66018ba2f6b; 52-record manifest 50fc48af70fb957ef7040bb28a55e0f038dee6195a1295554ff2400fcc34ca59. These are reviewer-supplied pins, not a new acceptance execution. Root retains that proof separately. Preserve all earlier auditor-only failures there.

The initial directory-read attempts received Coder HTTP 429 before any inventory or mutation. That is a preparation access limitation, not a cohort failure or incomplete result. Reads were paused until root explicitly released this one batched bounded inventory/proposal call.

## Future execution sequence, not released here

1. Root commits and verifies this proposal plus accepted raw/proof evidence, then explicitly releases packaging.
2. Produce the explicit-list raw archive and the three new gzip artifacts (A, B, exact source tar). Record original/compressed lengths and SHA-256; fully verify every raw member and decompress/byte-compare each new artifact. Verify the frozen source archive member mapping, including the shared seed representation. Preserve any failure without overwriting the originals.
3. Verify the existing committed E gzip and its original identity, and ensure independent acceptance remains exactly pinned. Commit all recovery artifacts and inventories before considering cleanup.
4. A later separately released guard must require exact two literal root identities, every current file name/type/length/hash and no additional leaf. It must reject missing/changed recovery or unknown files, remove only enumerated files and empty directories, then require ENOENT for those two scopes. No native rerun is part of cleanup.

No keys, stores, candidate/source-worktree cleanup, validation-package cleanup, preparation artifact deletion, main source, gate data or unrelated target path belongs to this closure. Original files remain present. Packaging and deletion are both pending explicit root release.
