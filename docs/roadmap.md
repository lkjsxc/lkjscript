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
It deliberately leaves effect/requirement-polymorphic resource transfer,
cross-package ownership, resource returns and general memory references open.
These are implementation increments toward the coordinated slice, not substitutes
for its region/trait/ownership design or evidence of zero-copy payload processing.

Choose a compact contract for a transferable owned region/buffer, a scoped read
view and a typed callable or trait implementation. Specify creation, borrowing,
move, result return and cleanup, including failure and cancellation. Keep declared
ownership/effect meaning separate from physical placement and recomputable analyses.

Exercise a generic producer/transformer/consumer across a package boundary.
Require independent negative cases for duplicate consumption, escaping a borrow,
wrong implementation witness, effect/grant mismatch and invalid retained capture.
Do not force every advanced generic feature into the first increment. Preserve
a coherent extension path for associated type families, higher-ranked borrowing,
effect polymorphism and region-aware traits.

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

Next prioritize the semantic boundary above: an ordinary package-level owned
producer/transformer/consumer with scoped reads and independent rejection cases.
Do not turn the initial host into a broad supervisor before the language can express
its transfer, lifetime and effect contracts. Dynamic CLI admission/removal and
component-granular pooling remain separate future extensions.

Reserve queue and destination capacity before transferring ownership. Queue-full,
cancelled-send and failed-receiver cases must leave exactly one valid owner or a
defined cleanup owner. Message acceptance is not application completion or exactly-once
delivery. A cancelled blocking native call needs an explicit lifecycle solution.

## Following increments, not a waterfall prerequisite

### Ownership and memory-management regions

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
Other-language compatibility and external user acquisition remain low priorities.

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
