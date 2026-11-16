# Final reference-capture recovery map

The exact seven completed scopes have complete byte-and-mode recovery coverage: **4,468 regular files, 342,778,212 logical bytes and 141 directories**, with **zero unresolved entries**. Every original regular file was compared in full with its recorded recovery provider. The complete original metadata inventory was unchanged after the comparisons.

All retained benchmark coordinates are pinned to commit `d84cac6edcee59e2c2ad631fb6ca15d63340a1c8`. The two producer seed recipes additionally name their exact historical commits. This is a finite recovery and closure proposal; no deletion is authorized or performed.

## Exact scopes

Every scope below is directly under `/home/coder/workspace/lkjscript/target/`.

| Scope | Files | Directories | Logical bytes |
| --- | ---: | ---: | ---: |
| compiler-reference-capture-comparison-2 | 486 | 31 | 254,519,936 |
| compiler-reference-capture-comparison-launch-2 | 15 | 1 | 1,279 |
| compiler-reference-capture-measurement-2 | 3,802 | 96 | 81,473,066 |
| compiler-reference-capture-trial-preparation-2 | 55 | 5 | 158,352 |
| compiler-reference-capture-comparison-acceptance-2 | 68 | 4 | 894,388 |
| compiler-reference-capture-comparison-recovery-2 | 36 | 3 | 5,711,591 |
| compiler-reference-capture-measurement-preparation-1 | 6 | 1 | 19,600 |
| **Total** | **4,468** | **141** | **342,778,212** |

The last scope is dependency-only now that the comparison is closed. This proposal does not include another target prefix, worktree, canonical executable, store or key.

## Recovery coordinates

`recovery-map.json` records every original path, file mode, size and digest, with an exact provider reference. It also preserves the observed UID/GID, inode, device, timestamps and link count as audit metadata. Directory paths and modes are recorded independently. Recovery restores regular bytes and modes plus directory layout/modes; it does not promise to recreate incidental inode allocation, ownership or timestamps.

Providers use only the existing retained data:

- **Comparison and outer capture:** `bench/compiler-reference-capture-data/comparison-recovery/recovery.tar.gz`, SHA-256 `2451a0a876fc2ddf839e5774280a39c500500b17d65a494028c1a18002af1833`, with exact member names from the committed inventory. Each original output alias was separately compared with its actual archived representative.
- **A/B executables and seed gzip:** exact `seed/lkjscript.gz` blobs from `7d0731e99aeaf241c4334fc5ff065a341fb388c8` and `e5294c339d9aefbbafc2e951c997f4fefabcc9ad`, using the recorded compressed or expanded bytes. Git blob identities and full SHA-256/size pins accompany every recipe.
- **Compiler input and factory:** the existing `setup2/artifacts/compiler-source.tar`, SHA-256 `79aa24a9eb6f8e7940b4424fc20839759a79dd8d480b1b746f7e29b17171d806`, supplies all 1,742 compiler-input files and all factory files except its one declared `src/main.lkj` overlay. That overlay is the retained `setup2/artifacts/scale-generator-source.lkj.txt`. The two source-archive per-file PAX path records were interpreted by the previously reviewed bounded parser; their exact raw offsets and associations are retained in `source-archive-pax.json`.
- **Generated input and generator:** the existing `setup2/artifacts/generated-100k.tar.gz` supplies all 170 generated members and the exact decompressed tar. The existing `setup2/artifacts/generator.gz` supplies the actual generator executable. These were decoded and compared; nothing was generated or recompressed.
- **Remaining setup, trial, acceptance, recovery and dependency records:** exact committed blobs beneath their existing `bench/compiler-reference-capture-data/` destinations.

The only file added after the trial-preparation copy was `compiler-reference-capture-trial-preparation-2/release.json`, 3,486 bytes, SHA-256 `6ea5952f0c0f366f86a829a85bb82292d8a9a656cd488d52305287eaae63d3ff`. Its full bytes match the already retained result-archive member `files/comparison/control/release.json`. This exact content alias needs no additional retained payload.

The map contains 2,716 provider records and introduces **no new payload archive or content compression**. No file is classified as disposable merely because its digest is known.

## Interpreter confirmation

The completed packaging capture explicitly invokes:

`/usr/bin/perl /home/coder/workspace/lkjscript/target/compiler-reference-capture-comparison-recovery-2/package.pl`

That original `capture/argv.json` is SHA-256 `075bfa8138f5d5ad804897f91d4e75fee4a3d178675d9401c16d79c6d90af773`. The actual interpreter was confirmed by reading this existing capture, without rerunning the package. The retained helper uses Perl Archive::Tar, IO::Compress::Gzip and IO::Uncompress::Gunzip. The new finite mapper and its wrapper likewise use Perl and shell only. `interpreter-confirmation.json` records the exact captured argv and digest.

## Execution and outcome

The one read-only mapping command was request `compiler-reference-capture-final-recovery-map-20261116-308`, completed by `compiler-reference-capture-final-recovery-map-wait-20261116-309`. It exited **0**, under a 4 GiB address-space ceiling and 900 s / 10 s watchdog.

- UTC: 2026-11-16T19:24:43.615342857Z → 2026-11-16T19:24:46.974848949Z.
- GNU time: **3.35 s wall, 2.72 s user, 0.52 s system**, peak RSS **344,196 KiB**.
- Load: **0.66 / 0.91 / 1.12 → 0.69 / 0.91 / 1.12**.
- Exact command, cwd, UTC/nanoseconds, load, status and raw output remain in `mapping-capture/`.

The original roots remain present and unchanged. No compiler/native execution, compression, deletion, key read or process/FD/syscall observer was used. Any later cleanup requires root's separate review and release, with these exact scopes and recovery records retained.
