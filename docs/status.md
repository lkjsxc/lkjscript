# Status — 2026-11-13

Host-boundary and World-input steps are complete. The production K compiler is
isolated behind explicit program-K/runtime-K inputs, with native byte identity.
It is cargo-built; no self-build or fixed point is claimed. Native and the
independent K machine are the execution routes. WebAssembly and its Node host
are gone; Node remains test orchestration.

## Current observations

| Observation | Result | Reproduce |
| --- | --- | --- |
| Full gate | Green, 39.503 s including 1.21 s release build | bash tools/check.sh |
| Conformance, each route | 853 pass, 4 pending; 547 Rust comparisons | node tools/conformance.mjs target/release/lkjscript |
| Bootstrap tests | 412 pass; Clippy all targets clean | cargo test; cargo clippy --all-targets -- -D warnings |
| Cargo K compiler parity | 535 verdicts, including 64 rejections; 471 identical artifacts; 2 pending | node tools/kc.mjs |
| World end-to-end | 91 observations, no failures, across standalone/native/K/Rust | node tools/world-io.mjs |
| Host boundary | Resource-independent ELF bytes; located resource traps; bounded output capture | bash tools/host-boundary.sh target/release/lkjscript |
| Full source recovery | 470 first semantic sites, down from outside review's 494 | target/release/lkjscript check --census src/lib.rs |
| K consumer recovery | 105 sites, 720 bodies; 18 reach ownership construction | target/release/lkjscript check --census src/kc_lib.rs |

The K root includes runtime sealing and excludes the independent model. Recovery
is partial: 36 attempted bodies fail, 16 are blocked, and most ownership verdicts
remain unknown. No total self-compilation distance follows from these counts.
The gate finds no accepted-program disagreement; the known conservative ownership
rejection remains. Counts include test migration and are not a width score.
Commands and machine-readable observations: observations/2026-11-13-checkpoint.json.

## Implemented course

Ordinary text comparisons/patterns, String coercions/construction/ordering, byte
strings, declaration-level non-test cfg, struct Default, scalar helpers, owned
Vec iteration, retain and sort now run. Diagnostics name source types and useful
rewrites. The outside calculator v1 checks; v0 gets a located owned-value/index
rewrite for the deliberate no-heap-views rule. Field/inner cfg and capability
method collisions reject explicitly.

World supplies owned argument bytes, binary standard streams, normal exit status
and files beneath directory capabilities. Handles close through local and nested
cleanup. Kernel confinement rejects parent/absolute/symlink escapes; no weaker
fallback is used. Program stderr and status cannot impersonate trap control.
Closed descriptors and broken pipes agree across the Rust and native hosts.

The source ordered map uses an indexed AVL tree. On xorshift insertion plus
333335 lookups, 50000 inserts moved from 9.647 s to 0.468 s. At 400000, native
takes 2.414 s versus 0.120 s for the same AVL source built by optimized Rust.
Raw samples, budgets and provenance: observations/2026-11-13-maps.json.
Generated-code performance still needs work on the real K compiler workload.

## Next consumer obligation

Close the real K compiler: replace shared Rc facts/view state with owned indices
where that is cheaper, rewrite ordinary uses or add source-library operations
from actual diagnostics, and reach its first complete ownership pass. Keep
prepared runtime K explicit. Then build kc1, compare verdicts and bytes with B0,
build kc2 and measure the fixed point. Full front-end closure and Cargo removal
follow those observations.

General From/Into, iter_mut, enum Default and broader associated-type evidence
remain unfinished. Concurrency and speculative reference contracts remain parked.
The existing K source rewrites removed a new collect dependency and an unnecessary
array-iterator dependency without adding a checker rule.

Native child budgets default to 256 MiB; K and batch workers to 512 MiB. Budgets
are operator choices, never source ceilings. Some infallible Rust allocations can
still abort a worker; the parent reports a host failure without a fabricated line.
See limits.md for controls and remaining representation/host constraints.
