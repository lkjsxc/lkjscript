# Evidence-gated roadmap

[Project direction](direction.md) records the owner's current choices.
[Status](status.md) owns availability; [specifications](spec/) own implemented
contracts; [campaigns](campaigns/) retain attempts and evidence. A roadmap entry
does not add a capability to an existing executable.

## Selected priority: language core and shared execution

The 2026-09-28 [owner mandate](campaigns/202609281300.md) supersedes the previous
application-editing-first priority. Preserve meaning-graph primacy and strengthen
generics, traits, ownership, lifetimes, effects and scalable concurrency.
Human comprehension and manual participation are not admission criteria.
Memory safety and avoidance of unnecessary copying are baseline requirements.
Immediate Rust benchmark superiority, extreme memory minimization, compatibility
migration and external adoption are not prerequisites for this research phase.

A genuine shared runtime is a central requirement. Do not defer it behind a complete
new type system, collector, JIT, AOT backend or web framework. Conversely, do not
call a process launcher, shared installation or unverified shared mutable heap a
solution. Develop a small end-to-end shared-execution slice with explicit ownership
and failure boundaries, then extend its language contracts.

## First coordinated slice

### Semantic boundary

The first scoped capability borrowing boundary is public in v0.1.54. The
[type-generic continuation](campaigns/202609281735.md) composes ordinary type
parameters, data callbacks and results with exact concrete borrowing/consumption.
The [recursive continuation](campaigns/202609282124.md) admits direct and mutual
synchronous helpers by checking their declared use contracts compositionally,
without adding a parallel ownership mechanism or treating termination as safety.
The [package continuation](campaigns/202609290026.md) extends the exact resource
contract to public libraries and forwarders, with imported authority explicitly
named by entry ports and still granted at deployment. Ordinary generic borrowing,
recursive implementation and final consumption compose across three packages.
The [exact-selector continuation](campaigns/202609290337.md) closes the same-name
deployment selection gap using existing package/requirement identities and one
shared preflight/preparation resolver. Name ambiguity never grants authority by
elimination, and independent durable roots remain distinct.
The [multi-resource continuation](campaigns/202609291100.md) composes a contiguous
final suffix of borrowed/consumed resources, each retaining exact concrete authority.
Repeated borrows may share an owner; any consuming alias in one helper call rejects.
Native two-queue and imported-library workloads retain this boundary through
ordinary generics, recursion, detached execution and independent persisted effects.
The [effect-callback continuation](campaigns/20260929-effect-resource-callbacks.md)
allows resource-free task callbacks to carry explicit effect parameters through exact-resource
helpers. Callback effects and resource authority remain separate: the resource binding stays
concrete, including across recursive/package calls and repeated-borrow suffixes. Requirement-
polymorphic resource transfer, resource returns and general memory references remain open.
These are implementation increments toward the coordinated slice, not substitutes for its
region/trait/ownership design or evidence of zero-copy payload processing.

The [concrete ByteBuffer increment](spec/owned-byte-buffers.md) implements pure
creation, scoped synchronous reads, consuming updates, direct owned result transfer
and lexical/failure cleanup across ordinary generic packages. Capability-resource
authority remains separate. The [first-order owned abstraction](spec/owned-generics.md)
implements rank-one Owned constraints and explicit nominal implementation witnesses
for ByteBuffer and an independent scalar cell. The
[structural product slice](spec/owned-products.md), now integrated in development, composes
explicit affine fields with closed ordinary metadata and complete consuming
decomposition. Generic abstraction is deliberately over Owned payloads; open
ordinary metadata parameters require a future first-order data proof. Extend this boundary toward
typed traits, general lifetimes and region policies; none is supplied by this
increment. Keep declared ownership/effect meaning separate from physical placement
and recomputable analyses.

The next preferred composition experiment is an owned choice/outcome: success may
transfer an owner, while a rejected operation can return its still-owned payload.
Ordinary unrestricted Option/Result containers are not an implicit escape hatch.
Require complete consuming case analysis, exact generic/package witnesses and
single-owner failure cleanup before using this boundary for asynchronous queues.
This is a selected experiment, not implemented syntax or a new release-number
milestone. Its purpose is to make recoverable failure compositional before growing
shared-host supervision or introducing more concrete memory carriers.

Exercise a generic producer/transformer/consumer across a package boundary.
Require independent negative cases for duplicate consumption, escaping a borrow,
wrong implementation witness, effect/grant mismatch and invalid retained capture.
Do not force every advanced generic feature into the first increment. Preserve
a coherent extension path for associated type families, higher-ranked borrowing,
effect polymorphism and region-aware traits.

