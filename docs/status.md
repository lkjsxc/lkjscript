# Status — 2026-11-15 UTC

**Direction.** Fourth note: AI-native general-purpose semantic operations at any program size; applications are specimens. Text remains the stored projection. Programs should rebase this direction before its runtime-demand table; compiler starts with authenticated child images, then structural questions/transactions and the size ladder.

**Source and landing.** Direction b579beb7 is pushed; [CI43747015139](https://github.com/lkjsxc/lkjscript/actions/runs/43747015139) passed. Authenticated-child source 72544e88 passes the full four-worker gate: 575.275 s (load 1.81→4.88), 5,356 fresh requests. Fixed point 17,259,129 B; seed refresh follows in its own commit. Source .lkj naming and Programs 9d145128 remain; child.lkj/child/mod.lkj/program.lkj have no alias/fallback.

**Failure interface.** Source 4048e172 passes 63 isolated transient-unit cases, 15.048 s (load 6.14→7.85); original-IR oracle plus 32,768 inline64 mutants, 91.790 s (5.99→8.88), zero findings. Path/line, capability/grant, operation and symbolic errno share one bounded line; native terminal formatting allocates nothing, supervisor interpretation/private decoding still allocate. Earlier code had a separate grant/errno line; why the live journal omitted it was not reproduced. Fixed point 17,039,220 B; reproducible seed 3,312,682 B. No production service was touched.

**Edit loop.** Unchanged build 0.55 s (load 3.98→3.98); checker one-body edit 6.35 s (3.98→3.82).

Cached/control images match. Fixed roots authenticate exact domains before allocation, then hash one owned payload; stale-root/substitution/truncation/length/accounting attacks pass. Edit store I/O is 2.677 s; eliminating it still leaves an observed 3.667 s (both load 3.98→3.82), not a lower bound on future designs. Parse/read 0.833, resolve 0.182, key/producer hash 1.044, codecs 0.667, elaborate/check 0.288, emit/runtime 0.623 s share that load window; unchanged syntax/arena and lazy-pack work remain. The 3 s target is unmet.

| Full four-worker gate | Seconds (load) | Reported reused /fresh |
| --- | ---: | ---: |
| Final cold | 575.444 (1.89→9.84) | 0 /5,356 |
| Final unchanged warm | 126.013 (9.84→7.90) | 5,331 /25 |
| Actual checker edit, before shorter entropy scan | 544.788 (4.89→5.83) | 28 /5,328 |

The actual checker edit changes all 721 compiler-dependent structural images; only 28 independent contracts retain identity. Same-input cross-producer images separately reuse all 749 contracts, not a whole-tree edit claim. The 60 s target is unmet; bench/README.md accounts every phase, the reported warm requests, unconverted closed CLI work and the named consumer_gate_closure pending case for programs (HTTP/program-directory first). The full entropy fill remains; its completed existential scan now exits early.

**Generated code stopped at step 1.** Whole-function allocation across blocks/loops and conditional callee saves already exist. Additional coalescing/operand use gained 1.00% wall/0.70% CPU, below 10%, and was discarded. Fixed-source wall ratios remain 1.826x opt0 and 6.053x -O; no speculative ABI/region change or cold-gate gain is claimed. Paired timings and loads are in bench/.

**Diagnostic causes: 16 tagged programs specimens, not attempt totals.**

| Cause | Count | Accepted rewrite |
| --- | ---: | --- |
| Expected Rust form/support gap | 8 | Typed inference 3, collection views/iteration 3, explicit trait impl 1, text fold 1 |
| Bad/incomplete message | 5 | Located caller/errno, named type/storage operation, value-less no-store |
| Rule with no Rust counterpart | 3 | Owned text/stable owner index and authenticated child image |

The binary's delta command prints frequent accepted forms. Mandatory formatting/fragment markers were declined without cause evidence.

**Authenticated child images.** build-child binds exact ELF/entry/ABI and canonical checked capability slots; image-authority verifies it. SealedImage retains the authenticated sealed file through Process.spawn_file, without a pathname reopen. The gate's 679 admission attacks and production descriptor-exec witness passed in 6.639 s (load 5.07→5.30); 25 optional native startup attacks also pass. No pre-admission descriptor writes. Programs now owns transfer/lifecycle/consumer completion of external_child_authority; compiler next integrates source/IR queries and checked text transactions, then the size ladder.

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
| Live IR admission, excluding producer build | 0.735 (33.28→33.28) | Value42, authority refusal, six mutants, located trap, deadline and recovery |
| Pool publication kill campaign | 3.455 (8.16→9.03) | 20 parent kills; complete old/new128-byte result; 32 adopted workers reaped |
| Independent process OS witness | 1.596 (7.70→7.48) | Pre-exec deadline, pre-prctl parent death, ancillary/buffer/descriptor cleanup |
| Consumer closure cold +producer renewal | 73.330 (4.15→10.15) | All32 HTTP/client/program-directory observations reused; zero reruns |
| Actual HTTP /program-directory wrappers | 30.544 (13.09→15.21) /43.107 (15.21→13.32) | Live I/O stays fresh; own processes, ports and state cleaned |

Targeted final IR campaigns reject all552 worker/region and517 foreign-session mutants; 46+2 controls admit, 48 Copy/Data checks pass. Earlier audit findings were fixed before these runs.
Remaining handoffs: semantic message identity/generated codecs, other capability message kinds, compiler-driver pool adoption, narrow admission provenance and runtime-grant key exclusion (including hardlinks). Foreign descriptor/shared-region transport, same-address-space threads and other targets remain explicit gaps.
Code wins: the old three scoped-concurrency pending cases were stale; only borrowed_closure_escape exists. consumer_gate_closure was prose; it now has an executable cross-producer proof. The initial worktree was eight commits behind its fetched baseline, not four.
This note touched24 tagged diagnostic/handoff records: Rust-form12 /new-rule8 /bad-message4, not attempt totals. The graph had four repaired diagnostic cases; all retry causes remain under tests/diagnostics.
Integration is complete; final empty-store four-worker gate and landing are next. Programs seed remains unchanged.
