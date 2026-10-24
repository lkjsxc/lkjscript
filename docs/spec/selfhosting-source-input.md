# Canonical source carrier and reference closure

This is a bounded source-admission slice toward the
[semantic self-hosting contract](selfhosting.md). It carries current canonical
source objects, logical directories and a claimed semantic-state identity.
It contains no normalized instructions, inferred types, reference summaries,
validation certificates, compiled units or executable templates.

`source-graph::validate` admits the carrier, exact object bindings and logical
state. `source-links::validate` additionally resolves every supported owner and
type reference, including unused owners and untaken expression operands. Success
does not establish lexical scope, unique syntax ownership, acyclicity, expression
typing, affine custody, effects or witness validity. The additional
`source-scope::validate` pass establishes the bounded ownership and value-local
scope contract below. `source-namespace::validate`, `source-type-scope::validate`
and `source-nominal::validate` successively add namespace uniqueness, contextual
type/R/E binder scope, structural type DAG proof and nominal member parameter flow
over the same decoded profile. `source-scalar::validate` adds whole-inventory
typing for the restricted scalar profile below, and `source-elf::compile` lowers
that profile to ELF. Full-profile normalization and the Tier 2 compiler-source
fixed point remain separate work.

The current carrier has one package and no dependencies or external blobs.
Recognized unsupported boundaries reject explicitly; importing a package's
interface or prior validation witness cannot supply its missing source bodies.
This carrier is separate from normalized LKJCI1: mode 0 uses profile 4 for ELF
and mode 1 uses profile 5 for canonical artifacts. The existing Tier 1 paths use
Rust admission and normalization. The scalar source route instead derives its
mode-0 input inside lkjscript from the raw carrier.

## Exact bytes

All scalar header fields and span fields are u32 little-endian. Complete input
is at least 128 bytes, and its actual length must equal the encoded u32 length.
There is no separate aggregate input-byte quota. The sum of owner, type and
retirement counts is at most 100,000. Empty directories are allowed. Package
identity is nonzero and the package name has 1 through 128 canonical name bytes.

| Header offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | Magic `LKJSG001` |
| 8 | 4 | Complete actual byte length L |
| 12 | 4 | Owner count O |
| 16 | 4 | Type count T |
| 20 | 4 | Retirement count R |
| 24 | 4 | Package-name byte length |
| 28 | 4 | Dependency count, exactly zero in this slice |
| 32 | 16 | Package identity |
| 48 | 32 | Claimed current semantic-state digest |
| 80 | 4 | Owner-directory offset |
| 84 | 4 | Type-directory offset |
| 88 | 4 | Retirement-directory offset |
| 92 | 4 | Object-span directory offset |
| 96 | 4 | First canonical-object byte offset |
| 100 | 4 | Traversal allowance |
| 104 | 24 | Zero |

The name starts at offset 128. The three directories follow immediately, in
owner/type/retirement order. Their row formats are:

| Directory | Row width | Fields |
| --- | --- | --- |
| Owners | 50 | Current 17-byte OwnerKey, current 33-byte OwnerBinding |
| Types | 32 | Raw TypeObjectDigest |
| Retirements | 49 | Current 17-byte OwnerKey, raw RetirementObjectDigest |

Each directory is strictly increasing in its complete key. Duplicate keys and
reversed keys reject. The span directory follows these rows and contains O+T+R
pairs `(absolute_offset, byte_length)`. Its entries correspond to owners, types
and retirements in that order. Objects follow immediately in span order. Every
span must begin at the preceding span's end and the final span must end at L.
There is no padding, overlap, gap, aliasing or trailing data.

Traversal allowance must be at least
`4096 + 128*L + 1024*(O+T+R)` and at most 2,147,483,647. It is an input admission
condition distinct from VM instruction/allocation quotas and native process
limits. The minimum admits the carrier; it does not guarantee that every later
proof finishes within that allowance. A caller may select a larger allowance
within the same maximum. Later proof exhaustion returns 6 without a semantic
validity claim. Bounds precede products, object access and hashing. Lookup uses binary
search over admitted immutable directories; it does not copy the complete source
or linearly rescan the directory for each reference.

