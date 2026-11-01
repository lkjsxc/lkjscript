# Canonical source carrier and reference closure

This is a restricted source-admission slice toward the
[semantic self-hosting contract](selfhosting.md). It carries current canonical
source objects, logical directories, a claimed semantic-state identity and the
original canonical RevisionCore binding that state to its semantic revision.
It contains no normalized instructions, inferred types, reference summaries,
validation certificates, compiled units or executable templates.

`source-graph::validate` admits the carrier, exact object bindings and logical
state. `source-links::validate` additionally resolves every supported owner and
type reference, including unused owners and untaken expression operands. Success
does not establish lexical scope, unique syntax ownership, acyclicity, expression
typing, affine custody, effects or witness validity. The additional
`source-scope::validate` pass establishes the ownership and value-local
scope contract below. `source-namespace::validate`, `source-type-scope::validate`
and `source-nominal::validate` successively add namespace uniqueness, contextual
type/R/E binder scope, structural type DAG proof and nominal member parameter flow
over the same decoded profile. `source-scalar::prepare` adds whole-inventory
active-profile typing and affine ByteBuffer custody. `source-elf::compile` lowers
the admitted source to standalone ELF or canonical artifacts. Implementation
capability and accepted native fixed-point evidence remain distinct; the full
P_self profile and same-process reclamation retain their separate obligations.

The current carrier has one package and no dependencies or external blobs.
Recognized unsupported boundaries reject explicitly; importing a package's
interface or prior validation witness cannot supply its missing source bodies.
This carrier is separate from normalized LKJCI1: mode 0 uses profile 6 for ELF
and mode 1 uses profile 7 for canonical artifacts. The existing Tier 1 paths use
Rust admission and normalization. The source route derives both mode-0 and
mode-1 inputs inside lkjscript from the complete raw carrier.

## Exact bytes

Header counts, offsets, the policy tag and span fields are u32 little-endian;
the explicit proof allowance is u64 little-endian. Complete input is at least
128 bytes, and its actual length must equal the encoded u32 length. There is no
separate aggregate byte or entity-count quota. Empty directories are allowed.
Package identity is nonzero and the package name has 1 through 128 canonical
name bytes. The only current carrier is magic LKJSG002 with explicit profile 3.
Predecessor profile values and other magic values reject without fallback.

| Header offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | Magic `LKJSG002` |
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
| 100 | 4 | Proof policy: 0 = unbounded, 1 = finite; other tags reject |
| 104 | 8 | Exact finite proof allowance; zero for the unbounded tag |
| 112 | 4 | Current source profile, exactly 3 |
| 116 | 4 | Provenance block offset P |
| 120 | 4 | Provenance block byte length Q |
| 124 | 4 | Reserved zero |

The name starts at offset 128 and ends at P = 128 + name_length. The provenance
block follows immediately. Its first 32 bytes are the claimed raw RevisionId;
its remaining bytes are the exact current canonical RevisionCore. The owner
directory starts at P + Q, followed by type, retirement and span directories.
Package-name scanning and semantic-state material stop at P; they never include
provenance bytes. Directory row formats remain:


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

The provenance block is mandatory. RevisionCore retains its current canonical
versions, nonzero RepositoryId, ordered zero/one/two-parent vector and
SemanticStateDigest. The canonical parser checks the fields, typed tags, ordering,
minimal integer encodings and exact end. Its 53/86/119-byte core lengths follow
the current history contract; length alone is not admission. Parent and revision
digest values are checked by exact derivation without an added nonzero rule.

After recomputing the complete source semantic state, source admission verifies
the original core's state binding and independently derives its RevisionId in
the current semantic-revision domain. It requires equality with the claimed ID.
This adds no witness, inferred type table, interface claim or generated code to
the carrier. Canonical schema epoch is unchanged; the new private carrier
profile disambiguates the incompatible projection layout.

Ordinary carriers select unbounded proof work with tag 0 and zero allowance
bytes. They neither compute nor enforce a synthetic minimum estimate. Tag 1
preserves every u64 value as an exact finite policy, including zero and
18,446,744,073,709,551,615. A finite allowance is never silently clamped and zero
is never interpreted as unlimited. Noncanonical unbounded allowance bytes and
unknown tags reject with diagnostic 12.

