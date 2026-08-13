# Current implementation status

**Status: currently implemented in this checkout.** This is a concise report of checkout behavior,
not a compatibility promise or normative specification. Code, tests, CLI definitions, schemas, and
manifests remain the executable authority.

## User path

The active product is local package check/run plus an in-process semantic workspace API. The
`.lkjscript` extension is fixed; the current line-oriented bytes are a provisional importer format,
not a textuality or compatibility promise and not semantic authority. `Workspace::empty` creates a
source-free revision with no entities, source/path/hash attachment, entry point, or body. Text and
path entry points import exactly once into the same partial-capable immutable `WorkspaceSnapshot`.

Snapshots own one clone-safe `SemanticProgram`, optional imported diagnostic/presentation origins,
opaque namespace/slot/generation IDs, allocator tombstones, derived indexes, type facts, real hole
and unresolved value-reference nodes, diagnostics, and structured completeness blockers. Source-free and imported complete
revisions derive one ephemeral complete HIR at `compile_snapshot`; they never render or reparse
source. The CLI's required-package compiler entry still verifies the root manifest, lock, selected
module, source identities, target, and capability grants once. Every product compile API delegates
to `compile_snapshot` before HIR memory planning, locked-package target validation, SSA lowering,
verification, and bytecode validation.

`lkjscript check <entry.lkjscript>` uses that required-package production path and discards the
`ExecutableProgram` before any host environment, executable installation, native entry, or VM
invocation exists. Human success writes zero bytes to both streams. `--json` writes one deterministic
`lkjscript.check` document; fail-fast source errors preserve their existing code, severity, category,
logical path, one-based line and Unicode-scalar column ranges, message, and ordered related ranges,
Source `path` and `range` fields are omitted when the producer has no source location; package,
incomplete, host, and later compiler failures likewise keep only facts their producer owns.
`lkjscript package check` is also silent on success. The former `describe` command and aggregate
contract-set digest are deleted because no independent tool or workflow consumed them.

Implemented source behavior includes typed functions and calls, bindings and explicit mutation,
conditionals and loops, nominal products and enums, exhaustive matching, generics and trait-dispatch
subsets, numeric conversions, bytes and byte vectors, lists, typed host resources, errors, and
explicit capabilities. Executable examples and compiler/runtime tests own the exact accepted
surface; this document does not copy their tables.

`lkjscript run` synchronously attempts one supported baseline-native group reachable from `main`.
Eligibility, lowering, installation, or typed pre-entry decline drops the entire native attempt and
executes the unchanged validated program in the VM. Once native entry begins, its result is final
and the VM is not run. Unsupported I/O, generic, recursive-stack, and other native shapes therefore
remain valid through the generic VM route. There is no public engine, threshold,
automatic-transition, or forced-native option.

The broadly tested host is Linux x86-64. Portable Rust may build elsewhere, but another host or
native target is not claimed as tested.

## Phase 2 broader-platform deletion

The workspace now has 11 members and one application binary, `lkjscript`, as reported by Cargo
metadata. The following speculative products and mechanisms are deleted rather than archived or
feature-flagged:

- `lkjscript-runtime`, `lkjscript-resource`, `lkjscript-database`, and
  `lkjscript-linux-host`;
- daemon, process-cell, cell-test-worker, session-broker, resource benchmark, scheduler,
  topology-observation, and app `system` wiring;
- service database, durable control-store, directory-capability, local-control, and database-tenant
  host providers that had no consumer after service deletion;
- process bootstrap/provenance and execution-outcome codecs, resource-plane/runtime-control/
  component contract descriptors, and the global platform revision; and
- target-matrix, platform-revision, and empty configuration placeholders.

The app no longer has dependencies or tests for those surfaces. Persistent package and lock bytes
retain exact language, source, module-interface, package-manifest, and package-lock identities.
Package lookup is one closed direct match and does not construct unrelated descriptors. Metrics and
memory inventory retain separately owned exact output identities. Capability, resource, operation,
and memory-witness vocabulary remains canonical typed data without registry membership. Verified
SSA and validated bytecode remain typed in-process authorities; there is no descriptor or digest for
typed HIR, verified SSA, bytecode, runtime-call slots, native layout, diagnostics, or structural
ownership domains, and no generic prepared descriptor, cross-representation program identity,
compilation cache, or unconditional native-specialization artifact. A locked snapshot retains only
the package target fact needed to compare its completed HIR memory plan with the lock.