The retained allowance representation also bounds the admitted byte domain.
With N = O+T+R, admission requires
`L <= floor((2,147,483,647 - 4096 - 1024*N) / 128) = 16,777,183 - 8*N`.
Retained layout and object constraints can tighten this allowance-derived upper
bound. It is below the u32 wire domain. Removing the former aggregate 8 MiB check
does not remove the count, per-object, per-field or allowance restrictions.
Individual object readers still require their current envelopes and payload
bounds. Logical owner and retirement maps receive only their directory slices;
the retained 100,000-entry bound limits those slices to 5,000,000 and 4,900,000
bytes respectively, below the map reader's independent byte limit.

The complete-length comparison precedes allowance multiplication, so even a
malformed u32 length times 128 fits I64. Index windows check bounded counts and
strides, then compare against the actual backing length using subtraction after
nonnegative and ordered-range checks. Wider backing storage does not grant an
invalid window access. Carrier/object scans advance through finite byte or
record ranges; directory searches strictly shrink their intervals. Existing
visited-state, scope and work checks remain unchanged.

## Binding and references

Each object must pass its current envelope/shape reader and independently derived
object digest. Owner and retirement records must agree with their directory's
exact identity and kind. A live owner cannot simultaneously be retired.

The compiler recomputes the logical owner and retirement map roots, the empty
dependency root, and the semantic-state digest from the actual package identity,
name and complete logical rows. It compares that digest with the header's claimed
state. Repository identity and physical map page locators do not enter this
semantic-state commitment. A digest match supplies no historical validation
certificate or execution authority.

The type directory contains content-addressed objects; its complete membership is
not a separate input to the kernel semantic-state digest. All supplied types are
still shape-checked, bound to their own digests and reference-checked. Adding a
valid unused type may therefore preserve the semantic-state digest while changing
the carrier bytes. The test receipt records a separate SHA-256 for complete
carrier identity; neither identity substitutes for validation. Reuse across inputs
must bind the complete carrier, not only the semantic-state digest; an additional
unused type can change reference closure without changing that state.

The reference pass reads original canonical record fields. Local typed identities
resolve in the exact owner directory and type digests in the exact type directory.
Package-qualified references must name the supplied package; a foreign package
cannot resolve by coincidentally matching a local identity. All included records
are visited, without selecting a root function or dropping unreachable syntax.

Retirement history fields are historical observations: their previous parent or
revision need not be a live source owner. Their canonical shape, directory
binding and separation from live owners are still checked.

| Diagnostic | Meaning |
| --- | --- |
| 0 | The selected carrier/reference checks succeeded |
| 6 | Traversal allowance outside the admitted bounds |
| 12 | Malformed carrier, object encoding, ordering, binding or state commitment |
| 13 | Recognized unsupported source form or package/blob boundary |
| 14 | Missing or incompatible reference endpoint, or live/retired identity collision |

Binary-filter test entrypoints return the independently recomputed state
commitment on success and no bytes on rejection. Those entrypoints inspect source;
they do not compile or publish it. Tests serialize canonical objects without a
semantic preprocessing pass, compare with independent current codecs and logical
state expectations, and run on VM, reference and standalone native execution.

## Interface, requirement and effect owner extension

The current E1 extension admits Interface declarations, Operation records and
Operation-owned Parameters, Component Requirements, and Function-owned
RequirementParameter and EffectParameter records. These names refer to separate
canonical discriminants; record tags, owner domains and namespace classes are
not interchangeable.

| Record | Canonical record tag | Directory OwnerKey domain | Directory owner kind | Namespace class |
| --- | ---: | ---: | ---: | ---: |
| Operation | 6 | 6 | 14 | 6 |
| Requirement | 10 | 10 | 18 | 8 |
| EffectParameter | 3 | 16 | 24 | 11 |
| RequirementParameter | 16 | 17 | 25 | 12 |

Directory domains and kinds use `IdentityKind::tag` and `OwnerKind::tag`.
The canonical owner envelope separately encodes `OwnerHeader` enum variants;
the readers validate both representations.

Interface uses declaration-kind byte 3, declaration payload 2, and directory
owner kind 4. Function/Operation Parameter parents use parent variants 0/1
and owner domains 2/6 respectively. The current codecs remain the wire authority.

