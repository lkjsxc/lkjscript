# Scalar cohort recovery package

This package retains the accepted scalar comparison that measured a 0.702679%
gain and missed the fixed 10% rule. The experimental production candidate remains
discarded; these artifacts do not adopt it to main or authorize another run.
Accepted result/evidence commit: 330272b97ddba28b376ce3c44f57b04c195991be.

## Contents

- raw-cohort-records.tar.gz contains all 154 original raw cohort/outer records,
  87,098 member bytes, with exact original paths. It preserves every capture,
  status, time, load, argv, result and fixed control record.
- Two content-addressed native gzips preserve the actual executed producer A
  and producer B bytes.
- One content-addressed source.tar.gz preserves the exact frozen source tar,
  including its Git identity 26bf2c3c021dd79810a81fa1d9054d59f3768b06 and embedded seed.
- retention-proposal/ is an unchanged 18-file copy of the approved proposal.
- capture/, pack.pl, commands.json, the member manifests, original and corrected
  verification reports, and the correction source retain the actual procedure
  and all outcomes.

| Artifact | Compressed bytes | SHA-256 |
| --- | ---: | --- |
| raw-cohort-records.tar.gz | 24,654 | 6ed29aa9625dd7a73f9c7c07f9bcf3bff4bcadf2b2eaa60d5617aab1d700304a |
| artifacts/c7b8d970ace8f836a559f21fd7d95867fd718b9e95160d686442380afa330db1.producer-A.gz | 4,530,022 | 1a6196c0b0e2cb965c310a722e544d8577a4275246342bb0a24b08fd35178093 |
| artifacts/12b1f645c3ae4085c998cde9b6d3c920bc6c4ca547839654586c196e9aaaad11.producer-B.gz | 4,529,831 | 6f6b832580a982aa16f8985359df4083c957ee5e855d75bb8e8f3c89b515a369 |
| artifacts/cf3cd4568e4b7581aefff46cc92f93a6f145d861f46481df76d1e861fd0d0c64.source.tar.gz | 3,824,641 | b1ee997837fd37d8eaa1a77bb1cf0fd8d0ed64ecc825be27dd9bef74f8f9d2e3 |

Original A/B/source-tar lengths are 22,937,783 / 22,929,591 / 7,434,240 bytes.
The content-addressed filenames retain their exact original SHA-256 digests.

## Verification and recovery

All 154 raw members were compared byte-for-byte with their original files,
in addition to exact member names, modes, lengths, hashes, tar framing and gzip
CRC checks. The raw archive came only from the approved explicit NUL path list.

Each of the three new gzip streams was decompressed and compared byte-for-byte
with its complete original. The recovered source tar was additionally traversed
in full: all 172 regular source/seed members match the extracted frozen tree
byte-for-byte; its 17 directory headers are expected parents; its sole global
metadata record is the exact frozen Git comment. Source archive modes are
recorded separately from physical extracted modes; source bytes are equal.

The seed/lkjscript.gz member, SHA-256
85b4e70de34dfa759aaf3cfb7e1009ba6b49b57499e01e4a3d49ae9182a6193c,
was recovered through the newly retained source archive and decompressed directly
into comparisons with fixture/expected and all six outputs. All seven are exactly
16,228,028 bytes and SHA-256
8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132.
No expanded seed, output copy, or output gzip was created.

Existing E recovery was verified in place once:
bench/compiler-scalar-simplify-data/preparation/artifacts/
f85624e3cc694f6a3a3e10c33f822789ae6084e2d5a5b41c3b1fc8fcc9bd2d64.compiler.gz.
Its 6,582,463 compressed bytes hash to
ec4d686eb3a296c97757c479ce308a7c6388f3da8d30dca0079af9cb5684e7c2
and match committed Git blob 95a68fd1c3c298e3686138bb28ad6bf63036e8e2
at the accepted result commit. Full decompression byte-matches the original
32,761,734-byte copied E executable. No additional E copy or gzip was created.

All 337 original cohort/outer files and all 18 proposal inputs remained unchanged.
The independent acceptance and its earlier auditor failures remain separate,
bound by the exact pins in the copied proposal and verification metadata.

## Preserved reporting defect and correction

Use verification-corrected.json for identity fields.
SHA-256: 1df0de7ff739413788067a4c131a7d74889f3a7a2b56784baeb349052e7fd8da.

The original verification.json is retained unchanged at SHA-256
c9e4be78f61bd1922bc5cebfda0b3dec76cbb81680807a485e861e65eb2d280e.
Seven identity fields in that original report contain empty-stream digests.
Digest::SHA::hexdigest resets its object: the first calls were the actual exact
pin guards and passed, but second calls used to populate those JSON fields
reported the reset state. The full byte comparisons, first-digest guards,
raw/source member hashes and compressed artifact hashes all succeeded in the
single original packaging execution.

correct-report.pl requires that exact original report, helper and policy,
successful captured status, empty packaging stderr and the complete PASS record.
It corrects only those seven reporting fields from the exact successfully checked
policy/committed-blob values, with a field-by-field explanation. It reads no
archive, binary or source input. No compression, decompression, E verification,
cohort run or benchmark was repeated. Both the defective report and correction
captures remain in this package.

## Execution

Packaging request compiler-scalar-cohort-package-run-1346;
session pipe-dlokccpipbmoeakojjodpamkkgpiabnl;
completion compiler-scalar-cohort-package-wait-1347.
Exit 0, empty stderr. One sequential compression worker, external 4 GiB AS,
900-second timeout, 10-second TERM grace.
GNU wall 4.48 s; user 4.51 s; system 0.12 s; peak RSS 11,708 KiB.
Load 1.73/1.27/1.53 before and 1.75/1.28/1.53 after.

Offline correction request compiler-scalar-cohort-package-correction-1350.
Exit 0, empty stderr; wall 0.04 s, peak RSS 9,968 KiB.
Load 1.78/1.43/1.56 unchanged. Its argv record is identified as recorded after
execution from the exact submitted gateway command.

These are retention timings, not compiler performance observations.
No compiler/native/performance job, deletion, key/store read or other-worktree
access occurred. Only the two released cohort scopes, the frozen proposal and
the one explicitly authorized existing E recovery artifact were read.
Original files remain present. Cleanup needs a separate release and guard.
