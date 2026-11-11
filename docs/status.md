# Status

Generation two is a runnable Rust-bootstrap compiler with a memory64 WebAssembly
backend. Whole-compiler source acceptance, a compiler fixed point, an owned CLI,
and independence from Rust/Node remain unestablished. Generation one is frozen on
`generation-one` at `05a9326c` and in the read-only `.generation-one` worktree.
Running deployments are independent and unchanged.

## Works

Functions, checked fixed-width integers and f64, structs/enums, matches, loops,
concrete generic instances, associated types and prerequisite bounds execute on
Node. Ownership includes moves, partial initialization, custom Drop, disjoint
mutable fields, one-source views, array/slice projections and reborrowing.
Ambient I/O and direct source calls to Drop are rejected.

Every initialized reference leaf has provenance or explicit inactive-variant
absence evidence. Moves, construction, cloning, replacement, field assignment,
call results and return preserve/check it. Replacement transports both the old
and installed values. External writes cannot silently change symbolic reference
leaf origins; explicit mutation contracts remain required for general updates.
The repair is committed and CI-verified at `35a5d862`.

Files, modules and declarations have explicit identities. The loader owns file
discovery; resolution handles lexical/qualified names, aliases, imports, visibility
and prelude shadowing. The library uses ordinary module scopes. Private String
bytes cannot be constructed or accessed by outside clients. Type/instance keys
are structural, with compact diagnostic names.

`compiler::compile(&SourceBundle)` takes declared source/library bytes and returns
Wasm bytes or diagnostics without host I/O. The CLI uses the same production
checking pipeline. Bootstrap byte packing is in `bridge/library.rs`. Supplied
library bytes affect emitted behavior; missing inputs are errors. This is a
working compiler-core boundary, not a self-hosting claim.

The source library implements Option/Result, bytes/UTF-8, String, vector ordering
and a sorted-vector BTreeMap (logarithmic lookup, linear insertion/removal). Its
integration tests load actual source bundles under lkjscript and use the same
components under rustc.

Lowering reuses physical locals and spills excess live values into memory64 frames.
Address-taken values have stable frame slots. Structural destruction uses a growing
explicit stack, including custom Drop, partial moves and recursive Box/Vec values.
Copy helpers and explicit recursive source calls still use the engine stack.

## Current verification

The integrated 2026-11-11 gate passes **141 Cargo tests** and reports
**64 conformance passes, eight pending, zero failures; 57 rustc comparisons**.
Formatting, Clippy, build and conformance are included. Elapsed: **12.663 s**.

Coverage includes direct-K provenance attacks/valid transfers; multi-file sibling
names, aliases, privacy, imports/re-exports, local prelude/intrinsic shadowing,
trait use and original-file diagnostics; source-bundle mutations; two 30,000-local
shapes; loop liveness; 20,000-node structures; destructor order; and allocation
reuse. The eight pending closure/callback/task cases remain pending. Structural
Send/Sync checks do not establish a task runtime. No World-backed I/O or
concurrency execution pass is claimed.

## Capacity observations

These 2026-11-11 debug CLI runs include parse, resolve/elaborate, check, emit, Node
load/compile, execution and cleanup. Each is one end-to-end observation; peak RSS
was not measured.

| Input | Result | Elapsed |
| --- | ---: | ---: |
| 30,000 successive local declarations | 29,999 | 0.253 s |
| 20,000-node Box/enum list | 20,000 | 0.023 s |
| 20,000-node custom-destructor chain | 20,000 | 0.024 s |
| 20,000-node Vec/enum chain | 20,000 | 0.027 s |

The repeated custom-destructor probe used 5,963,776 linear-memory bytes after its
first call and 9,109,504 after its second, then stayed unchanged through calls
three to five. The allocator still does not split/coalesce blocks; stable reuse
after initial placement does not establish absence of fragmentation.

Nested Box measurements cover parse, resolve/elaborate, check and K serialization,
not emission/execution. Structural keys and compact names remove the earlier
approximately quadratic expanded-name output for this input shape:

| Depth | Source bytes | K-text bytes |
| ---: | ---: | ---: |
| 128 | 678 | 16,933 |
| 256 | 1,318 | 20,133 |
| 512 | 2,598 | 26,533 |
| 1,024 | 5,158 | 39,433 |

On an isolated 2 MiB thread, parsing depth-256 Box types still overflows. With
syntax independently prepared on a larger test thread, elaboration also overflows;
resolution and K checking complete independently. The CLI stack shim does not
repair embedding. Explicit parser/elaborator frames remain required.

### Earlier evidence and its original scope

At `aff27e54`, Rust 1.97.0/Node 24.18.1 measurements reported incremental/fresh-target
gates of 1.724/3.994 s for the then-94-test suite. Release million-parentheses,
million-local and million-function probes measured checking through K, not emission
or execution: 0.144 s/218 MiB, 1.178 s/833 MiB and 2.322 s/1,396 MiB sampled peak
RSS respectively. These were not repeated for the current changes.

The earlier 2.125 GiB Vec execution reported 5.075 s/6,327 MiB sampled peak RSS,
retaining values across addresses above 4 GiB and growing linear memory to 8 GiB
plus one page. The earlier 50,000-unary-operator check under an external 1 GiB
address-space limit reported 0.276 s and a 766 MiB main stack. Neither was repeated.
Earlier `fib(35)=9227465` single observations after startup were 25.702 ms for Wasm
and 28.781 ms for rustc checked-arithmetic native code. These are not averages,
complete compiler benchmarks or an owned-engine result.

## Missing and next work

Generic/witness selection and finite-instantiation checking still precede
monomorphic K. Uninstantiated generic bodies are not yet fully checked under their
declared parameters. The immediate semantic task is reused elaboration/K checking
with rigid parameters and declared trait/associated-type contracts, without a
second expression checker or an arbitrary instantiation budget.

The normal CLI loads the compiler crate and reaches actual syntax barriers. The
latest attempt reports an unsupported range pattern in `src/wasm.rs`. No complete
compiler-core source closure is accepted. Strings/views, formatting, map/set APIs,
derives, closures, patterns and ownership layouts remain real dependency work.
No W1/W2, N1/N2 or committed owned bootstrap image exists.

Execute an actual production compiler component through the ordinary source-bundle
path, retaining the same implementation in the bootstrap compiler, then extend
that verified closure. In parallel, replace demonstrated nested-type parser and
elaborator recursion with explicit frames without changing source validity.

Vec/Box/allocation internals remain in the emitter. The one-source view model,
views-in-owned-containers restriction, write contracts, closures, explicit World
capabilities, tasks, CLI ownership and an owned/native execution route remain
unfinished. Add useful semantics when compiler needs repeatedly exceed the subset;
do not flatten every abstraction into a private compiler dialect.

## Bootstrap

`cargo build --release` builds C0; `bash tools/check.sh` is the routine gate.
`lkjscript run file.rs --entry probe` executes on Node. `build -o file.wasm` emits,
`k` serializes checked concrete meaning, and `check file.k` reaches the K checker.

The first compiler-core fixed point must run C0's language pipeline on the exact
production source/library closure to emit W1, then execute W1 with the same inputs
to emit equal W2. Conformance and behavior-changing compiler-source mutations are
also required. Rust directly compiling to Wasm is not this route. Node and a small
host adapter can remain at that scoped milestone. Native closure and independence
from Rust/Node require separate evidence, including an isolated rebuild from a
named owned seed. Shared-host tools and deployments stay intact.
