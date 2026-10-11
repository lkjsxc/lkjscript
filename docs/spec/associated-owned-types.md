# Witness-selected associated owned types

Status: selected specification; **implementation and acceptance pending**.
This specifies the next language increment, not a capability of an accepted
executable. [Status](../status.md) owns implementation availability and acceptance.
Existing [owned contracts](owned-contract-parameters.md),
[implementation schemes](generic-owned-implementations.md) and
[borrowed results](owned-read-results.md) retain their rules except where extended
below. The accepted meaning graph remains the sole program authority.

## Members, bindings and exact selection

An OwnedContract declares an inline, bounded, authored-order inventory of
associated type members. Each entry has a stable AssociatedTypeMemberId, a distinct
name and exactly the Owned bound. Member authority is the pair of its exact
package-qualified contract reference and member identity. A name, ordinal or naked
member identity cannot select it. Members are not separately owned declarations.

An OwnedImplementation retains one logical payload and adds an inline binding
inventory, strictly ordered by member identity. It supplies exactly one type
binding for every member of its exact contract: missing, extra, duplicate or
foreign members reject. All bindings are admitted, including unused ones.
Inventories contain at most 100,000 entries within existing object and proof limits.

Stored binding templates use only the implementation's own exact type parameters
and existing finite admitted Owned forms. Neither new associated type form may
occur anywhere in a stored RHS closure, including nested and phantom actuals.
Existing ordinary parameters may occur inside an Owned carrier only where their
independent category and bounds permit. No defaults, implementation search,
associated equations, type destructors or higher-kinded families are introduced.

The two associated type forms have distinct authority:

| Form | Meaning and legal context |
| --- | --- |
| AssociatedContractMember(contract, member) | Abstract member in a method signature type closure of that exact containing contract, including nested constructors. |
| AssociatedWitnessProjection(scope, parameter, contract, member) | Member selected by an exact lexical implementation parameter in its defining function or OwnedImplementation scope. |

A contract-local member cannot remain in an ordinary signature or stored binding.
A lexical projection checks the complete in-scope implementation inventory,
parameter identity, exact parameter contract and that contract's member inventory.
Referencing another declaration does not import its lexical parameters. External
declarations have no such inventory and cannot define a lexical projection scope;
their codecs are not widened. Every type occurrence is checked in its actual
context, even when the same digest was already admitted elsewhere.

Implementation matching and method calls substitute contract Self, ordered
arguments and members using the exact admitted implementation operand. Lexical
selection produces the corresponding lexical projection. Concrete selection reads
the admitted binding in the implementation's defining package and substitutes
that scheme's own formals simultaneously. Exact parameter order, type, use mode,
callable kind, effect row and borrowed-result source remain mandatory.

Callable substitution rebinds callee projections to actual caller operands without
capturing similarly named or numbered parameters. A concrete binding applied to
caller actuals may produce a finite symbolic type containing caller projections;
this does not violate the stored-template restriction. Witness substitution runs
even when ordinary type bindings are empty. Materialized types retain their exact
objects, not just newly derived digests.

Equal Self, contract or normalized member output does not make witnesses
interchangeable. Closed preparation and normalization retain the implementation
declaration, ordered actual types and complete ordered prerequisite applications,
including unused and phantom actuals. Caches are scoped to or keyed by this complete
environment. A projection grants no field/layout visibility. Owned classification
is proved independently from exact member authority; it does not imply ordinary
data, Transferable, Shareable or capture safety.

## Finite projected applications

First-order projections are permitted in callable and applied-witness type actuals
subject to complete category, scope and finite-preparation admission. There is no
global ban on projected actuals. Existing prerequisite-tree construction rules
first establish the finite selection-context domain; projected type actuals do not
widen those construction rules.

The selected growth proof extends each finite callable context with ordinary formal
slots, selected-scheme formal slots at exact witness/prerequisite paths, and member
output slots qualified by exact contract/member and path. Identical selection
shapes at different roots or paths do not merge actuals or slots. Unresolved
witness anchors retain full lexical scope, parameter identity and contract/member
authority; forwarding the same immutable witness connects anchors without growth.

