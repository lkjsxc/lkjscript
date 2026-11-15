# Status — 2026-11-15

## Compiler: authenticated reuse complete; profiling next

Shared SHA-256 passes standard, padding and streaming vectors on both routes.
The store reuses checked bodies, relocatable emission, images and closed observations.
HMAC binds compiler bytes, target, options and full inputs; producer slots coexist.
The private key stays outside source/program grants; key readers are trusted as compiler replacers.
Shared IR structure is always checked. Corpus roundtrips compare fresh/cold/warm bytes and verdicts.
Gate cases reject hostile records and cover dependency changes, refused/disappearing stores and races.
Only embedded-runtime closed executions reuse returns/traps; effects and resource failures rerun.

| Measurement (four workers for gates) | Seconds | Load before → after |
| --- | ---: | ---: |
| Self-build, store disabled | 15.29 | 7.14 → 7.43 |
| Cold self-build | 21.09 | 7.43 → 7.69 |
| Warm, one checker function changed | 16.21 | 7.56 → 7.62 |
| Same edit, store disabled | 16.13 | 7.62 → 7.18 |
| Warm, unchanged | 1.44 | 7.69 → 7.56 |
| Cold gate, refreshed seed | 212.913 | 10.04 → 11.42 |
| Warm gate, refreshed seed | 110.015 | 11.42 → 7.14 |

Cold overhead: +5.80 s (+37.93%). One edit reuses 9,127 checks and 8,986 emitted bodies;
authentication/serialization consume the saved checking time: edit latency remains at fresh-build cost.
Store: 49,949,909 bytes cold; 64,564,271 after two versions, including directory metadata.
Compiler: 14,613,309 bytes; 9,128 functions / 797,533 IR instructions; cached/fresh images identical.
Seed: 2,816,325 bytes; two native generations and the Rust witness agree. No entries are committed.
Warm-store mutation: 32,768, zero findings; 29,433/min (66.797 s, load 7.18 → 9.73).
26,168 rejected; 6,041 values; 466 traps; 93 nontermination allowances; zero resource failures.
24,966 checked and 9,850 emitted reuses; no rejected records. Runtime/drop helpers still emit normally.
Deleted the transitional seed probe. bench/reuse.sh reproduces the snapshot/edit/size measurements.
Next: the three diagnostic rewrites, then the compiler opt=0/-O triple and emitted-code-kind profiles.

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