Shape admission checks canonical names, arrays and enum discriminants. Operation
records carry ordered Parameters, a result type, idempotency and visibility.
Requirement constraints carry an Interface reference and a strictly increasing,
duplicate-free operation-reference list; each operation reference has the same
package identity as its Interface. Component Requirement limits have at most
1,024 entries, strictly increasing names, nonzero full-width u64 magnitudes and
current unit tags 0 through 4. Values above I64::MAX are valid u64 magnitudes.

Functions admit ordered requirement/effect binder arrays and pure/task effect
encodings. TaskFunction descriptors use type form 18. Each effect row contains
a requirement-operand array followed by an effect-parameter array; each array
is strictly increasing and duplicate-free under current canonical byte order.
The combined atom count is at most 100,000. A requirement operand distinguishes
a concrete Requirement from a RequirementParameter. Package-qualified references
retain both package and member identities.

Reference admission resolves every included endpoint, including unused type rows.
Operations require Interface parents; Requirements require Component parents;
R/E binders require Function parents. Every constrained operation must belong to
the exact referenced Interface. Reverse-parent and forward-membership agreement
is then checked by syntax admission, followed by namespace and contextual scope.
Malformed shape returns 12. Foreign package references retain the unsupported
package boundary 13; missing or incompatible local endpoints return 14. Invalid
containment returns 15, duplicate namespace keys return 16, and contextual binder
failure returns 17. Resource exhaustion remains 6.

Nonempty R/E application arguments on Call and FunctionValue remain unsupported
with 13. Implementation witnesses, associated owned types, nontrivial result
relations, read-scope/read-source metadata, Match and lexical Region forms remain
outside this extension. It does not implement E2 effect compatibility,
substitution, requirement satisfaction or source-to-normalized lowering.

## Syntax membership and exact value-local scope

After complete carrier and endpoint admission, `source-scope::validate` derives a
private table of 12-byte rows: child owner-directory index, parent owner-directory
index, and ordered membership position, all u32 little-endian. No derived table is
accepted from the producer. It is not a persisted schema or a validation witness.

Modules and targets have no containment parent. Declarations belong to their exact
module. Declared type parameters, fields, cases, callable parameters and component
ports and requirements have exactly one listed container, agreeing with their reverse
parent field. Interface declarations own Operations; Operations own their ordered
Parameters. Function declarations own their ordered requirement and effect binders.
Each expression has one owning expression, binding initializer, declaration root,
or expression-valued port. Each let binding belongs to one let, has the ordinary
let kind and owns its required initializer. Module membership is obtained from
canonical declaration fields; modules do not encode a separate member array.

The pass extracts every supported containment edge, sorts rows deterministically
by child index, and requires the complete sorted child inventory to be exactly the
owner directory. Duplicates and omissions reject, including unused and disconnected
syntax. Content-addressed type descriptors are shared and never receive this
syntax-parent uniqueness rule. Calls, function values and nominal references are
reference edges, not containment edges: direct/mutual calls and nominal recursive
references are not rejected as syntax cycles.

Repeated parent-pointer doubling proves that every admitted owner chain terminates
at a designated root. A disconnected cycle cannot hide behind valid incoming counts.
The algorithm uses logarithmically many flat passes, without recursively following
an input-controlled syntax stack. Intermediate tables remain derived Bytes values;
this is not native lexical region reclamation.

Every local expression is checked, including untaken branches, uncalled definitions
and final array items. Function-parameter references must reach their exact callable
container; ordinary lexical-binding references must reach their containing let.
The encoded reference variant must match that binder category. An operation-parameter,
match-payload or transaction-binding reference to an otherwise admitted function
parameter or let binding is a proved semantic mismatch. Genuine excluded owner
forms encounter the preceding unsupported-profile boundary instead.

The immediate child on the ancestry path determines visibility: initializer i may
refer only to bindings with smaller positions; the body follows all bindings.
Nested or sequential equal names are legal and never replace a committed binder
identity with a nearest-name lookup. Custody validation remains a later obligation.

`source-scope::with-work(graph, work)` selects an additional ancestry-proof allowance
in 0 through 2,147,483,647. Complete carrier/endpoint/membership admission precedes
that allowance check. The default entrypoint uses the carrier's admitted allowance.
Each doubling or final-root pass reserves O units before visiting rows; the value-
local scan reserves one unit per owner and one per visited ancestor. Exact fit
passes. Refusal returns diagnostic 6 without a semantic-invalid claim or output.
Carrier bounds independently bound decoding, edge construction and sorting;
these work units are not VM allocation charges, native allocator requests or RSS.

