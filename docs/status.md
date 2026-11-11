# Status

Generation two is a runnable Rust-bootstrap compiler with a memory64 WebAssembly
backend. Whole-compiler source acceptance, a compiler fixed point, an owned CLI,
and independence from Rust/Node remain unestablished. Generation one is frozen on
`generation-one` at `05a9326c` and in the read-only `.generation-one` worktree.
Running deployments are independent and unchanged.

## Works

Functions, checked fixed-width integers and f64, structs/enums, matches, loops,
generic instances, associated types and prerequisite bounds execute on Node.
Ownership includes moves, partial initialization, custom Drop, disjoint mutable
fields, one-source views, array/slice projections and reborrowing.
Ambient I/O and direct calls to actual destructor functions are rejected.

Generic free functions, impl methods and trait defaults are checked under rigid
parameters and declared contracts, independently of specialization or reachability.
The production elaborator and K checker are reused. Bounds, affine ownership,
associated identities and explicit method type arguments are enforced; inherent
methods and user traits do not become builtin Clone/Drop/Copy by spelling.
Concrete associated-result selection executes successfully.

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

The exact production unsigned/signed LEB128 encoders in `src/encoding.rs` now
compile and execute through the ordinary source-bundle pipeline. Tests compare
every encoded byte for 34 unsigned and 63 signed boundary inputs, include fixed
known vectors, and require a behavior-changing source mutation to alter the
artifact and result. An invalid mutation fails with the original source filename.
This is one executing compiler component. It still uses emitter-provided Vec
memory primitives and Node; it is not a compiler-core fixed point.

## Current verification

The final integrated 2026-11-11 gate passes **178 Cargo tests** and reports
**71 conformance passes, eight pending, zero failures; 64 rustc comparisons**.
Formatting, Clippy, build and conformance are included. Elapsed: **13.961 s**.

The suite includes 19 provenance tests, 28 generic-definition tests, 20 module
tests, source-bundle and executing-component mutations, parser/embedding capacity
cases, two 30,000-local shapes, deep destruction/order and allocation reuse.
Direct K fixtures reach the checker without frontend filtering. Rejections use
the intended semantic errors; source-level replacement/clone escapes and generic
definition/type-argument failures are also compared with rustc.

The eight closure/callback/task conformance cases remain explicitly pending.
Structural Send/Sync checks do not establish a task runtime. No World-backed
I/O or concurrency execution pass is claimed.

## Capacity observations

These 2026-11-11 debug CLI observations were taken after the backend repair,
before the following generic/type-frame changes. They include parse,
resolve/elaborate, check, emit, Node load/compile, execution and cleanup.
Each is one end-to-end observation; peak RSS was not measured.

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

Nested Box measurements after structural interning at `368cbb37` cover parse,
resolve/elaborate, check and K serialization, not emission/execution. Structural keys and compact names remove the earlier
approximately quadratic expanded-name output for this input shape:

| Depth | Source bytes | K-text bytes |
| ---: | ---: | ---: |
| 128 | 678 | 16,933 |
| 256 | 1,318 | 20,133 |
| 512 | 2,598 | 26,533 |
| 1,024 | 5,158 | 39,433 |

The subsequent parser/type-elaborator repairs replace the independently observed
depth-256 stack failures with explicit work frames. On an explicit 2 MiB thread:

- Parsing and AST destruction pass at depth 16,384; routine regressions cover
  eight simple/mixed shapes at depths 512 and 8,192, including malformed input.
- The ordinary compiler-core entry compiles builtin Box and transparent-alias
  graphs at depths 512 and 2,048 through emission; Node loads them and executes
  the scalar probe. An uncalled `consume` function carries the deep parameter type.
- User-defined Box types and reverse-ordered nominal fields pass at depth 512.
- A recursive alias/nominal/Box program builds, executes and cleans 1,000 nodes.
- Alias-only cycles reject semantically; no depth/fuel threshold was introduced.

Other expression/pattern parsing, elaboration and generic inference paths remain
outside this stack repair. Complete stack independence is not established.

Reference containment in K is now one reverse-edge graph propagation rather than
a fresh transitive scan for each container. Directly constructed nested-Box K
type tables gave the following single debug-library observations (K check only;
no source parsing, elaboration, emission or execution):

| Types in Box chain | Before | After |
| ---: | ---: | ---: |
| 1,024 | 5.583 ms | 0.274 ms |
| 4,096 | 85.892 ms | 0.942 ms |
| 8,192 | 345.529 ms | 1.848 ms |

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

Executable K is still monomorphic. Generic validation records declared method
evidence internally, but complete witness-bearing generic K and serialized
contract closure are unfinished. Reference content hidden inside arbitrary type
parameters still requires concrete K provenance validation. Deeply dependent
associated-witness chains are not comprehensively normalized; unresolved cases
reject. External view writes need explicit mutation contracts before their
symbolic source relationships can change.

The normal CLI starts from `src/lib.rs` and follows its real modules. The current
whole-crate check stops at `src/wasm.rs:1967` with `E_PARSE` on a range pattern
(`..=`). No complete compiler-core closure is accepted, and no W1/W2,
N1/N2 or owned bootstrap seed exists. Strings/views, formatting, map/set APIs,
derives, closures, patterns and ownership layouts remain real dependency work.

The next concrete ownership vertical is immutable text/literals and the minimum
source-owned copying/appending needed to consolidate and execute the duplicated
`qualified(module, name) -> String` helper in resolve, generics and elaborate.
Current probes reject `&str` as an unknown type and string literals as unsupported;
the existing source String/UTF-8/Vec implementation supplies the owned half.
Implement static literal provenance, immutable UTF-8 text and borrowed byte
projection; implement copying and `push_str` as source loops. Replace this helper's
formatting with those operations, share one production implementation under both
compilers, compare full output bytes, and mutate its separator in supplied source.
Reject local-owned-string view escapes and invalid direct-K static UTF-8. Avoid
a second owned-string representation or host-generated compiler results.

Other observations include missing borrowed collection iteration, `while let`,
and repeated Vec initialization. Add these when they close the next actual
component dependency. Keep the successful encoding component and extend its
production closure rather than counting parsed files.

Vec/Box/allocation internals remain in the emitter. The one-source view model,
views-in-owned-containers restriction, closures, explicit World capabilities,
tasks, CLI ownership and an owned/native execution route remain unfinished.
Choose generally useful semantics over repeated compiler-only source contortions.

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
