# Status — 2026-11-14

## Current milestone: 6.1, direct native backend

Milestone 6.0 is complete: lkjscript IR names throughout source, tools, tests and
references; .lkjir files require the first line `lkjscript-ir 1`; execution uses
`--engine interpreter`; the installed command is `lkjscript`.
Both native compiler components reproduce their bytes and match the Rust-built
witness output. The full eight-worker gate passes in 82.691 s: 462 Rust tests,
17 harness tests, 883 conformance passes and four pending on each route,
573 Rust comparisons, 91 World cases, zero IR mutation findings.

Deleted 31 observation files, the trusted command and its reporting tool.
AGENTS.md/direction/status hold the new course; docs/ir.md is the IR reference.
The gate prints one success line per suite and failure details.

Next: replace boxed aggregates and linear-memory execution with direct checked-IR
selection, inline aggregates, machine references and stack slots, rel32 control
flow, a source allocator over independent kernel mappings, and separate encoding.
Keep old/new/interpreter comparison until the replacement passes; then remove the
old lowering, encoder and resource-planning machinery in the same stretch.

## Rename checkpoint measurements

| Work | Self-built native | Rust-built twin |
| --- | ---: | ---: |
| Frontend source to IR | 216.54 s | 1.87 s |
| Frontend IR to ELF | 110.68 s | 0.90 s |
| Backend IR to ELF | 37.21 s | 0.31 s |
| Frontend source peak RSS | 766.1 MiB | 205.8 MiB |
| Frontend emission peak RSS | 735.1 MiB | 334.3 MiB |
| Frontend executable | 51,431,226 bytes | Identical emitted bytes |
| Backend executable | 16,845,618 bytes | Identical emitted bytes |
| Frontend own-source IR | 18,750,365 bytes | Identical bytes |

The review's 5068ad3a baseline remains the comparison for the new backend:

| Work | lkjscript | Rust comparison |
| --- | ---: | ---: |
| Hello build | 5.16 s | 0.05 s, twin |
| Complete native rebuild | 443.56 s | — |
| 20 M two-field Copy calls | 5.82 s | 0.031 s unoptimized; 0.009 s optimized |
| 200 M scalar iterations | 1.74 s | 0.17 s unoptimized; 0.05 s optimized |
| 100 M Vec reads / 5 M pushes | 0.97 s | 0.33 s unoptimized |

## Known gaps

Four pending cases: concurrency_moved, concurrency_shared, concurrency_alias,
borrowed_closure_escape. One conservative ownership rejection remains.
The current backend still boxes aggregates, uses heap frames and linear memory.
Resource traps lose source-file provenance at the host boundary.
The interpreter, gate and development tools still require the Rust/Node host.
No stored seed, single executable, HTTP consumer or entry grants yet.