| Diagnostic | Additional ownership/scope meaning |
| --- | --- |
| 0 | The specified ownership, containment and value-local scope checks succeeded |
| 6 | The additional ancestry-proof allowance was refused or exhausted |
| 15 | Invalid containment membership, reverse parent, syntax cycle or value-local scope |

Diagnostics 12, 13 and 14 retain their preceding carrier/endpoint meanings.
Native filters preserve distinct numeric diagnostics while using their common
rejection exit status 65 and emitting no partial image.

The syntax pass alone remains a restricted E1 layer: a well-owned, lexically valid
but ill-typed graph can pass it. The additional checks below preserve that layer's
entrypoints and diagnostics rather than treating syntax success as complete
semantic admission.

## Namespace uniqueness

`source-namespace::validate` first establishes the syntax contract, then enforces
uniqueness of `(parent identity, namespace class, canonical name)`. The class
projection follows all 12 kernel namespace classes with these parent rules:

| Classes | Parent | Current whole-carrier support |
| --- | --- | --- |
| Module, Target | None | Both supported |
| Declaration | Exact Module | Supported |
| TypeParameter, Field, Case, Port | Exact Declaration | All supported |
| Parameter | Exact Function Declaration or Operation | Both parent forms supported |
| EffectParameter, RequirementParameter | Exact Function Declaration | Supported |
| Operation | Exact Interface Declaration | Supported |
| Requirement | Exact Component Declaration | Supported |

Binding, Expression, HttpRoute, Documentation and Annotation do not participate.
Equal names across different parents or classes remain legal. Local binding
shadowing retains exact binder references and is not package namespace collision.

The pass derives 16-byte rows containing parent owner-directory index, namespace
class, absolute name offset and name length. The owner count denotes no parent;
directory indexes preserve complete OwnerKey identities, including their domains.
Parent indexes come from the admitted 12-byte containment table for this exact
carrier, including the root sentinel. Public validation entrypoints prepare that
table themselves; it is neither a producer certificate nor a cross-carrier cache.
Deterministic merge sort compares parent, class and original canonical name bytes.
Adjacent equal keys reject with 16. The table is private derived state; names are
not replaced with hashes or copied into independently authoritative keys.

Whole-carrier collision controls cover all 12 namespace classes after valid shape,
reference and containment admission. Independent parent projections cover both
Parameter parent domains and excluded owner forms. The exact admitted containment
parent is reused for Operation-owned Parameters as well as Function Parameters.

## Structural type DAG and contextual type scope

`source-type-scope::validate` additionally derives structural child edges from every
supplied type descriptor, including unused descriptors. Structural field types,
List items, pure/task function parameters/results and Applied arguments are child edges.
Named nominal declaration references are not structural backedges. All endpoints
must already exist in the admitted type directory. Repeatedly settling descriptors
whose children have known heights establishes acyclicity and the maximum structural
depth of 256 edges. Unsettled descriptors with no progress identify a cycle.

Every supported owner carrying type roots is checked in its enclosing semantic
declaration context. Traversal follows all child types; a TypeParameter descriptor
must name a parameter belonging to that declaration. A shared digest may therefore
be valid in one using declaration and invalid in another. Visited-descriptor state
is local to a using context, with no digest-only global scope cache.

Every direct task row and every nested TaskFunction row used by an owner checks
requirement-parameter and effect-parameter references against that exact enclosing
Function declaration. Concrete component Requirements are resolved endpoints, not
function-local binders. Function results, Function and Operation parameters,
Operation results, structural fields and expression type roots participate,
including untaken branches. Empty task rows remain task rows. These are lexical
binder checks, not E2 effect compatibility or requirement satisfaction.

All supplied descriptors receive shape, endpoint and structural checks. Contextual
binder checks apply at actual owner type roots. A structurally valid unused
TaskFunction descriptor combining binders from different functions is therefore
permitted until an owner uses it in an invalid context. Any later cache must bind
the complete carrier and using binder context, including method, read-scope and
witness identities when those forms are admitted.

