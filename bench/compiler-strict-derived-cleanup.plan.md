# Prepared cleanup of the accepted strict-derived fixture

**Prepared, not executed.** Root accepted the one-case proof after independently
running the frozen proof and live archive auditors. The result was integrated as
`d2e4cd11`, acceptance/status as
`38cac5f5488b51b57004d7cf6029bc7ceed9554f`.
The root acceptance SHA-256 is
`0d56e2b844b1b1f1db0a8e1f317474ccdf1e2d2c945f9e9ba1b83978bb1ca2ef`;
its 16-record manifest is
`19a72fb8b61973f867b0cc7b3e483f387e2de9e7613ef0abcb12fae0f0a391ca`.

## Exact disposable scope

Only this literal owned directory is proposed for removal:

```text
/home/coder/workspace/lkjscript-compiler-strict-derived-20261116/target/compiler-strict-derived.ZQv7cu
```

It contains **3,706 regular files / 71,949,767 logical bytes**: the copied
32,196,802-byte producer, two accepted historical archives, the private
2 GiB-capacity store (actual files are smaller), a 32-byte operation/store key,
and the owner/PID markers and completed transaction metadata. The target
directory has no other result mirror. No shared parent directory, canonical
producer, query fixture or other worktree is in the deletion set.

The copied producer SHA-256 remains
`34491766f5258dd3af3caf92db6b92dfeba09b405c00a8dd19cd955205c0d377`.
The private fixture root is owned by the current uid with mode 0700; all files
are owned regular files with link count one. No symlink, special file, mounted
subtree, pending intent or staging leaf is admitted by the inventory.

## Completed metadata and key lifetime

The before archive's empty `.lkjscript-edit.lock` and 64-byte
`.lkjscript-edit.generation` have been copied byte-identically into
`compiler-strict-derived-cleanup-data/metadata/`. Those durable evidence copies
will remain after deletion. Their original files are included in the accepted
disposable historical archive. There is no worktree-root edit metadata.
All existing campaign records and raw proof remain byte-identical.

The executing producer's pinned `src/structure_write.lkj` at `62fa76f4`
has SHA-256
`96408572b6a687d3906abd946b9644fb11175f03b354c65cc1ec67f94fd26afc`;
the exact source excerpts are retained beside this plan.

- `Writer::pending` uses the private secret to authenticate an existing
  `.lkjscript-edit.pending`; it returns None on ENOENT.
- `Writer::publish_prepared` derives the generation from a fresh entropy
  nonce, separately sealing the pending intent with the private secret.
- `Writer::complete` publishes that generation, completes and synchronizes
  the changes, clears stages, removes the pending intent and synchronizes again.
- `stamp` reads and validates the generation after checking for pending intent;
  it does not read a private key.

The completed generation is therefore not an authenticated object requiring
the operation key for future reading. No pending intent/stage remains; the only
other key-dependent objects are in the disposable private store. The key has no
retained recovery obligation in this fixture. The earlier inspection recorded
a SHA-256 only; key contents were never printed or copied. The prepared guard
and inventory deliberately **do not read the key contents**, validating only
its exact owned inode, mode, length and single-link metadata.

## Guard and release boundary

The new guard accepts only `--execute-after-root-release`, but that argument
does not itself constitute authorization. It must not be invoked until root
explicitly releases cleanup.

Before deletion it verifies the frozen campaign/preparation/producer manifests
and this cleanup manifest; compares the complete live fixture identity/content
fingerprint (private-key content excluded) with the prepared snapshot; checks
owner marker and original PID absence; compares both metadata copies; and
rejects any same-user process command line, mapping, executable, cwd/root or
descriptor referring to the fixture. The guard excludes only its own process
ancestry. Inspection failure on a still-live same-user process is a refusal.

Deletion is a single literal `rm -rf --one-file-system -- FIXTURE`. The guard
then requires absence and re-verifies the retained evidence and metadata.
It records start/end UTC, load and elapsed time only when released. It neither
runs the compiler nor invokes recovery or changes a campaign result.

No cleanup command has run. The existing replay report and all frozen result
records retain their historical wording. A separate completion record will be
added after an authorized removal; this preparation will not be rewritten.
