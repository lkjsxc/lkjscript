# Owned query-worktree closure proposal

No removal is authorized or performed. The eight literal scopes below contain
**688 regular files, 82 directories and 15,840,821 logical bytes**. All regular
files have one link; there are no symlinks or special entries.

Base path:
`/home/coder/workspace/lkjscript-compiler-query-container-20261116/target/`

| Literal directory beneath that base | Files | Directories | Logical bytes |
| --- | ---: | ---: | ---: |
| compiler-copy-sites-profiler-candidate | 8 | 1 | 183,565 |
| compiler-copy-sites-accounting-candidate | 322 | 41 | 630,813 |
| compiler-copy-sites-extractor-candidate | 104 | 25 | 460,223 |
| compiler-copy-sites-extractor-package-20261116 | 10 | 1 | 181,048 |
| compiler-copy-sites-census-join-20261116 | 125 | 6 | 13,410,540 |
| compiler-copy-sites-census-join-package-20261116 | 5 | 1 | 245,277 |
| compiler-copy-sites-family-census-20261116 | 109 | 6 | 594,062 |
| compiler-copy-sites-family-census-package-20261116 | 5 | 1 | 135,293 |

## Exact recovery

Recovery is pinned to main commit
**61049544e4461e3365d239191a6cbe704407a85e**. The inventory reads committed Git
blobs and indexes their archive members without extracting or executing them.

| Recovery class | Files | Bytes |
| --- | ---: | ---: |
| Matching committed archive member | 667 | 15,244,872 |
| Matching committed file/blob | 10 | 565,947 |
| Concrete package-metadata gaps copied for retention | 11 | 30,002 |

The five committed archives are the profiler proposal, accounting proposal,
extractor candidate, all-row join and complete-family census. Their exact Git
paths, archive digests and member counts are in
inventory-1/recovery-archives.json. Every local file's byte length, SHA-256,
metadata and selected recovery member/blob is recorded in
inventory-1/full-inventory.tsv and full-inventory.json.

The independent profiler check also byte-compared all eight candidate files to
their same-name members in the committed profiler archive. Its retained gateway
is compiler-profiler-recovery-exact-20261116-203. The other scope mapping uses
actual current-file length/SHA-256 matches to the committed archive payloads;
it does not assume that a live candidate still matches an older inventory.

The only remaining retention gaps are package-generation metadata:

- Seven extractor-package files: start/end, before/after load, archive.time,
  candidate-after.check and package.sha256.
- The join-package after.check and package.sha256.
- The family-package after.check and package.sha256.

All eleven are copied, unchanged, beneath inventory-1/pending-retention.
retention-gaps.json gives every original path, copy path, byte count and digest.
No uniquely held content is classified as disposable. Once these copies and the
closure evidence are committed, all 688 files are redundant local copies with
explicit recovery. No compiler executable, store or credential is in these
eight scopes.

## Preservation and release boundary

The entire compiler-container evidence, its key locator, the unrelated
/tmp/compiler-container-key.m4u6F6 credential, durable worktree edit metadata,
all tracked source, main and other worktrees are outside the literal scope.
This proposal directory also stays outside the cleanup. No key content was
read, and no process or FD scan was made.

Root must commit the new retention copies and this inventory before releasing
deletion. A later guard must recheck all eight literal path sets, file hashes,
identities and recovery records before unlinking anything. It should remove
only enumerated files and then enumerated empty directories, and require exact
ENOENT for the eight roots afterward. This proposal contains no removal code.

## Preparation record

The inventory/hash operation ran once, with no compiler, profiler, official
reader, extractor or previous census invocation:

- Gateway: compiler-copy-tests-closure-inventory-1275.
- Exit 0; elapsed 0.19 s, user 0.15 s, system 0.02 s.
- Peak RSS 56,224 KiB.
- Load 1.12 / 1.48 / 1.86 before and after.
- 4 GiB address space; 300 s watchdog; 10 s grace.
- Empty stderr.

The exact script, command/cwd, limits, raw capture, time and load records are
retained here. No source change or deletion occurred.
