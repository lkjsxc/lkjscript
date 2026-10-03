# Roadmap

Improve the language's compositional type, generic, trait, ownership, lifetime,
effect and concurrency design. Reliable agent development is the secondary objective;
personal application completion, immediate benchmark wins and new application
starters do not determine the order. The accepted meaning graph remains the sole
editable program authority. [Direction](direction.md) contains the durable choices,
[status](status.md) owns availability, and [specifications](spec/) define implemented
contracts. Historical prompts are not additional prerequisites.

These priorities order the next useful experiments, not a waterfall or a feature
promise. Mechanisms may change when a better complete design has evidence. Memory
safety, bounded checking and avoidance of unnecessary copying are baseline requirements.

## 1. Compose generic contracts with structured ownership

Retain the completed 0.1.68 distribution acceptance and publication under the
[existing procedure](release.md#published-structured-parallel-v0168).
[Status](status.md) records the exact public source and completed producer/promotion. Retain
the completed source, cross-package, artifact, overlap, quota and cleanup evidence;
renew proof when its bindings change. Preserve task kind independently of an empty
effect row throughout authoring, extraction and execution.

Use the [current pair contract](spec/structured-parallel.md) as a small coherent
foundation. Owned results now return through a separately sealed pair; development
0.1.70 adds exact closed generic child applications and nominal implementation
selection without mandatory monomorphic wrappers. Preserve the independent canonical
and memory oracles, complete type closure, cross-package source-free composition,
allocation identity, shared quotas and joined failure cleanup. Source and final-byte
acceptance remain separate facts in [status](status.md).

Development 0.1.71 adds [explicit transferable contracts](spec/transferable-types.md)
for reusable generic code that itself forms child groups. Transferability is
independent of ownership and capture safety; symbolic proofs become exact at
execution. Acceptance includes nominal/phantom closure, mixed generic result pairs,
exact implementation forwarding and independent bounded proofs. [Status](status.md)
owns the remaining acceptance boundary. Channels, task handles and effectful
children remain separate extensions.

After that boundary is accepted, prioritize the shared worker/runtime work below,
then concurrent semantic transactions with revalidated dependency footprints.
Keep region-local ownership possible: current owned carriers being transferable
does not make every future owner transferable.

## 2. Scale structured execution and the shared host

Evaluate reusable bounded workers, local queues, batching and locality-aware scheduling
with substantial transform/reduction work beside an independent I/O instance. Keep
nested execution from waiting for a worker or resident slot held by its ancestor.
Measure fairness under saturation, cancellation latency, preparation cost and repeated
start/stop behavior before making scaling claims.

Extend the [in-process shared runtime](spec/shared-runtime.md) with explicit pooling,
admission, readiness, stop/drain/unload and version coexistence. Share compatible
immutable code and metadata; retain private state, captures, grants, cancellation
and accounting. Isolates are authority/lifetime domains, not permanent OS threads.
Dynamic supervision and component-level pooling require their own evidence.

Develop typed bounded channels when the task/ownership contract can express exact
destination admission, capacity reservation, irrevocable acceptance, owner-returning
refusal and joined receiver cleanup. The existing runtime mailbox is a mechanism,
not a public channel certificate. Acceptance is not processing completion, and
in-process transfer establishes no distributed exactly-once guarantee.

## 3. Broaden ownership, generics, traits and effects together

Extend the [Owned library contracts](spec/owned-generics.md), [products](spec/owned-products.md)
and [choices](spec/owned-choices.md) through one ordinary cross-package composition
at a time. Prioritize useful owned containers, borrowing/lifetime relationships and
typed trait methods over disconnected syntax. Explore associated type families,
higher-ranked borrowing, effect polymorphism and bounded value/region parameters
where they support a sound abstraction.

Retain exact implementation identity and explicit coherence, ambiguity and termination
rules. Local inference must leave independently checkable boundary contracts.
Proof-search exhaustion is distinct from invalid meaning. Effect allowances,
implementation witnesses and deployment grants remain separate. Typed failures and
resource completion must compose with cancellation; retained affine continuations
cannot be duplicated by an effect handler without a valid ownership contract.

## 4. Establish ownership and region memory policies

Compare unique regions, scoped aliases, local tracing for cyclic data and transitively
immutable shared segments. Universal Arc-style ownership and a mandatory global
tracing heap are not the selected end state. Define region escape, cross-region roots,
freezing, transfer and reclamation before selecting placement mechanisms.

Relate moving collectors to borrows, handles/pinning, roots and native safepoints.
GC reclaims storage; it does not commit transactions or complete I/O. Preserve a
collector-free/no-allocation path where future freestanding effects permit it.
Compare copying, retained memory, pauses, throughput and preparation on matched
workloads, including cache/NUMA and large working-set behavior.

## 5. Support concurrent semantic development

Prepare private candidates from immutable revisions concurrently, then revalidate
semantic dependency footprints, including negative lookups, at publication. Test disjoint edits and hidden
conflicts through types, witnesses, effects and references; different edited owners
alone do not prove independence. A short serialized publication point is acceptable.
Start with two candidates before a larger agent experiment on authorized resources.

Select conventional revision, branch/tag, difference, merge and retained-root semantics.
Preserve useful stable identities without retaining every recomputable analysis.
Deletion and compaction need reachability and interruption proofs. Concurrent shell
commands are not evidence of safe concurrent program modification.

## 6. Advance native execution, storage and self-hosting

Derive dedicated-runtime programs and native executables from the same accepted meaning.
Hosted native code may embed required runtime services; freestanding profiles reject
unsupported services explicitly. Keep Linux x86-64 first and add other targets only
with execution/distribution evidence. Wasm and other-language interoperability remain
optional, lower-priority work.

Own the persistence engine and typed data semantics. Separate durable transactions,
revision history and heap collection. Investigate SSD layout, batching, recovery and
bounded caching on concrete workloads without requiring an external database engine.

Move libraries, tools, compiler and runtime toward eventual complete self-hosting,
including possible removal of Rust. Replace each host boundary when its native
successor owns the relevant semantics and failure behavior; a rewrite percentage
or premature deadline is not progress. Native authoring must remain usable without
an external semantic generator throughout this transition.

## Evidence and development cost

Use normal public operations and independently expected successes, failures and cleanup.
Maintained applications remain regression witnesses, not an application-first backlog.
Keep source acceptance, final distributable bytes and publication distinct. Reuse
valid evidence and run the smallest dependency-complete checks for changed inputs;
do not add overlapping receipts or repeatedly reconstruct completed history.

Keep current direction, status and priorities at their existing owners, with exact
logs at verification/release owners. Historical campaigns remain archives, not a
required prompt-reading sequence. Reduce repeated context and verification work,
while preserving evidence needed for actual claims. Measure API usage/cost only
from observed usage; document size alone establishes neither tokens nor money saved.

Turn retained audit hypotheses into small executed counterexamples before further
changes. Retain imported Owned-generic coverage in the independent memory oracle;
follow up immutable sharing of dependency interfaces and reuse of admitted expression types
within one extraction. The new copy-work admission bounds growth but does not remove
duplicate metadata. Preserve exact source, substitution and effect scopes when sharing.

For runtime comparisons, separate cold/warm admission, latency tails, throughput,
shared/private storage and RSS/PSS. Include a matched separate-process baseline and
retain slower cases. Compatibility cuts for authorized experimental data remain
available; they do not authorize unrelated destruction or rewriting failed evidence.
