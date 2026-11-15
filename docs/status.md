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
