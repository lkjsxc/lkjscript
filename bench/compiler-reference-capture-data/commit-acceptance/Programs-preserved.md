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
