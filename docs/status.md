# Status — 2026-11-15

**Source decision: .lkj, landed as 1,072 language sources.** Loader, program.lkj, cases/tools/docs and witness agree; only actual Rust stays .rs, with no alias/fallback. Gate 461.920 s (load 3.92→8.04), current generations byte-identical. Rename and required seed are separate commits pushed together because the old seed only discovers .rs; Programs can rebase now.

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

Third note 2.1–2.3 and 2.5 are implemented; site/journal/desk are .lkj program directories, closing the compiler's previously pending adoption cases. No deployment. Contracts and complete reports: examples/desk.md, desk/authority.txt and foreign-child.md.
Desk has one state root, two named read roots and a loopback listener, plus declared service mechanisms; no child authority. Native listen-begin/end require caller-owned flock/lifecycle. Wake uses durable high-water request/ack and idempotent relay; the 24-line switch/rollback requires Claude's board/relay/caller adaptations.
Delivered desk note at 100,000 rows: buffered 0.741 ms (load 9.88→9.88), sync 10.252 ms (9.52→9.88), Python 28.659 ms (9.88→9.88). One append write, no log copy; explicit durability and per-phase reports expose variable fsync cost.
SSE replay uses two durable byte cursors and row IDs; 990 idle +10 active peers receive a note in 252.958 ms (load 7.63→7.98), with fixed write deadlines and polling fallback. This measures protocol receipt; browser painting is not separately timed.
Desk source: 1,051 .lkj +254 HTML lines; command/wake wrappers 8/49 lines and units 87. Command/service 345,791/703,957 B; cold builds 916.833/897.239 ms (both load 7.73→7.73); observed service RSS 1,088 KiB, short SSE 852 KiB.
2.4 remains unimplemented pending compiler-owned authenticated image identity, canonical entry authority and descriptor-only startup: tests/diagnostics/external_child_authority.lkj. ForeignCommand is a separate kernel-confined form, not that proof; its exact exceptions and eight-line cxloop gap are documented.
Four-worker empty-store gate passes: 622.262 s (load 11.56→15.38), 4,512 fresh requests, no proof skips; current generations agree at 16,731,835 B, seed unchanged. Long inline64 campaign: 32,768 mutants, 0 findings, 58.851 s (load 14.25→16.98).

| Suite | Cold seconds (load before→after) | Proof |
| --- | ---: | --- |
| HTTP | 17.552 (10.63→9.80) | framing, readiness, deadlines |
| Journal | 2.122 (10.50→10.22) | commit/replay, authentication |
| Stop native / interpreted | 2.993 /6.442 (10.22→10.22 /10.22→9.57) | signal, bounded drain, sync |
| Site | 2.553 (9.80→9.57) | archive parity, occupied/TIME_WAIT bind |
| Desk + capabilities/read roots/wrapper/composition | 8.776 (9.57→9.29) | CLI, append/recovery, inode replacement, board/handoff, listener/wake |
| Desk events | 3.518 (9.57→9.57) | note push, POST, cursor replay/reset, slow reader |
| Foreign child | 3.318 (9.57→9.29) | three engines, actual Git, outside-root refusal, fail-closed host |
| Program directory | 18.516 (11.03→10.50) | generic fixtures plus real site/journal/desk builds, authority and focused cases |
| Read-root ENOSYS campaign (optional) | 3.407 (4.99→5.07) | anchored fallback and containment, three engines |
| Desk unit/live GET/wake retry (optional) | 7.953 (13.28→13.74) | exact settings, six live read routes, same-port restart, idempotent retry |
| Foreign kernel / adversarial campaigns (optional) | 0.358 /15.311 (8.39→8.39 /5.47→6.40) | kernel witness; three engines, deadline, reexec, owner death/reap |
| Desk append kill /1,000-peer SSE campaigns (optional) | 9.126 /7.528 (5.94→5.79 /7.63→7.98) | 96 kills preserve 286 acknowledged IDs and 910,000-byte prefix; 990 idle peers |

Consumer group sum 47.274 s, or 65.790 s including program-directory (individual load windows above); optional campaigns stay outside the gate. Sixteen tagged diagnostics: Rust-form 8 /new-rule 3 /bad-message 5. Code wins: append now rejects every final symlink with ELOOP; obsolete world-case expectations were corrected.