The [affine-work correction](campaigns/20260929-affine-validation-work.md) closes
unmetered metadata traversal and lost exhaustion reporting in the existing proof
boundary. Keep bounded proof and invalid meaning distinct while extending ownership;
this correction does not itself implement the owned-region slice.

The first boundary must be useful from ordinary graph authoring, query, package,
check and execution paths. A privileged Rust helper or new declaration name alone
does not establish a language feature. A proof-search limit is reported separately
from invalid meaning.

### Small shared host, early

The first [shared host](spec/shared-runtime.md) is public in v0.1.53:
repeated `serve --deployment` arguments, exact whole-artifact preparation
sharing, private instance state/grants/tasks and joined fixed-group termination.
The [implementation campaign](campaigns/202609281356.md) owns its verification.
This completes the initial hosting slice, not the coordinated ownership/type slice.

Extend this in-process host for multiple admitted program instances with
shared immutable preparation/code and independent instance state, grants and tasks.
An initial host may use current execution machinery; advanced region GC and native
code are not prerequisites. Do not bypass current type/origin/resource admission
to make code sharing possible.

At minimum, prove two instances of one exact program share the intended immutable
runtime object while retaining different private state, and prove two distinct
programs can coexist. Bind each instance to its selected version. Cover separate
arguments/results/captures, authority isolation, handled failure, stop/drain/unload,
and continued operation of the other instance. Account shared and private storage.
The public operation must not require opening the author's mutable project.

Single-process hosting does not prove CPU parallelism, hostile-code containment,
a universal pause bound or survival of a host crash. Never install a second global
authority for application state or deployment selection.

### Shared-host lifecycle

Define a compatibility/pooling key, instance admission, readiness and joined stop.
Keep code installation, instance activation and data migration separate. Bind
secrets and operational handles to an instance, not to reusable code metadata.
Test version coexistence and cancellation during admission as well as normal stop.

The first-order [Owned implementation](spec/owned-generics.md) now supplies the
symbolically checked producer/transformer/consumer boundary with explicit witnesses
for both ByteBuffer and OwnedI64Cell. Its focused native evidence includes exact
same-Self alternate selection, loan cleanup and source-free package execution.
Its exact `9afa7997` source completed all 26 frozen-source gates and reached main;
[status](status.md) separates this accepted implementation from public binaries.
General traits, owned containers, mutable or escaping borrows, asynchronous ownership
transfer and region placement remain future work. Prefer an independently tested
owned-data composition boundary before adding new dispatch or supervisor machinery.

Do not turn the initial host into a broad supervisor before those transfer,
lifetime and effect contracts compose. Dynamic CLI admission/removal and
component-granular pooling remain separate future extensions.

Reserve queue and destination capacity before transferring ownership. Queue-full,
cancelled-send and failed-receiver cases must leave exactly one valid owner or a
defined cleanup owner. Message acceptance is not application completion or exactly-once
delivery. A cancelled blocking native call needs an explicit lifecycle solution.

## Following increments, not a waterfall prerequisite

### Ownership and memory-management regions

Terminal ordinary-local transfers avoid a proved unnecessary duplication without
changing source meaning or resource authority. The
[control-flow continuation](campaigns/20260929-flow-local-moves.md) extends this to
exclusive branches and path-specific redefinitions with bounded fixed-point analysis
and a conservative fallback. Keep this as a disposable execution optimization, not a
substitute for the region/trait/ownership contract below. The original
[controlled evidence](campaigns/20260929-terminal-local-moves.md) and its continuation
distinguish retained payload identity from local-read counts and from any whole-program
zero-copy claim. [Immutable map-key sharing](campaigns/20260929-shared-map-keys.md)
removes another concrete copy boundary without changing logical admission or
language meaning. [Unique byte concatenation storage](campaigns/20260930-byte-buffer-reuse.md)
uses terminal ordinary transfers to amortize prefix copying while retaining immutable
aliases, keys and captures. Its extra metadata and spare capacity remain derived;
it is not the selected language-level ownership or region contract. These bootstrap
storage choices do not make universal atomic reference counting the future region model.
[Immutable byte ranges](campaigns/20260930-byte-ranges.md) add strict non-copying
selection and explicit backing detachment to ordinary Bytes. Their flat carrier and
retention tests close a concrete binary-processing boundary, not the semantic
owned-region/scoped-borrow/trait slice selected above. Prefer that semantic slice
next over treating further carrier optimizations as a replacement for it.

