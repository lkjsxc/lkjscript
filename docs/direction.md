# Direction

lkjscript is a Rust-compatible subset chosen for reliable work by agents. It is
not a promise to approximate all of Rust. Accepted surface behavior is compared
with Rust under the declared capability/trap host. A deliberate rejection that
names the supported rewrite is useful; an unpredictable checker is not.
Language design comes first, reliable agent development second. The project will
own its compiler, runtime, library and tools without Rust or Node. No backward
compatibility with previous versions is required.

## Consumers and the two-way rule

The production K compiler is the first consumer, repository tools second, and
the source front end third. Fresh independent programs test whether these users
have hidden an ordinary gap. A spent corpus is retained as regression evidence,
not a width score. Count actual remaining compiler diagnostics, including code
added in the same change; recovered census observations are partial evidence.

When a consumer needs a missing form, choose the cheaper sound change. Add
ordinary surface forms that use existing K or source library code. Rewrite the
compiler when supporting a rare form would add trusted primitives, checker rules
or reference contracts. Preserve source behavior and verification either way.
Prefer plainer trusted code and explicit owned indices over extending reference
contracts to preserve a convenient host-language API.

## The K compiler first

`src/kc_lib.rs` selects the same production files used by the complete compiler:
K representation/text, safety checking, packed owned facts, lowering, native
encoding and runtime sealing, plus shared diagnostics. It has no source parser,
elaborator or source loader. `kc::compile` consumes program K and runtime K. The
runtime is checked before its narrow raw interface is sealed; the host cannot
silently inject different source/runtime inputs into a pure compiler call.

The source front end produces K. The complete compiler's pure `compile` entry
produces native ELF. `lkjscript runtime-k` exports the prepared runtime as a
declared input. The separately cargo-built `kc` demonstrates this boundary but
does not establish any self-compilation claim.

The 2026-11-13e checkpoint reaches the four K-compiler observations below.
The native World driver in src/kc_main.rs shares the same nine production
modules; kc1, kc2 and kc3 have identical bytes, and its complete recorded corpus
parity includes 562 K verdicts and 498 identical ELF artifacts. Timings and
peak memory are recorded in observations/2026-11-13e-selfhost.json. The first
profile-driven allocator change preserves the fixed point and corpus parity;
its current measurements are in observations/2026-11-13e-selfhost-class-index.json.

The retained obligations and their measurements are:

1. The real K compiler source checks and B0 emits kc1.
2. kc1 gives B0's verdict for direct K and the K of every accepted conformance
   source, and emits identical ELF bytes on acceptance.
3. kc1 given its own K emits kc2; kc2 and kc1 are byte-identical.
4. Record elapsed time and peak memory beside B0 for the same work.

There is one compiler implementation, selected twice. No per-file acceptance
ritual substitutes for this result. The native seed and its exact K/runtime
inputs are retained independently of the build directory. Removing Cargo is an
observed hand-over, not a renamed bootstrap.

## Source front end: native fixed point

The measured route was a port of the existing SourceBundle-to-K producer. Its
AST already used owned arenas and index relationships. Preserving the algorithms
also retained declaration/type/function/local numbering, instantiation and
cleanup order, helper names and source locations. A replacement with identical K
would have had to rediscover those choices. The initial census and decision are
retained in observations/2026-11-14-frontend-census.json.

The first complete ownership pass reached 1,424 source bodies and reported
62 borrow errors plus one move error, with no declaration or type blockers.
The port repaired 78 observed sites, including later first faults, without
changing ownership rules, K, the encoder, runtime or source library. Rare host
forms became concrete owned collections and index traversals. Decimal floating
conversion is ordinary source code with an exact binary64 rounding oracle.
See observations/2026-11-14-frontend-port.json.

The production pure root is src/frontend_lib.rs. The World driver in
src/frontend_main.rs uses declared source/library inputs; frontend.rs owns the
single shared pipeline. At the first fixed point, its 1,433 source-body attempts
produce 7,334 K functions. Both native generations emit exactly B0's 18,489,341
own-source K bytes, and the already self-built K compiler produces identical
50,703,986-byte ELF files: frontend-b0 = frontend1 = frontend2. Here frontend-b0
is the front-end artifact emitted by the Rust bootstrap. The input protocol and
commands are in frontend.md; hashes and measurements are in
observations/2026-11-14-frontend-selfhost.json. Native corpus parity covers all
592 source conformance files and the K compiler source: 268 identical K outputs,
321 matching rejections, four unchanged pending and zero failures. The complete
record is observations/2026-11-14-frontend-native-corpus.json.

The unprofiled native source-to-K observation is 246.20 seconds and 739 MiB peak
RSS, beside B0's 2.24 seconds and 206 MiB. Emitting the same frontend K takes
110.82 seconds and 733 MiB through native kc, beside B0's 0.91 seconds and
335 MiB. These are measured costs, not acceptance limits. The full source
compiler core can therefore operate without Rust; Cargo tests and Rust reference
observations remain development tools.