Deleting the service database wrapper did not delete the language SQLite capability. VM host
operations still dispatch SQLite directly through `lkjscript-sys`; stdio, clock, filesystem,
network, terminal, and entropy behavior used by local programs also remains. The retained hello,
Mandelbrot, editor, HTTP, byte, filesystem, hash, SQLite, and comparison smoke paths exercise the
local product rather than a daemon.

## Executable boundary

Native image installation remains a pre-entry, failure-atomic operation. Every image is an opaque
typed value produced by the current in-process native encoder; there is no serialized image, cache,
plugin, or cross-version loader. Installation revalidates structural integrity, accounts the object,
applies checked relocations in a private RW mapping, seals the mapping RX, and publishes installer
usage only after success. The image carries no redundant same-build contract digest or configurable
version token. Dropping an installed image releases both its mapping and accounted lease. Persistent
package and lock contract validation remains unchanged at its independent filesystem boundary.

Collector-free `prepare_invocation` and explicit `prepare_region_invocation` validate entry and
typed arguments, materialize and reserve machine-call and runtime bookkeeping state, and perform
immediate cancellation, deadline, resource, and configured whole-group stack checks. Success
returns a non-cloneable `PreparedInvocation`; `enter(self)` consumes it exactly once across the
unsafe generated ABI call. Pre-entry and entered failures are disjoint types, and there is no VM
retry after entry.

Product metrics report `baseline-native` or `vm-fallback`, one nullable typed native decline with
stage/code/function/detail, whether native entry began, package validation and compiler/native/VM
timings, published installed-artifact size/work counts when available, and explicitly saturating
native runtime/cleanup observations when available. Missing native observations are `null`, not
measured zero. Automatic thresholds, per-function call records, retries, invalidation, runtime
sessions, forced execution, optimizing native lowering, optimization certificates, and the
proof-oriented optimizer are absent. Baseline normalization retains only the independently
verified simple pass sequence.

## Scale and policy result

The ordinary trusted local path has no compiler profiles, cross-phase count budgets, or source/HIR/
SSA count ledgers that decide language validity. Trusted source loading, initial bytecode validation,
and local execution explicitly select unrestricted policy. The former token, child, top-level-form,
nesting, source-unit, HIR-memory, SSA-CFG, SSA-ownership-work, and retained-state-cell admission
quotas are removed.

Source positions and spans, semantic workspace entity/node IDs, HIR and SSA identities, executable
operands and links, structural metadata, and runtime structural identities use wide representations
where they carry user-scale data, with checked conversion before host indexing. Parser/source-tree,
package, type/match, CFG, structural graph, and tested deep-destruction paths use explicit work
stacks or equivalent stack-safe designs. The owned `SemanticValue` product/enum tree uses iterative
clone, destruction, fallible and trait equality, symbol rewriting, image conversion, and outcome
validation. Its Debug implementations report bounded root kind/type/field or byte-count summaries;
they do not expand descendants or complete leaf bytes. Safe ownership makes cycles unrepresentable,
so owned-outcome validation performs one checked node/field/byte traversal rather than scanning an
ancestry list. The separate `SemanticDagSnapshot` remains a validated graph boundary.

Ordinary 2,048-level and ignored 20,000-level tests run the complete owned tree boundary on a 128
KiB native stack. A generated VM program also constructs, returns, clones, compares, and destroys a
20,000-level alternating enum/product result; baseline native declines its recursive call graph
before entry and the unchanged program executes once in the VM. A few compiler-recursive paths
remain localized behind [`stack.rs`](../crates/lkjscript-compiler/src/stack.rs); its heap-backed
segment geometry is private tuning, not a language-depth limit.

Committed generated tests cross former boundaries, including 20,000 nested source expressions,
10,000-block CFG verification, 44,000 owned SSA parameters/places, 300-field products and enums,
1,024 parameters/arguments/locals, table index 65,536, and runtime structural collections above
65,535. Some largest production geometries remain ignored release stress tests.

Borrow ownership builds one iterative liveness plan over complete HIR child traversal. Half-open
expression ranges and sparse per-binding direct-use indexes replace suffix reconstruction; targeted
loan expiration preserves call argument pinning, branches, loops, and mutable-local continuation
without scanning every live reference at every expression.

Bytecode local allocation classifies each SSA value's type, storage, and producer once. Coloring and
emission consume that same per-function metadata, and emitted frame size is checked highest physical
slot plus one rather than SSA value count. Straight-line borrow-call temporaries therefore reuse
three physical locals at every retained matrix size.

