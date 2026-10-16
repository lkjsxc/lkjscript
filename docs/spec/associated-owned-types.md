# Witness-selected associated owned types

Status: selected specification, implemented in the isolated **0.1.95** development
checkout. Integrated-source acceptance, mainline delivery and finalized-byte
acceptance remain pending. [Status](../status.md) owns exact availability and proof.
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
gaining representation access. The maintained literal owner is
[associated-blockread](../../examples/associated-blockread/README.md). Its exact
source-matched final-byte requirement belongs to
[verification](verification.md#associated-owned-blockread-and-final-byte-coverage);
source-only behavior does not attest released executable bytes.

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

The selected native authoring syntax retains the complete inline inventory on
each contract or implementation. Contract clauses use
`(associated-type assoc_type_HEX NAME (bound owned))`; implementation clauses use
`(associated-type assoc_type_HEX TYPE)`. Contract members preserve authored order.
Implementation bindings lower into strict member-identity order, rejecting
duplicates before normalization can erase them. Create and Set both supply the
complete inventory; an empty inventory replaces the old inventory with empty
data. Complete candidate admission decides whether that removal is coherent.

The two authored type forms are
`(associated-contract-member CONTRACT assoc_type_HEX)` and
`(associated-witness-projection SCOPE implparam_HEX CONTRACT assoc_type_HEX)`.
Declaration aliases resolve through existing exact-reference rules; every member
and lexical witness coordinate remains explicit. Native draft/readback retains
those coordinates, stable IDs, complete inventories and unused binding RHS types.
The compact counterparts are `owned.associated-member`,
`owned.associated-binding`, `type.associated-contract-member` and
`type.associated-witness-projection`; their fields belong to the maintained
discovery registry. Parsing a proposal does not establish semantic admission.

Authored intent, owners, types, interfaces, compiler units, bytecode and artifacts
share the single current schema epoch 1. Contract and implementation records
always encode their complete associated inventories inline, including empty
inventories. Associated member and witness-projection types use the ordinary
current type record. Full consumption and canonical re-encoding are required.
Member identities retain the `assoc_type_` domain; all-zero remains reserved.

Exact dependencies bind package, semantic revision and package revision.
Complete admission checks consuming owners and interfaces, supplier owners,
contract/member/witness identities, lexical scopes and every referenced type,
including unused bindings. Discovery and checksums grant no semantic authority.
Every encoded participant must use the current epoch.

Owner, interface and compiler-unit records retain complete headers, declaration
metadata, ordered fields and inline inventories. Type records retain all authority
coordinates above. Current owner 4 MiB, interface 1 MiB, unit 8 MiB, type 1 MiB
and depth/work limits remain. All binding RHS roots enter source/interface
admission and lowering tables. Summaries and compiler keys bind the complete
inventory; inventory-only changes invalidate downstream reuse.

Kernel, interface and compiler readers independently check inventories, roots,
scope and canonical/interface/compiled correspondence. Rehashed inventory loss,
foreign epochs, member/scope swaps, hidden projections, invalid unused bindings
and missing/extra imported objects reject.

Persisted acceptance and a matching epoch are not current semantic proof.
Revalidation rebuilds proof under the current validator without changing accepted
HEAD or canonical meaning. Complete source import independently rebuilds its
witness. Process proof reuse binds the implementation, exact source objects and
used transport selections. The embedded planner and standard artifact undergo
independent artifact admission and source-to-artifact comparison. Maintained
assets are rebuilt through ordinary product owners, without header patching,
proof relabeling, skipped admission or a host-planning fallback.
