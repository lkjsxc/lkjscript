# Language-first project direction

Owner decision: 2026-09-28 (Asia/Tokyo), reaffirmed by the 2026-09-29 repository-target correction. The owner clarified that the later Rust-only and lkjstr.lkjsxc.com request concerned lkjsxc/lkjstr, not this repository. The numbered owner answers are retained in
[the mandate](campaigns/202609281300.md). This document separates binding direction
from delegated, revisable engineering selections. [Status](status.md) describes
implemented behavior; [the roadmap](roadmap.md) selects the next work. A selected
design is not a claim of implementation, performance, proof or publication.

## Non-negotiable authority and priorities

The accepted typed meaning graph is the sole editable program-meaning authority.
Do not weaken this priority, restore text as a competing source of truth, or treat
the graph as merely a compiler cache. Physical storage and derived representations
remain replaceable. Type, ownership, lifetime and effect contracts belong to meaning;
placement, code generation and recomputable analyses are derived.

The language is agent-native. Human comprehension, manual authoring, readable
syntax, human-facing explanations and human participation are not requirements.
Do not introduce mandatory human review into the program-editing protocol.
Machine-readable discovery, exact contracts, diagnostic evidence and validation
remain necessary for agents and verifiers; machine verification is not a UI mandate.
Do not deliberately obscure information or couple the language to one model.

Prioritize the language core: powerful, composable generics, traits, ownership,
lifetimes, effects and concurrency, rather than another application starter.
Rust is a conceptual reference, not a syntax, source-compatibility or immediate
benchmark target. Better abstractions and their sound composition are the current
objective. Do not postpone architectural scale questions until optimization work.

Memory safety is a baseline. Avoid unnecessary copying. Immediate speed contests
are not a priority, but when performance tradeoffs must be selected, balance costs
with a preference for speed and scalability over minimizing memory at any price.
Keep CPU-cache locality, multicore/NUMA behavior and SSD-backed working sets in the
long-term design. Do not confuse allocation totals, live memory, RSS and PSS.

Shared runtime execution is a major product requirement, not a distant optional
wrapper. Multiple agents editing one project concurrently are also a required
future capability, desired early, but not a claim about the current CLI.

## Selected semantic direction (delegated, revisable)

Design one typed contract system across values and resources. Explore type,
lifetime/region, effect and bounded value parameters; associated type families,
higher-ranked borrowing and higher-kinded abstractions where they compose soundly.
Traits should carry typed methods, associated items and explicit implementation
witnesses. A trait constraint is not an execution grant. Select and retain exact
implementation identity rather than letting an added package silently change
resolution. Coherence, termination and ambiguity are explicit design obligations.

Infer local facts where unambiguous; preserve independently checkable contracts
across function and package boundaries. Keep proof checking distinct from search:
resource exhaustion while finding a witness is not proof that a valid program is
invalid. Neither an AI assertion nor a generated proof unchecked by the kernel is
acceptance. General dependent proofs and unrestricted solver power are not immediate
requirements. Add an expressive feature through a complete composition witness,
not through a disconnected syntax inventory. Prefer graph-native metaprogramming:
generated meaning passes the same checker, with no privileged generator bypass.
Explicit dynamic checks may be admitted where specified; unresolved static
ownership obligations do not silently become unchecked execution.

Use ownership modes for unique ownership, bounded borrowing, region-local aliases,
transferable ownership and transitively immutable sharing. These are semantic roles,
not frozen keywords or a requirement to expose a Rust-like surface.
Permit local mutation and reuse when the ownership/effect contract allows it;
retain pure external value semantics where promised. Do not silently insert deep
copies or globally shared mutable state to make an ownership error disappear.

Make effectful control and resource completion compositional. Explore typed effect
handlers, but a continuation retaining affine resources must not be duplicated
without a valid resource contract. Expected failures have typed outcomes. Traps,
cancellation and allocation failure have explicit boundaries; losing a response is
not rollback. Integer overflow and floating-point transformations require declared
semantics, not optimization-dependent accidental behavior.

