# Status — 2026-11-15

**Source decision: .lkj, landed as 1,072 language sources.** a3c2f046 plus separate seed26bf2c3c; child.lkj/child/mod.lkj/program.lkj, tools/docs/witness agree without alias/fallback. Rename gate461.920s (load3.92→8.04), current generations equal. Seed-only CI43718048863 passed572.075s (load0.69→3.39); witness119s (load not recorded). Programs rebased at9d145128.

**Compiler integration in progress.** Before programs9d145128, bounded path/line/operation/errno traps and complete private identities passed43 isolated transient-unit cases and32,768 original-IR mutants (92.391s, load11.40→11.08), zero findings. Earlier source had a separate grant/errno line plus misleading generic R0007 text; the live missing-line cause was not reproduced. Native terminal formatting allocates nothing; supervisor interpretation/private decoding still allocate. No production service was touched.

| Paired fixed-source store experiment | Before s (load) | Fixed-root s (load) |
| --- | ---: | ---: |
| Cold | 23.458 (11.38→9.44) | 23.601 (11.60→10.38) |
| Unchanged | 0.851 (9.44→9.44) | 0.573 (10.38→9.79) |
| Checker one-body edit | 8.387 (9.44→9.48) | 6.456 (9.79→9.48) |
| Edit store I/O/authentication | 4.086 (9.44→9.48) | 2.827 (9.79→9.48) |

All six cached/control images agree; exact producers/source hashes are in bench/README.md. Fixed root binds producer/target/domain/length/digest before allocation; one owned payload/EOF/digest check replaces repeated authentication/copies. Nine pure contracts and on-disk stale-root/substitution/truncation/length/accounting attacks pass.
Edit remainder: parse0.819, resolve0.180, key/hash1.024, codec0.616, elaborate/check0.264, emit0.591, write0.072, other0.063s (load9.79→9.48). Non-store floor3.630s already exceeds3s; AST checkpoints/stable file arena identities and lazy body/fragment storage are absent. No metadata claim substitutes for reading mutable bytes.

**Gate.** Before integration, corrected cold gate513.548s (load4.84→12.93) passed5319fresh requests; fragment sharing retained194,643,677B under256MiB and proof runners release locks before execution. Programs9d adds two closed contracts and two live suites: merged split749 closed (28 independent,721 compiler-dependent),25 live structural observations. Final cold/warm/checker-edit measurements restart on the combined source.

**Generated code stopped at step1.** Whole-function linear scan across blocks/loops and saves-only-when-used already exist. Additional operand/coalescing trial gave1.00% wall/0.70% CPU gain, below10%; discarded. Paired mean20.580s vs opt0 witness11.27s (loads6.42→7.80 /7.79→7.35),1.826x; -O3.40s (7.28→7.18),6.053x. No speculative ABI/region rewrite or cold-gate speedup is claimed.

**Diagnostic causes (16 tagged programs fixtures at9d; specimens, not attempt totals):**
| Cause | Count | Most frequent accepted rewrite |
| --- | ---: | --- |
| Expected Rust form/support gap | 8 | Typed inference3, collection views/iteration3, explicit trait impl1, text fold1 |
| Bad/incomplete message | 5 | Located caller/errno, named type/storage operation, value-less no-store |
| Rule with no Rust counterpart | 3 | Owned text/stable owner index and authenticated child image |
`lkjscript delta` prints accepted forms; these16 tags supersede the earlier34-fixture proxy. Mandatory formatting/fragment markers have no supporting cause evidence and were declined.

**Next:** validate merged capability diagnostics, full gates/edit measurements, separate seed refresh and CI. The existing external_child_authority pending case now records compiler agreement with programs: authenticated exact ELF/entry/ABI/canonical slots, sealed descriptor through exec, typed transfer startup; programs owns lifecycle/consumer APIs.

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
