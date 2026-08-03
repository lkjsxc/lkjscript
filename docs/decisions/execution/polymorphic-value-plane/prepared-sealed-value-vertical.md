# Prepared Compiler-Selected Sealed-Value Vertical

## Status
<!-- LKJ-STATUS id=compiler-selected-sealed-value-vertical status=current -->
**Current for the first narrow executable vertical.** Revision 17 implements this
record over the revision-16 canonical semantic closure. The complete polymorphic
value plane remains Experimental under its Accepted Target until its broader
source, container, and indirect-generic promotion boundaries pass.
## Authority Chain
One immutable prepared authority binds:

```text
package content and target
-> resolved typed HIR
-> independently verified memory plan and value placement
-> atomic executable witness groups
-> verified SSA
-> validated bytecode and structural representations
-> runtime-call and native-layout contracts
-> process bootstrap and result identity
```

A digest identifies exact bytes and facts; validation grants authority. Dense
slots, addresses, runtime keys, source spelling, compiler approval, signatures,
and tests cannot grant witness or placement authority.

## Atomic Witness Groups

Every installed witness belongs to one group. A nonrecursive type uses a
singleton group. A recursive declaration SCC, including a self-recursive
singleton, uses one recursive group and installs atomically.

Members sort by stable semantic declaration or instantiated-type identity.
Canonical group bytes contain the group kind, ordered semantic member
identities, complete executable member facts, role-bearing local edges, and
role-bearing external group/member dependencies. Local edges use canonical
member ordinals and never final witness hashes. External group dependencies form
an acyclic graph.

Identity is cycle-free:

```text
group = hash(canonical members with local ordinals, external group/member IDs)
member = hash(group, canonical ordinal, semantic member identity)
```

`MemoryWitnessId` remains the sole member identity. No competing member ID is
introduced. A validator reserves the whole group, recomputes every identity and
edge, validates the external DAG, and publishes all dense slots together.
Failure publishes none.

Producer and verifier independently derive SCC membership, member order,
capabilities, operations, and dependency policy. They may share contracts-owned
schemas, closed tags, bounds, canonical encoders, hashes, and policy-free graph
primitives only.

## Package Memory Provenance

Each public generic export has one typed memory interface derived from resolved
HIR, not source-form scanning. It binds declaration identity, ordered type and
trait parameters, minimal ordered witness operations, parameter memory modes,
result ownership, equality and process-codec constraints, and its exact digest.

The locked target binds module interfaces, package memory interfaces,
`MemoryPlanId`, closed group/member and external dependency closure,
specialization identities, and applicable process-codec identities. Missing,
extra, reordered, stale, or ownership-incompatible records reject before
compilation or effects. The scanner-derived transport requirement is removed.
## Prepared Program
The prepared program is immutable and contains no dynamic execution owner,
loan, provider, process handle, runtime address, or mutable application root.
Its descriptor binds platform revision, package and entry identity, interface
closures, package memory closure, `MemoryPlanId`, semantic and native SSA
identities, validated bytecode identity, witness-group closure, runtime-call,
native-layout, and process control/outcome contracts.

SSA and bytecode identities hash exhaustive target-neutral projections. The
prepared identity is excluded to break the cycle. After installation, a separate
verifier reconstructs every descriptor field from artifacts, plan, package,
contracts, and profile before exact comparison. This is not a native-image cache.

## Structural Relinking

Each structural type links exact semantic member, witness member, witness group,
runtime semantic type, layout, aggregate mode, kind, declaration identity, and
type arguments. Product and enum layouts retain stable field/variant identities,
source order, semantic child types, dependency roles, and physical tags.

Storage is a closed enum:

```text
inline static stack caller-destination unique-structural ordinary-region
sealed-region borrowed-view external-resource
```

Category remains separately `owner`, `view`, or `destination`. The same semantic
type may have several exact owner representations. Uniqueness is over the full
semantic-member, witness-member, group, layout, category, storage, and route
tuple. Every operation names one exact representation; first-match lookup by
type or category is rejected.
## Value Placement
A type witness states legal capabilities. Each HIR value separately records use
count, last use, escape, return/capture/process facts, branch divergence,
independent-owner demand, checked size and nodes, clone cost, dependency cost,
over-retention, release cost, representation, route, and failure cleanup.

Decision order is borrow, last-use move, unique reuse/fusion, bounded detached
clone, then coarse sealed sharing. Two semantic owners require independent
ownership only while their lifetimes overlap and diverge. Process return alone
does not force in-process sealing.

