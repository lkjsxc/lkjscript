# Status — 2026-11-15

## Compiler: authenticated reuse complete; seed refresh next

Shared SHA-256 passes standard, padding and streaming vectors on both routes.
The disposable store reuses checked bodies, relocatable emission and closed test successes.
HMAC binds exact compiler bytes, target, options and input closure; the private key is
outside source/program grants. Key readers and compiler replacers are equally trusted.
Shared IR structure is always checked; malformed, swapped, replayed and forged entries miss.
Gate cases cover compiler/key changes, missing/full stores, races and dependency changes.
Runtime/drop helpers still emit normally; unchanged whole images bypass assembly too.

| Measurement (four workers for gates) | Seconds | Load before → after |
| --- | ---: | ---: |
| Self-build, store disabled | 15.53 | 1.67 → 1.94 |
| Cold self-build | 20.92 | 1.94 → 2.19 |
| Warm, one checker function changed | 15.78 | 2.19 → 2.51 |
| Warm, unchanged | 1.32 | 2.19 → 2.19 |
| Cold gate, previous seed | 213.141 | 1.77 → 6.77 |
| Warm gate, previous seed | 114.747 | 3.34 → 4.06 |

Cold overhead: +5.39 s (+34.7%). One edit reuses 9,062 checks and 8,921 emitted bodies;
authentication/serialization currently consume the saved checking time. No edit speedup yet.
Two versions occupy 63,837,509 logical bytes, 121,652 KiB allocated, 17,990 files.
Compiler: 14,451,862 bytes; 9,063 functions / 787,647 IR instructions; fresh/cached identical.
Native fixed point and optional Rust witness agree; all cold/warm gate verdicts agree.
Warm-store mutation: 32,768, zero findings; 29,117/min (67.523 s, load 4.75 → 10.64).
26,168 rejected; 6,041 values; 466 traps; 93 nontermination allowances; zero resources.
24,966 checked and 9,850 emitted reuses; no rejected authenticated records in the campaign.
No store entries are committed. bench/reuse.sh reproduces the build/edit/size comparison.
Next: separate seed refresh, then compiler opt=0/-O triple and emitted-code-kind profiles.

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
