# Roadmap

Build composable types, generics, traits, ownership, lifetimes, effects and
concurrency around the accepted meaning graph. Reliable agent development is a
secondary objective. [Direction](direction.md) owns durable architectural choices,
[status](status.md) owns availability and unfinished publication, and
[specifications](spec/) own language contracts. Completed delivery belongs to Git
and release notes.

These priorities guide useful experiments. A complete capability needs ordinary
public authoring, cross-package composition, independent admission and meaningful
failure evidence. Memory safety, bounded checking and avoiding unnecessary copies
remain baseline requirements.

## Compose ownership and generic contracts

Use [scoped reads of owned children](spec/owned-borrows.md) to inspect products and
choices through exact implementation witnesses, then consume the preserved owner.
The acceptance witness combines a symbolic reader with independent buffer/scalar
implementations, nested composites, exact package transports and detached artifacts.
Loan provenance, cleanup and inactive branches require independent checking.

Use [dynamic owned sequences](spec/owned-sequences.md) to compose runtime-sized
collections with scoped reads and exact element witnesses. Select associated
scoped views and explicit lifetime relationships when a second useful storage
representation exposes an abstraction that those native algorithms cannot express
cleanly. Consider associated type families, higher-ranked borrowing and bounded
value/region parameters where they make that abstraction sound and useful.

Keep implementation identity explicit and contracts independently checkable.
Inference must retain boundary evidence; bounded proof-search exhaustion remains
distinct from invalid meaning. Effect allowances, witnesses and deployment grants
stay separate. Typed failures must compose with cancellation and resource completion.

## Establish region custody and memory policies

Extend scoped access into explicit region custody. Define escape, cross-region
roots, freezing, transfer and reclamation before selecting placement mechanisms.
Compare unique regions, scoped aliases, local tracing for cycles and transitively
immutable shared segments. A mandatory global tracing heap and universal Arc-style
ownership are not architectural requirements.

Relate moving storage to handles, pinning, roots and native safepoints. Region views
should admit different storage implementations while retaining independently checked
access rights. Reclamation does not complete I/O or commit a transaction. Preserve a
collector-free path where the selected execution profile permits it.

Measure copying, retained memory, pauses, throughput and preparation on matched
workloads, including locality and large working sets. Allocation identity alone
does not establish whole-program zero-copy behavior or a speedup.

## Scale structured execution and the shared host

Extend the [structured parallel contract](spec/structured-parallel.md) and
[shared runtime](spec/shared-runtime.md) with explicit scheduling, pooling,
readiness, drain and unload contracts. Share compatible immutable code and metadata;
keep state, captures, grants, cancellation and accounting private to their owners.
Isolates are authority/lifetime domains rather than permanent OS threads.

Use retained cold/warm, overlap, cancellation and stop evidence to select root
scheduling, local queues, batching and locality work. Reused auxiliary workers do
not establish a total CPU bound, fairness or preemption. Blocking roots need a
suspension/completion contract before joining a bounded CPU scheduler.

Introduce typed bounded channels only with exact destination admission, capacity
reservation, irrevocable acceptance, owner-returning refusal and joined receiver
cleanup. Acceptance is distinct from processing completion. In-process custody
does not establish distributed exactly-once delivery.

## Reduce repeated preparation and semantic-development work

Share immutable admitted dependency interfaces across snapshots as a separate,
measured follow-up. Preserve exact package/revision bindings, substitutions,
visibility and effects; loaders must continue independent admission. Compare repeated
cross-package checks before and after, including retained storage and unsuccessful
cases. Existing copy-work admission bounds growth without removing duplication.

Use [reviewed candidate refresh](spec/concurrent-changes.md) as the foundation for
larger concurrent-agent experiments and finer dependency capture. Distinct edited
owners alone do not prove independence. Branch/tag, difference, merge, deletion and
compaction need explicit retained-root and interruption contracts; a short serialized
publication point remains acceptable.

## Advance native execution, storage and self-hosting

Derive dedicated-runtime programs and native executables from the same accepted
meaning. Hosted native code may embed services; freestanding profiles reject
unsupported services explicitly. Add targets with execution/distribution evidence.
Wasm and other-language interoperability remain optional later work.

Own persistence and typed data semantics. Separate durable transactions, revision
history and heap reclamation. Investigate storage layout, batching, recovery and
bounded caching on concrete workloads without requiring an external database engine.

Move libraries, tools, compiler and runtime toward complete self-hosting, including
possible removal of Rust. Replace a host boundary when its native successor owns
the relevant semantics and failure behavior. Native development must remain usable
without an external semantic generator.

Use a native compiler or tooling pass over provisional typed proposals as the next
substantial ownership workload. Its working collection or future region owns only
candidate data; the ordinary admission and publication boundary remains responsible
for accepted meaning. This workload should justify further storage and view
abstractions before selecting general region mechanisms.

## Evidence and development cost

Use independent expected results, rejected proposals with unchanged accepted state,
and cleanup observations. Retain unfavorable performance results. Separate source
acceptance, finalized distributed bytes and publication; reuse proof only while its
bindings remain valid. Maintained applications are regression witnesses rather than
an application-first backlog.

Keep exact logs and receipts at verification/release owners. Reduce repeated context
and overlapping verification while preserving required coverage. Claim API-cost
improvements only from actual usage/billing measurements. Authorized experimental
compatibility cuts do not authorize unrelated destruction or rewriting original
publication and failed evidence.
