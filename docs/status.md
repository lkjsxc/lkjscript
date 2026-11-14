# Status — 2026-11-14, native source producer

The production source front end now rebuilds itself through the already
self-built K compiler. **The front-end ELF emitted by B0 is byte-identical to
frontend1 and frontend2.** Both native generations emit B0's exact K for their
own declared source bundle.
The production source-to-K and K-to-ELF implementations can run without Rust.
Rust remains a bootstrap/reference and development-test dependency; Node still
packs declared inputs and orchestrates tests.

## Counted checkpoint

| Observation | Result | Record |
| --- | --- | --- |
| First complete front-end ownership pass | 1,424 source-body attempts; 62 E_BORROW and 1 E_MOVE; no declaration/type blockers | observations/2026-11-14-frontend-first-ownership.json |
| Ownership repairs | 78 observed sites, including 15 exposed after the first pass; rules unchanged | observations/2026-11-14-frontend-port.json |
| Current World source producer | 1,433 source-body attempts; 7,334 K functions; zero diagnostics | observations/2026-11-14-frontend-port.json |
| Host port versus preserved B0 | 268 identical K outputs, 321 matching rejections, 4 unchanged pending; zero failures | observations/2026-11-14-frontend-port.json |
| Native producer versus B0 | 268 identical K outputs, 321 matching rejections, 4 unchanged pending; all 593 targets agree | observations/2026-11-14-frontend-native-corpus.json |
| Native own-source K | 18,489,341 bytes, identical in both generations | observations/2026-11-14-frontend-selfhost.json |
| Front-end ELF fixed point | 50,703,986 bytes; frontend-b0 = frontend1 = frontend2 | observations/2026-11-14-frontend-selfhost.json |
| Same source-to-K work | B0 2.24 s / 206 MiB; native frontend1 246.20 s / 739 MiB peak RSS | observations/2026-11-14-frontend-selfhost.json |
| Same K-to-ELF work | B0 0.91 s / 335 MiB; native kc 110.82 s / 733 MiB peak RSS | observations/2026-11-14-frontend-selfhost.json |
| Full gate | Green: 440 Rust tests, 17 harness tests, all-target Clippy clean; 72.566 s with 8 workers | observations/2026-11-14-frontend-gate.json |
| Execution conformance | 883 pass and 4 pending on each route; 573 Rust comparisons; 91 World observations | observations/2026-11-14-frontend-gate.json |

The native source corpus includes all 592 source conformance files plus the K
compiler source closure; it completed in 761.714 seconds with four workers.
Rejection equality means the same exit status and first diagnostic code; accepted
K is compared byte for byte. These bootstrap observations are separate from the gate.
Source-body attempts and instantiated K functions are different counts.

## Decisions and evidence

The port preserves the producer's algorithms and K numbering/order. Rare host
forms use concrete owned collections, indices and shorter lookup scopes.
Decimal-to-binary64 conversion is now exact ordinary source code, tested against
752 Rust oracle cases and exercised on both native and K execution routes.
Unsized source locals are diagnosed before their defining location is lost.
The trusted K checker, K rules, encoder, runtime and source library did not change.

The pure boundary and World driver share one SourceBundle-to-K implementation.
The explicit LKJSB1 bundle contains the root, optional library root and exact
logical-path/source-text pairs. Neither path loads hidden source bytes.
See [frontend.md](frontend.md) for the interface and reproducible tools.

The first native measurements above were unprofiled and separate from the gate.
The second backend emission overlapped the gate. The second front-end run
included one eight-second profile window; it took 253.54 s with 739 MiB peak RSS.
These are shared-host observations, not acceptance limits or speed requirements.
The experiment selected 8 GiB address space and inherited an 8 MiB stack;
plain program execution still inherits its operator's controls.

The earlier K-compiler fixed point, runtime-memory corrections, sharded gate and
K mutation evidence remain in their observations and Git history. The current
gate retains zero accepted disagreements and one known conservative source
ownership rejection; its mutation timeout remains a separate unresolved outcome.

## Next obligation

Make the native pipeline the routine compiler path over explicit inputs. Rust should remain only where it supplies
useful reference or development evidence. Port the small input-packing and
orchestration pieces as real consumers require them.

Before choosing a speed change, measure the complete source-compilation workload.
An eight-second elaboration sample found repeated full-type-table boolean vectors
in `has_view` and `is_copy`, along with allocation and call-prologue costs; it is
a bounded sample, not an overall cost ranking. Its exact window and raw evidence
are in observations/2026-11-14-frontend-profile-window.json. Keep substantial
optimization in rechecked K-to-K passes and preserve the independent K machine.

Runtime resource-trap filenames still lose source-file provenance at the host
boundary, and finite exhaustion locations may differ between representations.
