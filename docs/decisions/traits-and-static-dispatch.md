# Coherent Traits And Static Dispatch

## Purpose

Define the minimal bounded trait system required by ownership, generic
collections, static method dispatch, and future self-hosting.

## Status

Current annotation-driven generic functions have no trait declarations,
implementation selection, methods, associated types, or auto traits. The system
below is an **Accepted Target** and becomes Current only as implemented syntax,
resolution, HIR/SSA identity, and conformance tests land together.

## Syntax Candidates

Candidate A placed inline bounds and methods in signatures, for example
`T:Clone` and `Clone.clone`. It is compact but adds punctuation parsing,
conflates source names with resolved identity, and makes associated constraints
hard to delimit.

Candidate B uses fully marked forms:

```text
trait/
name/
Sequence
/name
params/
T
/params
associated-type/
name/
Item
/name
/associated-type
method/
name/
next
/name
sig/
SharedRef region-self T
->
Option Item
/sig
/method
/trait

impl/
trait/
Sequence
/trait
for/
Product Cursor
/for
associated-value/
Item
I64
/associated-value
fn/
...
/fn
/impl
```

Generic functions place zero or more marked `bound/` forms inside one
`bounds/` child; each bound contains one exact type, trait identity, and
optional associated-type equalities. Candidate B is **Selected**. Inline colon
syntax is rejected and is not retained as an alias.

## Declarations And Identity

Traits and implementations are top-level declarations and imports remain
declaration-only. A trait has a package/module-qualified nominal identity,
bounded type parameters, method signatures, and associated types. An
implementation records the resolved trait identity, exact implementing type
pattern, generic parameters/bounds, associated values, and method functions.

Trait resolution completes before ordinary SSA lowering. HIR calls retain the
selected implementation and method identity. SSA retains one canonical witness
or monomorphized-instantiation identity. The JIT consumes those identities and
never dispatches from source strings.

## Coherence

There is at most one applicable implementation for a fully substituted trait
and type. Initial coherence uses an orphan rule: an implementation is legal
only in the package that defines the trait or the outer nominal implementing
type. Structural built-in types are owned by the core package. Overlapping
implementations are rejected even if declaration order would choose one.

Specialization, negative implementations, overlapping instances, fallback
instances, and arbitrary implicit conversions are absent.

## Solver

The solver is deterministic over stable declaration identities. Configuration
bounds query depth, candidate inspections, generated obligations,
normalization steps, and cycle length. Exact repeated obligations are memoized.
A productive auto-trait structural recursion is handled by its dedicated rule;
other cycles produce a deterministic diagnostic. Budget exhaustion is a compile
error, not “no implementation”.

Method lookup starts from the receiver's exact type and in-scope trait
identities. It does not search arbitrary conversions. Associated types
normalize only through the selected implementation and are checked against
bounds before lowering.

## Core Traits

The compiler reserves exact roles for:

- `Copy`: permits implicit bitwise value copy; incompatible with `Drop`;
- `Clone`: explicit value duplication method;
- `Drop`: deterministic exactly-once destruction method/marker contract;
- `Send`: value may move between workers;
- `Sync`: shared reference may cross workers safely;
- later value equality, ordering, hashing, iteration, and futures traits.

`Send` and `Sync` are compiler-derived auto traits. Ordinary source cannot
assert them. Derivation inspects all product fields, future enum payloads,
closure captures, lexical references, worker-local GC references, pinned
values, synchronized wrappers, unique containers, and trait bounds. A
worker-local GC reference is neither `Send` nor `Sync`; an exclusive reference
is not `Sync`; immutable shared bytes may be both under their audited contract.

User `Copy` implementations are accepted only when every stored component is
`Copy`, no `Drop` implementation exists, and the compiler layout rule approves
the type. `Clone` and `Drop` bodies are ordinary statically dispatched
functions subject to effects and ownership checking.

## Native Dispatch And Monomorphization

Static dispatch is the default. Resolved generic calls are monomorphized for
native hot code under explicit per-function, per-group, total code-byte, depth,
and work limits. Equal canonical substitutions share one instantiation. A
budget failure remains an explicit unsupported compilation result; it cannot
silently reinterpret a generic operation.

Proven method calls lower to direct calls and may be inlined by the optimizing
tier. General dynamic trait objects and vtables are not implemented in this
cycle.

## Initial Coherent Slice

The first slice implements nominal marker traits, explicit non-overlapping
implementations, generic bounds, deterministic selection, resolved witness
identity, bounded solving/cycle diagnostics, and compiler-derived `Send`/`Sync`
for current scalar and nominal product types. Methods and associated types land
in the next coherent slice before collection/future abstractions require them.
Until then, syntax that declares a method or associated type is rejected rather
than stored inertly.

## Deferred And Rejected

Dynamic trait objects, specialization, higher-kinded types, overlapping
instances, negative bounds, source-asserted `Send`/`Sync`, and unbounded solving
are **Deferred** or **Rejected**. An inert accepted declaration, declaration-
order dispatch, and backend source-name interpretation are **Rejected**.