Type-object digests commit to their child digests, so rehashing ordinary objects
cannot create a self-consistent structural cycle. Independent derived graph fixtures
exercise cycle proof and the exact depth boundary directly. Raw-carrier tests
exercise acyclic structure and context-sensitive type-parameter mutations; direct
cycle tests must not be reported as end-to-end hash-consistent cyclic carrier
coverage. Raw TaskFunction boundary controls alternate parameter and result edges
at depths 256 and 257, with a fixed explicitly selected allowance within the
existing range. Namespace success precedes the structural outcomes 0 and 17.
A smaller allowance returning 6 is resource evidence, not a depth-semantic result.
Structural cycle or depth failure is code 17.

## Nominal member parameter flow

`source-nominal::validate` adds non-expansive nominal flow after the preceding checks.
It scans every included nominal field and case payload, including inactive members,
and traverses nested applications, function signatures and phantom type arguments.
Applications encountered through these member payloads must supply the exact ordered
type-parameter arity. This arity check does not cover applications appearing only in
function signatures/bodies or otherwise unused descriptors; general application
typing remains a later obligation.

For each source-parameter occurrence in a nominal application's transmitted argument,
derive an edge from that exact source slot to the target declaration's exact slot.
The edge is plain when the entire argument is the parameter, and expanding when a
constructor surrounds the occurrence inside that argument. An expanding edge with
a return path rejects: equivalently, no cyclic strongly connected component may
contain an expanding edge. `tree<T> = leaf(T) or branch(List<tree<T>>)` is permitted
because the transmitted argument is T; `grow<T> = stop or next(grow<List<T>>)` is
rejected because its argument expands T on a cycle. Permutation, duplication,
selection, closed replacement and expansion along acyclic slot paths remain legal.
Member-application arity or expanding-cycle failure is code 18. Callable generic
finite-flow is a separate later semantic check.

## Composed proof work and remaining boundaries

The composed entrypoints prepare syntax membership once and pass a shared remaining
allowance through scope, namespace, type structure/context and nominal checks.
Internal `check` functions return a nonnegative remainder or a negative diagnostic;
public wrappers return zero or the positive diagnostic. Later passes do not reset
the allowance. Work is reserved before its covered traversal or table growth.
Namespace work includes owner collection, sort nodes, merge rows, comparisons and
bounded name-prefix inspection. Type and nominal work includes roots, child edges,
nodes and membership scans. Every direct or nested effect-row atom reserves one
unit before its scoped lookup; the remaining allowance flows into child traversal.
Exact fit succeeds; exhaustion remains diagnostic 6.
These units are distinct from evaluator quotas, cumulative allocation and RSS.

| Diagnostic | Additional E1 meaning |
| --- | --- |
| 6 | Additional proof-work allowance invalid or exhausted; no semantic-invalid claim |
| 16 | Duplicate exact namespace key |
| 17 | Contextual type/R/E binder scope, structural cycle or structural-depth failure |
| 18 | Nominal member-application arity or expanding cyclic parameter-slot flow |

Earlier diagnostics 12 through 15 remain unchanged. Layer-reaching negative carriers
must rebind framing, objects, directories and state and pass preceding validators.
Tests compare exact Rust kernel diagnostics and independent expectations across VM,
canonical reference, Rust-native and selfhost-native execution. Standalone native
source validation still uses a Rust-bootstrapped normalized validator; it is not
source-to-normalized self-compilation. Complete carrier SHA-256 and semantic-state
identity remain separate evidence.

These layers establish restricted-profile E1 admission. The scalar extension below
adds a separate E2/E3 path. Broader source forms, full contextual read/witness
scope and effect applications, general application typing, callable generic
finite-flow, custody, witnesses and full-profile normalization remain required.
Regions, implementation calls and other excluded forms still reject with 13.
The full Tier 2 profile, compiler-source ELF/artifact fixed points and measured
native phase reclamation remain unchanged.

## Scalar source admission and ELF lowering

`source-scalar::prepare` composes carrier/reference, containment, lexical scope,
namespace, structural/contextual type and nominal checks before scalar profile
admission and expression typing. `source-scalar::validate` returns the resulting
diagnostic. These E2 entry points inspect raw complete `LKJSG001` carriers.
`source-elf::compile` consumes an explicit source request, additionally lowers
the selected admitted target and returns `{diagnostic: I64, image: Bytes}`; the
`compile-source-elf` command target exposes it as a native binary filter.
Success returns ELF bytes. Failure returns the diagnostic with an empty image.