The initial deterministic share rule requires at least two independently live
owners, immutable sealed and codec capability, at least 8 checked structural
nodes or 256 checked payload bytes, at most 64 dependencies, and release work
within the active structural profile. Single-owner, nonescaping, and last-use
values remain nonsealed. Hash order, time, process identity, telemetry, and
adaptive profiling never affect placement.

## Execution And Ownership

The mandatory ordinary source fixture uniquely constructs one nonrecursive
payload and creates two independently live owners inside one `shared-fork`.
Another fixture uses the same semantic payload with one owner. Nonescaping and
last-use fixtures prove borrow and move precedence. Branch and injected-failure
fixtures prove exact cleanup.

Publication preflights complete initialization, exact group/member and layout,
sealed representation, affine/resource exclusion, loans, dependencies, owner
slots, bytes, roots, and release work. Compatible unique backing transfers
without copying; otherwise copied bytes are reported. Publication is atomic.
Scope borrows add no owner traffic. Lifetime divergence acquires one checked
worker-local coarse owner. Final release ends no live borrow, executes exact
side drops, iteratively releases domain dependencies, frees storage, and
invalidates generations without liveness traversal.

Evaluator, VM, forced baseline, and forced proof consume the same prepared
placement and direct structural storage. One residual generic body executes
hidden `independent-owner` and `dispose` operations; native tiers pass validated
group/member slots rather than satisfying the requirement by specialization.
Specialized copy execution remains separately covered.
## Isolated Process Receiver
Parent and child independently prepare the same locked target and agree on the
full prepared identity before readiness. Every outcome binds application,
incarnation, cell, package, entry, prepared identity, platform revision, process
contract, return semantic type, and root group/member.

Only a bounded key-free semantic DAG crosses the process. No runtime key, owner
token, loan, dense slot, address, or provider crosses. The parent checks outcome
provenance before allocation, uses its prepared witness closure to import into a
fresh deterministic runtime, borrow-exports an equivalent canonical snapshot,
ends the borrow, releases all owners, and verifies zero domains, loans,
dependencies, and release work. Malformed results fail only the invocation.

## Predeclared Falsification

The mandatory payload compares detached deep clone, coarse sealed sharing,
one-node sealed domains, private non-atomic per-node reference counting, and
eligible unique fusion across two-owner, four-owner, branch, process, single,
borrow, and move workloads.

Semantic and cleanup correctness are absolute gates. An otherwise valid
candidate loses if operation p99 exceeds 2.0x, peak logical bytes exceeds 1.5x,
or copied bytes exceeds 2.0x the best eligible candidate. Production per-node
ownership also loses unless every non-node candidate breaches a gate. Retain
allocations, bytes, copied/fused bytes, coarse and per-node operations, atomics,
dependencies, logical live bytes, RSS, over-retention, compile time, code size,
and encode/decode/rehydrate/release percentiles. Delete losing production code.
Coarse region ownership is reported as coarse reference counting.

## Research Disposition

GHC `d415f38a75cc88921a344220ad2eed0e82fdb5ff` supports canonical recursive
ABI groups and group/member fingerprints; adopt the structure, reject MD5 and
compiler-private serialization. rustc `22057b88b091743bc0fd8d592a9264f0a6951403`
supports stable structural names and collision rejection; reject version-bound
`DefPathHash`, `StableCrateId`, and private metadata as semantic identity.

REAPI `becdd8f9ff811df88a22d3eadd6341753d51d167` and Nix
`8307c48d25b90582c3e49999cee4a7a46495d2b7` support complete canonical input
closures and distinct action/content identities; they do not prove semantic
validity. PCC (DOI `10.1145/263699.263712`) and translation validation (DOI
`10.1007/BFb0054170`) support receiver checking per artifact.

Swift `1f52e6384c7b59aba5e13a617900117559dad8e2` supports ordered hidden lifecycle
witnesses and staged metadata completion; reject pointers and type-wide value
placement. Lean `110db9cb751afaee8b2ac344887d6c7e632f77b4` supports compact immutable
regions without per-node counts; its unsafe erased load contract is rejected.
Automatic borrowing (DOI `10.1145/3798221`) and Perceus (DOI
`10.1145/3453483.3454032`) support borrow-first and last-use reasoning, not an
RC fallback or correctness claim for sealed cyclic graphs.

## Promotion Boundary

Revision 17 promoted only `compiler-selected-sealed-value-vertical` after atomic
witness groups, package/prepared provenance, independent placement verification,
exact relinking, four-tier direct and residual execution, isolated fresh-runtime
rehydration, complete cleanup, no-tracing verification, and the predeclared
comparison all passed together. The complete plane remains Experimental, and
source-selected sealing remains unreachable.
