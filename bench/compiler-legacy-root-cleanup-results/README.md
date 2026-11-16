# Completed cleanup of seven old root-owned scopes

The single root-released invocation of committed guard
2caa0525e8a011ff5c014e27e1f10ba90cc151221ea3ea68ed157807ddb7ab37
completed successfully against bundle commit
cd1a63435ee856a334fc08340b4748c987f4838b.

- Start/end: 2026-11-16 13:49:12–13:49:15 UTC.
- Wrapper capture: 3.640244944 s; GNU time: 3.63 s wall, 2.12 s user, 0.81 s system, 76,536 KiB peak RSS.
- Load before/after: 3.39/4.56/3.45 → 3.28/4.52/3.44.
- Exit status: 0; guard invoked exactly once.
- Removed: 16,365 regular files, 15 directories, 365,876,759 logical file bytes.
- Included intentional disposable state: 16,049 cache files, 318,819,150 bytes, whose payloads were not archived.

The guard verified the exact committed inventory, recovery mapping and historical
seed before deleting only the seven enumerated scopes. At 13:50:35 UTC a separate
read-only check obtained ENOENT for each of the seven exact paths. All 27 original
retained bundle records still pass SHA-256 verification; that frozen manifest
remains 436360a7987b8d34a015dede97589b60cd5b89ef565ee1bc8fd51e9e60945807.

No compiler, process/FD observer, private-key read, fallback deletion or repeated
guard invocation occurred. Shared target, worktrees, current copy-site scopes,
target/gate and both excluded credentials were outside the guard's deletion set.
The owned worktree's tracked status is empty. No shared documentation or repository
commit was changed by this cleanup.

The capture directory preserves the exact command, release record, preflight checks,
stdout, stderr, status, UTC/nanosecond clocks, aggregate load, GNU time, retained-bundle
verification and postcheck. result.json is byte-identical to captured stdout and
has SHA-256 7b7289442832529f3dff1d852f8113209c005a319253566ab191e2c54a512aca.
The original inventory/preparation bundle remains separate and unchanged.