## Selected memory direction (not an Arc-centered end state)

Prefer an ownership-and-region model over universal per-object shared ownership.
A uniquely owned region may contain internal aliases or cycles if its external
access contract remains sound. Allow a locally managed/traced region for dynamic
cyclic data; GC is a language-supported candidate, not forbidden and not a mandatory
whole-program heap. Static ownership, arenas, local tracing and immutable shared
segments may coexist under one contract system.

Keep ordinary object access local where possible. Cross-region references, roots,
freezing, region transfer and reclamation must have exact protocols. Freezing must
cover the reachable closure, not only its root. Regions must not retain unrelated
large objects indefinitely merely to reduce allocation calls.

A moving collector cannot invalidate live borrows: compare handles, pinning and
scoped no-move access with explicit costs before selecting a mechanism. Native
code must provide the necessary roots/safepoints or use an admitted nonmoving path.
GC releases storage; it does not implicitly commit transactions, acknowledge work
or reliably complete external I/O. Resource completion remains separately owned.

Retain a path for collector-free/no-allocation regions and eventual freestanding
profiles. A program requiring hosted services need not compile for those profiles.
Do not delete current Rust Arc/Box implementations merely to change a percentage;
they are bootstrap mechanisms, not the final language model or a prohibited tool.
Permit explicit allocation/layout constraints at resource-sensitive boundaries,
while keeping default placement derived. Ordinary application code gets no universal
unchecked escape hatch; any later low-level kernel must own a separately checked
safe boundary. No zero-copy, pause-free or bounded-RSS claim follows from this selection.

## Shared runtime and isolation domains

Distinguish shared installation, shared code/metadata and multiple live application
instances in one runtime process. Only the last, with actual shared infrastructure,
closes the requested shared-runtime capability. A launcher creating one full process
per application does not close it.

Share compatible immutable code, type/layout metadata and exact library instances,
plus scheduler, I/O services and derived caches where their contracts permit it.
Do not share application mutable state, captures, credentials, capability grants,
transaction state, cancellation lineage or per-application accounting by accident.
Code sharing never grants permission to execute an imported capability.

An isolate/domain is a lifetime, state, authority and accounting boundary, not an
OS thread and not necessarily one request or one application. Give it an exact
program revision, local roots and owned tasks. Allow multiple independently owned
tasks/regions within one application. Share only explicit frozen data or transfer
owned data through checked channels; no arbitrary cross-domain mutable pointers.

Use a compatible runtime pool, not one universal process forever. Runtime ABI,
trust boundary, target and required services determine pooling; incompatible
versions may coexist. Permit process/NUMA sharding without changing program meaning.
Initially support trusted code. Logical isolation does not establish hostile-code
containment, Spectre resistance or survival of a fatal host-process failure.

Own admission, readiness, stop, joined cleanup, unload and version replacement.
Bound queues and account shared resources as well as private allocations. Reserve
receiver capacity before an ownership transfer becomes visible. Distinguish send
acceptance from message processing and durable completion. A blocked native call
cannot become safely preemptible merely by cancelling a future; safepoints and
blocking-work placement are separate obligations.

## Parallel programs and parallel authors

Select structured concurrency with ownership-aware tasks, typed bounded channels
and data-parallel library operations. State-owning actors are useful, but do not
make every application a single serialized event loop. Permit parallel work on
disjoint regions and immutable inputs. Explore scheduling from declared access
requirements without confusing memory independence with external-effect ordering.
Work-stealing and locality-aware scheduling are mechanisms to compare, not proofs
of scaling. Backpressure, fairness, cancellation and reclamation must compose.

Separate multicore execution from future multi-machine distribution. A same-process
transfer may reuse storage; cross-process/host messages need a transport contract,
and cannot promise shared addresses, zero copying or exactly-once effects.

