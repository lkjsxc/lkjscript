# Status — 2026-11-15

**Compiler.** Cold/edit/unchanged builds 33.840/10.539/1.244 s (load 21.25→19.68 /19.68→18.58 /19.68→19.68). Edit rebuilds 1/2,714 definitions and 1/10,072 functions; reuses 10,071 checks and 9,922 fragments; runnable images equal no-store bytes. Initial dominant inference/construction now 0.242/0.015 s; 3 s unmet, whole-graph authentication/identity remain. Cold store +10.51% versus 30.64 s no-store (load 19.83→21.25); cap 256 MiB/Directory, 512 MiB with four workers; costs/overfill proofs in bench/README.md.

| Phase seconds | Cold | One-body edit | Unchanged |
| --- | ---: | ---: | ---: |
| Read and parse | 1.455 | 1.132 | 0.020 |
| Resolve | 0.285 | 0.251 | 0.000 |
| Elaborate / monomorphize | 16.971 | 0.257 | 0.000 |
| Key / hash | 1.629 | 1.606 | 0.473 |
| Check | 8.853 | 0.158 | 0.000 |
| Emit | 1.926 | 0.752 | 0.000 |
| Assemble / write | 0.122 | 0.079 | 0.006 |
| Store I/O + authentication | 2.199 | 5.313 | 0.744 |
| Other (checkpoint codec included) | 0.399 | 0.992 | 0.001 |

| Warm gate 147.070 s (load 16.30→9.43) | Group seconds (load) | Reused / rerun | Why rerun |
| --- | --- | --- | --- |
| Prelude + host/proof overlap | 87.188 (16.30→12.23) | 39/1 requests; 15 fresh suites | Host/proof observations; explicit-library fixture |
| Conformance 0 /64 | 32.629 (12.23→11.41 /11.41→9.73) | 1,536/22 requests | Effects/outcomes outside fixed-capture proof |
| Generated 0 /64 | 1.337 (11.41→11.41 /9.73→9.73) | 2,430/0 requests | Exact authenticated captures |
| Semantic structure | 23.512 (9.73→7.81) | 0/762 contracts | Fresh structural comparisons |
| Semantic/reference + mutation | 2.404 (7.81→9.43) | 449/0 requests; 128 fresh mutants | Case proofs reused; mutation comparisons fresh |

| Exclusive kinds: 25,029 samples, 26.23 s (load 19.07→16.24) | Kind / % | Kind / % | Kind / % | Kind / % |
| --- | --- | --- | --- | --- |
| Arithmetic split | Arithmetic/compare/convert 11.31 | Register moves 14.64 | LEA 5.95 | Constants 6.01 |
| Call split | Argument/result 2.25 | Prologue/epilogue 10.04 | Return 2.76 | Machine call 0.81 |
| Storage | Inline copy 13.44 | Frame load/store 4.63 | Other memory 10.40 | Allocator 11.01 |
| Other | Checks 1.39 | Branch 2.63 | Runtime 2.74 | Unknown 0.00 |
Native/opt0/-O self-build 27.24/15.35/4.55 s (load 13.21/19.14/17.88), byte-identical outputs; 1.775x opt0 (CPU 1.746x), 1.0x unmet: broader storage/register or library work deferred; bench/README.md includes every workload and regression. Cold gate 513.135 s (load 10.98→16.30), 150 s unmet; exact partition there. Warm skips 4,454 requests; actual checker edit reuses 5 closed ELF contracts across producers. Seed 3,145,023 B; 32,768 inline64 mutants, 0 findings, 67.973 s (load 9.43→15.71). First seed e9b8c940 CI passed 874.494 s (load 0.98→2.80), optional witness passed. Directory adoption cases remain pending; test-library forwarding is fixed; automatic unwrap caller/errno closed because generic E lacks formatting/output authority, explicit match preserves both.

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
