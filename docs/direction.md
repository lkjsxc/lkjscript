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
peak memory are recorded in observations/2026-11-13e-selfhost.json.

The retained obligations and their measurements are:

1. The real K compiler source checks and B0 emits kc1.
2. kc1 gives B0's verdict for direct K and the K of every accepted conformance
   source, and emits identical ELF bytes on acceptance.
3. kc1 given its own K emits kc2; kc2 and kc1 are byte-identical.
4. Record elapsed time and peak memory beside B0 for the same work.

There is one compiler implementation, selected twice. No per-file acceptance
ritual substitutes for this result. Afterward measure the front-end census before choosing between a port by the
two-way rule and a new front end written in the language. Either route must
produce B0-identical K text on conformance and its own source. Reach the
three-build identity and retain an owned seed compiler for the next build. Removing Cargo is an observed hand-over, not a renamed bootstrap.

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
