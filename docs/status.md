# Status — 2026-11-15 UTC

**Source decision: .lkj.** Landed 1,072 language-source renames in a3c2f046 and separate seed 26bf2c3c; child.lkj/child/mod.lkj/program.lkj have no alias/fallback. Rename gate 461.920 s (load 3.92→8.04), seed-only CI43718048863 572.075 s (0.69→3.39), both jobs passed. Programs 9d145128 is integrated.

**Failure interface.** Source4048e172 passes 63 isolated transient-unit cases, 15.048 s (load 6.14→7.85); original-IR oracle plus 32,768 inline64 mutants, 91.790 s (5.99→8.88), zero findings. Path/line, capability/grant, operation and symbolic errno share one bounded line; native terminal formatting allocates nothing, supervisor interpretation/private decoding still allocate. Earlier code had a separate grant/errno line; why the live journal omitted it was not reproduced. Fixed point 17,039,220 B; reproducible seed 3,312,682 B. No production service was touched.

| Current self-build | Seconds | Load before→after |
| --- | ---: | --- |
| Unchanged | 0.55 | 3.98→3.98 |
| Checker one-body edit | 6.35 | 3.98→3.82 |

Cached/control images match. Fixed roots authenticate exact domains before allocation, then hash one owned payload; stale-root/substitution/truncation/length/accounting attacks pass. Edit store I/O is 2.677 s; eliminating it still leaves an observed 3.667 s (both load 3.98→3.82), not a lower bound on future designs. Parse/read 0.833, resolve 0.182, key/producer hash 1.044, codecs 0.667, elaborate/check 0.288, emit/runtime 0.623 s share that load window; unchanged syntax/arena and lazy-pack work remain. The 3 s target is unmet.

| Full four-worker gate | Seconds (load) | Reported reused /fresh |
| --- | ---: | ---: |
| Final cold | 575.444 (1.89→9.84) | 0 /5,356 |
| Final unchanged warm | 126.013 (9.84→7.90) | 5,331 /25 |
| Actual checker edit, before shorter entropy scan | 544.788 (4.89→5.83) | 28 /5,328 |

The actual checker edit changes all 721 compiler-dependent structural images; only 28 independent contracts retain identity. Same-input cross-producer images separately reuse all 749 contracts, not a whole-tree edit claim. The 60 s target is unmet; bench/README.md accounts every phase, the reported warm requests, unconverted closed CLI work and the named consumer_gate_closure pending case for programs (HTTP/program-directory first). The full entropy fill remains; its completed existential scan now exits early.

**Generated code stopped at step1.** Whole-function allocation across blocks/loops and conditional callee saves already exist. Additional coalescing/operand use gained 1.00% wall/0.70% CPU, below10%, and was discarded. Fixed-source wall ratios remain 1.826x opt0 and 6.053x -O; no speculative ABI/region change or cold-gate gain is claimed. Paired timings and loads are in bench/.

**Diagnostic causes: 16 tagged programs specimens, not attempt totals.**

| Cause | Count | Accepted rewrite |
| --- | ---: | --- |
| Expected Rust form/support gap | 8 | Typed inference3, collection views/iteration3, explicit trait impl1, text fold1 |
| Bad/incomplete message | 5 | Located caller/errno, named type/storage operation, value-less no-store |
| Rule with no Rust counterpart | 3 | Owned text/stable owner index and authenticated child image |
The binary's delta command prints frequent accepted forms. Mandatory formatting/fragment markers were declined without cause evidence.

**Next:** land the source and separate seed refresh, then inspect both CI jobs. One existing external_child_authority case records the agreed exact ELF/entry/ABI/canonical slots and sealed descriptor through exec; authenticated-child implementation remains pending, with programs owning transfer/lifecycle/consumer APIs.

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
