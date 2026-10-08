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

Use [parameterized owned contracts](spec/owned-contract-parameters.md) to abstract
over runtime-sized collections with structural owner-returning methods. The
[worklist witness](guides/native-owned-worklists.md) composes flat and chunked
storage, independent elements, cross-representation transfer and a native
provisional graph validator. It builds on [owned sequences](spec/owned-sequences.md)
without requiring a new nominal ownership system.

Use [source-tied borrowed results](spec/owned-read-results.md) to return read-only
views through pure generic functions and owned-contract methods. The
[selection witness](../examples/owned-read-results/README.md) exports IndexRead and
first-max selection before concrete readers exist, then observes selected views,
drains the original collection and reuses its owner. Exact source provenance and
ordered cleanup cross each package and call boundary; availability remains
[status-owned](status.md).

Associated view families become useful when storage representations need different
view shapes or cursors. Select those families, higher-ranked relationships and
bounded value/region parameters against such a concrete workload. The existing
explicit Item argument serves readers that return their unchanged element type.

Use [explicit generic implementation schemes](spec/generic-owned-implementations.md)
to reuse storage and reader methods across independently authored owned items.
The [native witness](../examples/generic-owned-implementations/README.md) exports
flat/chunked schemes before cells, buffers and a nested product exist, preserving
exact applications through source-tied reads, transfer and joined tasks. Require
complete admission of mapped targets and finite cross-package callable flow.

The selected composition milestone adds ordered implementation prerequisites and
explicit mapped-function witnesses. The [adapter witness](../examples/composable-owned-implementations/README.md)
maps the existing generic selector directly, nests independently selected readers,
and forwards a consuming task's exact prerequisites. Every nested selection retains
its identity, ownership modes, closed effects and borrowed-result provenance.
Mapped operands are direct same-scheme prerequisites or finite concrete application
trees containing no lexical witness parameter; eligible symbolic type arguments
remain allowed. A conservative graph of potential method targets rejects lexical
witness wrapping inside recursive components, including recursion made possible
by unused same-contract alternatives. Unsupported construction and resource
exhaustion remain distinct. Method-local schemes and inferred selection require
separate designs.

Keep implementation identity explicit and contracts independently checkable.
Inference must retain boundary evidence; bounded proof-search exhaustion remains
distinct from invalid meaning. Effect allowances, witnesses and deployment grants
stay separate. Typed failures must compose with cancellation and resource completion.

## Establish region custody and memory policies

Use [borrowed tasks and joined parallel reads](spec/structured-parallel.md) to
separate storage custody from scoped access. The
[recursive reduction witness](../examples/scoped-parallel-reads/README.md) lends
flat and chunked storage to nested readers, then drains and reuses the original
owner. Shareable read admission and Transferable movement are independent
obligations. Exact availability and acceptance remain [status-owned](status.md).

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
Use [demand-driven idle worker custody](decisions/idle-worker-custody.md) to let
independent open executors reuse receipt-joined workers without another physical
start. Keep active jobs, grants, programs, cancellation and quotas private. Each
worker retains one join owner through handoff and shutdown races; eligibility does
not precede result cleanup. This is not fair CPU entitlement or root scheduling.
Use [owner-local dispatch](decisions/owner-local-worker-dispatch.md) to separate
physical custody from receipt-joined availability. Stable local dispatch and result
completion use only their current owner state; cold misses, new starts, handoff and
closure retain shared coordination. A directory entry is not availability, and
failed cleanup never grants transfer. Measure contention, locality and retained
owner metadata before selecting more complex queueing or retirement policies.
Exact acceptance remains status-owned.

Introduce typed bounded channels only with exact destination admission, capacity
reservation, irrevocable acceptance, owner-returning refusal and joined receiver
cleanup. Acceptance is distinct from processing completion. In-process custody
does not establish distributed exactly-once delivery.

## Reduce repeated preparation and semantic-development work

