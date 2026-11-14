# Status — 2026-11-14

## Current milestone: 6.3 complete; next 6.4

build --profile emits a static executable with an embedded function map and a
CPU sampler; profile OUTPUT.samples.PID reports self/inclusive source functions
in at most 40 lines. Counts survive exit/signals; recursive inclusion has no depth cap.
The profile identified type-sized boolean initialization/cloning, then called copies.
Sparse query visits, packed snapshot facts and inline known-size copies remove those
two costs. The old copy helper already used rep movsb. Stopping follows the two-cause rule.

Bare Debian gate without cargo/rustc/node: 130.308 s; bootstrap/fixed point 29.586 s.
699 conformance cases: 1,390 route checks, four pending specifications (eight routes).
805 generated cases: 2,430 route checks; 706 semantic contracts; 436 IR cases/872 checks;
13 reference cases/26 checks. Optional Rust witness: 264 value/output + 318 verdict cases.
Long mutation: 32,768, zero findings; 51,086/minute (38.485 s, eight workers).
26,168 rejected, 6,041 equal values, 466 equal traps, 93 exceeded the two-second allowance.

Deleted Node gate/generators/reducer, cases.json, 195 repetitive integer files,
44 Cargo integration files and duplicated unit/host scaffolding. Cargo/bridge live in witness/.
Reference programs are cases; language.md is 476 lines. Eager Option::or cleanup has
a permanent R|L case after the gate caught a wrong reference expansion.
Pending: three intended scoped-concurrency cases and borrowed_closure_escape.
Conservative holder liveness is a specified rejection with a lexical-scope rewrite.

## Compiler measurements

| Current sources | Self-built | Rust-built witness | Previous compiler |
| --- | ---: | ---: | ---: |
| Complete self-build | 15.19 s | 2.25 s | 23.83 s |
| Peak RSS | 437.8 MiB | 270.7 MiB | 434.4 MiB |
| Output bytes | 13,871,867 | Identical | Different emitter |
| IR | 8,688 functions / 751,815 instructions | Identical | — |

Stored seed: 2,657,484 compressed bytes; two native generations and the witness agree.
On unchanged 43cc8ae9 input, profiled build improved 21.32 → 14.50 s (witness 2.03 s).
The current gap is 6.8x. Its 15.40 s sampled build spends 5.19 s in typing, 4.21 s
building IR and 4.60 s checking; emission is 0.61 s. Remaining samples (15,184 total):

| Function | Self | Inclusive |
| --- | ---: | ---: |
| byte-slice ordering | 3.7% | 3.7% |
| bytes::append_bytes | 2.7% | 3.8% |
| runtime::allocate | 2.3% | 5.4% |
| runtime::release | 2.2% | 2.9% |
| check::check_function_body | 1.0% | 43.8% |
| elaborate::compile_function | <0.1% | 34.9% |
| elaborate::infer_resolved_report | <0.1% | 33.6% |

Inclusive rows overlap. The remaining cost is spread across checking, inference and IR construction.

| Workload (seconds) | Native | rustc opt=0 | rustc -O | Previous |
| --- | ---: | ---: | ---: | ---: |
| 10 M three-level 64-byte calls | 0.39 | 0.31 | 0.09 | 0.39 |
| 1 M 64-byte Vec elements + 10 M reads | 0.05 | 0.08 | 0.01 | 0.14 |
| 200 M scalar multiply/xor | 0.25 | 0.25 | 0.25 | 0.25 |
| 200,000 fresh 1,000-i64 buffers | 0.39 | 0.45 | 0.09 | 0.38 |

Shared-host single runs; outputs agree. Next: Listener, explicit capability entry grants,
checked-IR authority report, and a profiled hour-long local HTTP consumer run.
