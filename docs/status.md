# Status — 2026-11-15

## Compiler: reuse and diagnostic rewrites landed; optimizer baseline measured

Reuse gate cold/warm: 212.913/110.015 s, load 10.04→11.42 / 11.42→7.14; both CI jobs pass.
Reuse self-build cold/edit/unchanged: 21.09/16.21/1.44 s, load 7.43→7.69 / 7.56→7.62 / 7.69→7.56.
Cold overhead +37.93%; two versions 64,564,271 bytes. Warm 32,768 mutations: zero findings.
Three consumer diagnostics now carry tested JSON byte edits; receiver evaluation and closure scope survive.

| Compiler self-build before Readiness, store disabled | Seconds | Load 1/5/15 before |
| --- | ---: | --- |
| Self-built | 15.93 | 2.09/4.44/4.80 |
| Rust opt=0, one codegen unit | 8.41 | 1.91/4.29/4.74 |
| Rust -O, one codegen unit | 2.23 | 2.00/4.23/4.71 |
Outputs agree byte for byte: native is 1.89x opt=0 and 7.14x -O; the gap is not only optimization.

| Before Readiness: exclusive emitted-code samples | Self-build % | HTTP server % |
| --- | ---: | ---: |
| Inline copy | 19.7 | 3.2 |
| Checks | 1.1 | 0.5 |
| Call setup / prologue / return | 15.0 | 2.6 |
| Frame load/store | 4.3 | 17.4 |
| Other memory | 10.5 | 3.7 |
| Arithmetic / registers | 34.6 | 36.9 |
| Branch | 2.6 | 6.8 |
| Allocator | 9.5 | 1.1 |
| Runtime | 2.2 | 27.4 |
| Unknown | 0.0 | 0.0 |
Self: 15,486 samples, 15.86 s, load 3.43→2.89; HTTP: 14,719, 11,746/s over 30.002 s, load 4.17→3.38.
Copies/calls/runtime own their internal frame accesses; arithmetic includes register moves and addresses.
Combined Readiness/profiling gate: 253.475 s, load 1.53→4.90; seed bootstrap and every suite pass.
Next: scalar Clone→Read, then measured rechecked inlining; retain the original interpreter oracle.

## Programs

2.4 complete: Stop, Entropy, private file creation, signed wall time and HTTP dates reach both executors and authority reports.
Shutdown drains admitted requests to a fixed deadline, syncs and exits 0; native target is the program, interpreter target its broker.
Native hostile-stop proof: 2.010 s, load 6.16/5.15/4.96 unchanged; interpreted partial-request stop: 0.650 s, load 2.39/3.63/4.44 unchanged.
Journal writes require signed expiring tokens before body retention and again before mutation; the separate issuer holds key-directory authority.
Issuer/restart/authentication cases pass both routes: 62.150 s combined, load 2.41/3.73/4.49 -> 3.63/3.77/4.45; private keys survive restart.
Shared HMAC matches RFC/OpenSSL vectors; final-compiler equal-length MAC comparison has no content-dependent branch.
Rebased four-worker gate passes: 242.634 s, load 1.76/2.50/3.67 -> 4.49/3.51/3.80; seed fixed point and both shutdown routes pass.
2.3: 1,000 write kills + 1,000 confirmed recovery kills preserved 1,129 acknowledgements; 2,124 complete records, 1,127,051 checked frames.
Campaign 143.375 s, load 14.19/9.46/6.20 -> 8.66/10.14/7.02; seed 6840227782638526189; deterministic torn-tail/corruption cases also pass.
2.2: 990 silent plus ten active peers gave 22,032/s, 100.22% of bracketing unloaded mean; load 2.92/4.16/5.41 -> 3.19/4.08/5.33.
Idle process RSS cost 2,205.2 bytes/peer; slow header closed at 2.003 s, hostile traffic 20,854/s; load 3.12/4.09/5.34 -> 3.19/4.08/5.33.

| 2.1 server function (self samples) | Before | After, full |
| --- | ---: | ---: |
| bytes::append_bytes | 78.7% | 0.9% |
| io::file_read_to_end | 3.6% | removed |
| buffer::ReadBuffer::read | absent | 48.8% |
| platform::file_write | 3.4% | 20.8% |

2.1 full-body 6,161 -> 11,506/s; 17,513/15,001 samples; baseline load 8.67/6.26/4.40 -> 6.01/5.87/4.33, after 3.89/4.54/4.31 -> 3.67/4.44/4.29.
2.1 framing 21,975/s, load 3.67/4.44/4.29 -> 2.83/4.16/4.20; numeric Connector grants and lkjscript clients replaced curl.
Interpreter children now consistently belong to the broker; the unused local Child path is removed; three diagnostic cases record consumer friction.
CI fixtures pass Debian/Ubuntu; isolated repair gate 133.960 s, load 2.04/2.07/2.40 -> 3.83/2.70/2.58; capability semantics unchanged.
All consumer proof children, broker helpers, fresh ports, credentials, fixture directories and ignored proof outputs are removed.
Next: documentation-origin candidate and parity; live service and /opt remain untouched.