Projection is a transparent alias. A binding adds edges only from formals actually
occurring in its RHS. Type constructors inside a binding and outside a projection
both contribute positive expansion; diagnostic provenance is not a constructor.

| Binding and recursive use | Required outcome |
| --- | --- |
| Out = X, reconstructing the scheme with its selected Out | Zero-weight alias recurrence is finite. |
| Left = X, Right = Y, swapping selected outputs | Zero-weight permutation is finite; ordered witness identities remain distinct. |
| Out = Cell, reconstructing with Box<Out> | Constant reset is finite: no X-to-Out dependency exists. |
| Out = Box<X>, reconstructing with Out | Positive recurrence rejects, even when Self and witness shape remain fixed. |

Demand scans every effective callable and witness actual tree on recursive context
edges, including nested prerequisites, phantom arguments, method mappings,
function values, untaken syntax and contexts with no ordinary slots. Requested
members induce precisely their binding dependencies. A positive edge within a slot
strongly connected component rejects expanding preparation. Zero cycles and
constant resets are not rejected merely because they recurse. Complete admission
also checks all acyclic actuals and unused bindings outside growth demand.

Production adoption requires an independent term/binding/witness interpretation
and growth oracle, plus a separate eager all-path comparison for demand
completeness. A finite execution prefix or production/reference agreement alone
does not prove closure. If exact immutable-anchor forwarding is unproved at a
recursive position, that position remains explicitly unsupported; valid acyclic
projected applications remain permitted. Capacity exhaustion is Resource, not
evidence of invalid meaning. Reserve slots, edges, paths, caches and derived types
before growth and preserve cancellation and healthy retry. This proof concerns
finite preparation, not termination of recursive program execution.

## Borrowing and maintained BlockRead adoption

Associated types add no lifetime family or runtime borrowing mode. A pure method
may return its associated Owned type with the existing ReadFrom parameter-position
relationship. Every successful path returns the selected borrowed input or a real
owned descendant. Equal types or allocations cannot substitute another input's
provenance; manufacturing a cursor or wrapper containing a reference is excluded.

Existing lexical loan protection, argument order, read-only reborrowing and joined
cleanup remain mandatory. Views cannot become owners, captures, container members,
serialized values or retained references. Tasks and externals cannot return loans.
Synchronous borrowed calls require the existing Owned contract; joined child reads
require separate Shareable admission. No transfer/share/capture bound is inferred
from an associated member. Source and ancestor custody survives traps,
cancellation, quota refusal and result-transfer failure until loans are discharged.

The public vertical witness is an imported BlockRead<Self: Owned, Item:
Transferable> contract with associated Block: Owned. Its pure methods expose block
count, block-at(index, Self borrow), block length and block-get over Block borrow.
Block-at returns Block with ReadFrom its Self parameter; block-get returns admitted
ordinary elements. The generic traversal is authored before its independent
structural and nominal suppliers and needs no separate Block type argument.

Flat exposes its complete owned-sequence Item as one block. Chunked actually owns
nominal ChunkBlock<Item> descendants; their declaration may be public while field
layout remains module-private. Borrowing selects existing descendants rather than
manufacturing reference wrappers. Generic consumers use reader methods without
gaining representation access.

Maintained adoption replaces repeated element reads in the dependency planner's
stage materialization with scoped block traversal, preserving ascending global
positions, omission of zero levels and complete results. Random reads/updates keep
their existing contracts. Forward explicit BlockRead witnesses alongside integer
storage witnesses; equal Self does not infer selection. Maintain all four
flat/chunked combinations and rebuild the production native planner through public
author/import/build/export operations. Its Flat/Flat production selection must
actually consume this traversal. Source-detached execution independently agrees;
the language feature alone makes no speed claim.

