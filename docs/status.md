# Status — 2026-11-13, outside review (e)

The production K compiler builds itself: **B0 = kc1 = kc2 = kc3**, with complete
recorded K-corpus parity. The first profile-driven runtime change preserves that
fixed point and reduces the mean of the two necessary native self-builds by
**9.7%**. The source front end still needs Rust; its measured route is a port of
the existing producer.

## Counted checkpoint

| Observation | Result | Reproduce / record |
| --- | --- | --- |
| First ownership verdict, before repairs | 12 E_BORROW; zero declaration/type blockers; 823 source-body attempts | observations/2026-11-13e-first-ownership.txt |
| Ownership repairs | 26 sites across the 12 initially failing bodies; ownership rules unchanged | observations/2026-11-13e-ownership-rewrites.json |
| Current World compiler source | 833 source-body attempts; 2,982 K bodies; 200,997 instructions; zero diagnostics | target/release/lkjscript check --census src/kc_main.rs |
| Current fixed point | B0 = kc1 = kc2 = kc3, 16,712,946 bytes each | observations/2026-11-13e-selfhost-class-index.json |
| Same own-K emission | B0 0.32 s / 124 MiB; kc1 36.79 s / 300 MiB; kc2 35.95 s / 300 MiB | observations/2026-11-13e-selfhost-class-index.json |
| Profile-driven source allocator change | Previous self-builds 40.39 / 40.15 s; program K unchanged; one helper removed | observations/2026-11-13e-kc-profile.json |
| Native K compiler parity | 562 matching verdicts, including 64 rejections; 498 identical ELFs; 323 source rejections before K; 2 pending; zero failures | observations/2026-11-13e-selfhost-class-index.json |
| Plain run memory | 20 MB repeated Vec, 100 MB pushes and 200 MB stdin succeed | observations/2026-11-13e-memory.json |
| Explicit-budget memory | 100 MB pushes fit 256 MiB AS; 200 MB stdin fits 512 MiB AS | observations/2026-11-13e-memory.json |
| Warm conformance batch | 27.922 s serial → 8.310 s with 4 workers; all 887 records identical per route | observations/2026-11-14-gate-economy.json |
| Gate K mutation, seed 1701 / 128 edits | 100 rejected, 23 equal values, 4 equal traps, 1 two-route timeout; zero findings | node tools/generators/k_mutate.mjs target/release/lkjscript 1701 128 |
| Fresh K mutation, seed 1702 / 1,024 edits | 830 rejected, 173 equal values, 19 equal traps, 2 equal R0008; zero findings or unresolved cases; 685 unique texts | observations/2026-11-14-k-mutation-1702.json |
| Rust execution worker | 24 direct/mutual recursion observations report R0008 at unchanged budgets; idle mapped space 200,576 → 17,096 KiB | observations/2026-11-13e-host-resources.json |
| Outside-review recursion mutants | Both mutants now R0008 on both routes; exhaustion locations retained separately | observations/2026-11-13e-memory.json |
| Front-end census | 55 new module files / 31,703 physical lines plus 9 shared self-built files; temporary valid-Rust probe exposes 310 diagnostics | observations/2026-11-14-frontend-census.json |
| Final local gate | Green, 35.274 s after the CI-discovered host fix; 425 Rust tests; 17 Node tests; all-target Clippy clean | bash tools/check.sh |
| Conformance, each route | 883 pass, 4 pending; 573 Rust comparisons | node tools/conformance.mjs |
| World / source generators | 91 World observations; no accepted disagreements; one known conservative ownership rejection | bash tools/check.sh |

Source-body attempts and instantiated K bodies are different counts. The first
twelve ownership errors were saved before repairs; they did not forecast all edit
sites. Self-build times are two necessary generations, not a repeated benchmark
or an acceptance limit. Raw records and earlier checkpoints remain in observations/.

## Decisions

Facts and immutable checker tables use owned vectors. Rare compiler forms were
rewritten by the two-way rule; ordinary forms use existing K and source library
code. The Unicode constructor and compact array-repeat intrinsic have exact
checker contracts and independent machine implementations.

The native encoder caches the top operand and emits only functions reachable
from the entry or an indirect-call table target, after every input K body is
checked. Array repeats evaluate the seed once and physically copy Copy values.

Allocator classes describe payload capacity, with a separate header retaining
the computed class. Release indexes its free list directly. Tip blocks grow in
place; other growth transfers storage through source reallocate. Native startup
maps the selected stack first and gives the heap the remainder. Plain run
inherits the caller's address-space limit; the independent stack control
preserves tighter inherited limits. The K model charges actual frame/root-cell
storage, while aggregate and container payloads remain on the heap. Rust compiler
threads join before execution. Workers share one glibc arena by default, avoiding
secondary arena reservation overhead; explicit allocator policy is respected.
Static model resource diagnostics do not allocate their messages.

The gate reserves internal suite workers against one total. Direct K mutations
retain seeds, edits, hashes and independent process outcomes. Timeouts and
one-sided resource exhaustion remain unresolved observations.

## Next obligation: port the source front end

The first untouched producer blocker is resolve::resolve_nodes at
src/resolve.rs:948: an impl FnMut callback parameter. Retain the producer's
algorithms and their K ordering. The census crossed
declaration barriers with a 15-file temporary patch and no deleted bodies; it
reached 149 E_TYPE, 100 E_UNSUPPORTED, 48 E_BORROW, 8 E_TRAIT and 5 E_NAME.
That probe is not merged, has no complete K artifact, and has not established
behavioral equivalence. The 310 first faults are not all remaining edits.

Keep exact B0 K baselines, deliberately rewrite the rare declaration and
representation forms, then recount body diagnostics. Apply the two-way rule to
each consumer-driven family and require K-text identity on conformance and the
producer's own source. See direction.md and the census record for the decision.

For further speed work, profile the retained compiler again before choosing the
next change. Keep general optimization in rechecked K-to-K passes and the
encoder small; no general optimizer has been introduced.

Runtime trap filenames still lose source-file provenance at the host boundary;
lines come from the trapped function. Finite resource exhaustion can occur at
different calls on the two representations. These limitations remain explicit.
