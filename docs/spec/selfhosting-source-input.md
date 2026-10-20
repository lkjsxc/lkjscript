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
type-parameter scope, structural type DAG proof and nominal member parameter flow
over the same decoded profile. Source normalization and the Tier 2 source-graph
fixed point remain separate work.

The current carrier has one package and no dependencies or external blobs.
Recognized unsupported boundaries reject explicitly; importing a package's
interface or prior validation witness cannot supply its missing source bodies.
This carrier is separate from LKJCI1 profile 4, which retains its existing
backend input and output contract.

## Exact bytes

All scalar header fields and span fields are u32 little-endian. Complete input is
128 through 8,388,608 bytes. The sum of owner, type and retirement counts is at
most 100,000. Empty directories are allowed. Package identity is nonzero and the
package name has 1 through 128 canonical name bytes.

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
limits. Bounds precede products, object access and hashing. Lookup uses binary
search over admitted immutable directories; it does not copy the complete source
or linearly rescan the directory for each reference.

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

## Syntax membership and exact value-local scope

After complete carrier and endpoint admission, `source-scope::validate` derives a
private table of 12-byte rows: child owner-directory index, parent owner-directory
index, and ordered membership position, all u32 little-endian. No derived table is
accepted from the producer. It is not a persisted schema or a validation witness.

Modules and targets have no containment parent. Declarations belong to their exact
module. Declared type parameters, fields, cases, callable parameters and component
ports have exactly one listed container, agreeing with their reverse parent field.
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
| Parameter | Exact Function Declaration or Operation | Function parent supported; Operation owner unsupported |
| EffectParameter, RequirementParameter, Operation, Requirement | Exact Declaration | Unsupported owner forms, diagnostic 13 |

Binding, Expression, HttpRoute, Documentation and Annotation do not participate.
Equal names across different parents or classes remain legal. Local binding
shadowing retains exact binder references and is not package namespace collision.

The pass derives 16-byte rows containing parent owner-directory index, namespace
class, absolute name offset and name length. The owner count denotes no parent;
directory indexes preserve complete OwnerKey identities, including their domains.
Deterministic merge sort compares parent, class and original canonical name bytes.
Adjacent equal keys reject with 16. The table is private derived state; names are
not replaced with hashes or copied into independently authoritative keys.

Whole-carrier collision controls cover the eight currently decoded classes. Direct
projection controls separately cover all 12 classes, both Parameter parent domains
and excluded owner forms. These projection tests do not establish complete carrier
admission for the four unsupported classes. Their adoption requires the preceding
shape, reference and containment layers to admit the corresponding owner forms.

## Structural type DAG and contextual type scope

`source-type-scope::validate` additionally derives structural child edges from every
supplied type descriptor, including unused descriptors. Structural field types,
List items, function parameters/results and Applied arguments are child edges.
Named nominal declaration references are not structural backedges. All endpoints
must already exist in the admitted type directory. Repeatedly settling descriptors
whose children have known heights establishes acyclicity and the maximum structural
depth of 256 edges. Unsettled descriptors with no progress identify a cycle.

Every supported owner carrying type roots is checked in its enclosing semantic
declaration context. Traversal follows all child types; a TypeParameter descriptor
must name a parameter belonging to that declaration. A shared digest may therefore
be valid in one using declaration and invalid in another. Visited-descriptor state
is local to a using context, with no digest-only global scope cache. Any later reuse
must bind complete declaration/callable context and relevant binder identities and
kinds, including method, read-scope, effect and witness contexts when admitted.
The current slice does not admit those broader contexts through unsupported forms.

Type-object digests commit to their child digests, so rehashing ordinary objects
cannot create a self-consistent structural cycle. Independent derived graph fixtures
exercise cycle proof and the exact depth boundary directly. Raw-carrier tests
exercise acyclic structure and context-sensitive type-parameter mutations; direct
cycle tests must not be reported as end-to-end hash-consistent cyclic carrier
coverage. Failure is code 17.

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
nodes and membership scans. Exact fit succeeds; exhaustion remains diagnostic 6.
These units are distinct from evaluator quotas, cumulative allocation and RSS.

| Diagnostic | Additional E1 meaning |
| --- | --- |
| 6 | Additional proof-work allowance invalid or exhausted; no semantic-invalid claim |
| 16 | Duplicate exact namespace key |
| 17 | Contextual type-parameter scope, structural cycle or structural-depth failure |
| 18 | Nominal member-application arity or expanding cyclic parameter-slot flow |

Earlier diagnostics 12 through 15 remain unchanged. Layer-reaching negative carriers
must rebind framing, objects, directories and state and pass preceding validators.
Tests compare exact Rust kernel diagnostics and independent expectations across VM,
canonical reference, Rust-native and selfhost-native execution. Standalone native
source validation still uses a Rust-bootstrapped normalized validator; it is not
source-to-normalized self-compilation. Complete carrier SHA-256 and semantic-state
identity remain separate evidence.

This is restricted-profile E1 work, not E1-final or Tier 2 completion. Broader source
forms, full contextual read/effect/witness scope, general application and expression
typing, callable generic finite-flow, custody, effects, witnesses and normalization
remain required. Regions, implementation calls and other excluded forms still
reject with 13. The full Tier 2 profile, source-driven ELF/artifact fixed points and
measured native phase reclamation remain unchanged; no bounded-RSS claim is made.