Use [exact callable-cycle proof selection](decisions/context-scoped-callable-proof.md)
to admit compact acyclic prerequisite DAGs without unfolding every typed path.
All source operands remain independently admitted. Use
[demanded recursive provenance](decisions/demanded-callable-provenance.md) to follow
only exact paths that can feed declaration type slots, preserving all expanding
cycles without merging same-shaped witnesses. Use
[operation-local incoming call inputs](decisions/operation-local-callable-inputs.md)
to read each demanded call once while preserving its distinct witness paths. Lexical
declaration and type-object reads, plus large genuinely demanded path sets, remain
separate proof costs. Use [operation-local lexical projections](decisions/operation-local-lexical-projections.md)
to retain ordered implementation-parameter IDs without repeatedly cloning their
whole declaration. Exact lexical owners, witness paths and type slots stay separate;
modeled lookup work and retained metadata are additional measured costs. Retain exact
substitutions, scopes, expansion adversaries and bounded refusal.

Use [whole immutable prepared witness nodes](decisions/shared-prepared-witness-nodes.md)
to share complete exact applications across catalogue entries, prerequisite edges
and function bindings. Concrete DAG storage was already present; the full
[recursive consumer](../examples/concrete-callable-proof/README.md) is now maintained
as public coverage rather than described as an unsupported future capability.
Keep selected identity, complete admission and owner-private execution state.
Measure node storage, preparation and invocation separately without raising limits.

Preserve the existing shared prepared instruction bodies for the same exact
function, type arguments and closed effect/requirement context. Complete witness
identities and resolved call-site bindings remain on separately admitted
applications, including tail calls, function values and parallel children. This
mechanism is implemented, not a new backlog item. Continue measuring retained
instructions, application metadata and preparation on matched selections; sharing
storage alone does not establish faster preparation or execution.

Use [admission-local shared interface record maps](decisions/shared-dependency-interfaces.md)
across importing snapshots. Each exact record map is materialized once per source
admission; every load retains independent interface, body and composed-callable
validation. The [diamond reader workload](guides/native-shared-dependencies.md)
keeps distinct private implementations and ordered borrowing through shared imports.
Flattened dependency type maps, cross-operation reuse and repeated validation remain
separate costs. Measure them before extending sharing or changing admission limits.

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

Use the native worklist-based dependency-graph validator as a tooling witness over
provisional proposals. It validates all nodes and references before reachability;
its owned working collection carries candidate identities. The native
[dependency-component pass](../examples/dependency-components/README.md) additionally
partitions all admitted nodes, including unreachable nodes, and distinguishes
singleton self-loops. Explicit enter/leave frames keep graph depth out of the
call stack; complete public results retain authored identity order across flat
and chunked worklists. The first pass needs their LIFO behavior in addition to
the checked ownership/type signature. Integrating these passes into the
production compiler requires an explicit host boundary that retains ordinary
admission and publication authority. Larger typed passes should justify storage,
view and region abstractions through working-set and reclamation measurements.
Use the [native dependency-first planner](../examples/dependency-plan/README.md)
to condense complete proposals into explicit cyclic units, distinct component
dependencies and earliest dependency-first stages. Preserve authored identities,
complete unreachable validation and the separate scope of derived consistency
checks. A stage is graph precedence, not parallel-execution or publication
authority. Integrate this witness into a maintained compiler consumer only through
an independently admitted boundary; exact availability remains status-owned.
The [owned metadata workload](../examples/owned-metadata-costs/README.md) separates
ordinary list/map projection from packing, unpacking and borrowed metadata reads.
Use its complete ingress checks and non-regressing cost ceilings.
The [borrowed immutable metadata projection](decisions/borrowed-immutable-metadata.md)
retains a live owned product's exact allocation-bound admission when selecting an
ordinary immutable field. Raw input, packing, consuming unpack and transfer still
own independent admission; extend those boundaries only with an exact checked
construction/extraction contract. Neither shape equality nor a type annotation
authorizes skipped raw admission.

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
