# Status — 2026-11-15 UTC

**Direction.** AI-native general-purpose semantic operations at any program size; applications are specimens. Text remains the stored projection. The compiler is the first consumer; the global checked graph/pack is a migration wall, not the chosen ten-million-line checking unit.

**Landing.** Authenticated child source72544e88 and fixed-point seed6632d1ca are pushed; [CI43751124428](https://github.com/lkjsxc/lkjscript/actions/runs/43751124428) passed. Full gate575.275 s (load1.81→4.88), 5,356 fresh requests; image admission679 attacks in6.639 s (5.07→5.30). Programs owns transfer/lifecycle/consumer. Structural source through ef604590 is locally committed; its full gate, new fixed point and CI are pending.
**Asking.** The twenty real compiler questions below use a pinned historical snapshot. “Old minimum” is selected source-span bytes, not invented historic response totals; actual shared IR retrieval was9,238 B once. Totals41,687/47,370 B give no aggregate saving. Whole queries248–282 ms (load7.88→7.48 and7.53→7.53), index traversal0.89–9.39 ms in those windows:100 ms is unmet. Exact producer hashing remains. [Baseline, shared-read provenance and index costs](../bench/compiler-query-baseline.md); final-producer measurements are running.
**Changing/comparing.** [Transactions, identities and trust](structure.md) and [seven refactors](structure-refactors.md) are implemented.65 native planner cases and seven CLI kinds pass; refusal preserves source.32/32 journal kills recovered committed text,39.029 s (load6.94→5.36). Review excludes checkout-prefix noise and stays≤30 lines; current historical replay/comparison campaigns are unfinished. Recorded retained language-line adoption: root172/176, refactor helper≥1,005/1,823, native postactivation298/304 (scopes differ; no aggregate percentage). Basic replacement/rename edits, scope guards and checked-closure gate proofs also pass focused suites.
**Size/code.** [Ladder](../bench/scale.md):10k/100k cold1.70/13.43 s (load6.15→6.15 /6.35→5.76), RSS108,552/698,008 KiB; time/memory/body-edit exponents0.898/0.808/0.848.1M refused under8 GiB;10M is not run. [Fixed-source attribution](../bench/compiler_attribution.md):18.515 s native mean versus2.740 s Rust-O (6.757x; paired loads in the table), no4x milestone or backend floor. Retained rotation/canonical-result steps improve the separately measured SHA workload; final queries/renames at10k/100k are pending.
**Next.** Finish bounded original-last300 replay and last50 review, member rename/move, final hash/query and ladder measurements; then pause workers, run the full four-worker gate, prove the fixed point, refresh the seed separately, push and inspect CI. No service deployment.

| Question | Old minimum B | Reply B |
| --- | ---: | ---: |
| `outline driver::Options` | 874 | 4,486 |
| `outline driver::Memo` | 769 | 3,692 |
| `item driver::compile` | 9,045 | 11,888 |
| `signature driver::execute_source` | 10,348 | 733 |
| `signature driver::read_path` | 311 | 674 |
| `signature driver::diagnostic_source_path` | 434 | 709 |
| `outline ir::TypeKind` | 1,797 | 4,895 |
| `outline ir::Op` | 2,356 | 5,582 |
| `outline ir::Function` | 265 | 1,498 |
| `outline ir::Program` | 264 | 1,023 |
| `outline ir::Local` | 112 | 834 |
| `outline ir::ReturnSource` | 470 | 963 |
| `outline semantic::Catalog` | 424 | 2,110 |
| `outline semantic::Item` | 516 | 2,835 |
| `outline semantic::Reference` | 174 | 1,267 |
| `outline reuse::AuthenticatedRoot` | 161 | 1,317 |
| `signature test_runner::closed_contracts` | 360 | 674 |
| `signature source::load_report` | 10,949 | 794 |
| `outline native::PreparedImage` | 78 | 785 |
| `authority authority::analyze` | 1,980 | 611 |

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
