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

## Programs

2.1 complete: exact numeric Connector grants, persistent HTTP and lkjscript clients.
41 functional/persistent requests pass; the interpreted server run was 130.491 s,
load 4.88/4.27/4.23 -> 3.09/3.76/4.04. Socket-option cases also pass both routes.
Full-body baseline: 184,896 / 30.006 s = 6,161/s; load 8.67/6.26/4.40 -> 6.01/5.87/4.33.
Full-body after: 345,216 / 30.003 s = 11,506/s; load 3.89/4.54/4.31 -> 3.67/4.44/4.29.
Framing after: 659,296 / 30.001 s = 21,975/s; load 3.67/4.44/4.29 -> 2.83/4.16/4.20.
All runs: one persistent connection, pipeline 32, 40,108-byte body; server RSS 372 -> 408 KiB.
Full verification used 50% server / 94% client CPU; framing used 88% / 19%, draining every byte.

| Server function (self samples) | Before | After, full |
| --- | ---: | ---: |
| bytes::append_bytes | 78.7% | 0.9% |
| io::file_read_to_end | 3.6% | removed |
| buffer::ReadBuffer::read | absent | 48.8% |
| platform::file_write | 3.4% | 20.8% |
| read_header | 3.7% | 7.7% |

Profiles contain 17,513 / 15,001 CPU samples. Owned chunks removed byte-at-a-time assembly.
accept_nodelay removes the introduced 42-ms packet stalls; stop after these two causes.
Removed curl execution and --curl. Three pending diagnostic cases record concrete rewrites.
Unchanged-seed fixed point and four-worker gate pass: 184.141 s; load 1.38/3.02/3.76 -> 7.10/5.76/4.74.
The independent interpreter helper drains acknowledged writes (32 MiB delayed-reader proof);
delete that adapter once the compiler stream's stored seed admits direct Connector calls.
All measurement/verification processes were reaped and their temporary ports/directories removed.
Next: readiness for many clients, then durable journal state, stop/sessions and the origin candidate.
