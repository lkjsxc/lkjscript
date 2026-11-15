# Status — 2026-11-15

## Compiler: proportional construction, authenticated reuse and directory programs

Build cold/edit/unchanged 23.011/7.791/0.953 s (load 11.72→9.42 /9.42→9.06 /9.42→9.42); phase seconds below.

| Phase | Cold | One-body edit | Unchanged |
| --- | ---: | ---: | ---: |
| Read and parse | 0.879 | 0.885 | 0.011 |
| Resolve | 0.174 | 0.228 | 0.000 |
| Elaborate / monomorphize | 11.575 | 0.170 | 0.000 |
| Key / hash | 1.084 | 1.084 | 0.349 |
| Check | 6.226 | 0.077 | 0.000 |
| Emit | 1.419 | 0.549 | 0.000 |
| Assemble / write | 0.067 | 0.063 | 0.002 |
| Store I/O + authentication | 1.352 | 3.998 | 0.591 |
| Other (checkpoint codec included) | 0.236 | 0.737 | 0.001 |
Edit rebuilds 1/2,713 definitions and 1/10,071 functions; reuses 10,070 checks and 9,921 fragments; runnable images equal no-store bytes. Initial dominant inference/construction now 0.163/0.007 s; 3 s unmet, whole-graph authentication/identity remain.
No-store 20.35 s (load 13.46→11.72), cold +13.12%; remaining costs in bench/README.md. Cap 256 MiB/Directory, 512 MiB with four workers; cold/edit contents 71,676,829/88,449,732 B; overfill/oldest-generation eviction passes.
Gate cold/warm 535.033/156.966 s (load 6.71→10.97 /10.97→18.47); warm groups below partition wall time, not overlapping suite-time sums.

| Warm suite group | Seconds (load) | Reused / rerun | Why rerun |
| --- | --- | --- | --- |
| Prelude + host/proof overlap | 85.672 (10.97→13.72) | 39/1 requests; 15 fresh suites | Host/proof observations; explicit-library fixture |
| Conformance 0 /64 | 39.642 (13.72→12.47 /12.47→11.76) | 1,534/22 requests | 11 open observations per policy |
| Generated 0 /64 | 1.529 (12.47→12.47 /11.76→11.76) | 2,430/0 requests | Exact authenticated captures |
| Semantic structure | 27.907 (11.76→18.47) | 0/762 contracts | Fresh structural comparisons |
| Semantic/reference + mutation | 2.216 (18.47→18.47) | 449/0 requests; 128 fresh mutants | Case proofs reused; mutation comparisons fresh |
Warm skips 4,452 requests on exact authenticated producer/input closure. Actual checker-body edit skips 5 closed contracts on equal fresh checked ELF/stable runner; verdicts stay generation-bound. Cold 150 s unmet; every wall slice is in bench/README.md.
Directory program.rs SOURCE/ENTRY/GRANTS/CASES is ready; site/journal/desk adoption cases are pending. Consumer diagnostics and test-library forwarding pass; automatic unwrap caller/errno is closed: generic E lacks implicit formatting/output authority; explicit match retains payload/location.
Seed 3,242,241 B; current generations equal 16,772,694 B. Long inline64 mutation: 32,768, 0 findings, 67.715 s (load 18.47→14.74). Current seed-only CI awaits push; next: current split profile and proven-dead aggregate storage, with 1.0x opt0 unclaimed.

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