Bytecode control-flow validation stores abstract state only at basic-block entries, mutates one
working state through each straight-line block, and merges only at block edges. Failure-cleanup
ranges use a block-local sorted-range cursor, while an incrementally maintained summary makes
cleanup-required checks constant-time between plan-validation boundaries. VM execution uses binary
lookup for unwind and nonlocal cleanup-range movement plus one advancing cursor per active frame for
sequential boundaries. Failed call entry cleans moved arguments in reverse order, and post-step
policy failure unwinds the exact next boundary before ordinary frame cleanup.

The retained 16,385-call fixture completes in release mode on the measured host without a validity
quota. Final medians are 7.303 ms HIR ownership analysis, 7.924 ms bytecode lowering, 6.612 ms VM
execution, and 290.426 ms total compile time; compact protocol, tails, RSS, and exact-stress evidence
are recorded in [`performance.md`](performance.md).

## Retained validation and host boundaries

Artifact validation may select a total-artifact-byte policy. The reusable execution API still
supports explicit limited policy for fuel, VM values/frames, heap/allocation, handles, output, wall
time, and cleanup-report retention; ordinary local compile/run selects the unrestricted form.
These policies control an execution or artifact boundary and do not redefine language validity.
There is currently no untrusted semantic request service or request-byte policy.

Fail-closed validation remains at source importer input, package/manifest/lock/import and compiler
path/symlink entry, typed workspace transactions, capability dispatch, bytecode validation,
relocation and W^X installation, generated native entry, FFI, SQLite,
filesystem/socket/terminal calls, and operating-system errors. Filesystem/network/SQLite language operations use the current process's explicitly
granted capability and direct system mechanism; the deleted service sandbox, process framing,
peer-authorization, database-tenant, durable-store, and platform-observation boundaries are not
claimed as retained.

The execution-outcome wire codec is absent. `SemanticDagSnapshot` and authenticated sealed
structural snapshot import/export remain in memory for VM/JIT behavior and differential tests.
Their memory witness facts are named semantic-snapshot facts rather than process-codec facts.

## Semantic workspace compiler cutover

The compiler's public `workspace` module owns one syntax-independent in-memory authority. An
in-process `Workspace` owns its current `Arc<WorkspaceSnapshot>` and stages the exact
identity-allocator state. Public namespace/revision/entity/node/hole/unresolved-reference IDs are
opaque. Tagged private
entity addresses are independent of public ordering, so adding `main` does not renumber an existing
function. Removed slots retain tombstone generations across snapshot cloning and reopening.

`Workspace::empty` reports `Incomplete`, one missing-entry blocker/diagnostic, zero entities/nodes,
and no attachments. `Transaction` adds non-generic `CreateProduct` and `CreateEnum`, generalized
generic-or-non-generic `CreateFunction`, and `CreateMain` with ordered explicit capability
parameters; identity-preserving rename for functions,
parameters, locals, products, product fields, enums, variants, and enum fields; replacement and hole
operations; and unresolved copy-load value-reference introduction and resolution.
Function creation uses one ordered declaration-local binder handle domain and a creation-only
`DeclarationType`; staged validation allocates stable function, type-parameter, and value-parameter
entities before canonical universal-signature construction. The local handles never enter published
state. Products, enums, variants, fields, functions, function type parameters, value parameters,
locals, bodies, and holes receive opaque stable entities independent of compiler-dense nominal/layout
identities. Public published inputs and queries use one exact structured `SemanticType`; products,
user enums, and type parameters carry stable entities, while all five prelude enums and all five core
traits have explicit builtin identities. Recursive operations and public/internal conversion for
both type boundaries are iterative and unrestricted by type depth.
Main parameter names/types use the source entry restrictions: exact capability values, unique names,
strictly sorted unique capability kinds, and a valid public result. Main parameters receive stable
entities owned by main and enter its body-hole scope. Invalid names, duplicate declarations/members,
ownership-containing aggregate fields, foreign/stale/wrong-kind type identities, and allocation
failure reject without consuming their reserved stable IDs.

