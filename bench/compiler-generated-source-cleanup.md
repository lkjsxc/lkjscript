# Accepted generated-source repair: artifact cleanup

After the combined gate passed and root released this exact deletion set,
the guarded cleanup removed **seven owned files totalling 159,697,969 bytes**.
It ran once, exited zero, and executed no compiler or historical replay.
The original repair report, its 437-entry manifest, source, and all retained
observations remain byte-identical. This follow-up records a later cleanup;
it does not revise those historical observations.

| Observation | Result |
| --- | --- |
| UTC start / end | 2026-11-16T11:06:52Z / 2026-11-16T11:06:53Z |
| GNU elapsed / user / system | 0.23 / 0.17 / 0.06 s |
| Timestamp interval, including wrapper work | 0.237220826 s |
| Peak RSS | 6,140 KiB |
| Load before, 1 / 5 / 15 minutes | 1.83 / 1.80 / 2.74 |
| Load after, 1 / 5 / 15 minutes | 1.83 / 1.80 / 2.74 |
| Same-user process inventories inspected | 44 |
| Live owned executable/open-descriptor references | None |
| Live command-line references outside cleanup ancestors | None |
| Pending intent or projection staging paths | Absent |
| Original committed entries verified before / after | 437 / 437 |
| Lock and generation metadata | Preserved byte-identically |

The two timestamps and all GNU/load output are retained raw. The bytes above
are the sizes of removed files, not a measurement of recovered filesystem
blocks or a compiler performance result.

## Exact scope and guards

`compiler-generated-source-cleanup-data/expected.tsv` contains the seven
absolute paths, expected sizes, modes and SHA-256 digests. They are the five
isolated executable copies, their operation-key locator, and its paired
external 32-byte key. The shared parent directory, canonical compilers,
query fixtures, other worktrees and all evidence were outside the deletion set.

The retained script `compiler-generated-source-cleanup.sh` has SHA-256
`59c72073160a97016cd7cba7ad3dbe95481ce7980aa9440fe069a94f55ce962b`.
Before its single exact-path unlink it checked regular-file/non-symlink status,
canonical path, coder ownership, mode, byte count and every hash; verified the
original repair manifest; inspected same-user executable/open-file inode
references and command arguments; and checked pending/staging absence.
It repeated the file and pending/staging guards immediately before deletion.
It neither removes directories nor uses recursive removal.

`before.tsv` retains observed ownership, modes, sizes and hashes.
`after.tsv` verifies absence of exactly those seven paths.
The process and pending/staging records, stdout/stderr, exit status, timestamps,
load and resource observations are retained separately. A JavaScript template
preparation error occurred before any gateway command or filesystem mutation;
it is documented in `preparation-note.txt`. There was only one cleanup run.

The gateway invocation was
`compiler-generated-source-cleanup-run-20261116-094`.
The prelaunch `command.txt` is an abbreviated command display.
`actual-command.txt` is explicitly a retrospective exact transcription of
the executed argv/redirection paths from that invocation.

## Why no operation credential must remain

The actual baseline producer's source commit was
`830896f51b449532e99c203a6531844fa399fb50`.
Its `src/structure_write.lkj` is byte-identical to the repair worktree's
writer: SHA-256
`96408572b6a687d3906abd946b9644fb11175f03b354c65cc1ec67f94fd26afc`.
The cleanup guards verified both source hashes.

The relevant source rules are narrow:

- `Writer::open` reads the external 32-byte key and acquires an empty
  `.lkjscript-edit.lock`.
- `Writer::pending` needs the key to authenticate the pending intent.
- `Writer::publish_prepared` derives the generation token from a fresh
  entropy nonce; it does not derive that token from the signing key.
- `Writer::complete` publishes that token and removes the authenticated
  pending intent after completion.
- `stamp` reads the completed generation token without a key.

The retained root had only an empty lock file and a 64-character lowercase
hexadecimal generation token. The preceding read-only inspection also verified
that the token was not the operation-key digest. The pending intent and
metadata/projection staging files were absent immediately before deletion.
Consequently no surviving edit metadata in this owned root requires the key.
This conclusion does not authorize removing credentials belonging to any
other root, outstanding intent or authenticated store.

The preserved metadata hashes are:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `.lkjscript-edit.lock` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `.lkjscript-edit.generation` | 64 | `b79e9acd141ea40b4e77e31ccc9735c0849a197a59d1d735aeff6545ac8aaa07` |

Their before/after manifest files compare exactly. The cleanup record contains
only key identity/path/hash metadata; no private key bytes were printed or
retained. No language-source lines changed in this follow-up.
