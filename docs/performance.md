# Performance and economy evidence

Measurements are observations, not promises. Environment: Linux `7.0.0-29-generic` x86-64, 20
logical CPUs, `rustc 1.96.0`, Cargo 1.96.0, Python 3.12.3. Process RSS and provider token, cache, and
price telemetry are unavailable. No token or money claim is inferred from bytes.

## Reproduced predecessor baseline

At starting commit `cc9b465227237a1600e9f2cb4e8e7f85ae59093a`, release `lkjedit` measured
27.768608630 s for 10,000 mixed transitions, 4.405541596 s for 1,006 growing-insert transitions,
2.815967447 s for 795 transitions over 100 tabs, 83.456675 ms for a 65,536-scalar paste, and
51.612490 ms for a 1-by-1 resize. One large-editor orientation took 998.527023 ms and emitted 1,247
bytes; an exact function query emitted 37,897 bytes. Its public backup retained 51,704,083 bytes.

The predecessor full check passed 15 gates in 730.608 s. Workspace tests took 537.914 s, editor
deep doctor 108.278 s, and editor acceptance 75.424 s. Its receipt is
`.artifacts/check/20260821T122930.983038Z-4129073-full/receipt.json` in the campaign worktree.

These workloads are retained baseline evidence only: the products and implementation were deleted,
so the service cutover is not presented as an equal editor-performance improvement.

## Current source and package workflows

Five warm release samples were taken per command on 2026-08-21. The table reports p50 and the bytes
written to stdout plus stderr.

| Command | p50 | Output bytes |
|---|---:|---:|
| `standard project orient` | 3.365969 ms | 1,621 |
| `lkjournal project orient` | 44.718854 ms | 1,356 |
| `standard package test` | 3.930802 ms | 307 |
| `lkjournal package test` | 76.677093 ms | 311 |
| `standard project doctor --deep` | 1.796068 ms | 436 |
| `lkjournal project doctor --deep` | 1.817768 ms | 441 |
| `lkjournal component inspect service.Web` | 78.800446 ms | 7,714 |
| `lkjournal module show service` | 84.051330 ms | 40,322 |

The standard authority is 38 files and 31,950 bytes; lkjournal authority is 48 files and 300,919
bytes. Those counts include content-addressed accepted history, not working source alone. The checked
exact dependency artifact is 9,602 bytes and the complete service artifact is 41,587 bytes.

Thirty-one warm release package-test samples compared the same test expressions inside one process.
For `standard`, bytecode p50 was 22,732 ns and AST-oracle p50 28,895 ns (bytecode 21.3% lower). For
`lkjournal`, bytecode p50 was 88,997 ns and oracle p50 134,712 ns (bytecode 33.9% lower). Ranges were
17,151--33,985 ns versus 23,475--32,752 ns for standard and 80,514--113,864 ns versus
131,227--220,714 ns for lkjournal. The test sets are small, but the equal in-process comparison
meets the predeclared retain gate and package tests continue to compare results/instruction facts.

## Live service workflow

The current isolated PostgreSQL acceptance at
`.artifacts/service/20260821T171340.084371Z-131501/receipt.json` passed in 3.160614082 s. It checked
13 route/failure observations, created and exactly updated one durable resource, reconstructed two
snapshot entries, streamed and verified a 200,000-byte object, completed one queue attempt, stopped
service and worker with zero cleanup failures, produced a 12,520-byte PostgreSQL custom backup,
restored it to a second database, restarted, and read revision 1 unchanged.

After closing a reproduced first-connection/readiness race, ten consecutive equal fresh-container
runs passed; their retained receipts span
`.artifacts/service/20260821T171057.675379Z-119920/receipt.json` through
`.artifacts/service/20260821T171137.972457Z-126746/receipt.json`. This is stability evidence, not a
tail-latency distribution.

That receipt records nanosecond latency for health, bootstrap denial, initialization, login,
create/list/read/update/stale/history/unauthenticated/strict-JSON/object and restored-read requests.
One sample is not a p95 or throughput claim. Startup, database pool contention, S3 first-byte,
multipart throughput, sustained overload, RSS, and multi-hour worker behavior remain unmeasured.

## Agent and verification economy

Current `module show` loads one authored module rather than a full semantic graph, and orientation
names modules, declarations, exact dependencies, targets, and expansion commands in 1.3--1.6 KiB.
`package test` emits one aggregate differential result rather than passing-case lines.

`tools/check` contract 2 supports `focused`, exact conservative `changed`, `product`, `service`, and
`full`. Default all-pass output is one line plus a receipt locator. Each gate keeps separate logs,
bounded to 64 MiB combined; the newest eight runs are retained. Pass receipts are keyed by current
worktree digest, Git head, command, toolchain, and selected environment facts, but no cross-run pass
reuse is implemented. Uncertain changes widen to full.

There is no equal-task old/new edit study with provider request counts or correction depth. The
available old graph queries and current source-module commands differ in task and program size, so
their byte reduction is not claimed as an equal comparison. Provider telemetry remains unavailable.

## Reversal gates

- Retain content-addressed source history while current open/deep doctor remain bounded and the
  object/manifest publication matrix stays simpler than reconstructing a journal. Revisit after a
  maintained package exceeds 10,000 modules or one accepted apply/open p50 exceeds 250 ms.
- Retain bytecode as the production route only while differential tests agree and equal release
  workloads show no material regression. Revisit specialization/JIT only after execution is at
  least 30% of a complete maintained workload.
- Retain resident prepared-program reuse while admission, shutdown, and corruption fallback remain
  exact. A cache may be added only with artifact/toolchain identity and a reference fallback.
- Retain local whole-response HTTP until a maintained response exceeds the 4 MiB deployment bound
  or response buffering dominates retained memory; then implement task-scoped response streaming.