### Explicit source request

The current source compilation request is `LKJSRQ01`, profile 1. Its 32-byte
header precedes the exact complete `LKJSG001` carrier:

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | ASCII `LKJSRQ01` |
| 8 | 4 | Little-endian u32 profile, exactly 1 |
| 12 | 17 | Canonical encoded Target owner key: domain tag 12 and 16 identity bytes |
| 29 | 3 | Zero reserved bytes |
| 32 | Remaining bytes | Unchanged complete `LKJSG001` carrier |

The all-zero Target identity is invalid. Owner-key domain tag 12 is distinct
from Target owner-kind tag 20. Unknown magic/profile, wrong domain, zero identity,
nonzero reserved bytes and incomplete framing reject with 12. Raw-only
`LKJSG001` input is no longer accepted by `source-elf::compile`.

After checking the request header, the compiler admits the entire embedded
carrier through E1 and E2. Every supplied target is checked, including unselected
targets. It then resolves the selected key in the live owner directory. Missing
or retired targets reject with 19; negative lookup rows are never used for table
access. Component, port, callable signature and the normalized entry index are
derived inside lkjscript. The host supplies only the caller's TargetId.

The request has no second length field. Existing carrier length, partition and
commitment checks reject truncated or trailing payload bytes. The carrier length,
proof allowance and object constraints define the supported payload domain;
request transport adds exactly 32 bytes.

### Admitted scalar profile

| Dimension | Current scalar contract |
| --- | --- |
| Value types | Unrestricted Unit, Bool and I64 |
| Supplied descriptors | Those three primitive types and pure function descriptors whose parameters and result are scalar |
| Declarations | Modules, pure monomorphic Functions, the eight exact scalar External implementations below, and Components without requirements; their Parameters, ordinary Let Bindings, Expressions, function Ports and Target |
| Expressions | Unit/Bool/I64 literals, parameter/binding Local, If, Let, nonempty Sequence and direct Call |
| Calls and functions | Exact argument arity/types and declared result type; no type, requirement or effect parameters or application arguments |
| Entry | One or more command Targets, each with no runner arguments and selecting a function Port of its Component with pure `() -> I64` signature; the request selects one live TargetId |
| Excluded source families | Other External/intrinsic declarations, Text/StaticText, Bytes, ByteBuffer, List, records/choices, task callables, first-class function values/Invoke, generics, witnesses, read views and Regions |

A function descriptor is admitted for signature/port description; it does not
admit function-valued expressions. Parameters are unrestricted. Let annotations,
when present, must be scalar and equal the initializer's inferred type. If
conditions must be Bool and both branches must have the same type. Sequence
typing checks every item and takes the final item's type. Direct and mutual
recursion use declared signatures; they do not require expanding call bodies.

External admission uses the complete implementation identifier rather than the
declaration name. The following pure implementations require an empty type-parameter
list and exactly two unrestricted parameters with these types:

| Implementation | Mode-0 primitive code | Parameters | Result |
| --- | --- | --- | --- |
| `core.i64.add` | 1 | I64, I64 | I64 |
| `core.i64.subtract` | 2 | I64, I64 | I64 |
| `core.i64.multiply` | 3 | I64, I64 | I64 |
| `core.i64.divide` | 4 | I64, I64 | I64 |
| `core.i64.equal` | 5 | I64, I64 | Bool |
| `core.i64.less` | 6 | I64, I64 | Bool |
| `core.bool.and` | 9 | Bool, Bool | Bool |
| `core.bool.or` | 10 | Bool, Bool | Bool |

Every supplied External is checked, including unused declarations and aliases.
Unknown or other implementation identifiers, generic binders, and borrowed or
consumed parameters reject with 13. A recognized implementation with the wrong
scalar signature or arity rejects with 19. External remains a pure callable
declaration; this permission does not admit a TaskFunction with an empty effect
row. Calls preserve eager left-to-right argument evaluation, including Boolean
and/or, and the existing normalized integer overflow and division traps.

Profile checking visits every supplied type and owner. Inference visits every
Binding and Expression, then checks every Function body against its declared
result. Uncalled functions, untaken branches, unused supplied types and unselected
targets receive the same checks. A carrier without any Target rejects with 13. An unused non-scalar descriptor therefore rejects with 13.
A task Function or TaskFunction descriptor remains outside the scalar profile
even when its effect row is empty.

