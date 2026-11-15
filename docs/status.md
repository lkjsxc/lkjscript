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

2.2 complete: affine Readiness, nonblocking handles and explicit owned HTTP connection states.
1,000 open connections (990 silent, ten active), pipeline 32, 40,108-byte responses.
Ten-second phases: 22,518/s unloaded, 22,032/s loaded, 21,450/s unloaded; loaded is 100.22% of bracket mean.
Run load 2.92/4.16/5.41 -> 3.19/4.08/5.33; loaded phase began at 2.78/4.09/5.37.
RSS 1,092 -> 3,224 KiB: 2,205.2 bytes per silent peer, including allocator growth, excluding kernel sockets.
Default idle/header-body/write policy is 60/2/2 s; progress never extends request or write deadlines.
Slow header closed at 2.003 s; all hostile server handles observed gone by 2.351 s; load 3.12/4.09/5.34 -> 3.19/4.08/5.33.
Useful traffic during hostile peers: 104,274 / 5.000 s = 20,854/s; same load range; no active peer starved.
41 functional/pipelined requests pass on both routes; the slow interpreter fixture explicitly uses 60/120/120 s.
Interpreted suite 181.654 s, load 3.19/4.08/5.33 -> 4.00/4.21/5.17; native deadlines stay at defaults.
Independent broker operates actual sockets, preserves process streams/closed handles and reaps children; no relay queues.

| 2.1 server function (self samples) | Before | After, full |
| --- | ---: | ---: |
| bytes::append_bytes | 78.7% | 0.9% |
| io::file_read_to_end | 3.6% | removed |
| buffer::ReadBuffer::read | absent | 48.8% |
| platform::file_write | 3.4% | 20.8% |

2.1 full-body 6,161 -> 11,506/s; 17,513/15,001 samples; baseline load 8.67/6.26/4.40 -> 6.01/5.87/4.33, after 3.89/4.54/4.31 -> 3.67/4.44/4.29.
2.1 framing 21,975/s, load 3.67/4.44/4.29 -> 2.83/4.16/4.20; numeric Connector grants and lkjscript clients replaced curl.
Upstream-seed reproduction and four-worker gate pass: 258.696 s; load 6.53/4.29/4.65 -> 7.26/6.02/5.28. HTTP: 199,113 bytes, 205 functions, 14,623 IR instructions.
Deleted relay queues and three scoped-thread pending cases; eight actionable diagnostic cases added.
Measurement servers/clients/helpers were reaped; fresh listener ports and temporary measurement directories are gone.
Next: crash-durable journal state and JSON, then graceful stop, sessions and the documentation candidate.