Proof work remains distinct from VM instructions, allocations, native process
limits, effects and cancellation. Carrier shape, object bindings, complete
reference closure and membership are admitted before the metered scope proof.
Those preliminary checks do not consume this particular proof allowance. The
scope, namespace, type-structure/context and nominal passes compose one state
with an explicit finite flag, two u32 remaining limbs, two u32 observed-work
limbs and a saturation flag. Finite enforcement subtracts only after a checked
comparison. Observation saturates independently at u64::MAX and sets the flag
only when an additional charge cannot be represented exactly; saturation never
grants extra finite capacity. A refused charge changes no remaining allowance or
successful-work observation. Proof exhaustion returns 6 without a semantic
validity claim. Public with-work(graph, work) convenience entrypoints accept
every nonnegative I64 allowance; with-policy takes the explicit state produced
by source-budget constructors, including the full u64 wire domain.

Termination does not depend on a finite allowance. Carrier/object scans advance
over finite byte or record ranges. Binary directory searches shrink their
intervals. Ownership forests use bounded pointer doubling followed by a root
check; cycles still reject. Later parent walks run only after that forest proof.
Structural type admission settles distinct nodes or rejects a no-progress pass.
Type/nominal queues retain visited-state deduplication. Namespace merge-sort
splits finite row ranges. These structural arguments also apply in unbounded
mode.

Index windows establish nonnegative ordered extents and compare the requested
count with (end - at) / stride before multiplying. Header layout arithmetic
starts with u32 fields and fixed row widths, so intermediate products fit I64;
the computed extent must fit the actual carrier before any directory is read.
Logical owner/retirement map roots retain canonical key/row encodings and
ordering, while allowing arbitrary row and byte counts within their checked
backing extent. Their former 100,000-entry and 8 MiB quotas no longer apply.

This cut does not widen the carrier's u32 length/count/offset/span fields or
canonical per-object formats. Canonical child-array limits, payload bounds,
structural depth admission and later normalized/native backend limits remain
separate coupled work. No full-capacity or Tier 2 completion claim follows from
the optional proof policy.

## Binding and references

Each object must pass its current envelope/shape reader and independently derived
object digest. Owner and retirement records must agree with their directory's
exact identity and kind. A live owner cannot simultaneously be retired.

The compiler recomputes the logical owner and retirement map roots, the empty
dependency root, and the semantic-state digest from the actual package identity,
name and complete logical rows. It compares that digest with the header's claimed
state. Repository identity and physical map page locators do not enter this
semantic-state commitment. The separately verified original RevisionCore binds
repository identity and revision parents to that state. Neither digest match
supplies a historical validation certificate or execution authority.

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
| 6 | Explicit finite proof allowance exhausted; no semantic-invalid claim |
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

`source-scope::with-work(graph, work)` selects an exact nonnegative I64
ancestry-proof allowance. `with-policy` also supports explicit unbounded work and
the full u64 finite domain. Complete carrier/endpoint/membership admission
precedes this proof. The default entrypoint uses the carrier's tagged policy.
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
Internal `check` functions return the shared tagged allowance, charged-work
observation and an independent nonnegative diagnostic code; public wrappers
return zero or that diagnostic. Later passes do not reset
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

These layers establish restricted-profile E1 admission. The active-profile
extension below adds compound typing, exact ByteBuffer custody and both E3 output
paths. Full contextual read/witness scope, effect applications, general callable
generics, nominal owned aggregates, first-class functions and lexical Regions
remain separate semantic extensions. The full Tier 2 profile, complete-source
fixed-point equations and measured same-process reclamation requirements are
unchanged.

## Active compiler source admission and both output paths

`source-scalar::prepare` composes carrier/reference, containment, lexical scope,
namespace, contextual type/DAG and nominal checks before active-profile typing
and affine custody admission. It returns source-derived facts, containment
parents, admitted types, type mapping and custody evidence. These are internal
compiler results, never producer-supplied certificates. `source-scalar::validate`
returns the same admission diagnostic without emission.

`source-elf::compile`, exposed by the `compile-source-elf` binary-filter target,
returns `{diagnostic: I64, image: Bytes}` and serves both current output modes.
It admits the complete embedded source once before resolving the caller's target
or choosing an output path. Rejection returns an empty image.

### Explicit source request

The only current request is `LKJSRQ01`, profile 2. Its 32-byte header precedes
the complete `LKJSG002` profile-3 carrier:

| Offset | Width | Meaning |
| --- | --- | --- |
| 0 | 8 | ASCII `LKJSRQ01` |
| 8 | 4 | Little-endian u32 request profile, exactly 2 |
| 12 | 17 | Canonical Target key: domain tag 12 and its nonzero 16-byte identity |
| 29 | 1 | Output mode: 0 for standalone ELF, 1 for canonical `.lkja` |
| 30 | 2 | Reserved zero |
| 32 | Remaining bytes | Unchanged complete current source carrier |