A materializer may forward its exact Reader prerequisite using existing mapping
rules. A generic DelegatingReader binding its own Block to Reader.Block is outside
this increment because stored RHS projections are forbidden. Prerequisite Self
remains an exact in-scope Owned type parameter; Observer<Self = Reader.Block> is
not enabled. General regions, lifetime inference, alternative borrow sources,
escaping composite views and new recursive nominal Owned layouts are also excluded.

## Canonical capability and independent admission

The separate reader capability is lkjscript-owned-associated-types-1, version 1.
Its supported profile combines the frozen Graph33, Interface21, CompilerUnit33,
Bytecode27, Artifact40 and Manifest3 base with the added object contracts below.
Advertisement is discovery, not proof. Every owner/interface consumer of either
new type form requires Graph33 under complete closure admission, including nested
and unused digest references; a newer referenced object does not upgrade its owner.

| Object | Extension magic | Checksum domain |
| --- | --- | --- |
| Parent contract/implementation | LKJOAT01 | lkjscript.kernel.owned-associated-owner-envelope.v1 |
| Associated type | LKJTAT01 | lkjscript.kernel.owned-associated-type-envelope.v1 |
| Public interface parent | LKJPAT01 | lkjscript.package-interface-associated-owner-envelope.v1 |
| Compiler unit parent | LKJCAU01 | lkjscript.compiler-unit-associated-envelope.v1 |

Each uses packed envelope version 1 and a canonical extension body beginning with
u16 version 1 and an explicit u8 tag: 1 contract, 2 implementation; type tags are
1 contract member, 2 lexical projection. Parent bodies retain exact header/module/
name/visibility, frozen base fields and inventory; interface bodies retain base
version21, header/name, base fields and inventory; unit bodies retain the current
prefix through optimization, exact tables, base fields and inventory. Type bodies
retain the complete authority coordinates shown above. Full consumption and exact
canonical re-encoding are required. Existing enum ordinals remain unchanged;
TypeForm appends member25/projection26 with form generation1. Member ids use
assoc_type_, allocation domain owned_associated_type and binary domain tag23;
all-zero remains reserved.

One logical contract/implementation model serves both wire paths. Empty parent
inventories use exact existing base bytes only; nonempty inventories use extension
bytes only. Frozen base codecs and historical converters reject nonempty inventory
rather than erase it. New type forms never use old type envelopes. Ordinary
function owners/interfaces/units retain their existing shape when their signatures
reference new type digests. No opcode, artifact flag or object domain is added.
Current owner4MiB/interface1MiB/unit8MiB/type1MiB and depth/work limits remain.

All binding RHS roots enter source/interface closure admission and lowering tables,
including unused bindings. New type references retain exact contract relations and
lexical scope relations without inventing child owners or dependencies. Nonempty
summary material binds the full extension body and implementation bindings;
empty base dimensions and ownership aggregation remain byte-identical. Associated
parent unit keys use lkjscript.compiler-unit-associated-key.v1 over version/tag
and the existing key core. Select from canonical inventory before lowering and
recheck from full unit payload; ordinary keys retain their base derivation.

Kernel, public interface and compiler readers independently check inventories,
roots, scope and exact canonical/interface/compiled correspondence. Consistently
rehashed inventory loss, wrong tags/versions, empty extensions, member/scope swaps,
stored hidden projections, invalid unused bindings and missing/extra imported
objects reject. Inventory-only changes invalidate the appropriate summaries and
downstream reuse. Integrity hashes cannot authorize unsupported semantics.

Validator39 adds witness_selected_owned_associated_types version1 while retaining
Witness9/Summary11 encoding; old validator seals are not current proof. Public
authoring/discovery selects compact41/authored38. The unchanged genuine embedded
planner must first load through independent artifact admission, then support
ordinary public rebuilding of feature-bearing assets. No header patch, proof
relabeling, skipped admission or host-planning fallback is allowed. Feature-free
base bytes and builtin-standard source-to-artifact equality remain mandatory.
