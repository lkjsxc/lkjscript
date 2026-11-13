# Status — 2026-11-13, outside review (e)

The production K compiler has produced **byte-identical kc1, kc2 and kc3** from
its own K input and matches B0 across the complete recorded K corpus.
Allocator ratios, default execution resources, conformance sharding and direct
K mutation are implemented. The joint gate passes in **30.699 seconds**.

## Counted checkpoint

| Observation | Result | Reproduce / record |
| --- | --- | --- |
| First ownership verdict, recorded before repairs | 12 E_BORROW; 0 declaration/type blockers; 823 source-body attempts | observations/2026-11-13e-first-ownership.txt |
| Current World compiler source | 833 source-body attempts; 2,982 K bodies; 200,997 instructions; zero diagnostics | target/release/lkjscript check --census src/kc_main.rs |
| Ownership repairs | 26 sites across the 12 initially failing bodies; ownership rules unchanged | observations/2026-11-13e-ownership-rewrites.json |
| Fixed point | B0 = kc1 = kc2 = kc3, 16,712,946 bytes each | observations/2026-11-13e-selfhost.json |
| Same own-K emission | B0 0.32 s / 123 MiB; kc1 40.39 s / 300 MiB; kc2 40.15 s / 300 MiB | observations/2026-11-13e-selfhost.json |
| Native K compiler parity | 562 matching verdicts, 64 rejections, 498 identical ELF artifacts, 2 pending | node tools/kc.mjs target/release/lkjscript target/selfhost-observation/kc2.elf --world |
| Native encoder on fixed K | 7 values/output agree; median fib 336→191 ms, sort 1,927→1,172 ms, map 2,492→1,700 ms | observations/2026-11-13e-encoder.json |
| Hello size | ELF 9,393,606→83,398 bytes; K 828,726→95,496 bytes at step 2 | observations/2026-11-14-array-repeat.json |
| Plain run memory | 20 MB repeated Vec, 100 MB pushes and 200 MB stdin succeed | observations/2026-11-13e-memory.json |
| Explicit-budget memory | 100 MB pushes fit 256 MiB AS; 200 MB stdin fits 512 MiB AS | observations/2026-11-13e-memory.json |
| Warm conformance batch | 27.922 s serial→8.310 s with 4 workers; all 887 records identical per route | observations/2026-11-14-gate-economy.json |
| Direct K mutation, seed 1701 / 128 edits | 100 rejected, 23 equal values, 4 equal traps, 1 two-route timeout; zero findings | node tools/generators/k_mutate.mjs target/release/lkjscript 1701 128 |
| Fresh K mutation, seed 1702 / 1,024 edits | 830 rejected, 173 equal values, 19 equal traps, 2 equal R0008; zero findings or unresolved cases; 685 unique texts | observations/2026-11-14-k-mutation-1702.json |
| Outside-review recursion mutants | Both mutants now R0008 on both routes; exhaustion locations retained separately | observations/2026-11-13e-memory.json |
| Full gate | Green, 30.699 s; 425 Rust tests; 17 Node tests; all-target Clippy clean | bash tools/check.sh |
| Conformance, each route | 883 pass, 4 pending; 573 Rust comparisons | node tools/conformance.mjs |
| Cargo K compiler parity | 562 verdicts, 64 rejections, 498 identical artifacts, 2 pending | node tools/kc.mjs |
| World / source generators | 91 World observations; no accepted disagreements; one known conservative ownership rejection | bash tools/check.sh |

Source-body attempts and instantiated K bodies are different counts. The first
twelve errors were saved before any ownership repair; they were not a forecast
of all edit sites. Raw records and prior checkpoints remain in observations/.

## Decisions

Facts use an owned packed bit vector; immutable checker tables use owned vectors.
Rare compiler forms were rewritten by the two-way rule. Ordinary source forms
needed by the compiler were implemented through existing K and source library
code. The Unicode constructor and compact array-repeat intrinsic have exact
checker contracts and independent machine implementations.

The native encoder caches the top operand and emits only functions reachable
from the entry or an indirect-call table target. Every input K body is checked
first. Array repeats evaluate their seed once and physically copy Copy values;
they do not invoke Clone. Zero/one-element behavior is preserved.

Allocator classes now describe payload capacity, with the header separate. Tip
blocks grow in place; other growth transfers storage through source reallocate.
Native startup maps the selected stack first and gives the heap the remainder.
Plain run inherits the caller's address-space limit. LKJ_RUN_STACK_BYTES is
independent and preserves tighter inherited limits. The K model charges actual
frame/root-cell storage; array and container payloads stay on the heap.

The gate reserves each suite's internal workers against one total. Direct K
mutations use saved seeds, edits, input/binary hashes and independent route
processes. Timeouts and one-sided resource exhaustion remain unresolved
observations; they are not counted as semantic agreement.

## Next obligations

The actual self-build profile puts 64% of self CPU in runtime helpers
(observations/2026-11-13e-kc-profile.json).
Class computation accounts for 16% and is the first source-runtime experiment:
store the already-computed class in the block header and avoid recalculating it
on release. Preserve the fixed point and corpus parity after any retained change.
Keep substantial general optimization in rechecked K-to-K transformations and
the encoder small. The front-end census is now being measured to choose a port
or rewrite with B0 K-text identity; no unrelated language-width work precedes
that decision.

Runtime trap filenames still lose source-file provenance at the host boundary;
lines come from the trapped function. Finite resource exhaustion may occur at
different calls on the two representations. These limitations remain explicit.
