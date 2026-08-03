# Authenticated Executable Memory Witnesses

## Status

**Current for the prepared sealed-value vertical.** Revision 16 established the
target-neutral semantic closure; revision 17 adds atomic groups, exact structural
relinking, malformed-artifact coverage, typed package provenance, prepared
identity, and evaluator/VM/baseline/proof execution. Witness use outside this
narrow vertical remains Experimental with the complete polymorphic value plane.

## Problem

Current HIR witness identities commit to compiler-private `MemoryWitnessFacts`.
SSA projects those facts into an executable descriptor, but bytecode copies the
HIR ID and validates only closed shape, dependency presence, and route shape.
A copied ID therefore does not authenticate modified executable facts.
Hand-authored sealed flags and caller-selected type closures remain Rejected.

## Authorities

Resolved typed HIR and its independently reconstructed memory plan decide type
legality, operation eligibility, and value placement. Verified SSA preserves
that decision. Bytecode independently recomputes the executable content
identity and relinks it to validated metadata. Runtime executes only the
validated route. A package or process receiver resolves the full identity
against its own installed package authority.

No backend, runtime, caller, source annotation, dense slot, pointer, or local
root may authorize a witness.

## Canonical Executable Identity

A contracts-owned semantic descriptor contains one root semantic type and only
its transitively reachable nominal declarations. Primitive, capability,
resource, product, enum, list, function, parameter, and `forall` forms have
closed tags. Products and enums are referenced by stable 32-byte Semantic
Source declaration identities; instantiated enum references retain exact type
arguments. Canonical declarations sort by identity, while fields, variants,
type parameters, and type arguments retain declaration order. Product fields
and enum members carry stable identities and source indices. Recursive local
references remain nominal identities, so mutually recursive declaration closure
has no content-hash cycle. Checked type, declaration, edge, work, and byte bounds
apply before hashing.

The executable encoder consumes that descriptor's semantic contract hash plus
aggregate mode, domain, root projection, routes, sizing, ownership, portability,
contention, sorted operation identities, and ordered role-bearing dependencies.
Closed dependency roles are list element, exact product field, exact enum
variant field, and exact nominal constructor type argument. Each target is
either an exact external child witness ID or a local semantic nominal/member
identity that occurs at that exact role in the authenticated closure. Complete
roles are unique; declaration order is never sorted away.

Both encoders use exact domain separators, fixed tags, big-endian fixed-width
integers, explicit lengths, and exhaustive field order. They never use `Debug`,
display, serde, host layout/discriminants, pointers, maps, or filesystem order.
Changing any reachable semantic fact, role, target, operation, or executable
fact changes the authenticated identity. Unreachable declarations do not.

HIR-only derivation evidence remains committed by `MemoryPlanId`; it is not
duplicated as runtime policy. The executable identity does not include
`MemoryPlanId`, because the plan already contains witness identities and would
form an identity cycle.

## Producer Projection

The HIR producer independently constructs the contracts schema from resolved
HIR, closes only declarations reachable from the root, and derives all direct
roles. Nonrecursive targets resolve exact already-interned child witness IDs;
same recursive declaration-SCC edges use local semantic targets and never a
witness self-hash. The HIR verifier implements a separate traversal and
construction. Producer and verifier share only the contracts schema, canonical
encoders/hashes, closed tags, and limits; neither calls the other's traversal or
compiler policy.

SSA copies only independently verified descriptors and role records. Bytecode
retains the semantic descriptor so IR and core validators can recompute its
contract hash and executable witness ID rather than trusting copied IDs.

## Accepted Atomic Group Cutover

The [prepared sealed-value vertical](prepared-sealed-value-vertical.md) replaces
individual installation with one singleton or recursive-SCC group. Canonical
members use stable semantic order and provisional local ordinals; external
edges name exact group/member identities. The group hashes complete member
facts and edges, and the sole `MemoryWitnessId` hashes group, ordinal, and
semantic member identity. Validation publishes no dense member slot until the
whole group and external group DAG pass. Producer and verifier independently
reconstruct group policy.

## Installation Provenance

An installed table is globally bound to one exact nonzero `MemoryPlanId`.
Structural routes retain local dense IDs only as bounded locators. Validation
resolves those locators and requires exact agreement among:

- witness semantic type and value kind;
- structural type witness identity and mode;
- runtime semantic and layout identities;
- structural layout identity and exact fields or variants; and
- owner representation category and storage.

Local dense IDs do not participate as semantic identities. Package memory
interface and lock digests bind the plan and installed witness closure outside
the witness ID; scanner-derived requirements cannot provide this authority.

## Consumer Validation

For every installed witness, SSA and bytecode validators require:

1. canonical executable fact form;
2. sorted unique nonzero table IDs;
3. ordered, unique, complete role resolution and exact role/target legality;
4. local targets present at the claimed authenticated declaration role;
5. an acyclic external-witness graph (local semantic recursion is not a graph edge);
6. semantic-contract and executable-ID recomputation;
7. operation, physical-route, structural type/layout/mode, and unambiguous owner
   representation compatibility; and
8. exact plan and package provenance where the enclosing artifact provides it.

Validation reserves bounded work before installation. Runtime receives only the
validated table.

## Type Capability And Value Placement

A type witness describes permitted executable capabilities. It does not select
the placement of every value of that type. Per-value HIR facts separately select
inline, static, stack, destination, unique, ordinary-region, sealed-region,
borrowed-view, or external placement from exact use, last-use, escape,
independent-owner, branch, process, size, and cleanup facts.

Borrow and last-use move precede clone or share. Sealed share is legal only for
fully initialized immutable values with an acyclic dependency plan, no affine
resource, no unique mutable owner, no escaping short borrow, and preflighted
memory and release work.

## Rejected Paths

Rejected production authorities include:

- trusting an encoded witness ID without recomputation;
- caller-supplied allowed type sets, operations, contracts, or sealed flags;
- treating structural shape validation as witness authentication;
- treating one type-level domain as every value placement;
- changing dependency order without changing identity;
- copying runtime root keys for independent ownership; and
- universal per-node or atomic reference counting.

Malformed-record constructors remain test-only.

## Non-Claims And Promotion Boundary

Revision 16 does not implement recursive executable witness groups,
compiler-selected sealed placement, package provenance, all-tier sealed
operation execution, or process rehydration. It does not promote the broader
Accepted Contract or Experimental vertical to Current.

Promotion requires a reachable source producer and independent verifier through
verified SSA, validated bytecode, evaluator, VM, forced baseline, and forced
proof execution. The sealed-value vertical additionally requires
witness-derived DAG rehydration, fresh-runtime process import, exact owner
cleanup, zero forced fallback, no-tracing verification, and retained
no-per-node-RC evidence.