Inference stores one unresolved-or-scalar fact per owner. Each complete pass
either resolves all remaining facts, strictly decreases the unresolved count,
or rejects with 19. Published facts never change type. This is a finite progress
measure over admitted syntax; it is separate from the earlier carrier proof-work
allowance.

### Internal normalization and bounds

The planner inventories every supplied Function and admitted External in canonical
owner order, assigns stable parameter/binding slots and computes instruction counts.
Each External has a zero-instruction callable row with its exact parameter/result
types, parameter slots and primitive code. Calls use this same canonical callable
directory. The cursor
walks admitted containment and preserves source operand, initializer and sequence
order. State and trace construction derive operand types and lexical local
availability from the inferred facts and exact parents. Lowering emits literals,
loads/stores, sequence drops, conditional/unconditional jumps, direct Call and
Return. Even a call in tail position uses ordinary Call/Return. This profile has
no owned values and makes no claim about asymmetric consumption, borrow lifetime
or disposal lowering.

`source-scalar-pack` writes the existing mode-0/profile-4 LKJCI1 format, including
its section offsets, fixed built-in type table, flow certificates and checksum.
`source-elf` passes those internally produced bytes through the existing
`elf::compile-elf` validation and emission path. No Rust target normalization,
inferred tables or custody certificates are inputs to this process.

The source carrier retains 100,000 combined directory entries, its existing
proof allowance and individual object/field bounds. The complete carrier-byte
domain follows from the header and allowance constraints above. Generated mode-0
input remains at most 8,388,608 bytes, with 1 through 1,024 callable rows
(Functions and admitted Externals), at most 128 parameters and locals per row,
operand depth at most 128 and
a positive total instruction count. There is no separate instruction-count quota:
`2176 + 40*F + 32*N + B <= 8,388,608` bounds the complete mode-0 encoding,
where F counts function rows, N counts instructions and B counts blob bytes.
Function parameters are included in the local count. Lowering checks section
lengths and output bounds before assembling the final input; its generated
backend allowance must fit 2,147,483,647.
These are retained implementation/profile restrictions, not expanded capacity
or evidence of unmetered source compilation.

| Diagnostic | Scalar extension |
| --- | --- |
| 0 | Selected scalar admission/lowering checks succeeded |
| 13 | Recognized form outside the scalar profile or retained lowering/profile bound |
| 19 | Scalar type, call arity, annotation, function result, target/port agreement, absent/retired selected target or lowering-consistency failure |

Earlier diagnostics 6 and 12 through 18 retain their meanings. In particular,
malformed carrier/binding errors remain 12, endpoint failures remain 14 and
containment/value-local scope failures remain 15. Backend diagnostics propagate
if the internally generated input fails backend validation. Rejection produces
no executable bytes.

### Bootstrap and evidence boundary

A Rust-built native compiler is permitted as the initial executable. The target
program crosses that boundary only as complete raw `LKJSG001` bytes and the
caller's canonical TargetId in a source request. Rust may serialize that request,
transport bytes, launch processes and orchestrate independent controls.
It must not pre-filter source owners/types or supply semantic or lowering
results for the scalar target. The public meaning remains the accepted semantic
project; the `.lkjc` files remain authoring proposals.

Acceptance evidence must identify the complete carrier digest, complete request
digest, canonical selected Target key, compiler executable, target/format profiles
and execution context. The semantic-state
digest alone is insufficient, including for unused-type mutations. Independent
controls must cover valid scalar execution and fully rebound malformed/ill-typed
carriers, reaching the intended admission layer. The
[scalar decision](../decisions/20261023-source-scalar-e2-e3.md) records the current
positive programs, semantic/profile controls and independently constructed missing
endpoint and cyclic carriers. The latter pass commitment checks before rejecting
at their intended structural layers. Executable and carrier identities bind these
observations; publication still requires the complete frozen-tree gates.

The scalar route emits ELF only. It does not establish compilation of the
maintained compiler's complete source closure, source-driven C2/C3 identity,
canonical `.lkja` A1/A2/A3 equality, Rust canonical-artifact parity or same-process
native memory reclamation. Those remain separate Tier 2 obligations.