Unknown profiles or modes, wrong target domain, zero target identity, reserved
bits and malformed framing reject with 12. Predecessor request/source readers
and raw-carrier-only compilation requests are not retained. The request has no
second length field: exact carrier partition and end checks reject truncation
and trailing bytes. Request transport adds exactly 32 bytes.

Every supplied target receives E1/E2 checks. After complete admission, the
selected key must resolve to a live Target; absent or retired selectors reject
with 19 before any row access. Component/port agreement, entry signature and
the native entry index are derived inside lkjscript. The host supplies only the
caller TargetId and output mode. Mode 1 still requires a valid selected target,
but emits the complete admitted package artifact, including all compilation
owners and targets. Target selection does not prune its source or unit closure.

### Admitted active profile

| Dimension | Current contract |
| --- | --- |
| Copyable values | Unit, Bool, I64, Text/StaticText, Bytes, List<I64>, and structural records over admitted copyable values |
| Affine values | ByteBuffer, with exact consume moves and synchronous borrowed length reads |
| Supplied descriptors | The corresponding compound descriptors, pure Function signatures, and the scoped TypeParameter/List<T> descriptors of the recognized list intrinsics |
| Declarations | Modules; pure monomorphic Functions; exact active External implementations; Components with empty requirements; their signature, syntax and function-port owners; command Targets |
| Expressions | Literals, structural Record and Field, List<I64>, Local, If, Let, nonempty Sequence and direct Call |
| Generic calls | The closed I64 substitutions for the exact `core.list.length` and `core.list.get` signatures; no general generic function-body expansion |
| Entry signatures | Pure `() -> I64` or `(Bytes) -> {diagnostic: I64, image: Bytes}`, through a component function port with no runner arguments |
| Remaining families | Nominal owned aggregates/choices and Match, arbitrary List<T>, first-class closures/Invoke, task/effect applications, explicit implementation witnesses/read views and lexical Regions remain separate extensions |

Pure Function descriptors describe signatures and ports; they do not admit
function-valued expressions. Structural field descriptors retain exact names
and child types. Source record evaluation preserves authored field order while
runtime layout uses canonical field order. Let annotations must agree with the
initializer's type, If conditions are Bool, both branches agree, and Sequence
checks every item before taking its last type. Declared signatures type direct
and mutual recursion without expanding callees.

External admission uses the complete implementation identifier, independently
of the declaration name. The maintained `ci` declarations use these 24 exact
implementations. Parameters are unrestricted unless their mode is shown:

| Implementation | Parameters | Result |
| --- | --- | --- |
| `core.i64.add` / `subtract` / `multiply` / `divide` | I64, I64 | I64 |
| `core.i64.equal` / `less` | I64, I64 | Bool |
| `core.bool.and` / `or` | Bool, Bool | Bool |
| `core.bytes.equal` | Bytes, Bytes | Bool |
| `core.bytes.get` | Bytes, I64 | I64 |
| `core.bytes.length` | Bytes | I64 |
| `core.bytes.slice` | Bytes, I64, I64 | Bytes |
| `core.bytes.concat` | Bytes, Bytes | Bytes |
| `core.bytes.from-list` | List<I64> | Bytes |
| `core.bytes.from-text` | Text | Bytes |
| `core.bytes.blake3` | Bytes | Bytes |
| `core.bytes.blake3-derive-key` | Text, Bytes | Bytes |
| `core.buffer.empty` | None | ByteBuffer |
| `core.buffer.push` | I64, consume ByteBuffer | ByteBuffer |
| `core.buffer.length` | borrow ByteBuffer | I64 |
| `core.buffer.freeze` | consume ByteBuffer | Bytes |
| `core.buffer.discard` | consume ByteBuffer | Unit |
| `core.list.length` | List<T>, closed with T=I64 | I64 |
| `core.list.get` | List<T>, I64, closed with T=I64 | T |

Every supplied External and descriptor is checked, including uncalled aliases
and unused Types. Recognized signatures must have the exact arity, parameter
uses and result. Unknown/unsupported forms reject with 13; type, arity, result,
annotation or custody disagreements reject with 19. Task functions remain
unsupported even with an empty effect row. Calls preserve eager left-to-right
argument evaluation and current arithmetic trap order.

### Affine custody and native lowering

Owned ByteBuffer parameters consume custody. Ordinary owned locals require
explicit ByteBuffer annotations. A consume operand transfers a live owner and
cannot use an already moved or borrowed owner. The recognized borrowed buffer
length call creates only a synchronous loan; it uses qualifier 1 and releases
that loan at call return. Borrowed values cannot be stored, aggregated, returned
or consumed. General borrowed helper functions remain outside this profile.