`ExpressionDraft` is a flat non-recursive tree with transaction-local lexical binding handles; its
physical node order is irrelevant. It implements scalar and byte literals, every canonical operation
whose canonical metadata selects the ordinary runtime-operation expression route (including numeric,
string conversion, and explicit-capability stdio operations), exact generic and non-generic calls,
conditionals, ordered sequences, immutable lexical locals, explicitly typed mutable locals,
assignment, `while`, explicitly typed `loop`, nearest-lexical `break` and `continue`, early `return`,
copy-safe loads, byte-vector moves and shared borrows, product construction/projection, enum
construction and variant tests, and exhaustive closed Boolean, I64, exact-product, and non-generic
enum matches. Empty sequence is pure
`unit`; non-empty sequence yields its last value. A return value must be non-divergent and exactly
match the owning callable's declared result type; the return node is `never`-typed, carries canonical
divergence and child effects, and uses the existing HIR cleanup path. An explicitly moved affine
result transfers to its caller and remains usable there. Typed-loop results use structured
`SemanticType`, reject `never` recursively, and resolve stable nominal and binder identities in the
owning callable. One iterative staging walk derives the nearest published loop from the immutable
target ancestry, enters and exits new draft loops/whiles, and assigns private HIR loop identities
before lowering transfers; it creates no public loop identity or target edge. Break requires one
exact non-divergent payload, while requires `unit`, and both break and continue are `never`-typed.
Complete-HIR validation independently checks unique per-callable loop identities, nearest active
targets, and exact payload types. Loop-local affine owners clean before exits and backedges; a private
ownership control stack checks every transfer edge independently, permits local temporary moves or
borrows whose projected loop-header state is unchanged, and rejects a changed outer-owner state or
loop-carried lexical loan. Mutable initializers activate their binding only for the body, affine live
overwrite is rejected, and affine reinitialization after move/drop follows the canonical place
termination, initialization, and cleanup route. Generic drafts identify binders by stable parameter
entities and provide exact structured type arguments; the same resolver validates importer-inferred
and explicit substitutions, argument types, ownership/reference restrictions, and trait bounds, then
derives auto
or explicit implementation witnesses. Each ordered match arm owns a flat `PatternDraft`; wildcard,
named binding, Boolean literal, I64 literal, exact product, and non-generic enum-variant nodes lower
through the canonical usefulness, exhaustiveness, match-plan, ownership, memory, SSA, and VM path.
Product nodes select the exact stable product and every stable field identity once; submitted field
order is non-semantic, while HIR and query fields follow declaration order. These closed patterns
compose under product fields and enum payloads. Nested structural enum checks retain active-variant
provenance through the canonical short-circuit SSA merge chain. Payload bindings are stable public immutable-local
entities. Compiler-only scrutinee and field-projection locals have an explicit hidden binding kind
and never enter entity/search/constructor results.
Malformed/disconnected/cyclic/reused pattern or expression trees, duplicate handles/names/fields,
unknown or duplicate declaration-local type binders, invalid or unused binders, malformed bounds,
foreign/stale/wrong-kind type and trait identities, forward or cross-arm binding uses, field
coverage/type failures, empty/nonexhaustive/useless arms, incompatible arm results, invalid mutable
storage/kinds/types/scopes, non-Boolean loops, divergent return values, wrong callable return values,
foreign/stale/deleted callees beneath returns, control transfer outside a loop, non-exact or
divergent break payloads, invalid loop result types, unreachable sequence/loop body entries, and
contradictory overlapping or deletion-owned edits reject. Generic patterns, unresolved generic
forwarding, ownership/reference generic instantiation, and executable placeholders remain absent. Imported and source-free mutable-local subtrees share stable
identity, lexical visibility, ordinary replacement, tombstoning, and compaction lifecycle behavior.

The authoritative `SemanticProgram` permits absent `main`, real hole expression leaves, one explicit
`UnresolvedValueReference` leaf, and durable semantic `Match` nodes linked to canonical match plans.
The unresolved leaf is childless, carries only a validated requested-name hint, has known expected
type and unknown effects, and contains no selected binding or executable fallback. Missing body and
typed-hole metadata describe hole leaves; snapshot-side unresolved records derive revision, owner,
context, type, and visible stable entities from the same semantic node. No prior expression survives
introduction. Match arm/body relationships remain directly
queryable, and scrutinee, arm-body, or whole-match nodes use the ordinary targeted edit/hole
operations. Complete-HIR derivation iteratively replaces each semantic match with the existing
canonical `Let`/ordered-`If`/`MatchUnreachable` lowering; memory, SSA, bytecode, and VM layers never
accept an unlowered semantic match. Effects use an explicit unknown fact while holes or unresolved
references remain and are recomputed after every semantic-program-changing or mixed transaction.
`SemanticProgram::try_complete` independently rejects either incomplete leaf before deriving
compiler HIR. A nonempty transaction made
only of `RefineHole` operations validates the same namespace, identity, exact expected type, and
nonempty goal through one narrow path; it stages the allocator and hole records, rebuilds the
goal-bearing diagnostics, and shares the unchanged semantic program, indexes, and blockers.
Repeated refinements of one hole publish one base-to-final semantic diff; returning to the original
goal or refining once to the existing goal emits no semantic change. Diagnostics are snapshot-derived
facts beside holes/blockers rather than part of semantic index storage. Shape, lexical scope, type,
usefulness, and exhaustiveness preflight lower once into staged
semantic state; canonical complete-HIR ownership validation decides move/borrow legality and cleanup
before publication.
Unresolved introduction accepts a typed body hole or ordinary typed expression, preserves its root
node identity, removes the displaced subtree/record, and emits an explicit semantic diff. Resolution
uses the ordinary one-node `DraftNode::Load` lowering path, so stable target identity, lexical
visibility, value-binding kind, exact type, copy safety, and storage have one authority. It preserves
the root identity, creates the normal reference edge, removes the unresolved blocker/diagnostic, and
emits explicit resolution plus ordinary reference rewiring. Whole-program ownership validation runs
in the same transaction when resolution makes the snapshot complete; if another incomplete node
remains, the later completion edit runs it before publishing a complete revision.
Requested spelling remains unchanged across candidate rename; current candidate names and exact
case-sensitive name equality are revision-derived. Multiple candidates are likewise a derived query
classification, not a stored incomplete state. A one-item page plus continuation exposes that choice
without duplicating candidate authority. The implemented edit surface cannot add, move, or directly
delete an in-scope binding while preserving the unresolved site, and it has no consumer for a
deliberately deferred finite subset; explicit stable-identity resolution is the complete current
workflow.

