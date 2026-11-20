# Completed cleanup of seven old compiler scopes

The [committed proposal](compiler-legacy-root-cleanup-data/README.md) was released
once using bundle commit `cd1a63435ee856a334fc08340b4748c987f4838b` and guard
`2caa0525e8a011ff5c014e27e1f10ba90cc151221ea3ea68ed157807ddb7ab37`.
It removed **16,365 regular files and 15 directories**, totaling
**365,876,759 logical bytes**, exclusively inside the seven named old root-owned
scopes. A separate postcheck and root verification confirm all seven are absent.

The operation ran at 2026-11-16 13:49:12–13:49:15 UTC. GNU time reports
**3.63 s wall, 2.12 s user, 0.81 s system and 76,536 KiB peak RSS**, with load
**3.39/4.56/3.45 → 3.28/4.52/3.44**. The wrapper's separate elapsed observation
is 3.640244944 s over that same load window. No compiler workload overlapped.

All non-cache files have exact committed recovery: 309 prior evidence blobs,
the historical Git seed/canonical bytes and the newly retained landing/control
records. The **16,049 cache files / 318,819,150 bytes** were deliberately discarded
as disposable store state; their payloads are not archived or claimed exactly
recoverable. The proposal preserves this distinction and the full original
path/type/metadata/hash inventory.

The [25-record completion manifest](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-legacy-root-cleanup-results/manifest.sha256)
is `af97a24d0eb7a3a32df2e9da134782ea30960a9c106ebda6d3b397a983ad2e0e`.
Root verifies it and the seven final absences. The [result](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-legacy-root-cleanup-results/result.json)
is `7b7289442832529f3dff1d852f8113209c005a319253566ab191e2c54a512aca`;
the separate postcheck is
`555e55a6e032442b4208d1b1251d1c5e69a081bbf663e5848c43ac51c24ca6d6`.
All 27 committed preparation records still verify.

Current copy-site evidence/canonical, target/gate, shared worktrees, the global
store key and unrelated operation credential were outside the deletion set.
The guard used exact inventories and the known completed owned commands;
there was no process/FD observer, private-key read or fallback cleanup route.
