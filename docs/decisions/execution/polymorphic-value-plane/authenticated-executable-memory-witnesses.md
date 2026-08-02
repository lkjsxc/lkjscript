# Authenticated Executable Memory Witnesses

## Status

**Accepted Contract; not Current.** This record binds the executable witness
cutover. Promotion requires independent HIR reconstruction, SSA preservation,
bytecode identity recomputation, exact structural relinking, malformed-artifact
coverage, and package provenance. Existing residual transport remains
Experimental until those boundaries pass.

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

One contracts-owned canonical encoder consumes:

- semantic type and semantic contract identities;
- aggregate mode, permitted executable domain, and root projection;
- copy/share, drop, equality, codec, and list-element routes;
- checked size, alignment, borrow, and dynamic-owner facts;
- portability and contention;
- sorted closed executable operation identities; and
- ordered child executable witness identities.

The encoder uses an exact domain separator, fixed tags, big-endian fixed-width
integers, explicit option and sequence lengths, and declaration-order fields.
It never uses debug text, serde output, host enum layout, pointers, or implicit
discriminants. Changing one executable fact, operation, or ordered dependency
changes the witness ID.

HIR-only derivation evidence remains committed by `MemoryPlanId`; it is not
duplicated as runtime policy. The executable identity does not include
`MemoryPlanId`, because the plan already contains witness identities and would
form an identity cycle.

## Producer Projection

The HIR producer projects canonical executable facts before SSA and computes
the witness ID from that projection. Child identities are resolved from the
already-derived exact type dependency order. The independent HIR verifier
reconstructs HIR facts, repeats the executable projection, recomputes the ID,
and rejects any mismatch.

SSA copies the verified projection. It does not derive additional operations or
replace the identity.

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
3. ordered, duplicate-free, complete dependency resolution;
4. an acyclic dependency graph;
5. exact executable ID recomputation;
6. operation and physical-route compatibility;
7. exact structural type, layout, mode, and representation relinking; and
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

## Promotion Boundary

This contract becomes Current only after a reachable source producer and its
independent HIR verifier feed the authenticated projection through verified SSA,
validated bytecode, evaluator, VM, forced baseline, and forced proof execution.
The selected sealed-value vertical additionally requires witness-derived DAG
rehydration, fresh-runtime process import, exact owner cleanup, zero forced
fallback, no-tracing verification, and retained no-per-node-RC evidence.
