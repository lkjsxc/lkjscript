# Status — 2026-11-14

## Current milestone: 6.2, one binary and a stored seed

Milestones 6.0 and 6.1 are complete. The only backend selects x86-64 directly
from checked lkjscript IR: inline aggregates, machine references and stack
frames, rel32 control flow, and a source allocator over kernel mappings.
src/x86.rs separates encoding; --stats reports phases/RSS/sizes. Frontend, backend and result-reader executables match the Rust-built twin's bytes;
frontend/backend fixed points and own-source/runtime IR identity hold.

The eight-worker gate passes in 176.680 s: 452 Rust tests, 17 harness tests,
884 conformance passes plus four pending on each route, 573 Rust comparisons,
91 World observations and all source generators. Of 128 IR mutants, 100 reject,
27 agree and one times out in the interpreter; native is skipped for that case.
The shared 120 s watchdog accounts for most gate time. There are zero findings.

Deleted the operand-stack lowering, old encoder/linear-memory model, startup
extent arithmetic, budget variables/tests, compiler-stack host code, census
machinery, five unused tool scripts and native/frontend/limits documents.
The language/IR references and --help describe current behavior. Old observation
files and the trusted reporting command were removed in 6.0.

Next: combine frontend, checker, backend, interpreter and CLI in lkjscript source.
Add process/clock/directory mutation capabilities for the driver; port the
interpreter to owned values/indices while preserving its independent checks.
Pass IR in memory and check once, embed library/runtime, then store a compressed
seed and verify a Rust/Node-free rebuild before removing the component pipeline.

## Compiler measurements

| Work | Self-built | Rust-built twin | Previous self-built |
| --- | ---: | ---: | ---: |
| Hello build | 0.62 s | 0.04 s | 5.16 s |
| Complete component rebuild | 59.08 s | 3.32 s | 443.56 s |
| Frontend source to IR | 34.47 s | 1.73 s | 216.54 s |
| Frontend IR to ELF | 12.73 s | 0.72 s | 110.68 s |
| Backend IR to ELF | 3.67 s | 0.24 s | 37.21 s |
| Frontend source peak RSS | 262.1 MiB | 196.0 MiB | 766.1 MiB |
| Frontend executable | 16,742,209 bytes | Identical | 51,431,226 bytes |
| Backend executable | 4,429,265 bytes | Identical | 16,845,618 bytes |

Previous compiler rows are the rename checkpoint, except hello/full rebuild
from the supplied review. The twin uses Rust standard collections; lkjscript
uses its indexed AVL map, so the 18–20x compiler ratio includes library differences.

| bench/ workload | Native | Previous native | rustc opt=0 | rustc -O |
| --- | ---: | ---: | ---: | ---: |
| 20 M aggregate calls | 0.134 s | 5.708 s | 0.023 s | 0.018 s |
| 200 M scalar iterations | 0.911 s | 1.414 s | 0.260 s | 0.262 s |
| 5 M pushes + 100 M Vec reads | 0.535 s | 0.912 s | 0.263 s | 0.073 s |

These are identical runtime-seeded sources across routes, with equal output.
Vector RSS rose from 41.9 to 65.8 MiB during allocate-copy-release growth.
Native selection still spills all IR locals; scalar parameter staging is redundant.

## Known gaps

Pending: concurrency_moved/shared/alias and borrowed_closure_escape. One conservative
ownership rejection remains. Interpreter/gate/tools still need
Rust/Node; the single binary, stored seed, HTTP consumer and entry grants are next.
