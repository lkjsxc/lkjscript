# Performance and economy evidence

Measurements are observations, not promises. Current environment: Linux
`7.0.0-29-generic` x86-64, `rustc 1.96.0`, Cargo 1.96.0. The final measured release binary was
13,399,896 bytes with SHA-256
`f7d985142b4e019d22c5d3ab8a2bdfcdb99b7043a663edd5be2379aafefd13ab`.
CPU time, peak RSS, provider tokens/cache/requests/retries, and monetary telemetry were unavailable.
No token or money result is inferred from byte counts.

## Historical baselines

At campaign start `6747754bdcad4dc9000c3d7891db7ef207c8ec2f`, the source-authority lkjournal
workflow had these retained warm p50 observations: orientation 44.718854 ms and 1,356 output bytes;
package test 76.677093 ms and 311 bytes; service-module show 84.051330 ms and 40,322 bytes. The
package test covered the same 11 program tests and bytecode/AST differential contract now used by
the graph workflow. Source authority has since been deleted, so these figures are historical.

The older graph-product baseline emitted 37,897 bytes for one exact-function query, took about
998.5 ms for one orientation, and took about 730.6 seconds for its complete verification profile.
Those observations explain the selected local query protocol; they are not current product
benchmarks.

## Current maintained graph

Seven warm release samples were taken per command on 2026-08-22. Fresh unique output files were
used for build, review, and backup. The table reports p50 wall time and stdout plus stderr bytes.

| Public command | p50 | Output bytes | Derived file bytes |
|---|---:|---:|---:|
| `lkjournal semantic status` | 1.863 ms | 712 | — |
| `lkjournal semantic orient --limit 20` | 3.642 ms | 1,710 | — |
| `lkjournal semantic find handle --exact` | 1.684 ms | 645 | — |
| `lkjournal semantic show decl_ec993… --body` | 2.863 ms | 5,613 | — |
| `lkjournal semantic test` | 22.101 ms | 418 | — |
| `lkjournal semantic doctor --deep` | 5.217 ms | 299 | — |
| `lkjournal semantic build` | 17.505 ms | 607 | 160,195 |
| `lkjournal semantic text-project` | 14.639 ms | 630 | 1,328,003 |
| `lkjournal semantic backup` | 11.141 ms | 476 | 160,697 |

The source and graph orientation commands are the same user objective but differ in returned
fields: graph orientation was about 91.9% lower wall time and 354 bytes larger. The package-test
task retained the same 11 tests and differential obligation: graph time was about 71.2% lower and
output was 107 bytes larger. These two comparisons do not establish universal graph superiority.
The source service-module show and graph exact-function show are not equal tasks and are not used
as a performance ratio.

The canonical standard authority is 21,062 bytes across 16 transportable files. Canonical
lkjournal authority is 160,419 bytes across 8 files. Current disposable query indexes add 50,858
bytes for standard and 790,720 bytes for lkjournal. The explicit full review projection is larger
than canonical authority and is therefore out-of-band, never routine command output.

## Scale topology

`tools/semantic-scale` creates deterministic empty modules only through public ID allocation rules,
graph-artifact import, and exact-base semantic transactions. It then runs public orient, exact
find/show, deep doctor, build, and backup. Raw structured results are retained at
`docs/evidence/20260822-semantic-scale.json`.

| Observation | 10,000 added modules | 90,000 added modules |
|---|---:|---:|
| first 10,000-module apply | 462 ms | 469 ms |
| final 10,000-module apply | 462 ms | 10.949 s |
| apply response | 5,378 bytes | 5,378 bytes each |
| cold exact local-index lookup | 73.5 ms | 630 ms |
| warm exact lookup, work = 1 | 15.8 ms | 138 ms |
| exact show | 13.7 ms | 134 ms |
| orientation | 129 ms | 1.283 s |
| deep doctor | 93.5 ms / 10,024 module versions | 3.661 s / 450,120 module versions |
| deterministic build | 132 ms / 1,330,505 bytes | 1.269 s / 10,370,515 bytes |
| backup | 98.4 ms / 2,183,176 bytes | 3.995 s / 44,315,558 bytes |
| canonical store | 1,832,489 bytes | 41,163,949 bytes |
| store with derived indexes | 6,631,953 bytes | 82,844,955 bytes |

The original file-by-file durability prototype took 20.523 seconds for the identical first
10,000-module revision. Batched Linux filesystem synchronization reduced that to 462 ms without
changing its deterministic revision or artifact digest. The final 90,000-module batch still grows
to 10.949 seconds because accepted publication reconstructs and validates the complete candidate
and verifies retained immutable objects. This is a recorded incremental-engine defect/limit; no
timeout or work budget was raised to hide it.

Graph contract 1 caps one root at 100,000 modules, so the requested million-module topology is
outside current resource policy and was not run. The retained topology covers many tiny modules;
dense calls, large literals/docs, branch conflict fanout, compaction, and million-owner behavior
remain unmeasured.

## Compiler and service

Current graph package tests pass 6 standard and 11 lkjournal cases with bytecode/reference equality.
The seven-sample command table measures complete preparation and test execution, not an isolated
instruction microbenchmark. The old small-sample source-era bytecode p50 advantage remains
historical; no new percentage claim is made from the short current runs.

The final graph-cutover PostgreSQL service/worker acceptance passed in 4.431 seconds at
`.artifacts/service/20260822T061313.861237Z-420089/receipt.json`. It checked 13 route/failure
observations, exact update/history, one 200,000-byte object, one productive worker iteration, zero
shutdown cleanup failures, a 12,520-byte PostgreSQL backup, restore, restart, and equal restored
read. This is one complete observation, not a latency distribution. Live S3, sustained overload,
RSS, database contention, multipart throughput, multi-hour worker behavior, and p95 service
latency remain unmeasured.

## Verification and agent economy

Normal exact lookup is hundreds of bytes, body expansion is explicit, broad transaction output
inlines at most 64 affected owners, package tests return aggregates, and all-pass verification emits
one aggregate result plus a retained receipt path. `tools/check` stores separate bounded logs under
`.artifacts/check`; current full verification does not reuse a prior pass.

The final authoritative local profile passed 15/15 gates in 94.743 seconds at
`.artifacts/check/20260822T061143.565460Z-417922/receipt.json`. It binds worktree input SHA-256
`d822a652537d07e1859f0faea1e481b6d0e037f93f0eab3452416147074d0910`; documentation updates after
that run require one final fresh verification before publication.

No complete provider-instrumented comparison exists for the campaign's full edit/refactor matrix.
Commands, corrections, elapsed time, request/response bytes, and stable-ID continuity were observed
for orientation, exact lookup/show, mutation, build, doctor, review, and backup. Provider token and
cost superiority remains unknown.
