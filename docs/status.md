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

2.0 complete: the failed deployment was Listener EADDRINUSE after TIME_WAIT, not a journal Output failure.
Fresh journal output already worked. Native, independent interpreter and witness now reuse closed TCP addresses; active listeners remain exclusive.
Fallible entry errors identify the grant kind and errno; endpoint detail has a pending case.
Hardened proof matches installed output, DynamicUser, working directory and effective unit settings, then restarts on the same port with 76 TIME_WAIT sockets.
Both complete live-parity rounds pass: 63 paths, 12 archived bodies and 13 protocol probes, including all 51 search queries.
Build 763.847 ms, load 5.14/6.68/5.99 unchanged; candidate 867,645 B, RSS 1,376 KiB, cgroup peak 1,646,592 B.
Hardened parity/restart 2.171 s, same load; cold offline 1.937 s, load 5.14/6.68/5.99 -> 5.13/6.65/5.99.
Offline native/interpreter cases refuse occupied ports with errno 98 and restart after TIME_WAIT.
Switch/rollback uses a fresh attempt directory; failed-release collisions, interrupted activation and repeated rollback pass without touching /opt.
The 16 imported site tests passed once (38.668 s, load 5.44/5.32/5.03 -> 6.35/5.55/5.12); --campaign retains them outside the gate.
Stored-seed A differs after runtime changes; B=C exactly, 16,018,836 B. Bootstrap verifies a bridge only when needed; seed unchanged.
C build 17.640 s, load 7.62/7.48/6.13 -> 7.25/7.40/6.13; four-worker cold gate passes in 482.715 s, load 4.34 -> 7.37.
All owned proof processes, units, ports and stages are removed; live PID 1624182 unchanged. No deployment performed.
Next: desk commands and its isolated service; external-child authority must be proved before board/listen/wake.
