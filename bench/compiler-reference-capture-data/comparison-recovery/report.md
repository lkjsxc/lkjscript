# Completed reference-capture comparison: recovery package

The completed comparison and its outer launch capture have complete recovery coverage for all **501 regular files, 254,521,215 logical bytes and 32 directories**. This package stores the raw records and actual output bytes. The three producer/seed paths are recovered from the exact committed Git seed blobs, which were compared in full with the originals. The two original roots remain unchanged.

## Scope and representation

The only original roots are:

- `target/compiler-reference-capture-comparison-2`
- `target/compiler-reference-capture-comparison-launch-2`

| Original class | Original paths | Original bytes | Recovery |
| --- | ---: | ---: | --- |
| Raw records, controls, summary and outer captures | 484 | 1,095,429 | Every file stored separately, byte-identically |
| Six new-100k outputs and its final output slot | 7 | 31,277,813 | One actual 4,468,259-byte representative |
| Six compiler outputs and its final output slot | 7 | 150,423,028 | One actual 21,489,004-byte representative |
| A/B producer executables and B seed gzip | 3 | 71,724,945 | Exact Git blob / gunzip recipes with full-byte verification |
| **Total** | **501** | **254,521,215** | Complete original-path inventory |

`inventory.json` records every original path, type, mode, size, digest and recovery mapping. Directories retain their path and mode. Each output alias is restored as an independent file with its recorded mode; deduplication does not require hardlinks. Filesystem inode allocation, ownership and incidental timestamps are outside this byte-and-mode recovery contract.

## Exact stored artifact

`recovery.tar.gz` is **5,237,981 bytes**, SHA-256:

`2451a0a876fc2ddf839e5774280a39c500500b17d65a494028c1a18002af1833`

It contains **490 regular members**: 484 ordinary files, two actual output representatives and four recovery metadata files. The uncompressed tar is 27,746,304 bytes. Members use sorted short paths, their recorded payload modes, UID/GID zero and mtime zero. Gzip uses time zero, empty name/comment and level 9. Compression ran once.

The output representatives are:

| Family | Bytes | SHA-256 |
| --- | ---: | --- |
| new-100k | 4,468,259 | `42443b75fe211c9f9efa7c0ee26c9233280c38a124ec5e5b514666aab97ce361` |
| compiler | 21,489,004 | `5ab411b7fb91498adc1453a08d96893f85c1d644a2e050b93b3e9dc850585597` |

A is recovered from `7d0731e99aeaf241c4334fc5ff065a341fb388c8:seed/lkjscript.gz`; its expanded digest is `6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc`. B is recovered from `e5294c339d9aefbbafc2e951c997f4fefabcc9ad:seed/lkjscript.gz`; its expanded digest is `b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db`. `producer-recovery.json` records the compressed and expanded sizes/digests, exact Git blob identities and recovery commands. No producer binary was recompressed into the archive.

## Verification and capture

Before deduplication, all 14 original output paths were compared in full to the actual family representatives. Both executables and B's compressed seed were compared in full to the pinned Git recovery bytes. After writing the archive, the helper decoded it, checked all 490 member paths/bytes/modes and normalized header fields, then compared all 501 original regular files again against their archived or Git-recovered bytes. The complete source path/type/mode/size inventory was unchanged. Unknown paths, symlinks, special entries, hardlinked files and credential-like paths were refused by the fixed inventory guards.

The comparison's original `summary.json` remains SHA-256 `b531ef397e7420418562125d44fadc6399139ac3cc8c3c58c37d1a45c415c33c`. This recovery operation does not reinterpret its timing observations. Independent result acceptance is separately retained under `target/compiler-reference-capture-comparison-acceptance-2`.

The once-only packaging command was gateway request `compiler-reference-capture-recovery2-package-20261116-297`, completion request `compiler-reference-capture-recovery2-package-wait-20261116-298`. It exited **0**, under a 4 GiB address-space ceiling and 900 s / 10 s watchdog:

- UTC: 2026-11-16T19:02:20.328106238Z → 2026-11-16T19:02:23.833589672Z.
- GNU time: **3.50 s wall, 3.30 s user, 0.11 s system**, peak RSS **418,864 KiB**.
- Load before/after: **1.00 / 1.08 / 1.51 → 1.08 / 1.10 / 1.51**.
- Exact argv, cwd, nanoseconds, UTC, load, status, stdout/stderr and GNU time remain in `capture/`.

These are packaging costs, not new compiler measurements. There was no compiler/native execution, workload rerun, deletion, process observer or key read. Source preparation and benchmark input recovery remain in their separate committed package. The original roots are retained for root review; this package grants no cleanup or automatic restoration.