Compare unique/scoped storage, region-local aliases, local tracing and frozen
shared segments. Support cyclic data without making universal Arc-style ownership
or a global tracing heap the language's only answer. Connect borrow lifetime,
region escape, transitive freezing, moving-collector roots and native safepoints.
Keep externally visible resource completion distinct from GC.

First require semantic and failure correctness, then compare allocation, copying,
retained memory, pauses, throughput and preparation costs on identical workloads.
A region's ability to grow without a small toy bound is not a scalability proof.
Keep a no-collector/no-allocation path for future freestanding use where its effect
contract permits it; not every hosted application must satisfy that profile.

### Parallel execution within and across applications

Use structured task ownership, typed bounded channels and disjoint-region work.
Do not equate an isolate with one permanent OS thread or serialize every application
through one event loop. Start with a real CPU-parallel transform/reduction and an
independent I/O-heavy instance, then test fairness under a saturated neighbor.

Evaluate work-stealing, local queues, bounded batching and locality-aware placement
after the semantic contracts exist. Preserve explicit external-effect ordering.
Cooperative safepoints, native blocking work and fatal host errors need distinct
claims. Multi-machine transport, durable messaging and distributed consistency are
separate work; in-process zero-copy observations do not prove them.

### Concurrent AI modification

Make immutable reads and private preparation parallel, with exact-base publication
and semantic conflict validation. Exercise disjoint edits and hidden dependencies:
a changed trait implementation, type contract or effect can invalidate a candidate
even when edited owners differ. A short serialized commit point is acceptable.
Do not claim concurrent editing merely because two agents can run shell commands.

Select conventional revision/branch/tag/difference/merge and retained-root history
semantics without retaining every derived analysis forever. Stable identities should
help exact changes, not force heuristic identity matching or an immutable storage
layout. History deletion/compaction requires its own reachability and interruption
proof; it is not ordinary cache eviction.

### Execution and storage horizon

Derive both native executables and dedicated-runtime programs from the same accepted
meaning. Hosted native executables may embed scheduler/collector services. Later
freestanding profiles reject unavailable services. Keep Linux x86-64 first; expand
targets with actual execution and distribution evidence. Wasm remains optional.

Own the persistence engine. Keep durable transactions separate from heap collection
and program revision history. Study SSD locality, paging/prefetch, batching and
bounded caches on concrete data workloads rather than promising disk-backed RAM.
Do not introduce an external database as the required engine.

Advance native libraries/tooling, then compiler/runtime self-hosting toward eventual
Rust removal. This is a genuine horizon, not a language-percentage acceptance metric.
The owner corrected the 2026-09-29 Rust-only and custom-domain request to target
`lkjsxc/lkjstr`; it does not supersede this language's [direction](direction.md).
Replace supported host boundaries only when native successors can own their
semantics and failure behavior. Preserve independent language improvements and
historical evidence rather than broadly reverting them. Other-language compatibility
and external user acquisition remain low priorities.

## Acceptance and measurement

Each selected increment needs a written semantic boundary, normal public consumption,
independent success/failure expectations and a complete mainline delivery point.
Ambition changes which problem is selected; it does not turn prototypes into proofs.
Keep current verification owners until a deliberate replacement assumes their duty.
Do not waive a relevant failing gate or add duplicate proof inventories for appearance.

For shared runtime experiments, record actual process topology, code/preparation
sharing, instance count and workload. Separate cold/warm admission, latency tails,
CPU throughput, task fairness, compiler preparation, copying, live memory and
total RSS/PSS. Include a matched separate-process baseline; OS-shared code pages
already present in that baseline must not be counted as a new runtime saving.
Include repeated start/stop cycles and a resource-heavy neighbor. Scaling numbers
require observations, not extrapolation from a two-instance demonstration.

A documentation-only direction change runs relevant documentation/policy checks,
not a synthetic new runtime acceptance campaign. Later product changes still need
their dependency-complete source and affected target evidence. No new release is
required merely to publish these direction choices.

## Prior work and reversal

The previous application-first roadmap remains available at its
[original revision](https://github.com/lkjsxc/lkjscript/blob/1f63d1197d047012c61e63eb9eb93098e9d7f02b/docs/roadmap.md).
Native web/editor, form codecs, named drafts, history and deployment lifecycles remain
useful implemented consumers and regression witnesses, not a compulsory sequence
of further convenience features. Their failures and release records remain intact.

Keep the graph-first constraint fixed. Revise concrete mechanisms when evidence
favors a better complete design. The owner permits compatibility cuts without a
migration promise and resets of owner-authorized experimental data; this does not
authorize unrelated destruction or changes to historical evidence. State actual
behavior, unfinished work, retained resources and measured regressions accurately.
