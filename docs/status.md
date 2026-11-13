# Status — 2026-11-13, outside review (e)

The production K compiler and its World driver now pass the complete source
pipeline: **833 source-body attempts, 2,978 instantiated K bodies and 200,616 K
instructions, with zero unsupported, blocked or semantic diagnostics**. The
native operand cache, compact array repeats and unreachable-function removal
are implemented. Rust still builds B0; kc1/kc2 follow the allocation work next.

## Counted checkpoint

| Observation | Result | Reproduce |
| --- | --- | --- |
| Recorded first ownership run | 12 E_BORROW, 0 unsupported; 823 source-body attempts | observations/2026-11-13e-first-ownership.txt |
| Current World compiler driver | Zero diagnostics; 2,978 K bodies / 200,616 instructions | LKJSCRIPT_TIMINGS=1 target/release/lkjscript check --census src/kc_main.rs |
| Source ownership repairs | 26 source sites across the 12 initially failing bodies; no ownership rule change | observations/2026-11-13e-ownership-rewrites.json |
| Native changes on fixed K inputs | All 7 values/output agree; median of 3 runs: fib 336→191 ms, sort 1,927→1,172 ms, map 2,492→1,700 ms | observations/2026-11-13e-encoder.json |
| Hello size | ELF 9,393,606→83,398 bytes; K 828,726→95,496 bytes | observations/2026-11-14-array-repeat.json |
| Repeat representation | Both 16 and 8,192 elements: 6 locals, 8 instructions, 1 repeat intrinsic | cargo test --test array_repeat |
| World driver adapter | 16 observations: 1 check success, 12 error cases, 3 Cargo-kc-identical artifacts | cargo test --test kc_world |
| Full gate | Green, 55.678 s; 421 Rust tests, all-target Clippy clean | bash tools/check.sh |
| Conformance, each route | 883 pass, 4 pending; 573 Rust comparisons | node tools/conformance.mjs |
| Cargo K compiler parity | 562 verdicts, 64 rejections, 498 identical artifacts, 2 pending | node tools/kc.mjs |
| World/generators | 91 World observations; no accepted disagreements, one known conservative ownership rejection | bash tools/check.sh |

The first 823 count records source-body attempts, not instantiated K functions.
Each invalid body stops at its first diagnostic; the twelve were the first
verdict, recorded before fixing them. Current counts include the World entry
and compact repeats. Raw observations remain under observations/.

## Decisions

Facts use an owned packed bit vector; immutable checker tables use owned vectors.
Rare compiler forms were rewritten by the two-way rule. Ordinary source forms
needed by the compiler were added through existing K and source-library code:
shared primitive operators, checked mixed-width shifts, Result ? from &str to
String, structural ordering, collection operations, text/slice iteration and
parsing. The Unicode constructor and compact array-repeat intrinsic have exact
checker contracts and independent K-machine implementations.

The native encoder caches only the top operand and flushes at control/call
boundaries. Reachability starts at the selected entry and all table targets,
then follows direct calls. It preserves indices/signatures and omits dead code.
Every input K body is still checked before lowering; this is not a separate
validator for arbitrary malformed lower-only programs.

Array repeats evaluate the seed once, physically copy Copy values and transfer
the original last. They do not invoke Clone. Zero/one-element behavior remains
unchanged. The World compiler entry shares all nine production modules with B0;
its inputs are explicit relative program-K/runtime-K paths and its output is ELF
on stdout. Adapter tests do not establish self-compilation.

## Next obligations

Fix payload size classes, tip growth and the stack/heap split. Remove the
runner's small default address-space quotas and give stack extent its own
control while preserving tighter inherited operator limits. Ordinary programs
whose data fits comfortably in memory must run under the default.

Then shard conformance, adopt direct K mutation, and align unbounded-recursion
resource classification. Build kc1, compare its verdicts/bytes with B0, build
kc2 and measure the fixed point beside B0 in time and peak memory.

Afterward profile kc, keeping substantial optimization in rechecked K-to-K
passes. Measure the front-end census before choosing a port or a rewrite with
B0 K-text identity. No unrelated front-end width work precedes these needs.

The native 256 MiB / worker 512 MiB defaults and quarter-extent heap split still
exist at this checkpoint. Runtime trap filenames also lose source-file
provenance at the host boundary; lines come from the trapped function. See
limits.md for current controls. Neither is hidden by the green regression gate.