`RenameEntity` changes only the selected declaration or member's presentation name. Product types
carry private dense `ProductId`; enum types carry stable `EnumId` plus explicit arguments; memory
planning and SSA retain identity-bearing nominal metadata. Canonical memory blocker/drop paths carry
stable product-field identities plus ordinals instead of field names. Generic memory-witness
parameter and substitution inspection is iterative at user-controlled type depth. Product/member and
enum/member stable identities, enum runtime-layout identity, selected aggregate operations, match
plans, references, nodes, and old snapshots remain unchanged. Global nominal rename rejects reserved
or colliding names;
product fields and enum variants reject sibling collisions, enum fields reject collisions inside
their variant, and every same-name rename rejects. Focused source-free and imported match evidence
compiles and executes after rename without parsing or source loading; an eligible renamed product
also enters baseline native exactly once and returns the same result. Failed rename leaves the
published `Arc` and future allocation unchanged.

Failure preserves the exact `Arc`, revision, diagnostics, projection, tombstones, and deterministic
future IDs. Replacement, hole introduction, and unresolved-reference introduction remove local-
defining `let`, imported mutable-local, and semantic-match subtrees. Return insertion recomputes derived sequence, conditional, local-body,
and match result types through the affected callable and refreshes canonical match arm/result facts
before validation. One iterative staged compaction visits mutable initializers
before activating their bindings, prunes unreachable bindings and match plans, rewrites dense
binding/plan references, and rebuilds per-callable places, slots, and local counts before canonical
validation. Removed local and payload identities tombstone; unaffected entities, nodes, holes, and unresolved
references retain identity across private relocation. Ancestor or owner removal prunes contained
unresolved records; old snapshots remain queryable.

`DeleteEntity` supports `main`, ordinary imported or source-free non-builtin functions, and
user-defined products and enums. Callable deletion owns type parameters, value parameters, locals,
payload bindings, nodes, holes, hidden match descendants, plans, and function-layout participation.
Product deletion owns its fields and explicit implementations targeting that product; enum deletion
owns its type parameters, variants, and fields. Direct contained-member, trait, and implementation
deletion remains unsupported.

Deletion dependencies are checked against the final staged semantic state rather than base indexes
or edit order. Surviving signature, field, expression, hole, generic-witness, and match dependencies
reject deterministically; explicitly deleting their independent owners or structurally removing body
uses in the same batch succeeds. One compaction boundary rewrites dense product, implementation,
binding, plan, slot, and place IDs, including nested product identities in signatures, declaration
fields, expressions, generic substitutions/witnesses, match plans, holes, and unresolved-reference
expectations; relocates private enum/product/implementation vector addresses; and preserves survivor
public entity and node identities. Stable product/member semantic identities
and enum/variant/field/layout identities are not compacted. Deleted identities tombstone, old
snapshots remain valid, later same-name recreation receives fresh generations, and same-batch
same-name recreation is rejected. Deleting `main` still yields `MissingEntryPoint`.

