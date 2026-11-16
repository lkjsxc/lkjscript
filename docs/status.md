# Status — 2026-11-16 UTC

**Direction.** AI-native general-purpose semantic operations at any program size; applications are specimens. Text remains the stored projection. The compiler is the first consumer; the global checked graph/pack is a migration wall, not the chosen ten-million-line checking unit.

**Landing.** Producer64 source d4d6911a passes the cold gate in 591.945 s (load 1.95→5.09): 5,448 fresh case requests, zero case reuse and 128 mutants. Its current-emitter fixed point is 30,969,800 B, SHA-256 `ed0368b6…60e59`; bootstrap 78.013 s (1.95→2.34), gate peak RSS 1,333,480 KiB. Optional Rust witness passes 276 value/output cases, 344 verdicts and compiler byte identity in 68.37 s (3.52→2.92). Separate seed refresh and new CI follow; pushed a4fd685a's [CI43791875654](https://github.com/lkjsxc/lkjscript/actions/runs/43791875654) remains green. No service deployment.
**Asking.** [Forty cached-reader requests](../bench/compiler-query-cache.md) take 2.334–53.525 ms (load 4.09→3.93); [ten additional kinds](../bench/compiler-query-kinds.md) take 2.338–21.521 ms (3.07→3.07). All 50 are below 100 ms at the recorded bounds/warm sequence, with exact old-reader stdout parity; historical provenance grants no current admission. Long signatures now retain every UTF-8 fragment: 24,610 B in 25 chunks across stored pages; the gate regression takes 0.429 s (2.92→2.92). Slice median 3,772 B; 2/20 continue. Index 132.088 s (5.88→6.29), RSS 2,770,148 KiB; unchanged 592.698 ms (6.29→6.29). Earlier misses remain recorded. Historical old-route minima below total 41,687 B versus replies 47,370 B, without aggregate saving; actual shared IR retrieval was 9,238 B once ([provenance](../bench/compiler-query-baseline.md)).
**Changing/comparing.** [Transactions](structure.md) and [refactors](structure-refactors.md): 141 planner cases plus CLI pass in 18.008 s (load 3.09→4.00). Associated member references require declared-trait or closed-owner selection evidence; anonymous parameter IDs use signature positions. All 32 journal kills recover committed text in 39.029 s (6.94→5.36). [Original history](../bench/compiler-history.md) remains 8/179; [exact-name ordinary roots](../bench/compiler-history-roots.md) add one separate applied case. [New self-use window](../bench/compiler-self-history.md): 30 compiler-scoped replays/103 attempts, 206 operations, canonical checked-meaning equality; all original eight overlap, and seven revisions exclude other language files. Thirty whole-commit replays remain unmet. All 50 original comparisons succeed in ≤23 lines; rename/comment IR preservation has focused cases, not a historical claim.
**Adoption.** All own retained added/deleted `.lkj` lines since 39d18d44 inclusive: independently audited operations ≥4,400/6,515 (67.5%) through 9047a677. The `src/` and `library/` subset is 4,400/6,253 (70.4%). Imported Programs changes are excluded; 16 manual integration lines, two query-store bootstrap lines and 262 benchmark/test lines outside that subset are uncredited. Another 778 reported operation lines lack retained evidence; initial file/bootstrap work is uncredited. Counts sum per-commit deltas, not net tree size.
**Size/code.** [10k/100k ladder](../bench/scale.md): cold 1.50/15.41 s (load 3.27→3.89 /4.35→5.82), RSS 110,408/694,740 KiB; cold-time/RSS exponents 1.0117/0.7988. Body transactions 2.48/22.08 s (3.90→3.91 /6.81→6.87); renames 6.26/61.85 s (3.86→4.35 /7.43→7.32), exponent 0.9948. 1M refused under 8 GiB; 10M not run. [Fixed-source native/Rust-O baseline 6.757x](../bench/compiler_attribution.md); inline16's 7.03% trial stays discarded. [Producer-inline64](../bench/compiler-native-inline64.md) reduces mean wall 13.466% across all six runs (2.78→2.39), with 41.623% image growth; ordinary output defaults to zero. [Attribution](../bench/compiler-profile-inline64.md) leaves 10.369% copy samples but no joint copy-origin evidence. No new Rust-O milestone or backend floor.
**Next.** Finalize the larger-inline trial report, refresh the verified seed separately, push and inspect CI. A separate four-candidate probe will test the remaining scope-complete cases without replaying prior attempts or rewriting either frozen population. New-file/root coverage, whole-program edit/store work and generic inherent associated-call evidence remain gaps; checked definition/interface units still need to replace the global pack.

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

