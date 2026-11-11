# Status

Generation two runs. Generation one is preserved on `generation-one` at
`05a9326c`, with a read-only `.generation-one` worktree. Old untracked drafts are
preserved under `.artifacts`; the unused 35 GB generation-one target was removed.

## Works

Rust-compatible parsing; functions, checked integers/f64, structs, enums, matches,
loops, generic instantiation, trait methods/associated types/prerequisite bounds,
Box and Vec, partial moves/reinitialization, projected references, disjoint mutable
field borrows, and last-use borrow endings. Explicit K checks types, ownership,
loans and enum variants once. Text K can be read, written and checked directly.
The backend emits memory64 WebAssembly with reusable allocation and explicit drop.

`check`, `run`, `test`, `build`, `k` and `trusted` work. Node runs the generated
modules. Source libraries own Option/Result, byte operations, vector ordering and
an ordered map over sorted entries (logarithmic lookup, linear insertion).

The complete check takes **3 seconds** on this host: formatting, 56 Cargo tests,
Clippy, and conformance. Conformance: **40 pass, 11 explicitly pending, 0 fail**,
with **33 rustc comparisons**. Hand-written invalid K tests reject forged
capabilities, unproved variants, dangling references, bad dead code and missing
cleanup. Backend tests include 195 arithmetic edge comparisons and allocation
reuse. Implemented conformance cases cannot silently become pending again.

## Missing or broken

Reference-containing aggregate views, arrays/slices, custom Drop, closures,
capability callbacks and structured concurrency remain unsupported. The 11 pending
cases name these boundaries. Vec/Box/allocator operations still live in the
emitter, so the standard-library milestone is incomplete. String is not runnable.
Module loading/name resolution and the compiler's own source subset are incomplete.
No self-check, W1=W2, own machine, native backend or N1=N2 has been established.
Only Node execution exists; its capacity limits remain relevant.

## Bootstrap and bridges

`cargo build --release` builds the initial compiler. `bash tools/check.sh` is the
routine gate. `lkjscript run file.rs --entry probe` prints a scalar result;
`lkjscript build file.rs -o file.wasm` emits a module. `lkjscript check file.k`
checks hand-written K; `lkjscript k file.rs` writes explicit K text.

Cargo, bridge/main.rs and bridge/run.mjs are bootstrap bridges. Move native host
operations behind World when source support permits; replace the Node dependency
with our machine after the WebAssembly fixed point. The Rust-compatible sources
are not yet all accepted by the new compiler; no self-hosting claim is made.

## Next action

Track one-source view origins through structs/enums and calls, add arrays/slices,
and make cleanup on early return/break observable with custom Drop. Then complete
the source memory/collections boundary and measure whole-compiler self-check.
