# Status — 2026-11-13, outside review (e)

The real K compiler source now checks cleanly: **823 source-body attempts,
2,960 instantiated K bodies and 214,157 K instructions, with zero unsupported,
blocked or semantic diagnostics**. The first twelve borrow rejections remain
recorded separately. Rust still builds B0; kc1/kc2 follow the speed, size and
allocation work below.

## Counted checkpoint

| Observation | Result | Reproduce |
| --- | --- | --- |
| Recorded first K consumer ownership run | 12 E_BORROW diagnostics, 0 unsupported; 823 source bodies attempted | target/release/lkjscript check --census src/kc_lib.rs |
| Current K consumer | Zero diagnostics; 2,960 actual K bodies / 214,157 instructions | LKJSCRIPT_TIMINGS=1 target/release/lkjscript check --census src/kc_lib.rs |
| First-rejection families | native 3, lower 2, checker 3, scoped checker 3, K text 1 | observations/2026-11-13e-first-ownership.txt |
| Native and lowering source rewrites | 7 fixed program/runtime K pairs each give byte-identical ELF; 3 native integration tests pass | cargo test --test native |
| Checker model/scaling | 34 unit and 2 scaling tests pass | cargo test --lib check::; cargo test --test check_scaling |
| New structural ordering | 4 fixtures agree with Rust/native/K, including NaNs and operand evaluation | tests/conformance/pass/derived_order_*.rs |
| Source library additions | 12 native/K observations agree; positive cases also agree with Rust | tests/conformance/pass/kc_*.rs |
| Unicode constructor boundary | 8 malformed intrinsic signatures rejected independently; scalar boundaries agree | cargo test --test unicode_scalar |
| Full gate | Green, 68.262 s; 413 Rust tests, all-target Clippy clean | bash tools/check.sh |
| Conformance, each route | 880 pass, 4 pending; 570 Rust comparisons | node tools/conformance.mjs |
| Cargo K compiler parity | 560 verdicts, 64 rejections, 496 identical artifacts, 2 pending | node tools/kc.mjs |
| World/generators | 91 World observations; no accepted disagreements, one known conservative ownership rejection | bash tools/check.sh |

The 823 count records source-body attempts, not the number of instantiated K
functions. Each invalid body stops at its first diagnostic, so twelve is the
first verdict rather than a prediction of all remaining edit sites. There are
no remaining reported type/construction blockers. Source diagnostic recovery
still cannot authorize an artifact. The earlier narrow 18-body recovery count
was not a measure of self-hosting distance.

The current checked source closure retains the recorded 823 body attempts.
Detailed gate data
is in observations/2026-11-13e-checkpoint.json.

Raw first-construction and first-ownership observations are retained under
observations/. A missing type annotation that formerly appeared at 0:0 now names
its containing function and location; this also prevents independent failures
from collapsing into one deduplicated diagnostic.

## Decisions made under the two-way rule

Facts now use a packed owned bit vector. Shared immutable checker tables use
owned vectors. Opaque iterators, map-entry conveniences, collect/flatten chains,
static views in heap storage, numeric host conversions and byte serialization
were rewritten into plain compiler code. The K compiler still uses the same
production modules as B0.

Ordinary source additions include shared-primitive operator forwarding,
mixed-width checked shifts, Result ? from &str to the canonical String,
structural PartialOrd/Ord execution, collection equality/owned iteration,
slice operations, text iteration/splitting and checked integer parsing. Ordering
is generated as checked K and preserves unordered floating comparisons. The
small char_from_u32 intrinsic admits only a checked Unicode scalar and the exact
Option<char> result shape; the independent K machine validates it separately.

Native and independent K remain the two execution routes. The K compiler takes
explicit program-K/runtime-K inputs. Wasm/Node execution stays removed; Node is
still temporary test orchestration.

## Next obligations

The recorded borrow failures are repaired by staging offsets, owning parser
work items and ending temporary borrows before mutation. Twenty-six source
sites were changed across the twelve initially failing bodies. Ownership rules
are unchanged. The seven fixed K/runtime pairs retain identical ELF bytes.

Next cache the native top operand, lower array repeats
compactly and omit unreachable function bodies before encoding. Fix allocator
size classes, tip growth and the stack/heap split before raising run defaults.
Shard conformance, add direct K mutation and align unbounded-recursion resource
classification. Build kc1, compare its verdicts/bytes with B0, build kc2 and
measure the fixed point beside B0 in time and peak memory.

After that, profile kc and place substantial optimization in rechecked K-to-K
passes. Measure the front-end census before choosing a port or a rewrite with
B0 K-text identity. No front-end width program precedes these consumer needs.

The source-closure checkpoint passes the full gate. The existing native
256 MiB / worker 512 MiB defaults and quarter-extent heap
split are still present at this checkpoint. Their rejection of ordinary data
sizes is a defect to fix by allocation ratio and explicit stack control, not by
adding source limits. Runtime trap filenames also still lose source-file
provenance at the host boundary; their lines currently come from the trapped
function. See limits.md for the current controls.
