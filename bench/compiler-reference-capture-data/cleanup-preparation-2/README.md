# Finite cleanup guard: preparation only

The guard has not been executed. This package proposes exactly 29 older literal roots, 3,050 regular files and 160,418,647 logical file bytes. The root list is hardcoded in `guard.pl` and repeated in `release.template.json`; the executable guard requires that exact ordered list. No prefix expansion or automatic addition of newly discovered roots occurs.

## Fixed inputs and release

- Guard: SHA-256 `e182101cf27ff0f2bed93a015ecfd2cc93aef1d13234eb2809eccbe361042e64`.
- Map: `target/compiler-reference-capture-recovery-map-1.json`, SHA-256 `ff159c384304fb66cd5aa644a06485a5cec2bcc4aab393ccd679a3b7f6faa4bd`.
- Proof: `target/compiler-reference-capture-recovery-proof-1.sha256`, SHA-256 `347e1900c21920ae9b0b6bfad44edd117721c45e4c0765019b90c74ec85c8729`.
- Exact map/proof and all seven proof members are committed at `d84cac6edcee59e2c2ad631fb6ca15d63340a1c8`, under `bench/compiler-reference-capture-data/cleanup-mapping`. The guard checks those committed bytes directly.
- Recovery data is limited to `e68ea0eb29b755cb48df5034c57fb226a1f7c3c6:bench/compiler-reference-capture-data/...` and the two exact seed blobs at `7d0731e99aeaf241c4334fc5ff065a341fb388c8` and `e5294c339d9aefbbafc2e951c997f4fefabcc9ad`.

The template intentionally has `execute=false`, `cooperative_quiescence_confirmed=false` and an unfilled approval commit. It cannot authorize an execution. After separate review and release, root may write `release.json` beside the helper, set the two flags true, and supply the commit containing the exact reviewed guard. The guard reads that committed helper and requires full byte equality with its live source. The approved Git path must remain under the named bench evidence prefix.

The capture destination is exclusively created at `target/compiler-reference-capture-cleanup-execution-2`; any existing path or unexpected lookup error refuses execution. Invocation must use the literal release path and `--execute-after-root-release` argument.

## Independent Perl preflight

The earlier recovery map is treated as an untrusted set of descriptors. Its acceptance flags, counters and Python-derived verdict are not used to authorize a removal.

Before the first unlink, Perl independently:

1. Verifies the exact release, committed reviewed guard, map/proof hashes, committed copies and all seven proof-member hashes.
2. Selects only the 29 hardcoded roots, rejects overlapping/duplicate/out-of-range file records, validates exact path scope and recovery indices, and recomputes the fixed file/byte totals.
3. Enumerates every selected root without following links. It requires the complete recorded regular-file/directory population and owner/type/link/identity/mode metadata, rejecting extras, missing entries, special files, symlinks or hardlinks. It opens and retains directory handles.
4. Reads each committed Git blob and checks its object identity; independently decodes the two permitted existing gzip/tar archives or seed gzip expansions as required; requires each named recovery's full length/hash.
5. Reads every original file and compares its full bytes with the independently verified recovery as well as the map's SHA-256. It does not accept a matching recorded digest alone in place of the byte comparison.
6. Rechecks the entire selected population and stored metadata before beginning removal.

Archive parsing supports the existing USTAR/GNU headers, bounded GNU long names and bounded PAX path records, with header checksums, size/padding/terminator checks, duplicate/path validation and refusal of links or unsupported member types. It decodes in memory only. No member is extracted, no archive is recompressed, and no program embedded in an artifact is executed.

## Exact metadata qualification

The frozen map contains relative path, uid, gid, mode, size, link count, device, inode, SHA-256 and recovery index for files. Directory records contain the corresponding identity/size/link metadata; root records contain uid, gid, mode, device and inode. **It contains no historical mtime/ctime nanosecond fields. The guard makes no historical nanosecond equality claim.**

Perl's before/after read check additionally observes its ordinary stat mtime/ctime values for that read; these are supplementary, second-resolution observations, not reconstructed historical nanoseconds. Every original also undergoes a full byte/hash comparison. The syntax capture's nanosecond timestamps are exact decimal strings produced by `date +%s%N`, without float conversion.

## Removal and partial stops

After complete preflight, the guard rechecks each file's ancestor paths against the retained directory identities, rereads its full bytes and recovery, and checks its pathname metadata immediately before its enumerated unlink. It records an intent and completed event for each removal. It then removes only the enumerated empty directories deepest-first and the original root. Final success requires ENOENT for all 29 roots.

Directory identity/mode/owner checks remain active during removal. Directory sizes and link counts are checked in preflight; they naturally change during enumerated removal and are not incorrectly required to retain their initial values afterward.

Any failure stops the run. `progress.jsonl` is flushed, and `failure.json` records the exact completed removals and roots when writable. An intent without a completion can identify an interrupted step; the guard does not guess its result. There is no rollback or atomic multi-root guarantee. Path checks and held directory handles operate under the explicitly confirmed cooperative quiescence contract; this is not a claim of immunity to arbitrary hostile concurrent pathname mutation.

## Exclusions

These five roots are hard exclusions even if their later activities finish:

- `compiler-reference-capture-measurement-preparation-1`
- `compiler-reference-capture-measurement-2`
- `compiler-reference-capture-trial-preparation-2`
- `compiler-reference-capture-comparison-2`
- `compiler-reference-capture-comparison-launch-2`

The new preparation/execution roots, all other names, canonical `target/lkjscript`, `target/gate`, global or foreign credentials, and unrelated worktrees are outside the literal removal set. No process/FD observer, syscall experiment, kill/race test or workload is involved.

## Preserved no-Python violation

The original mapper actually ran under `/usr/bin/python3`, and its syntax check also used Python. That violated the user's no-Python instruction. Its exact original `.py` and run/syntax captures remain unchanged. Root retained its source as non-executable `.py.txt` at the committed evidence destination, with the true original path recorded in `copy-coordinates.json`; that retention name does not reclassify the original execution.

This guard and its preparation use Perl/shell only. The guard verifies the preserved Python source's proof hash as inert evidence but never executes it. It independently rederives every proposed original/recovery byte match and the complete selected population rather than trusting the Python verdict.

## Direct execution contract after release

Run from exactly `/home/coder/workspace/lkjscript`, with the separately reviewed release file installed. The command body is:

```sh
/usr/bin/timeout --signal=TERM --kill-after=10s 900s \
  /usr/bin/prlimit --as=4294967296 -- \
  /usr/bin/perl target/compiler-reference-capture-cleanup-preparation-2/guard.pl \
  --release /home/coder/workspace/lkjscript/target/compiler-reference-capture-cleanup-preparation-2/release.json \
  --execute-after-root-release
```

Root should retain its usual outer argv/cwd/UTC/exact-nanosecond/load/GNU-time/status/stdout/stderr capture. The guard itself exclusively creates invocation/release records and progress/final-or-failure records in the fixed execution directory. This preparation does not launch that command.

## Validation performed

Only `perl -c guard.pl` ran. It passed under 4 GiB / 60 s / 10 s, exit 0, 0.02 s wall, 13,284 KiB peak RSS, load 1.10 / 1.15 / 1.23 unchanged. Gateway request: `compiler-reference-capture-cleanup-syntax-1449`. Before/after helper hashes match. Runtime preflight, archive decoding and deletion remain unexecuted in this preparation; static review is not represented as a completed cleanup.
