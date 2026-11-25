# Native register allocation trial

**Cut.** On the frozen compiler workload, mean wall time rose from 27.843333 s
to 28.903333 s, a **3.807% slowdown**. The candidate needed about 5% lower wall
time and agreement with the independent interpreter on original checked IR.
All nine measured executions produced the exact frozen image. The three
allocation changes were evaluated as one bundle. [Design and census](compiler-regalloc-design.md)

## Implementation

The hypothesis was that fewer register moves and frame accesses would shorten
the compiler build. The historical census exposed at most 1,299 of 14,658
samples (8.862%), an attribution ceiling used to select the trial.

The trial starts with the existing linear-scan assignments and saved-register
set. A whole-local primitive read or assignment can keep the dying source's
register when both lifetimes start in the same block, both assignments use
the same register bank and the register is free for the full destination
interval. The register pass checks the latest prior interval and the next
incumbent in order of interval starts. The existing emitter elides the
resulting self-move.

The remaining unaddressed primitive spills can use free volatile registers.
`Discriminant` preserves all five. `TagTest` reserves R10. A projected
primitive read reserves R10 and RSI. All other barriers retain the baseline
conservative classification. The planner preserves loop extensions and
keeps parameters out of volatile promotion. Residual spills can share a
representative frame slot when their size and alignment match and both
intervals lie in the same block with strictly disjoint points. Aggregate
slot coalescing keeps its own eligibility rule. The frozen witness had zero
emitted scalar zero-frame leaves, so the trial kept ordinary scratch
reservation. The checked IR, checker, interpreter and runtime were outside
the implementation diff. The implementation is
`37fcc6e7593d623753b9e420b2899ebbd4f2e178` and the frozen specimens are
`bd950fcdd`.

## Frozen measurement

Both native emitters were built from the `d461c2d9` generation with its seed.
Only the candidate's two allocator files differed. Each emitter constructed
the same `26bf2c3c021dd79810a81fa1d9054d59f3768b06` witness with
`--inline-limit 64 --no-store`. B reproduced the retained cohort-2 ordinary
ELF. The selected post-Inline64 IR matched byte for byte in both arms and
matched the retained hash
`32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca`.
Both builds accepted one recheck with 30,619 inlined calls in 5,617 functions,
4,455 in-memory checked-body reuses and zero rejected optimizer candidates.
R used the frozen Rust witness, `rustc 1.97.0 (2d8144b78 2026-07-07)`,
locked offline Cargo release, one Cargo job, one codegen unit and opt-level 3.
[Historical identity](compiler-profile-copy-sites-cohort-2.md)

| Producer | ELF bytes | ELF SHA-256 | Construction wall s | CPU s | Peak RSS KiB |
| --- | ---: | --- | ---: | ---: | ---: |
| B | 22,936,514 | `20cfe7aae7dddba36da8edca9b56e17f8983bed7e48eed965ce0a01d43b40856` | 33.02 | 32.15 | 769,308 |
| C | 22,805,442 | `453e90963ba8bb55a6303f56a1f2bfc14ea0446addf3ac9fd748f0b834dac185` | 38.62 | 37.58 | 784,012 |
| R | 6,040,720 | `050a380a2acd02302debea4dc0bf051b16059be4f4403944ab0b66cc51b269b7` | 52.72 | 51.56 | 1,043,892 |

The B and C emitters had SHA-256
`6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc` and
`4fa6f4ab28a7fb3549f4841edd0c5536b7ed837cfdf1b74f391458c571b5bd57`.
The selected IR and ELF encode source paths. Construction preflights with a
relative root and then a different absolute root exposed this dependency.
A read-only bind mount in a private namespace reproduced the retained
absolute paths for both final constructions and all executions. The two
preflights stopped at construction.

The serial cohort ran on the shared AMD Ryzen 9 9955HX host from
19:51:35 to 19:54:39 JST. Each producer built literal `src/main.lkj` with the
same frozen source root, library and runtime, `--inline-limit 0 --no-store`,
and a fresh output path. Each row used the historical external 4 GiB
address-space limit, 900 s watchdog and 10 s kill grace. GNU time supplied
elapsed wall, user CPU, system CPU and peak RSS. Construction, verification
and profiling were outside the timed rows. The cohort contains all nine
rows in the specified order. Each command exited zero with empty stdout
and stderr. Every output was byte-for-byte identical to the 16,228,028-byte
frozen image, SHA-256
`8ca3225bebb5738376479d7556cd6d88b666805391fd2477331523ee6c666132`.

| Order | Wall s | User s | System s | Peak RSS KiB | Load before → after | Start–end JST |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| B1 | 31.21 | 28.72 | 0.37 | 502,644 | 26.63/16.53/11.89 → 26.83/17.61/12.40 | 19:51:35–19:52:06 |
| C1 | 30.17 | 28.85 | 0.54 | 508,708 | 26.83/17.61/12.40 → 24.80/18.00/12.70 | 19:52:06–19:52:36 |
| R1 | 4.60 | 4.28 | 0.16 | 364,724 | 24.80/18.00/12.70 → 24.50/18.05/12.75 | 19:52:36–19:52:41 |
| C2 | 29.23 | 28.03 | 0.31 | 506,560 | 24.50/18.05/12.75 → 23.84/18.57/13.09 | 19:52:41–19:53:10 |
| R2 | 4.63 | 4.38 | 0.15 | 360,564 | 23.84/18.57/13.09 → 23.21/18.52/13.11 | 19:53:10–19:53:15 |
| B2 | 26.67 | 25.27 | 0.28 | 510,584 | 23.21/18.52/13.11 → 22.16/18.66/13.30 | 19:53:15–19:53:41 |
| R3 | 4.41 | 4.09 | 0.13 | 362,768 | 22.16/18.66/13.30 → 22.06/18.70/13.34 | 19:53:41–19:53:46 |
| B3 | 25.65 | 24.69 | 0.27 | 508,616 | 22.06/18.70/13.34 → 20.72/18.64/13.46 | 19:53:46–19:54:11 |
| C3 | 27.31 | 26.43 | 0.32 | 498,200 | 20.72/18.64/13.46 → 20.45/18.76/13.64 | 19:54:11–19:54:39 |

