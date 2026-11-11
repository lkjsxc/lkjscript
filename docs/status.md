# Status

Generation two is a runnable bootstrap. Milestone 0 is complete; milestones 1 and
2 are partial; whole-compiler self-check and both fixed points remain unestablished.
Generation one is preserved on `generation-one` at `05a9326c`, with the read-only
`.generation-one` worktree. The unused 35 GB old target was removed without
stopping deployments. Old untracked drafts remain under ignored `.artifacts`.

## Works

Functions, checked fixed-width integers and f64, structs, enums, matches, loops,
generic instantiation, associated types and prerequisite bounds run on Node.
Ownership includes partial moves/reinitialization, affine Vec elements, recursive
Box types, borrowed matches, disjoint mutable fields, one-source aggregate views,
array/slice projections, explicit borrow endings and custom Drop on normal and
early exits. Calls can reborrow mutable references; `v.push(v.len())` works.
Trait coherence, signatures, structural Send/Sync/Data bounds and expanding
parameter-flow cycles are checked. Ambient I/O and direct Drop calls are rejected.

K has a text form for adversarial tests and records concrete types, places,
copies, moves, borrows, drops, variant proofs and a projected result's source.
The K checker independently checks those concrete operations. Generic selection,
witness selection and finite-instantiation validation currently precede K;
explicit generic/witness contracts in K remain required work.

`check`, `run`, `test`, `build`, `k` and `trusted` work. The memory64 backend
has reusable allocation, checked indexing, canonical arithmetic NaNs and
structural/custom drop. The source library implements Option/Result, bytes and
UTF-8 validation, String, vector ordering and a sorted-vector BTreeMap with
logarithmic lookup and linear insertion/removal. Library integration tests run
these implementations under both compilers. The CLI loads only the prelude and
vector comparison helper automatically; general module loading is still missing.

## Verification and measurements

The 2026-11-11 provenance repair passes **113 Cargo tests** and the full gate
(**64 conformance pass, eight pending, zero fail; 57 rustc comparisons**). The
clean isolated build/gate took 4.121 s. Nineteen new tests exercise returned and
installed replacement values, cloning, direct and projected reference assignment,
empty enum alternatives, exclusive/disjoint valid uses, and external writes.
Direct K fixtures reach the checker without frontend filtering. External view
writes with changed symbolic leaf origins are deliberately rejected until K has
mutation contracts; clearing or retaining their exact origins remains supported.

Earlier measurements at `aff27e54` on this host (Rust 1.97.0, Node 24.18.1):
the incremental full check took
**1.724 s**; rebuilding into a fresh Cargo target takes **3.994 s**. Both include
formatting, **94 Cargo tests**, Clippy, build and conformance.
Conformance: **64 pass, 8 explicitly pending, 0 fail**, including **57 rustc
comparisons**. The Cargo tests include 31 adversarial K checks, seven K-text tests,
195 arithmetic edge comparisons, source-library integration and a regression
for bootstrap startup under an external address-space constraint. A formerly
implemented case becoming unsupported fails the suite.

All non-closure ownership/borrow/trait design cases pass, including view structs,
a two-source view rejection, custom cleanup, trait overlap and expanding recursion.
The eight pending programs are borrowed closure, escaping borrowed closure,
moved/shared/aliased task scopes, a Data-bound callback, a capability-owning
callback and a stored task. Structural Send/Sync checks pass independently of
task execution. No World-backed I/O or concurrency pass is claimed.

Physical production lines and exact `o200k_base` tokens (excluding inline test
modules, including comments and blank lines):

| Component | Lines | Tokens |
| --- | ---: | ---: |
| Compiler, all nine src modules | 12,125 | 89,127 |
| Current trusted base | 3,811 | 30,348 |
| Source standard library | 372 | 2,347 |
| Rust and Node host bridges | 265 | 2,565 |

The trusted-base count covers K definitions/checker, the emitter containing the
memory/collection implementation, and both host bridges. It overlaps the compiler
and bridge rows; there is no machine yet. Including inline tests, src totals
13,969 lines / 101,598 tokens and the trusted-base files total 5,049 / 37,261.

Release compiler capacity probes check through K; they do not emit or run their
large modules. RSS is sampled peak resident memory:

| Probe | Time | Peak RSS |
| --- | ---: | ---: |
| 1,000,000 nested parentheses | 0.144 s | 218 MiB |
| 1,000,000 locals | 1.178 s | 833 MiB |
| 1,000,000 functions | 2.322 s | 1,396 MiB |
| Node Vec<i64>, 2.125 GiB of elements | 5.075 s | 6,327 MiB |

The Vec probe checks length and retained values across growth above 4 GiB
addresses; its linear memory grew to 8 GiB plus one page. Geometric predecessors
were also exercised. Large-local attempts initially hit the runner's external
20-second timeout; sparse active predicates removed the quadratic checker scan.
A separate bootstrap probe checked 50,000 nested unary operators under an external
1 GiB address-space constraint in 0.276 s, growing its main stack to 766 MiB.
The 100-million-frame recursion probe is skipped until heap-frame execution exists.

`fib(35) = 9227465`: generated WebAssembly on Node takes **25.702 ms**;
rustc's checked-arithmetic optimized native oracle takes **28.781 ms**.
These are single execution measurements after startup (and Wasm instantiation),
not steady-state averages. Our machine and native backend do not exist.

## Missing or broken

Self-check succeeds for **0/9 compiler source modules**; syntax-only parsing
accepts 6/9. This is not whole-crate coverage. Module/name resolution, library API
coverage and unsupported source forms (including closures, let-else and range
patterns) block self-check. No W1, W1=W2, native compiler or N1=N2 exists.

Vec, Box, allocation, copy and structural drop still live in the emitter.
Moving their internals into trusted source memory modules remains milestone 2
work. String constructors validate UTF-8, but privacy/module enforcement must
land before the type can promise that invariant to all clients. Allocation
reuses blocks without splitting/coalescing yet. Recursive helpers still use the
engine stack. Full witness-bearing K, parameterized trait bounds, closures, World,
capability transcripts and structured task scopes remain missing. Query/signature
views and the two owned execution engines are later work.

## Bootstrap and bridges

`cargo build --release` builds the current compiler; `bash tools/check.sh` is the
routine gate. `lkjscript run file.rs --entry probe` prints the scalar result.
`lkjscript build file.rs -o file.wasm` emits a module.
`lkjscript check file.k` checks hand-written K; `lkjscript k file.rs` prints K.
`lkjscript trusted` lists the current trusted implementation boundaries.

The trusted Linux host shim grows the main stack on demand, preserves the host
hard stack extent and restores the original extent in children before exec. This
avoids the eager pthread reservation that prevented startup on GitHub runners.

Cargo is the bootstrap compiler/build bridge. Move bridge/main.rs host operations
behind World and compile that entry in the language; replace bridge/run.mjs with
our machine after the Wasm fixed point. The compiler sources are Rust-compatible
bootstrap sources, not yet wholly accepted lkjscript. The intended fixed-point
procedure is rustc -> W1; run W1 on Node and our machine -> W2; compare W1/W2
bytes and run conformance plus behavior-changing source mutations. Then use W2
to emit N1 and N1 to emit N2. No committed bootstrap image exists yet.

## Next action

Implement source module loading and qualified name resolution, with correct
prelude shadowing and privacy. Make the CLI consume the source library and check
the compiler as one crate, then remove unsupported source constructs in measured
groups. Keep explicit generic/witness K, source memory internals and the eight
pending closure/capability/task cases visible as unfinished first-vertical work.
