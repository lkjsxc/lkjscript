# Status — 2026-11-15

## Current milestone: 6.4 complete; next content-addressed reuse

build --profile samples CPU instruction/frame addresses into OUTPUT.samples.PID;
profile reports self/inclusive source functions in at most 40 lines without DWARF.
Owned capability entries require named grants; a sole World declares full authority.
Listener owns IPv4 TCP acceptance. authority follows checked-IR calls and owned cleanup.
HTTP: 8,017,696 verified requests in 3,600.002 s (2,227/s), with no errors.
All 121 post-warmup RSS samples and peak were 452 KiB; virtual size stayed unchanged.
Server profile: 788,786 samples; append_bytes 23.8%, slice equality 12.5%, header_end 9.2% self.
All 26 functional requests pass natively and in the interpreter (129.417 s for the latter).

Bare Debian gate without cargo/rustc/node: 153.050 s; bootstrap/fixed point 30.345 s.
706 conformance cases / 1,404 route checks; four intended pending specifications (eight routes).
805 generated cases / 2,430 checks; 711 semantic contracts; 436 IR cases / 872 checks.
13 reference cases / 26 checks; 19 authority fixtures / 38 checks plus explicit grant tests.
Optional Rust witness: 265 value/output + 321 verdict cases; compiler bytes agree.
Ubuntu 26.04 env symlink/multicall handling is fixed in the optional witness orchestration.
Long mutation: 32,768, zero findings; 50,812/minute (38.693 s, eight workers).
26,168 rejected; 6,041 equal values; 466 equal traps; 93 two-second timeouts; no resource failures.

Deleted at 6.3: Node gate/generators/reducer, cases.json, 195 repeated integer files,
44 Cargo integration files and duplicate unit/host scaffolding; Cargo lives in witness/.
language.md is 487 lines. Three scoped-concurrency cases and borrowed_closure_escape
remain intended pending rules; conservative holder liveness has a rejection and scope rewrite.

## Compiler measurements

| Current sources | Self-built | Rust-built witness |
| --- | ---: | ---: |
| Complete self-build | 16.15 s | 2.40 s |
| Peak RSS | 386.8 MiB | 273.9 MiB |
| Output bytes | 14,133,686 | Identical |
| IR | 8,802 functions / 767,765 instructions | — |

Stored seed: 2,711,289 compressed bytes; two native generations and the witness agree.
On unchanged 43cc8ae9 input, the profiled build improved 21.32 → 14.50 s (witness 2.03 s).
The profile found type-sized boolean initialization/cloning, then called aggregate copies.
Sparse query visits, packed snapshot facts and inline known-size copies remove those costs.
The old helper already used rep movsb. Optimization stopped under the two-cause rule.
The current gap is 6.7x. Sampled build: 16.12 s; typing 5.38 s, IR construction 4.56 s,
checking 4.73 s, emission 0.64 s. Remaining function samples (15,942 total):

| Function | Self | Inclusive |
| --- | ---: | ---: |
| byte-slice ordering | 3.9% | 3.9% |
| bytes::append_bytes | 2.6% | 4.0% |
| runtime::allocate | 2.0% | 5.3% |
| runtime::release | 2.1% | 2.7% |
| check::check_function_body | 1.0% | 43.3% |
| elaborate::compile_function | <0.1% | 35.4% |
| elaborate::infer_resolved_report | <0.1% | 33.4% |

Inclusive rows overlap; the remaining cost is spread across checking, inference and IR construction.
64-byte calls (10 M): native 0.36 s / rustc opt=0 0.30 s / rustc -O 0.10 s.
64-byte Vec (1 M elements, 10 M reads): 0.06 / 0.08 / 0.02 s, with equal results.

SHA-256 is shared source; standard, padding and streaming vectors pass on both routes.
Four-worker gate: 174.560 s, load 4.35 → 5.04. Next: authenticated checked-work reuse.