| Arm | Wall total s | Mean wall s | Mean CPU s | Mean peak RSS KiB | Wall / Rust -O |
| --- | ---: | ---: | ---: | ---: | ---: |
| B | 83.53 | 27.843333 | 26.533333 | 507,281 | 6.124× |
| C | 86.71 | 28.903333 | 28.160000 | 504,489 | 6.357× |
| R | 13.64 | 4.546667 | 4.396667 | 362,685 | 1.000× |

The paired C-minus-B wall differences were −1.04 s, +2.56 s and +1.66 s.
Mean CPU rose 6.131% while the producer shrank by 131,072 bytes. The host
load moved during the cohort, and the experiment identifies the whole
bundle's result only. The observed slowdown fails the retention rule.

## Native qualification

The qualification used emitter builds from the current checkout of the
baseline and the trial. The 22 frozen cases and existing depth-two
`continue-to` case agreed with the baseline independent interpreter on
original checked IR. Both native producers ran the specimens with
`--no-opt` and `--inline-limit 64`, using fresh outputs and `--no-store`.
Source cases also replayed their exact checked text IR, with identical
source/IR images within each arm and policy. The comparison covered
complete stdout and stderr bytes, process status, exact NaN bits and
located traps. The forged uninitialized read produced the same `E_MOVE`
diagnostic and left no executable. The distinct-file bounds trap named
`failing.lkj:2`. The secondary entry trapped at `custom-entry.lkj:7`
with `R0016 regalloc: ordered é`.

Disassembly showed the dead-source `Read` and `Assign` lose
`mov r8,r9` and return directly from R9, while the live-source twin
keeps the move. The pressure fixture moved the sentinel 4 from
`[rbp-0x20]` into RDI across the tag and projected read. The projected
address still uses R10 and RSI. The call fixture kept both live values
in spill homes through the call and used `[rbp-0x8]` and `[rbp-0x10]`
again for two later disjoint intervals. It reduced the frame from
144 to 128 bytes. The rotation emitted `ror r8d,0x8` and the aliased
right operand used EAX for subtraction before writing its destination.

| Temporary native fault | Result of the focused control |
| --- | --- |
| Coalesce a live source | The result changed from 43 to 2. |
| Permit R10 across tag and projected address operations | The result changed from 65 to a stack-address-derived integer. |
| Share overlapping spill homes across a backedge | The result changed from 68 to 49. |
| Omit the live R15 restore on recursive return | The result changed from 112 to 115. An initial RBX omission left this fixture's result unchanged. Disassembly identified that R15 carried the live sentinel. |
| Omit general scratch in a non-leaf | The independent frame check found the caller's frame changed from 128 to 64 bytes. The value remained 112. |

Ordinary emitted images produced the interpreter's four ordered stream
bytes, including destructor effects and the returned-view read. The
two-unit imported call returned 42 through both producers and the
interpreter's whole-source route. B and C each checked and emitted two
units on their first build and reused both on their next build. C's
first build ran over B's populated store. Each producer's warm image was
byte-identical to its fresh image. The transfer-only child suite passed
on both producers, including 679 admission controls per producer.

The gate's bootstrap ran `sh tools/bootstrap.sh` in 308.166 s. The
seed needed one bridge. The two current-emitter generations reproduced
the same 42,198,703-byte self-hosted compiler, SHA-256
`1fdee02c0336ad235dbf421a63d69c8791389966519283947bfa85b7248e2c22`.
The full four-worker gate passed in 2,559.545 s, rerunning 7,356 case
requests with zero request reuse. Both conformance policies passed 3,484
route checks and retained two known pending routes for borrowed closures.
The gate checked one located heap exhaustion and one located stack
exhaustion. Its original-IR mutation ran 128 cases with zero findings.
A separate `mutate 1701 128 --jobs 4 --no-store` reproduced that result
with 104 rejections, 22 values, one trap and one nontermination.

The branch restores the baseline allocator and retains the frozen
differential specimens and this report.

## Draft for the optimization ledger

Append this row in the frozen compiler workload section of
`docs/optimization-ledger.md`.

| Idea | Hypothesis / Bar | Method | Measured Result | Verdict | Why | Commit / Report Link |
| --- | --- | --- | --- | --- | --- | --- |
| Native register allocation trial | Same-bank transfer affinity, refined volatile clobbers and equal-width spill-home reuse could reduce moves and frame traffic. Retention requires about 5% lower frozen wall time and original-IR interpreter agreement. | Reconstruct matched historical Inline64 producers, compare the retained selected IR and run B/C/R in three serial interleaved blocks. Compare native values, trap payloads and effects against the independent interpreter and inject four native faults. | **−3.807% reduction** (27.843333 → 28.903333 s). C was 6.357× Rust `-O` versus B at 6.124×. All nine builds emitted the exact frozen image. | Cut. Restore the baseline allocator. | The image shrank by 131,072 bytes, but measured wall time and CPU rose. The combined trial gives no separate timing for its three mechanisms. | `37fcc6e75`, [native register allocation trial](../bench/compiler-regalloc-impl.md) |