`MoveSequenceChild` now reorders one live direct child inside one live semantic sequence by stable
sequence, child, and optional sibling-anchor identities; `None` appends, removal precedes insertion,
and semantic no-ops reject without publication. One transaction-local block permutation maps the
sequence plus every moved or shifted child-subtree node to its new private address and composes with
ordinary survivor/entity reconciliation, so ancestors, holes, unresolved references, and all other
survivors retain identity through preorder or callable-root relocation without allocating or
tombstoning public identities. The semantic diff reports one stable sequence/child movement with
stable old and new predecessor/successor neighborhoods and emits no numeric position, replacement,
descendant, reference, or call noise. Full type,
effect, index, incomplete-context, and complete HIR ownership/cleanup validation remains canonical.
One move may batch with unrelated rename, complete product or enum creation, or deletion work;
multiple moves, same-callable structural edits, an absent entry point, creation of another incomplete
callable, and final incomplete state in another callable are rejected in this first vertical. This
prevents an unrelated blocker from suppressing canonical validation of an otherwise complete moved
owner. Old snapshots preserve their old order. Containment and projection expose the new order;
incomplete snapshots remain blocked before HIR derivation, and complete source-free or post-import
snapshots compile directly. Focused source-free/imported final-order and bytecode/VM evidence agrees,
source loading and parsing remain zero after the semantic edit, and eligible moved snapshots execute
once through baseline native or the unchanged VM after a pre-entry decline. A valid reorder of two
independent affine-owner moves enters baseline native once, performs two allocations and two drops,
and ends with no live owner, loan, release backlog, stale/forged failure, or teardown failure.
Cross-parent and broader movement remain absent.

Completeness blockers distinguish missing entry point, missing body with declaration/hole/type,
typed hole with hole/type/owner/context, and unresolved value reference with node/requested name/type/
owner/context. The unresolved diagnostic code is `workspace.unresolved-value-reference`. Incomplete
snapshots remain fully queryable and projectable.
`compile_snapshot` returns those revision-labelled blockers before deriving HIR or entering memory,
SSA, bytecode, or runtime phases. A complete snapshot derives one source-optional HIR, installs fixed
compiler-owned core context only in that derived compiler value when needed, validates consistency,
and lowers directly. Selected source-free scalar, nominal aggregate, immutable/mutable lexical-local,
counted-loop, typed-loop/break, nested-continue, affine break/continue cleanup, early-return
ownership-control, byte-vector-borrow-then-move, enum-payload-match, and exact generic-call edits enter
source loading and parsing zero times, retain canonical memory-plan
obligations, compile to validated bytecode, execute in the VM, and clean up on normal and trapped
paths. The source-free ownership-control equivalent also enters the selected baseline-native path,
returns `7`, allocates and drops one unique owner, and ends with no live owner, loan, release backlog,
or teardown failure. Imported and source-free typed loops also enter the same baseline-native path
and return `42`; the affine-continue pair performs three iteration-local allocations and three drops,
returns `3`, and ends with no live owner, loan, release backlog, or cleanup failure. Canonical
memory-plan origins explicitly tag source-backed and source-free cases; current package locks reflect
that non-colliding encoding.

Revision-labelled queries implement deterministic pagination, definitions/references, calls,
structured entity/function/node types, diagnostics, hole context with exact lexical and arm-local
visibility, unresolved value-reference state, filtered copy-load candidates, expected- and control-
context-filtered legal constructors including typed loop, exact break payload type, continue, early
return, and canonical direct-operation candidates marked as requiring submitted-argument validation,
exact generic signatures and call instantiations, node
semantics, and
a structured `MatchView` containing the scrutinee, ordered arms, arm-body nodes, pattern types/kinds/fields, stable
enum/member identities, and payload-binding entities. Node semantics expose kind, actual/expected
type, canonical operation identity, and named effect flags. Generic views expose stable binders and
trait identities, canonical substitutions, instantiated parameters/results, derived witnesses, and named effect flags
without compiler-dense IDs or binder strings. Copy loads are advertised only for copy-safe values;
affine move/borrow candidates are marked `RequiresOwnershipValidation`, and unsupported generic enum
constructors are omitted. Unresolved candidates include stable entity, current name, kind, declared
structured type, exact requested-name equality, and `RequiresCanonicalValidation`; exact-name
matches sort first, then current name and stable identity. Continuations bind the unresolved node as
part of the query identity. Hole and unresolved visibility refresh after semantic revisions.
Projections render state and blockers before selected entity/body/type/reference/hole/unresolved-
reference/match sections, including declared entity types, operation
identities, node effects, and arm/pattern structure, use stable review-local labels, and require no
source attachment. Containment remains in semantic child/evaluation order. Index root-address
resolution performs one map lookup per semantic node, while enum, variant, enum-field, and match-plan
relation indexing uses private identity maps rather than repeated declaration scans. Draft lowering
builds one callable binding-location map, so each stable assignment/load target is one lookup rather
than one full body scan.

