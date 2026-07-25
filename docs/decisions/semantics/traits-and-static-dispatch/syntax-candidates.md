# Coherent Traits And Static Dispatch: Syntax Candidates

[Authority](../traits-and-static-dispatch.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

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

In the Current marker slice, every explicit core-trait implementation is
rejected. `Copy`, `Send`, and `Sync` facts are compiler-derived only; `Clone`
and `Drop` bounds are rejected until executable methods/drop elaboration exist.
Audited user `Copy` implementations and executable `Clone`/`Drop` bodies are an
**Accepted Target** for the ownership/method slice, not Current behavior.
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

The first slice is declaration-only nominal marker traits. A `trait/` contains
exactly one `name/`; an `impl/` contains exactly one `trait/` and one `for/`
whose target is an exact monomorphic nominal product. Generic `fn/` declarations
may place one `bounds/` block after `forall/` and before `sig/`; every
`bound/ T TraitName /bound` names a declared type parameter and a resolved
trait, with no duplicate `(parameter, trait)` pair. Imported traits and impls
are declarations; imported execution and imported `main` remain forbidden.

Dense trait identities begin with compiler-owned `Copy`, `Clone`, `Drop`,
`Send`, and `Sync`, followed by source traits in source-closure/declaration
order. Explicit implementation identities follow source-closure/declaration
order. Until packages exist, the exact loaded program closure is the coherence
domain: duplicate `(trait, product)` implementations are rejected independent
of declaration order. Source cannot declare a core trait, implement `Copy`,
`Send`, or `Sync`, or assert auto-trait facts.

The bounded marker solver derives `Copy`, `Send`, and `Sync`. Unit, Bool, I64,
and F64 have all three facts. Str and Symbol are `Copy` within one worker but,
like List, Option, Result, and nominal products, are worker-local GC references
and therefore do not derive `Send` or `Sync`; structurally eligible contained
values still determine their `Copy` fact. Legacy Buf, Handle, and function types have no automatic facts in this slice.
In the Current initial ownership island, `Owned Buf` and `RefMut Buf` derive no
Copy/Send/Sync fact; `Ref Buf` derives Copy but not Send or Sync. All remain
worker-local. Exact repeated product recursion and solver
depth/work exhaustion are deterministic compile errors rather than optimistic
inference. Other user marker bounds require one exact explicit product
implementation.

Bounded generic functions are callable only at concrete direct-call sites and
are not first-class values in this slice; generic-context bound forwarding and
loading one for an indirect call are rejected because an abstract caller-bound
witness is not yet represented. Every generic instantiation whose signature or
substitution directly or transitively contains `Owned`, `Ref`, or `RefMut` is
also rejected by the Current ownership safe island. Resolved concrete generic HIR calls retain canonical
ordered substitutions and one erased witness per bound: either an auto-trait fact or an exact implementation
identity. `Clone` and `Drop` bounds are unavailable until their method/drop
contracts are implemented. Typed SSA retains trait/implementation metadata, signature bounds,
and the same call instantiation and witness identities; verification rejects
unknown, duplicate, mismatched, unbounded type nesting, core-trait assertions,
or non-canonical facts before evaluator,
bytecode, or native consumers can erase marker witnesses. Methods, associated
types/values, generic impls, specialization, dynamic dispatch, blanket impls,
and overlapping impls remain absent and their syntax is rejected rather than
stored inertly.
