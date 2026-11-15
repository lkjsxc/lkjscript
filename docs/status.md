# Status — 2026-11-15

## Compiler: checked reuse, rechecked inlining and executable diagnostic rewrites

Reuse cold/edit/unchanged: 23.27/17.20/1.42 s (load 3.76→3.72 / 3.72→4.92 / 3.72→3.72); edit reuses 9,639/9,640 checks.
No-store/edit control: 17.13/17.63 s (load 3.76→3.76 / 4.92→5.49); cold overhead +35.84%; store 54,320,913 B, two versions 70,300,130 B.
Seed 3,077,565 B; gate cold/warm 394.211/200.040 s (load 0.84→4.56 / 4.56→4.45); warm 32,768 inline64 mutants: zero findings, 64.435 s (load 1.42→6.26).
Scalar Clone→Read; inline16 removes 17,622 calls in 3,701 bodies, retaining 5,939 original proofs. Default zero: candidate admission exceeds code gains.
All 23 diagnostic inputs and 618 Rust checks pass alongside applied edits; literal panic payloads were previously discarded. Automatic unwrap caller/errno remains pending; explicit match works.

| Compiler self-build, store disabled | No inline: seconds (load before) | Inline16: seconds (load before) |
| --- | ---: | ---: |
| Self-built | 17.21 (3.76) | 21.31 (3.72) |
| Rust opt=0, one codegen unit | 10.01 (3.76) | 12.81 (4.92) |
| Rust -O, one codegen unit | 2.61 (3.57) | 3.29 (5.49) |
Outputs agree byte for byte for each policy. Native0 is 1.72x opt=0 and 6.59x -O; inline16 is 6.48x -O. The 3x target remains unmet.

| Exclusive emitted-code samples | Self-build %, 0 / 16 | HTTP server %, 0 / 16 |
| --- | ---: | ---: |
| Inline copy | 19.3 / 18.7 | 5.2 / 4.8 |
| Checks | 1.1 / 1.2 | 0.8 / 1.0 |
| Call setup / prologue / return | 14.0 / 11.2 | 2.7 / 2.2 |
| Frame load/store | 4.4 / 4.6 | 15.3 / 15.0 |
| Other memory | 11.4 / 11.4 | 5.5 / 6.4 |
| Arithmetic / registers | 35.4 / 37.1 | 36.7 / 35.5 |
| Branch | 2.4 / 2.8 | 5.7 / 6.0 |
| Allocator | 9.3 / 10.5 | 2.1 / 2.2 |
| Runtime | 2.2 / 2.0 | 25.5 / 26.5 |
| Unknown | 0.0 / 0.0 | 0.0 / 0.0 |
Self0/16: 16,826/20,034 samples, 17.30/20.50 s (load 3.03→3.20 / 2.50→2.21).
HTTP0/16: 27,025/26,971 samples, 35,354/36,701 requests/s over 30.001/30.000 s (load 3.76→4.47 / 4.92→5.45); same 8,195-byte body and client.
Copies/calls/runtime own internal frame accesses; arithmetic includes register moves/addresses. Next: source graph/copy and reuse-authentication costs.

## Programs

2.5 complete: 12 published paths, 51 computed search queries and 13 HTTP probes match the live origin; HEAD/range/conditional/traversal cases included.
Hardened parity: 0.730 s, load 5.40/5.35/4.80 -> 5.21/5.31/4.79; process RSS 1,376 KiB, cgroup peak 2,097,152 B under the original 128 MiB limit.
Sealed executable 863,424 B, 383 IR functions, 38,257 instructions; build 744.114 ms, load 5.40/5.35/4.80 unchanged.
RestrictSUIDSGID blocks openat2; embedding the public archive removed the runtime Directory grant while retaining the original unit settings.
Review repair: buffered pipelines retain receive-time deadlines; old server fails, header/body/later-read/idle regression passes in 7.215 s, load 6.48/5.11/4.71 -> 5.94/5.04/4.69.
Handoff requires all 51 unique query probes and supports interrupted rollback within 25 lines; 16 cases pass in 36.241 s, load 4.35/4.71/4.60 -> 3.81/4.52/4.54.
2.4: Stop drains admitted requests to a fixed deadline, syncs and exits 0; signal native program PID or interpreted broker PID.
Kernel Entropy and wall time support private durable keys and signed expiring write tokens; final default/inline64 native MAC comparison has no content-dependent branch.
Issuer/restart/authentication passed both routes in 62.150 s, load 2.41/3.73/4.49 -> 3.63/3.77/4.45; lifecycle/socket CI repair is green.
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
2.1 framing 21,975/s, load 3.67/4.44/4.29 -> 2.83/4.16/4.20; Connector/lkjscript clients replaced curl. Relay queues, three thread cases and unused local Child are removed.
Four-worker gate passes: 262.690 s, one-minute load 2.93 -> 7.22; a sixth site diagnostic case records the excluded cfg(test) helper's misleading error.
All test processes, ports, units and proof files are gone; live PID 256 unchanged. Candidate/unit/25-line switch and rollback are ready for Claude; no switch performed.