The source/path importer privately owns loading, parsing, initial analysis, package validation, and
source provenance capture, then moves all language forms into the same `SemanticProgram`. Fixed
compiler operations/prelude/core traits are excluded from mutable program-entity queries, and HIR
operations carry only canonical catalog operation identity/signature. Imported and source-free
scalar, product, enum, lexical-local, counted-loop, borrow/move, early-return ownership-control, and
exhaustive enum-payload-match fixtures agree on normalized entities and structured types, ordered
containment, references/dependencies, node kinds/types/operation identities/effects, selected
memory-obligation kinds, the main bytecode stream, VM outcomes, traps, and cleanup. The retained
source fixtures are `crates/lkjscript-app/tests/fixtures/imperative-counted-loop.lkjscript` and
`crates/lkjscript-app/tests/fixtures/ownership-control.lkjscript`. Equivalent
imported and source-free generic identity declarations agree on structured binders, bounds, exact
substitutions, instantiated types, witnesses, effects, normalized function and main bytecode, and VM
result. Generic declaration and call edits invoke source loading and parsing zero times. A direct
copy-load program and the equivalent unresolved-introduction/resolution program agree on stable
entities/nodes, containment, references, calls, dependencies, types, effects, memory obligations,
function/main bytecode, and VM result `42`; both source-loading and parser counters remain zero.
A focused ambiguity fixture leaves four visible exact-type copy-safe candidates, derives multiplicity
from pagination, proves that an exact name does not auto-resolve, explicitly resolves either of two
stable choices at the same root, preserves candidate identities across forced private binding
relocation, and excludes a matching parameter created outside the lexical scope. Rename and old-
snapshot assertions keep requested intent distinct from current presentation. Focused generated
candidate pages cover 257 visible bindings without loss or duplication. The 128-level fast fixture and ignored locked-release 20,000-level fixture on a 128 KiB worker stack each
introduce, query, project, replace, compile, execute, and destroy an unresolved leaf at the deepest
selected branch through the same iterative paths. Attachment changes preserve IDs and projection. Separate ignored locked-release fixtures construct
and compile a 20,000-level nested expression, 20,000 alternating immutable/mutable lexical locals,
or 20,000 nested semantic enum matches, then project, execute, and destroy the complete path on a
128 KiB worker stack. Separate
type-only fixtures perform public `SemanticType` construction, clone, equality, hashing, display,
transaction validation/conversion, query, projection, and destruction and creation-only
`DeclarationType` construction, clone, equality, debug, local-binder resolution, stable publication,
signature query, projection, and destruction at 20,000 levels on the same stack without duplicating
the full compiler stress geometry. Draft and pattern
traversal/lowering, semantic type/match derivation, semantic clone, indexing/reconciliation,
projection, and canonical block ordering are iterative on these paths. Bytecode structural-local
classification computes nonowned structural values once per function with linear predecessor-edge
propagation rather than rescanning the CFG for every emitted value.

The retained locked-release harness measures scalar, hole-only, counted-loop, ownership/early-return,
nominal-match, exact-generic mixed, deletion/compaction, and incomplete-recovery authoring loops. At
512 helpers, the 524-node counted-loop transaction/query/projection/compile check has 1.616 ms
transaction and 12.554 ms combined medians; the 538-node mixed generic check has 1.851 ms and
8.280 ms medians. Compilation dominates both. Same-binary full-versus-narrow hole refinement at 513
nodes is 1.153 ms versus 6.98 us per transaction and 1.227 ms versus 84.75 us per selected check.
One-pass work counters and a 2,074-node ignored stress retain the full-recompute path for every
semantic-program-changing transaction. The exact protocol, broader results, output, RSS limitations,
and reversal conditions are in [`performance.md`](performance.md); this evidence supports neither
general incrementality nor a warm service.

The hidden-body hole overlay, test-only HIR construction surrogate, syntax-shaped editing service,
dense source-node identities, protocol/session schemas, text journal/publication path, CLI routing,
unsupported draft placeholders, and unconsumed development semantic digest are deleted. No wire
replacement exists pending a measured consumer.

## Local verification

