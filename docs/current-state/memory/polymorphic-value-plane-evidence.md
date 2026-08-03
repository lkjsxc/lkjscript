# Bounded Polymorphic Value Plane Evidence
## Status

**Current at revision 18 for the prepared compiler-selected sealed-value and
bounded residual generic compare verticals.** Revisions 14 through 17 retain
predecessor evidence. The complete polymorphic value plane remains an Accepted
Target; persistent structural lists, residual codecs, and indirect generic
callables are not Current.

## Evidence Identity

Residual implementation commit: `9f82cc5cf9836d07e16267b2387b007547ebeaef`.
Evaluator/VM owner-list commit: `96653f9c0a9368464e5c00d49901a5f82823a1d1`.
Revision-14 four-tier cutover commit: `941252e0f7ad5b4d91eb43f94cd8277101bbc3ae`.
Exact native identity commit: `727f1207f4be31d53207a9661807ea4dd8d13d5c`.
Iterative native aggregate-equality commit: `a6de268fcc992349c47c4bca336402e263ba8545`.
Revision-15 authenticated witness commit: `715a13b93fd907660505254bfb8f3570a740d0e1`.
Revision-16 semantic-closure commit: `187ec9bc8f68dd6a437e69d3781e81f7de6a6694`.
Revision-17 prepared vertical commit: `5102f06a0c4c37a8c51677fa025000361300317e`.
Revision-18 residual compare commit: `4e176e7f858778e066ab607f9dda5d6155077a37`.
Historical predecessor: revision 17; Current platform: revision 18; environment: locked Linux x86-64 workspace.

## Experimental Residual Witness Slice

- HIR independently produces and verifies sorted hidden witness requirements and
  direct-call witness arguments. The sole experimental operation is closed
  `transport`; witnesses are not source values.
- SSA retains a content-addressed immutable descriptor table, exact dependency
  IDs, structural routes, function requirements, concrete substitutions, and
  call bindings. Verification rejects stale type, operation, route, ordering,
  dependency, and substitution records.
- Validated bytecode installs only executable witness closure needed by
  structural routes or residual calls. It validates table limits, IDs,
  dependencies, routes, prototype requirements, call sites, and runtime value
  categories before dispatch.
- Evaluator and VM execute the residual hidden binding. The VM stores bindings
  in frame metadata outside source locals and operand values and validates both
  call arguments and generic results.
- Forced native tiers perform verified exact identity specialization. Each
  specialization accepts zero or one exact owner place, one type parameter, one
  transport requirement, one value argument, and an instruction-free identity
  body. Canonically ordered exact substitution, trait-witness, and memory-witness
  identities deduplicate repeated calls and select distinct generated functions.
  The bounds are 32 instances per declaration and 1,024 per package closure;
  rewritten SSA is independently verified. Baseline and proof tests require
  equal results, nonzero native entry, and zero VM fallback.
- The experiment admits direct calls, naked type-parameter transport, and at
  most 16 parameters and arguments. Nested unresolved parameter uses, indirect
  calls, nonidentity bodies, identity mismatch, bound overflow, and unknown
  operations reject rather than falling back.

## Revision-15 Authenticated Witness Slice

A contracts-owned binary encoder now commits exact executable facts, closed
operations, and ordered dependency identities with fixed tags and framing. HIR
producer and independent verifier reconstruct permitted capabilities separately
from the selected type-level route. SSA and bytecode preserve the projection and
recompute every installed identity; malformed facts, operation routes,
dependency records, and cycles reject. Move, exclusive-borrow, and external
handle routes do not acquire clone authority.

The only public sealed DAG import resolves the validated return representation
and its compiler-installed witness. It requires exact witness identity, mode,
structural root, sealed and codec capabilities, decode operation, and owner
representation before validating DAG shape. The former public API accepting a
caller-supplied root and type closure is private test machinery. Compiler-source
product, path, option-string, result-path, and general-enum tests exercise this
boundary; the product test crosses the key-free execution-outcome codec before
import into a fresh runtime. Exact enum physical tags, active variants, field
order, nested types, rollback, borrow/export, and final coarse-owner release are
covered. Focused Miri, ASan, LSan, TSan, release scaling, canonical verification,
and Docker verification pass on the commit.

## Revision-16 Canonical Semantic Closure Slice

Products and ordered fields now have stable Semantic Source content identities.
A contracts-owned target-neutral descriptor closes the exact reachable product,
enum, argument, list, function, and recursive declaration graph without Rust
`Debug`. Product-field, enum-variant-field, type-argument, and list-element roles
target exact child witnesses or same-SCC local semantic identities, avoiding a
witness self-hash. Separate producer and verifier traversals construct the data;
SSA and bytecode retain it and independently recompute semantic-contract,
semantic-type-closure, and executable-witness identities. Malformed roles,
local targets, external semantic links and cycles, and duplicate bytecode owner
representations reject. Unrelated declarations do not perturb a witness.

At revision 16 this remained receiver-side import without compiler-selected
placement, atomic recursive groups, package provenance, complete relinking, or
daemon receiver evidence. Revision 17 closed only the narrow vertical below.

