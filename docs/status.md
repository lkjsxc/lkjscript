# Status — 2026-11-13

The production K compiler is the first self-compilation target. Its isolated
cargo build now takes program K and prepared runtime K and emits the same ELF as
the complete compiler. It is not self-built. Native and the independent K machine
are the execution routes; WebAssembly and its Node host have been removed.

## Current observations

| Observation | Result | Reproduce |
| --- | --- | --- |
| Full gate | Green, 43.144 s including 16.282 s release build | bash tools/check.sh |
| Conformance, each route | 841 pass, 4 pending; 540 Rust comparisons | node tools/conformance.mjs target/release/lkjscript |
| Bootstrap tests | 412 pass; Clippy all targets clean | cargo test; cargo clippy --all-targets -- -D warnings |
| Cargo K compiler parity | 528 verdicts, including 60 rejections; 468 byte-identical artifacts; 2 pending | node tools/kc.mjs |
| Host boundary | Byte-identical under different build resource limits; located native/K stack, heap and bounds failures | bash tools/host-boundary.sh target/release/lkjscript |
| Full source recovery | 470 first semantic sites, down from outside review's 494 | target/release/lkjscript check --census src/lib.rs |
| K consumer recovery | 105 semantic sites, 688 bodies; 18 reach ownership construction | target/release/lkjscript check --census src/kc_lib.rs |

The K source measurement includes runtime sealing and excludes the independent
machine. It is a partial recovery count: 36 attempted bodies fail, 16 are blocked,
and 670 have not reached layout/ownership. No claim about those ownership verdicts
or a fixed point follows from the cargo boundary check. The known generator
ownership rejection remains a deliberate conservative rejection; no generator
finds an accepted-program behavior mismatch.

Ordinary text equality/patterns, String coercions/construction/ordering, byte
strings, declaration cfg(test), Default, scalar helpers, owned Vec iteration,
retain and stable sort now run. The outside calculator v1 checks; v0 reports the
view-container rule at its expression with an owned-value/index rewrite.
Diagnostics name source types, receivers and conflicting expectations. cfg on
fields and inner attributes rejects explicitly instead of silently changing Rust
behavior. The view-container rule remains deliberate.

The source ordered map is an indexed AVL tree with free-slot reuse. On xorshift
inserts plus 333335 lookups, median native time at 50000 inserts moved from 9.647 s
to 0.468 s. At 400000 it finishes in 2.414 s under an explicit 1 GiB process budget;
the same AVL source built by optimized Rust takes 0.120 s. These working-tree
measurements and raw samples are in docs/observations/2026-11-13-maps.json. Generated
code remains a major cost; no speed ratio is an acceptance requirement.

## Open obligations

- World currently derives stdout. Add arguments, binary stdin/stderr, requested
  exit status and files beneath directory capabilities, with a matching Rust shim.
- Close the real K consumer. Rc-backed facts, shared view masks and scoped paths
  are candidates for one per-function owned arena. Cost and measure before replacing.
  Entry/retain/extend and ordinary source rewrites must serve this consumer.
- iter_mut needs a disjoint mutable slice contract; general From/Into and associated
  Item equality need definition-site evidence. They have not been added speculatively.
- Only then build kc1, compare every applicable verdict/byte with B0, build kc2,
  and measure the self-built workload. The full compiler and removal of Cargo follow.

Native execution defaults to 256 MiB and K workers to 512 MiB address space, with
explicit override. Artifacts decide extents at startup and report code/line to
stderr on traps. Some infallible Rust allocations can still abort inside K; the
parent reports a host failure without inventing a location. These are execution
budgets, never source limits. See limits.md for controls and remaining constraints.

Next action: inputs through World, then the K consumer's actual ownership pass.