For parallel AI editing, use immutable base revisions and private candidates.
Read and prepare in parallel, then revalidate dependency/contract footprints at
publication. Disjoint edited owners are not sufficient when type, trait resolution,
effect or reference dependencies overlap. A short serialized publication point is
compatible with parallel preparation. History should provide conventional revision,
branch/tag, difference, merge and retained-root concepts rather than every analysis
forever; exact features and reclamation remain unimplemented until verified.

## Execution, storage and long horizon

Support both a dedicated runtime and native executables from the same accepted
meaning and semantic lowering. A native executable may embed needed runtime services;
it need not depend on a shared daemon. A later freestanding profile should reject
unsupported hosted effects instead of silently changing their meaning.

Ordinary hosted programs come first. Keep OS, driver and embedded use in the eventual
horizon. Linux x86-64 remains the initial supported target. Wasm is optional; if
selected, derive it from the meaning graph, not from a second authored language.
Other-language interoperability is not a priority or an excuse to defer native
capabilities. Preserve ordinary development without external semantic generators.

Own the persistence engine and typed data semantics; do not select an external
database engine as the required persistence substrate. OS/filesystem interfaces
remain implementation boundaries. Separate memory reclamation from durable storage,
transactions, snapshots and disk compaction. SSD layout, caching, batching and
recovery are explicit storage design questions, not a transparent unlimited heap.

Keep offline development possible and dependencies exact, with explicit updates.
A convenient single distribution need not make every application retain every
service. Move tooling, compiler and runtime toward eventual complete self-hosting,
including the possibility of removing Rust entirely. Do not impose a distant date
or rush a rewrite before its replacement can own the relevant semantics.

The owner explicitly corrected the 2026-09-29 Rust-only request to target
`lkjsxc/lkjstr`. It does not supersede this repository's self-hosting direction.
The already-integrated Rust policy classifier and experimental documentation origin
are implementation facts, not a permanent Rust-only mandate. This correction does
not roll back their code or the separately developed language improvements.

## Governance and experimental compatibility

lkjsxc has final authority over the official project's direction and acceptance;
agents exercise delegated engineering judgment. Apache-2.0 remains the license.
Official-mainline governance does not alter the license or third-party permissions.
There is no current stability, compatibility, user-acquisition or support promise.

The owner permits breaking changes without a required migration path, including
resetting owner-authorized experimental lkjscript data. Do not reintroduce a blanket
product-level confirmation or backwards-compatibility requirement. This permission
does not extend to unrelated work, third-party data, credentials or access controls.
An intentional compatibility cut is not permission to conceal corruption or relabel
failed effects, tests or releases as successful. This direction update deletes no
operational data and changes no running service or published artifact.

Use existing authorized resources; no monetary budget was specified, so delegation
does not imply unlimited purchases or new recurring charges. Preserve the smallest
independent verification needed for each claimed property. Concrete mechanism
choices remain revisable, but agents must not weaken meaning-graph priority.

## Research inputs, not inherited guarantees

[Workers](https://developers.cloudflare.com/workers/reference/how-workers-works/)
illustrates hosting many isolates in one runtime. Its
[security model](https://developers.cloudflare.com/workers/reference/security-model/)
is a separate system, not a guarantee obtained by copying the topology.
[Verona's region work](https://arxiv.org/abs/2309.02983) explores local memory policies
under reference capabilities. [Pony](https://tutorial.ponylang.io/reference-capabilities/passing-and-sharing.html)
illustrates checked transfer and immutable sharing; its
[performance notes](https://www.ponylang.io/use/performance/pony-performance-cheat-sheet/)
also expose collection and inter-actor bookkeeping costs.
[Koka](https://www.microsoft.com/en-us/research/project/koka/)
provides a research reference for type/effect composition. None establishes that
lkjscript already implements these models or can combine them without new proof.
