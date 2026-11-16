# Status — 2026-11-16 UTC

**Direction.** AI-native general-purpose semantic operations at any program size; applications are specimens. Text remains the stored projection. The compiler is the first consumer; the global checked graph/pack is a migration wall, not the chosen ten-million-line checking unit.

**Landing.** Authenticated child source72544e88 and fixed-point seed6632d1ca are pushed; [CI43751124428](https://github.com/lkjsxc/lkjscript/actions/runs/43751124428) passed. Structural source through7ce088e4 passed the empty-store gate in632.597 s (load7.24→7.16),5,394 fresh/0 reused. Programs e5a54dd2 is integrated without changing frozen measurement commits; child-entry authority, all39 codec types/108 intrinsics and refactor intrinsic annotations include its additions. Integrated Rust witness passes276 values/344 verdicts and compiler byte identity in76.70 s (load1.75→13.50). Refreshed seed and integrated gate/CI are next. No service deployment.
**Asking.** [Pinned snapshot v2](../bench/compiler-query-v2.md) authenticates the producing compiler without rehashing the reader; it grants no current admission.38/40 whole commands are below100 ms, range2.707–207.807 ms (load5.95→6.03); the universal target is unmet. Both misses remain, including source::load_report102.204 ms. Slice median3,761 B;2/20 continue. Index125.212 s (load10.60→5.95), RSS2,647,832 KiB; unchanged479.580 ms (5.95→5.95). The twenty-row table below remains the [historical old-route comparison](../bench/compiler-query-baseline.md): minima41,687 B versus replies47,370 B, no aggregate saving; actual shared IR retrieval9,238 B once.
**Changing/comparing.** [Transactions and review](structure.md), [refactors](structure-refactors.md):127 native planner cases pass in isolated integration; undemanded-library edits are checked, embedded/include origin collisions refuse, and Self constructor facts support field changes.32/32 journal kills recover committed text in39.029 s (load6.94→5.36). [Frozen history](../bench/compiler-history.md):8/179 complete-source replays,84 operations;30 is unmet. All50 comparisons succeed,≤23 lines; historical rename/comment preservation is not claimed. Focused rename/comment IR-equality cases pass.
**Adoption.** Own retained added/deleted `src/` and `library/` .lkj lines since39d18d44: independently audited operations≥4,101/5,945 (69.0%) through20d5f7e9. Imported Programs changes are excluded; seven Receiver merge-glue lines and two query-store test bootstrap lines are uncredited. Another778 reported operation lines lack retained evidence; initial file/bootstrap work is uncredited. Counts sum per-commit deltas, not net tree size.
**Size/code.** [Completed10k/100k ladder](../bench/scale.md): cold1.50/15.41 s (load3.27→3.89 /4.35→5.82), RSS110,408/694,740 KiB; cold-time/RSS exponents1.0117/0.7988. Body transactions2.48/22.08 s (3.90→3.91 /6.81→6.87); renames6.26/61.85 s (3.86→4.35 /7.43→7.32), exponent0.9948.1M refused under8 GiB;10M not run. [Fixed-source native/Rust-O gap6.757x](../bench/compiler_attribution.md); [native producer inlining7.03%](../bench/compiler-native-inline.md) discarded. No4x milestone or backend floor.
**Next.** Cache only fully validated logical pages within an immutable query snapshot, then measure the same forty requests once with the candidate reader. Preserve all earlier outliers. Then integrated four-worker cold gate, fixed point, separate seed refresh, push and CI. Whole-program edit/store work, new-file replay and associated-function reference evidence remain measured walls.

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