## Revision-18 Current Prepared Sealed And Compare Verticals
Producer and independent verifier reconstruct semantic closure, atomic witness
groups, exact representations, value placement, and prepared authority. Typed
package interfaces and locks bind plan, witness closure, SSA, bytecode,
specialization, codec, and one immutable `PreparedProgram` identity.
Resolved HIR selects sealed storage for one ordinary independently shared value
while borrow, move, unique fusion, clone, and single-owner routes remain exact.
Evaluator, VM, forced baseline, and forced proof execute publication, borrow,
coarse owner acquisition, release, and failure cleanup. One nonidentity residual
body dispatches hidden `independent-owner` and `dispose` in every tier with
nonzero native entries and zero fallback. The bounded direct compare body derives
only `transport` and `compare`; authenticated evaluator, VM, baseline, and proof
dispatch produce equal output, nonzero native entries, and zero fallback.
An isolated child returns a provenance-bound key-free semantic DAG. A fresh
parent authenticates the prepared identity and witness closure, rehydrates,
canonically re-exports, and releases every owner with zero live obligations.
The retained strategy comparison selects unique fusion or coarse sealed sharing
and rejects production per-node ownership. Source sealing, persistent structural
lists, residual codecs, and complete-plane promotion do not follow.

## Locked Package Interface

Each locked module now contains `interface_sha256`, sorted exports, and sorted
public `witness_requirements`. A public direct generic signature contributes one
requirement per type parameter used naked in its parameters or result. Each
requirement binds the export, parameter, ordered closed operations, current
module-interface contract, and exact digest. The module and package hashes frame
the interface digest; stale source or requirement records therefore change the
content identity. The package decoder accepts no older or alias layout.

`src/examples/polymorphic-transport/` is a real locked local package graph. Its
`polymorphic-history` dependency exports generic `keep-history`; separate
consumer modules import it through the dependency path. The package lock binds
two package content identities and the dependency's public transport requirement.

## Structural-Owner List Prerequisite Slice

The HIR producer and independent verifier select bounded segmented regions for
exact immutable structural element witnesses. Each dynamic element is a
detached owner in one list-region side ledger; `list-first` creates a separate
owner, and teardown releases retained owners before runtime emptiness checks.
Evaluator, validated VM, forced baseline, and forced proof tests execute dynamic
string and option ownership, product `list-first`, nested option lists, recursive
nested-list equality, exact cleanup, and zero fallback. Native structural
islands bind list and element layout plus semantic identities, accept only
verified frame-home heap sites, clone detached owners into one list ledger, and
release it before runtime emptiness verification. Product equality, paths, and
process-boundary structural-owner lists remain blocked.

## Experimental Core Semantic DAG Prerequisite ([evidence](sealed-semantic-dag-evidence.md))

The core boundary model and execution-outcome codec accept one bounded,
key-free, reverse-topological semantic DAG. Focused construction, malformed
import, codec, and process-protocol tests cover product-list-product values,
nested immutable structural owners, shared subgraphs, exact type/layout facts,
full framing consumption, and forward, cycle, unreachable, type, and bound
rejection. Construction remains private until complete validation.

Private core tests retain a direct root/type-closure adapter for malformed and
scaling coverage. Production callers cannot use it. The public import derives
its exact root and bounded type closure from validated bytecode and the installed
compiler witness, then copies the DAG into one private sealed region. It
atomically publishes one coarse owner, supports borrowed key-free export,
returns the unchanged snapshot on failure, and rolls back its dropless builder
without allocating cleanup. An 8-versus-2,048-node test keeps owner/dependency
release planning at one region unit; chunk reclamation is not claimed invariant.
No execution tier, list, or ordinary-region path yet selects this adapter.

## Four-Tier Workload And Snapshot

The locked workload constructs and transports one nested `list<list<i64>>` per
history operation. Balanced source helpers execute exactly 4,096 operations and
8,192 segmented-list prepends while retaining bounded native frame depth. The
observable sum is `8,390,656` in evaluator, VM, forced baseline, and forced proof.
Both native tiers report nonzero list activity and zero VM fallback.

A separate locked entry returns a key-free nested copy-list snapshot through the
same cross-package generic witness. VM, baseline, and proof execution accept the
entry. The daemon application-control test installs the exact package identity,
runs it in an isolated process cell, crosses both process and authenticated
control codecs, and observes `Returned(#<owned-list:1>)`. Runtime list keys and
witness slots are absent from the owned snapshot.

The Current durable application quota retains its 32 KiB heap ceiling. The
4,096-operation scalar-result workload and the nested-list process snapshot are
therefore separate evidence; this change does not weaken that Current boundary.

## Sealed Sharing Falsification

`sealed_sharing_counts_regions_not_nodes` constructs otherwise identical sealed
regions with 8 and 2,048 payload nodes. Each receives 128 independent coarse
owners. Both cases record exactly 128 retains, 129 releases, and 129 release-work
units. Final reports release all payload objects, but retain/release work is
invariant with payload-node count. This proves the Current sealed-region
substrate has no per-node reference-count traffic; it does not claim a source
`sealed` type or language selection.

## Command Evidence

Exact revision-18, revision-17, and retained earlier commands are
recorded in the [verification evidence](polymorphic-value-plane-verification.md).

## Explicitly Not Current

- no source `sealed T` or `seal` operation;
- no residual encode, decode, list-store, or list-load body;
- no persistent structural-owner list or authenticated structural-list import;
- no general product or enum equality;
- no repeated compare or capture-free indirect generic callable ABI;
- no complete selected list/process matrix or complete-plane promotion claim.
