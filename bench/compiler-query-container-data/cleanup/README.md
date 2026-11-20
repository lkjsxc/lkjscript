# Accepted container fixture cleanup

The root accepted the fixed container experiment after independently checking all
1,666 final result-manifest records, the exact 1,646-file raw copy, four live
store censuses, emitted image pairs, final source pins, and producing executable
pins. The report and both historical proof manifests remain unchanged.

On 2026-11-16, the guarded cleanup removed exactly the eleven paths listed in
[accepted-cleanup.json](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-query-container-data/cleanup/accepted-cleanup.json): the complete measured run, the
tiny helper run, their two external private-key directories, five copied compiler
executables, and the standalone case-validation `src`/`library` copies.
The measured run included all four stores, generated source trees, factory
archives, emitted images, generator, and local producer copies.

Before any removal, the script verified:

- The 789 implementation records, 1,666 result records, and 215 standalone
  source/library copy hashes.
- Exact run owner-repository, owner-evidence, owner-key, and key owner-run markers;
  current-user ownership, real canonical directories, and private-key size/mode.
- All five executable digests against the frozen proof records.
- No observed live process whose command or working directory referred to a
  removal path, excluding the cleanup tool's own ancestry.
- Every removable descendant was an owned regular file or directory, with no
  symlinks or durable `.lkjscript-edit.*` metadata.

Every named path was checked absent afterward. The deletion inventory records
78,358 regular files and 2,437,428,197 logical file bytes. This is a logical-byte
sum, not a claim about physical blocks reclaimed.

Both worktree and case-validation `.lkjscript-edit.lock` and
`.lkjscript-edit.generation` files remain byte-identical. The case-validation
directory now contains only those durable metadata files. The separate operation
key directory `/tmp/compiler-container-key.m4u6F6` remains because it belongs to
the retained worktree's durable edit generation; it is not either disposable
experiment key. No key contents were read or copied into evidence. Global/main
keys, other worktrees, services, and all committed and intentional text evidence
were untouched.

The first guard attempt stopped before any removal because it interpreted the
implementation manifest's relative paths against the worktree rather than the
data directory. Its script and exact logs are retained in `guarded-cleanup.pl`
and `attempt-1/`: **0.01 s**, load **3.80,4.33,5.16 → 3.80,4.33,5.16**.
The corrected base-only guard is `guarded-cleanup-2.pl`; it completed in
**2.52 s**, load **3.47,4.20,5.09 → 3.47,4.20,5.09**, with all eleven removals
verified. This was filesystem cleanup, not a compiler, test, query, or benchmark
rerun.

The cleanup record is a separate documentary commit. It does not modify the
accepted experiment, its reported observations, its implementation source, or
the deferred current-compiler query protocol.
