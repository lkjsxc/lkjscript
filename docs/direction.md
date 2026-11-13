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
K representation/text, safety checking, persistent facts, lowering, native
encoding and runtime sealing, plus shared diagnostics. It has no source parser,
elaborator or source loader. `kc::compile` consumes program K and runtime K. The
runtime is checked before its narrow raw interface is sealed; the host cannot
silently inject different source/runtime inputs into a pure compiler call.

The source front end produces K. The complete compiler's pure `compile` entry
produces native ELF. `lkjscript runtime-k` exports the prepared runtime as a
declared input. The separately cargo-built `kc` demonstrates this boundary but
does not establish any self-compilation claim.

The observations to reach are:

1. The real K compiler source checks and B0 emits kc1.
2. kc1 gives B0's verdict for direct K and the K of every accepted conformance
   source, and emits identical ELF bytes on acceptance.
3. kc1 given its own K emits kc2; kc2 and kc1 are byte-identical.
4. Record elapsed time and peak memory beside B0 for the same work.

There is one compiler implementation, selected twice. No per-file acceptance
ritual substitutes for this result. Afterward apply the same two-way rule to the
front end, reach the three-build identity, and retain an owned seed compiler for
the next build. Removing Cargo is an observed hand-over, not a renamed bootstrap.

## Order and execution

Unblock ordinary text, collections, configuration and diagnostics first. Keep
artifacts independent of build-host resources: execution-time Linux controls
supply budgets, and traps identify their source and resource. Then give World
arguments, input, error output, status and directory-relative file capabilities.
These are prerequisites for an independently running K compiler.

Profile that compiler and remove the largest runtime cost. A source ordered map
with logarithmic insertion can precede self-compilation because it is already a
compiler dependency. No prescribed speed ratio is a language requirement. Native
and K remain independent execution checks; obsolete encoders and hosts are gone.
New semantic tests are source/K data; Rust-specific bootstrap observations remain
only while they protect a real boundary and are migrated when their subject changes.

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
