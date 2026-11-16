# Scalar cohort: finite cleanup preparation

**Prepared only. No cleanup is released or executed by this document.** Root must commit/review these controls and explicitly release the exact launcher and guard before the proposed command below is used.

The measured scalar gain remains 0.702679%, below the fixed 10% rule. Recovery preserves that miss; the experimental production candidate was not adopted.

## Two literal removal scopes

Only these main-worktree directories are eligible:

| Directory below /home/coder/workspace/lkjscript | Regular files | Logical bytes | Directories including root |
| --- | ---: | ---: | ---: |
| target/compiler-scalar-simplify.20261116-1 | 322 | 207,024,988 | 21 |
| target/compiler-scalar-trial-capture.20261116-1 | 15 | 2,151 | 1 |
| Total | 337 | 207,027,139 | 22 |

No caller-supplied root or file list is accepted. The guard binds snapshot.json and the accepted original inventory, then requires exactly these two roots, 337 files and 22 directories. Preparation recorded directory device/inode/mode and confirmed the original file names, types, device/inode, size and mode; the released guard performs the full current-file hash checks.

## Recovery proof before any unlink

Committed recovery is pinned to main commit **0b0b27803d27b932a5b91cde2d766bd3c8bb82b9**.

The guard checks all 71 files of bench/compiler-scalar-simplify-data/cohort-package against its 70-record SHA-256 manifest plus the manifest's external pin. It also checks the existing committed E gzip. For all 72 files, a current byte stream must match both its accepted SHA-256 and the exact Git blob at the pinned commit. The package manifest pin is 3bea8d64d065041a61c44e654c7e65315f4a6f28ddd0d8124310d9b69e602438.

The guard uses the committed, corrected verification report (1df0de7ff739413788067a4c131a7d74889f3a7a2b56784baeb349052e7fd8da) and exact accepted inventory to establish one recovery mapping for every original:

| Original class | Count | Committed recovery |
| --- | ---: | --- |
| Raw cohort/outer records | 154 | Exact members of raw-cohort-records.tar.gz |
| Actual A/B producers | 2 | Their separately retained content-addressed gzips |
| Original source.tar | 1 | Its exact content-addressed gzip |
| Extracted source files | 172 | Exact members of the retained source tar |
| Expected plus six output images | 7 | Expanded seed/lkjscript.gz member in that same source tar |
| Copied experimental emitter E | 1 | Existing preparation f85624e3 compiler.gz |

The maps require exact per-original path, class, length and SHA-256, no duplicate or missing original, full-byte-equality proof flags, and expected class counts. This verifies that the previously accepted recovery bytes and mapping remain present and committed. It does not repeat decompression, archive verification or the compiler/native experiment. The original defective metadata report and all seven documented corrections remain in the committed package and are never edited.

## Current-state and removal contract

Before the first unlink, the guard verifies:

- Real directory components and pinned parent/root/directory identities.
- No symlink, special entry, additional directory or extra/missing file.
- Each current file's original device/inode, length, mode, mtime and SHA-256.
- The complete 337-file population and all 337 committed recovery mappings.
- Canonical target/lkjscript remains SHA 6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc, 32,584,542 bytes, with its recorded identity.

It then visits only the enumerated original files. Immediately before each unlink, it rechecks that file's hash/metadata and enumerated parent chain. It removes directories only by rmdir, deepest first; unknown or newly introduced contents cannot be recursively deleted. Success requires explicit ENOENT from lstat for both root paths and an unchanged canonical compiler.

This is **cooperative, non-atomic closure**. It is not a defence against hostile concurrent filesystem mutation. A refusal before unlink removes nothing. A later failure can leave a partially removed fixture; progress is flushed after each removal, no rollback or automatic retry is attempted, and root reviews the retained state. No process, file-descriptor, syscall, race or kill observer is used.

All paths outside the two roots are excluded from deletion. That includes main target/lkjscript, credentials and keys, durable edit metadata, source/candidate worktrees, stores/caches, all recovery/evidence packages, and this preparation directory. Credentials are neither discovered nor read.

## Prospective invocation and capture

Only after separate root release:

    bash /home/coder/workspace/lkjscript/target/compiler-scalar-cohort-cleanup-preparation-1/launch.sh --remove

The launcher exclusively creates target/compiler-scalar-cohort-cleanup-execution-1 and records argv, cwd, reviewed guard hash, UTC/nanosecond endpoints, actual load, GNU time, stdout, stderr and exit status. It applies 4 GiB AS, a 900-second TERM watchdog and 10-second grace. The once-only capture path prevents silently overwriting a refusal or partial result.

The guard emits JSON-lines progress including completed preflight, every unlink/rmdir, each final ENOENT and canonical preservation. A check-only mode exists for an explicitly released read-only run, but it has not been executed and is not part of the proposed cleanup sequence. No new compiler/native/profile work, recompression or performance measurement is part of closure.

## Preparation evidence

The directory includes the exact metadata snapshot and gateway-derived preparation records. Only perl -c of guard.pl and bash -n of launch.sh are planned for this preparation. Neither guard mode nor launcher is executed. Any syntax/setup refusal is retained and must be corrected prospectively before handoff.

One orchestration-level JavaScript parse failure occurred while composing the launcher text, before any nested file-write tool was submitted. Its exact diagnostic and cause are preserved in preparation-wrapper-1.error.txt. No guard, compiler, cohort read or deletion occurred in that failed call.