Both If branches are admitted. Branch-balanced disposal removes owners retained
on only one branch. Scope exit disposes remaining owners in reverse lexical
order; returning an owner transfers it and disposes other live owners. Sequence
discards owned intermediate results. A tail-position call is emitted as TailCall
only when its transfer leaves no owned local live. Otherwise native lowering
uses Call, required cleanup and Return. The generated backend independently
checks the resulting flow certificates.

The planner inventories all callables in canonical owner order, derives local
slots and control flow, and writes current mode-0/profile-6 input. Primitive and
structural-record runtime descriptors, type payloads, qualifier cells and every
instruction state come from source admission and lowering. This internal format
is passed through `elf::compile-elf`; no host-normalized IR is an input.

Plan rows, trace headers and runtime state cells retain their checked u64 fields
and signed-I64 byte-index extent. Dynamic record metadata is included in layout
measurement before emission. Generated mode-0 policy is unbounded. Source u32
framing, canonical object bounds, canonical IR u32 identities and existing depth
guards remain separate capacity work. Heap-backed evaluator frames and generated
native CALL frames have different physical growth behavior; this profile does
not claim general native stack unbounding or same-process reclamation.

### Canonical artifact construction

Mode 1 derives every canonical CompilerUnit from original source fields. The
source summary pass computes all seven owner-summary dimensions; canonical units
use interface, implementation, type, effect, capability and validation-dependency
dimensions. Unit lowering preserves original IDs, authored names and declared
type descriptors. It follows canonical pre-tail instruction order, including
ordinary Call and forward owned-Let Unit/StoreLocal cleanup. Native TailCall/Drop
traces are not reinterpreted as canonical unit IR.

The compiler independently projects the public interface, selects exact runtime
owners and callable reference containment, and derives their required transitive
Type closure. Valid unused supplied Types are admitted but do not become unrelated
artifact objects. It constructs PackageRevision from the original admitted
RevisionCore, package identity, derived interface digest and empty dependencies.
The source request contains no interface digest, witness, owner subset, compiler
unit, compilation map or generated code.

These source-derived results form current mode-1/profile-7 input inside lkjscript.
Object rows use exact ObjectKey order. Equal keys are collapsed only after exact
length and byte-window agreement; different bytes for one key reject before
emission. Unit directories, metadata, descriptors and payload extents are checked
before construction. The existing artifact validator then checks the generated
carrier, and the canonical backend constructs units, maps, manifests, targeted
packs and the final `.lkja` container.

Interface/reference/compilation maps use the host's ordinary 16 KiB leaf target
for their fixed-width rows, within the existing physical MapPage contract.
Artifact packs use the current 4 MiB target, automatically adding segments and
retaining valid oversized singleton objects. These are physical layout targets,
not logical package ceilings. No canonical schema epoch change is made by this
private source/provenance cut.

### Diagnostics and evidence

| Diagnostic | Active source path |
| --- | --- |
| 0 | Admission and the selected output path succeeded |
| 3 | Generated internal representation/layout inconsistency or extent failure |
| 6 | Exact caller-selected source proof allowance refused work |
| 12 | Request/carrier/object framing, provenance or binding error |
| 13 | Recognized form outside the supported profile |
| 14 | Missing/incompatible source endpoint or live/retired collision |
| 15 | Containment or lexical scope error |
| 16 | Namespace collision |
| 17 | Contextual binder or structural type-DAG error |
| 18 | Expanding nominal parameter-slot cycle |
| 19 | Type, custody, borrow, pure-call, result, target or lowering agreement error |

Backend diagnostics otherwise propagate. Existing proof-policy metrics are not
redefined as total compiler work. Their unbounded path remains distinct from
every exact finite u64 allowance, including zero and u64::MAX.

Rust may construct the initial compiler executable, frame unchanged complete
source, transport requests and provide independent oracles. Subsequent native
stages must consume that source directly, without Rust semantic preprocessing
or an external VM. Evidence binds exact semantic revision and state, original
RevisionCore, complete carrier/request SHA-256, caller target, both current
profiles, executable bytes, target/ABI/flags and execution environment. A state
digest alone does not bind unused supplied Type records.

Source-level and stored-object controls are separate from native fixed-point
acceptance. Milestone A requires complete maintained F_active source to establish
C2=C3 and Ahost=A1=A2=A3, strict loading/provenance for every generation, independent
program behavior and compiler-source mutations, complete-inventory negatives and
fresh frozen-tree gates. The broader P_self semantic families and same-process
phase reclamation retain their separate requirements in [selfhosting.md](selfhosting.md).
