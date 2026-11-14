# Status — 2026-11-14

## Current milestone: 6.2, one executable and a stored seed

6.1 is complete: cold trap locations, interval-allocated scalar registers,
straight-line forwarding, effect-free operation removal and closed operation enums.
The source allocator reuses geometric regions and medium classes; large buffers
use mremap. Stack reservation adapts, yields unused pages on heap refusal and
regrows without moving live frames. Hello runs under 4 GiB; a 16 MiB vector plus
recursive frames and live references works under 32 MiB.

Eight-worker gate: 80.011 s, including both compiler fixed points, twin byte
identity and own-source/runtime IR identity. 464 Rust tests, 17 harness tests,
886 conformance passes plus four pending per route, 574 Rust comparisons,
91 World observations and all generators pass. Fixed mutation slice: 128,
101 rejected and 27 equal. Long campaign: 16,384, zero findings; 13,052 rejected,
3,289 equal values/traps and 43 did not terminate within their two-second allowance.

Removed hot location writes, redundant scalar frame traffic/no-ops, string
operation dispatch, Clippy configuration/gate, the misleading library-gap claim
and three empty obsolete directories. No optimizer was added.

Next: combine frontend/checker/backend/interpreter/CLI into one executable with
in-memory checked IR. Port interpreter values to owned indices; add embedding
syntax and World-derived process/clock/directory operations. Then store a compressed
seed and delete the component launcher, installer, reader and packet/bundle plumbing.

## Compiler measurements

| Work | Self-built | Rust-built twin | At 98462339 |
| --- | ---: | ---: | ---: |
| Hello build | 0.37 s | 0.05 s | 0.62 s |
| Component rebuild, sum of phases | 45.38 s | 3.85 s | 59.08 s |
| Frontend source to IR | 26.35 s | 1.96 s | 34.47 s |
| Frontend IR to ELF | 9.99 s | 0.80 s | 12.73 s |
| Backend IR to ELF | 2.46 s | 0.33 s | 3.67 s |
| Frontend source peak RSS | 287.3 MiB | 198.8 MiB | 262.1 MiB |
| Frontend executable | 9,320,393 bytes | Identical | 16,742,209 bytes |
| Backend executable | 2,438,833 bytes | Identical | 4,429,265 bytes |

Single runs on the shared host; repeated unchanged workloads varied materially.
A separate backend check spends 6.45 CPU seconds parsing/rechecking the frontend's
already-checked IR: the next structural cost to remove in 6.2. Paired frontend
checks take 17.10 CPU seconds self-built, 7.01 Rust debug and 1.63 Rust release.

| bench/ workload | Native | At 98462339 | rustc opt=0 | rustc -O |
| --- | ---: | ---: | ---: | ---: |
| 20 M aggregate calls | 0.066 s | 0.137 s | 0.024 s | 0.022 s |
| 200 M scalar multiply/xor | 0.262 s | 0.938 s | 0.261 s | 0.258 s |
| 200 M scalar add/xor/mask | 0.177 s | 0.834 s | 0.137 s | 0.131 s |
| 200,000 fresh 1,000-i64 buffers | 0.344 s | 1.771 s | 0.553 s | 0.107 s |
| 5 M pushes + 100 M Vec reads | 0.180 s | 0.598 s | 0.272 s | 0.072 s |

Identical runtime-seeded sources and equal output across routes. Buffer kernel
CPU time fell from 0.80 s to below 0.01 s; vector peak RSS fell from 67.1 to 43.1 MiB.

## Known gaps

Pending: concurrency_moved/shared/alias, borrowed_closure_escape and one conservative ownership rejection.
Interpreter/gate/tools need Rust/Node; single binary, seed, independent gate, HTTP and entry grants follow.
