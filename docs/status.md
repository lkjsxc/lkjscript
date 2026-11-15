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

2.0 landed at d2044ada; local and CI gates/witness pass. Real-unit proof found Listener EADDRINUSE after TIME_WAIT, not journal Output.
Same-port restart with 76 TIME_WAIT sockets passes both complete parity rounds: 63 paths, 12 archived bodies, 13 protocol probes. Claude owns deployment.
2.1 commands plus isolated serve pass: note/ask/close/inbox/peek/path, Japanese, concurrent writers, shared files and the page/state/say routes.
Arguments, exact-name Environment and ExitStatus are affine; named directory roots, blocking locks and realtime nanoseconds work on standalone/native-run/interpreted routes.
96 SIGKILLs retain all 128 acknowledged IDs and the original 10,000-row byte prefix; Python reads the surviving state. Campaign stays outside the gate.
Exact-path external execution cannot enforce the required child-authority subset. Stop at 2.1 plus serve without board/handoff/wake; arbitrary listen stays outside.
No live desk handoff or push-page implementation is claimed. Full contract, metrics and both complete authority reports are in examples/desk.md and examples/desk/authority.txt.
Direct source: 581 Rust + 186 unchanged HTML lines (Python 249 + 186). Binaries: command 299,635 B; service 498,357 B; service RSS 656–660 KiB.
Cold command/service builds: 752.396/786.849 ms, measurement interval load 8.12/7.15/5.50→7.87/7.12/5.50.
100-command mean, empty state: native 29.914 ms (load 2.40→2.37), Python 18.767 ms (2.37→2.37); native is slower with durable publication.
At 10,000 rows: native 27.613 ms (2.37→2.37), Python 18.695 ms (2.37→2.34); append still copies the old log.
Four-worker cold gate: 534.967 s, load 8.50→5.92; 778 conformance cases on both policies pass. Current generations agree at 16,171,123 B; seed unchanged.
Consumer cold seconds below total 34.039; all start without reusable consumer results. Long campaign proofs stay outside the gate.

| Suite | Cold seconds (load before→after) | Short regression proves |
| --- | ---: | --- |
| HTTP | 16.826 (8.67→8.54) | framing, readiness, deadlines |
| Journal | 1.661 (8.30→8.30) | commit/replay, authentication |
| Stop native | 2.814 (8.30→8.30) | signal, bounded drain, sync |
| Stop interpreted | 6.287 (8.30→8.12) | independent broker drain |
| Site | 2.168 (8.30→8.12) | frozen parity, occupied/TIME_WAIT bind |
| Desk + capabilities | 4.283 (8.12→7.87) | three routes, CLI, env/status/lock, safe publication |

Pending compiler diagnostics include test --library-dir child forwarding; program-directory migration awaits the compiler's pending case.