Fourth note implements the [runtime demand order](runtime-demands.md); site/journal/desk remain unchanged specimens, with no deployment. Contracts: [workers](../examples/process-workers.md), [foreign sessions](programs-foreign-session-contract.md), [runtime admission](runtime-admission.md), [graphs](../examples/object_graph/README.md).
Concurrency is process isolation: affine Image/Worker/Channel, moved File values, fully sealed ReadRegion and separate heaps. No mutable shared heap, Send/Sync promise or thread scheduler. The interpreter checks seeded protocol histories; its hosted children execute natively.
The parent retains listener/root; channel-only parsers handle failure, deadline and bounded restart. Reports bind exact image/manifest identity and distinguish entry Channel from later moved Files. File transfer consumes on success and failure.
Runtime admission receives IR after startup, uses the build-time checker, admits no authority and contains located traps in children. Its dedicated compiler producer explicitly holds World; a narrow public Admission value is not claimed.

| Specimen / proof | Seconds (load before→after) | Result |
| --- | ---: | --- |
| Ordered pool, 4,096 jobs | 6.915→2.240 (6.76→6.03); 6.759→2.106 (6.03→5.65); 6.780→2.206 (5.65→5.34) | Four workers 3.087× /3.209× /3.073×; all pairs retained |
| Worker/runtime/supervision suite | 20.498 (18.07→15.12) | Three engines, eight HTTP requests, two bounded restarts, trap/deadline values |
| Protocol model | 25.080 (4.71→5.88) | 64 success +64 death schedules, 64 map histories; no native scheduling claim |
| Foreign session suite | 9.205 (5.67→5.94) | Three engines; persistent Git 28.775 µs vs spawn 771.479 µs, 26.81× (5.67→5.67) |
| Million-node graph native /Rust0 /Rust3 | 0.986 /0.660 /0.274 (each 5.35→5.35) | Same source/output; 254,556 /245,908 /246,256 KiB; 40,449 stale references |
| Live IR admission, excluding producer build | 0.735 (33.28→33.28) | Value 42, authority refusal, six mutants, located trap, deadline and recovery |
| Pool publication kill campaign | 3.455 (8.16→9.03) | 20 parent kills; complete old/new 128-byte result; 32 adopted workers reaped |
| Independent process OS witness | 1.596 (7.70→7.48) | Pre-exec deadline, pre-prctl parent death, ancillary/buffer/descriptor cleanup |
| Consumer closure cold +producer renewal | 73.330 (4.15→10.15) | All 32 HTTP/client/program-directory observations reused; zero reruns |
| Actual HTTP /program-directory wrappers | 30.544 (13.09→15.21) /43.107 (15.21→13.32) | Live I/O stays fresh; own processes, ports and state cleaned |
| Integrated cold gate: local /CI | 771.134 (4.18→11.24) /1,023.704 (0.81→2.28) | 0 reused /5,409 fresh requests each; 128-mutant gate; both passed |

Targeted final IR campaigns reject all 552 worker/region and 517 foreign-session mutants; 46+2 controls admit, 48 Copy/Data checks pass. Earlier audit findings were fixed before these runs.
Remaining handoffs: semantic message identity/generated codecs, other capability message kinds, compiler-driver pool adoption, narrow admission provenance and runtime-grant key exclusion (including hardlinks). Foreign descriptor/shared-region transport, same-address-space threads and other targets remain explicit gaps.
Code wins: the old three scoped-concurrency pending cases were stale; only borrowed_closure_escape exists. consumer_gate_closure was prose; it now has an executable cross-producer proof. The initial worktree was eight commits behind its fetched baseline, not four.
This note touched 25 tagged diagnostic/handoff records: Rust form 12 / new rule 8 / bad message 5, not attempt totals. The graph had four repaired diagnostic cases; all retry causes remain under tests/diagnostics.
Source e5a54dd2 passed [CI43778996002](https://github.com/lkjsxc/lkjscript/actions/runs/43778996002): required native gate and optional Rust witness (274 value/output cases, 344 verdicts and compiler byte identity). The stale canonical fixture inventory is repaired (721 structural contracts). Fixed point 17,929,344 B; Programs seed unchanged.