The documented host and CI boundary runs already-silent Rustfmt plus quiet Clippy, workspace
all-target all-feature tests, and a locked workspace release build. These forms suppress routine
progress while retaining command status and diagnostics. The default owned-cleanup width fixture
uses 640 arguments while dynamically proving every byte- and `u16`-width, cleanup, execution, and
resource-failure boundary; the unchanged 1,024-argument geometry remains an ignored locked-release
stress. This direct fixture split reduced the measured warm native four-command median by 31.0%
without a runner, concurrency, dependency, or coverage-family change. The Docker verification uses
the same Rust feature/target semantics, builds the workspace release once, checks hello without
entering it, and then exercises the retained run and host-capability smoke paths with that built
binary. Its source layer refreshes Cargo-input mtimes before using persistent target-cache mounts, so
a changed Rust test cannot reuse a newer stale executable. Product package/source, benchmark, and
smoke files enter only the verified stage; documentation and policy files are not unconsumed release-
build inputs.

## Known gaps

- Text remains a persistent package/import format, but not a compiler or editing authority. The
  concise projection is review/debug output, not a complete source renderer. Declaration creation
  covers non-generic products and enums, generic and non-generic functions, and `main` with ordered
  explicit capability parameters; expression construction covers immutable and mutable locals,
  ordered sequence, assignment,
  `while`, explicitly typed `loop`, nearest-lexical `break` and `continue`, early `return`, exact calls
  to imported or source-free generic functions, the selected byte-vector move/borrow vertical, and
  exhaustive closed Boolean, I64, exact-product, and non-generic enum matches. Source-free generic
  function declaration authoring
  supports ordered binders, exact builtin or stable trait bounds, nested binder-bearing signatures,
  stable
  lifecycle, and direct compilation/execution. Source-free flat pattern drafts support wildcard,
  binding, Boolean literal, I64 literal, exact product, and non-generic enum-variant nodes. Product
  and field selection uses stable identities, requires every declared field exactly once, and
  publishes declaration-order query fields regardless of draft field order; nested closed patterns
  compose. Generic pattern construction remains an explicit unsupported edit. Source-free nominal
  declaration fields still reject ownership/reference types, but imported ownership-bearing product
  and enum fields remain selectable by stable identity; nested source-free patterns over those
  declarations compile and execute through the same ownership and VM route. Source-free unresolved
  copy-load value references now have complete introduction, inspection, candidate, resolution,
  replacement, owner-deletion, compile-rejection, and execution behavior. Text import still fails
  fast on unresolved source names. Unresolved moves, borrows, calls, type names, nominal members,
  patterns, and imports; ambiguity, conflict, and parser recovery; nominal member
  addition/deletion/reordering; and cross-parent, entity, declaration, match-arm, branch, loop-body,
  callable, and generic public movement remain gaps. Generic ownership/reference instantiation and
  forwarding a caller's unresolved type parameter remain narrow explicit unsupported cases. Direct
  operation drafts exclude control, numeric-conversion, and enum-construction operations that
  require dedicated HIR forms. A retained source-free recursive factorial plus capability-bearing
  main compiles directly, prints the same `3628800` bytes as the imported hello oracle, and invokes
  source loading and parsing zero times.
- There is no persistence, journal, wire service, or collaboration layer for workspace snapshots.
  Add one only after a measured consumer establishes the boundary and resource policy.
- Owned runtime structural values and source-free nested-expression, alternating immutable/mutable
  lexical-local, typed-loop-control, nested enum-match, and public semantic-type workspace paths have
  20,000-level release evidence on a 128 KiB worker stack. This does not prove every compiler form,
  internal type traversal, ownership failure, or general runtime throughput path stack-safe.
- The SSA evaluator is an explicit test oracle behind `lkjscript-ir/test-oracle`; it is not a public
  runtime engine. Workspace `--all-features` verification compiles it for tests.
- Compact native layouts, machine-code offsets, registers/opcodes, OS fields, SQLite fields, and host
  `usize` remain private or external representation boundaries. Native lowering must decline to the
  generic VM before entry when it cannot represent an otherwise supported program.
- Daemon, multi-tenant database, distributed, scheduler, and broader platform products are absent by
  design until the local semantic model and measurements justify them.
- The representative five-sample selected-product and semantic-workspace baselines in
  [`performance.md`](performance.md) cover process wall, approximate process-tree RSS, compiler and
  runtime phases, typed native declines, published native code/mapping sizes, exact outcomes,
  cleanup/host effects, and source-free scalar, imperative, ownership, nominal-match, generic-mixed,
  lifecycle, and incomplete transaction/query/projection/compile latency at up to 512 helpers and
  538 representative nodes, plus one 2,074-node generated stress point. Total allocator counts/bytes,
  exact retained snapshot bytes, broader query density, other targets, and application-scale
  steady-state throughput remain unmeasured.