## Routine native work and complete profiling

The installed `lkj` command now reads declared source/library directories through
the production loader, packs reproducible bundles, prepares runtime K, checks,
builds, runs and evaluates programs. Native `frontend`, `kc` and `result`
programs do the compiler/result work. Bash connects them and publishes completed
outputs. The explicit installer and rebuild command use no Rust or Node; the
new generation reproduces all three native binaries and runtime K exactly.
See [native.md](native.md) and observations/2026-11-14-native-workflow.json.

A complete 99 Hz profile of the unchanged compiler on its frozen own-source
bundle found two source lookup scans taking 13.29% of sampled CPU time. Generic
ranges are now indexed by file/name, with containment checks preserved; derived
bounds are indexed by qualified declaration name, preserving union/any-match
semantics. These are immutable source indices and add no trusted rule.

On the same frozen input, the combined candidate takes 218.35 seconds versus
248.42 seconds, a 12.10% wall-time reduction, with 3.24% higher peak RSS and
identical K. The selected lookup union falls from 32.71 to 1.41 sampled CPU
seconds. This is one complete profiled pair on a shared host, not a repeated
benchmark. The candidate also includes the direct-source driver. Full controls,
mapping proof and the failed live-proc metadata capture are retained in
observations/2026-11-14-native-profile.json.

The current complete profile still attributes 24.64 sampled CPU seconds
(11.43%) to `has_view`/`is_copy`; repeated full-type-table boolean vectors overlap
that cost. This is the next concrete bookkeeping cost to investigate. Broad
allocator/drop/prologue totals overlap many operations and are not independent
savings estimates. Keep substantial optimization in rechecked K-to-K passes,
and keep unrelated surface width behind measured consumer needs. Retain host
test orchestration while it supplies independent evidence; a direct K generator
remains the next useful self-built development tool.

## Order and execution

1. Reach the complete K compiler ownership verdict and record its rejections
   before fixing them. Declaration blockers must be named and located; partial
   coverage counts do not measure distance.
2. Make the small native top-operand improvement, encode array repeats compactly,
   and remove unreachable functions before native encoding. Use the existing gate.
3. Correct allocator payload classes, tip growth and the heap/stack split. Give
   the stack its own visible control. An ordinary program whose data comfortably
   fits memory must work under run defaults; gate generators keep explicit budgets.
4. Shard conformance batches and adopt direct K mutation with reproducible seeds,
   separate outcome classes and saved records. Align resource-trap classification
   between native and K for unbounded recursion.
5. Build kc1/kc2, verify the fixed point, and record time and peak memory beside B0.
6. Profile kc and remove its largest measured cost first. No speed ratio is an
   acceptance requirement.
7. Measure and choose the front-end route; do not put unrelated front-end width
   ahead of these consumers.

Steps 1–5 are complete at the recorded checkpoint. The actual self-build
profile attributed 16% of self CPU to allocator class computation. Retaining
the class in its existing header removes one helper and avoids recomputation
on release; the same program K self-builds in 36.79 / 35.95 seconds, versus
40.39 / 40.15 seconds before. Those two necessary generations show a 9.7% lower
mean with unchanged peak memory to measurement precision. They are observations,
not a speed threshold. Re-profile before selecting the next substantial change.

The encoder belongs to the trusted base. Local operand caching can stay small
there; inlining, propagation and dead-copy removal belong in ordinary K-to-K
passes. Re-check their output and discard a rejected change. Compare native on
optimized K with the independent machine on the original K to catch wrong values.
A rejected optimizer must never become a route around the K checker.

The source Rust oracle protects today's surface. The checker, independent K
machine and direct K generators survive a different surface or tool-written K.
A pure seeded K generator is a natural second self-built daily tool after kc.
New semantic tests are source/K data; Rust-specific bootstrap observations remain
only while they protect a real boundary and migrate when that subject changes.

## Durable semantic choices

K is the semantic language; the Rust-compatible surface is today's bootstrap
and oracle dialect. A later agent-oriented surface or tool-written K can be a
new producer without replacing the trusted core. Nothing below K requires Rust
source concepts.

Views live in locals, parameters, results and view aggregates; heap containers
own their contents. An index or owned value expresses a stored relationship.
Extensions to that discipline require a consumer and an explicit trust cost.

Capabilities express effects. A function without authority in its arguments or
captures has no ambient input/output. World grants authority explicitly; file
operations must remain beneath their directory root. Value results are then
reusable from declared inputs, with resource traps treated separately.

Generators are pure seed-to-source programs and are candidates for the next
self-built tools. Their Node orchestration is a temporary development dependency.
Concurrency and speculative reference contracts remain parked until a real
consumer needs them.
