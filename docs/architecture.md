# Active architecture

## Source and compiler flow

The current compiler authority starts at package files and the provisional line-oriented text
projection. The source loader validates paths and imports, the analyzer creates resolved typed
HIR, ownership/effect/memory passes derive executable obligations, SSA lowering creates a typed
control-flow program, and the IR verifier publishes an opaque verified program. Bytecode lowering
and validation provide the generic executable representation used by the runtime. This trusted
pipeline has no compiler resource profile or cross-phase budget ledger; source-file counts, phase durations, and HIR memory-plan expression, entry, constant, and verifier
work are observation only. Ownership analysis does not
perform a separate aggregate-expression admission scan. Source parsing and source-tree operations
use explicit work stacks. Recursive expression mechanisms whose control and mutable compiler state
make immediate continuation-stack rewrites disproportionately invasive use the localized `stacker` boundary,
which repeatedly adds heap-backed stack segments and therefore defines tuning geometry rather than
a finite accepted depth. Recursive source and HIR ownership is dismantled by custom non-recursive
destruction. SSA ownership verification admits wide semantic state without work or retained-cell
budgets. Single-source worklist states move into block processing; join-capable states share
copy-on-write ordered fact sets and maps, so unchanged CFG propagation does not copy whole states.
Join comparison remains exact. Compiler `Type` and IR `SsaType` shape verification, ownership
queries, canonical identity, formatting, clone/equality/hash, and destruction are stack-safe on
user-controlled nesting through explicit worklists or localized repeatable stack segments.
Compiler and SSA auto-trait solvers intern canonical type nodes, memoize obligation states, and
propagate false facts over deterministic reverse dependency edges from an initially true greatest
fixed point. A nominal cycle therefore satisfies an auto trait unless a reachable intrinsic or
field dependency disproves it. Enum recursion and HIR memory-plan type graphs use explicit graph
worklists. Producer and independent-verifier graph construction iterate declaration storage once
and resolve edges through indexes rather than repeatedly scanning all declarations. Type-node,
graph-edge, witness/group/dependency, structural metadata table, semantic descriptor, and type-SCC
work counts are not admission rules. HIR memory-plan function/use/loan/call/obligation,
witness-arity, destination, borrow-scope, drop-path, SSA region-product, and executable structural
reference quotas are also gone. Producer accounting is checked observational `u64` telemetry; the
independent verifier reconstructs exact totals and equality without an admission comparison.
Source positions/spans and revision-scoped source-node indexes are `u64`; Semantic Source JSON
requests, responses, transaction relations, and declaration/source-order references preserve that
width. HIR source, binding, trait, implementation, place, loan, loop, match-plan/arm/path, and all
memory-plan dense identity domains use `u64`. Memory witness parameters/bindings and group
ordinals, local witness dependency targets, HIR destination/borrow-scope/drop-path/drop-glue IDs,
and executable structural type/layout/representation/destination IDs use checked host conversion. Unresolved witness grouping
and synthetic origins are typed variants rather than reserved numeric values. Placement retains only two private seal-selection thresholds;
when they are not met, the total generic placement route is selected. They do not reject a program.

The memory-plan producer indexes expression entries, destination children, source places, direct
functions, declaration metadata, binding loads, and loan uses. Its implementation is organized by
lifecycle and expression dispatch, expression walking and place registration, and record
publication, while the producer root owns shared type and memory-authority support. The former
mechanically numbered producer fragments and their include path are gone; there is no parallel
producer implementation. Placement precomputes binding-use,
independent-owner, branch-divergence, and aggregate-estimate indexes. The independent verifier
builds separate expression/child/use/load, call/place/scope, loan-entry, destination-child,
declaration, witness, and drop-path indexes and still reconstructs authority rather than trusting
producer indexes. SSA lowering and bytecode installation use indexed witness, placement, owner,
layout, and structural-destination routes. Deterministic vectors remain canonical; hash indexes are
lookup accelerators only. Checked telemetry and allocation failure remain explicit. Match usefulness is a deterministic
explicit-continuation state machine over interned pattern and matrix arenas. Default matrices,
constructor specializations, and complete usefulness states are memoized through lookup-only hash
indexes; constructor and witness order always comes from declaration and source vectors. Witnesses
remain flat until a fallible iterative renderer produces the complete semantic diagnostic. Match
patterns use stack-safe clone/destruction, recursive pattern parsing, lowering, and verification use
the localized repeatable heap-stack boundary, and match-plan flattening plus HIR placement-fact
collection use explicit stacks.

SSA control-flow verification indexes sorted successor and
predecessor adjacency once, uses iterative DFS/SCC worklists, computes immediate dominators in
reverse-postorder, and publishes dominator-tree intervals for constant-time dominance queries.
Neither block count nor CFG work is a validity rule. Active-enum provenance consumes the same
indexed graph and preindexed values through explicit visited worklists. Bytecode validation separates well-formedness from resource policy:
trusted compiler output and prepared binding select `ValidationPolicy::Unrestricted`, while an
explicit untrusted artifact boundary may select a limited policy over only the checked total-byte
observation. There is no finite default or per-table, metadata, cleanup-action, or constant-data
admission policy. Failure cleanup is a deterministic hash-consed arena in SSA and
bytecode: each node stores one action and an optional backward-only link, while instruction and
range metadata store optional loan, unplaced-owner, and place roots. The fixed segmentation
preserves cleanup order without reconnecting an unchanged owner segment whenever another segment
changes. Code generation pre-indexes SSA value types, definitions, and moved call arguments, then
maps nodes in producer order without iterating hash tables or searching whole plans.

Executable-width ownership is explicit at each boundary: analyzer/HIR arity, local storage, and
codegen colors use host indexes. SSA function/block/value/binding/trait/implementation/place/loan
IDs, source/origin links, frame-state slots and bytecode positions, and bytecode links use `u64`;
all vector access converts through `usize::try_from` or an equivalent checked helper. Bytecode constant
and global IDs, prototype constants, global-prototype metadata, call-witness/link prototype
references, and runtime function/symbol/static-constant references use `u64`. Constant/global
interners own hash-backed lookup indexes alongside insertion-order vectors, including exact-bit
floating keys and owned text/bytes keys. Hash iteration is never canonical order. Bytecode operand
metadata selects one of no operand, retained `u16`, fixed `u64` index, or two independent fixed
`u64` place/local indexes. Branch targets, constants, globals, and closure operands use the fixed
`u64` index layout. The decoder checks complete operands and host conversion before publishing
`DecodedInstruction`; validators and the VM consume the typed decoded shape or the same fixed
layout. Bytecode links, call-witness instruction offsets, cleanup roots, and cleanup range offsets
use canonical `u64` values with checked host conversion. Validator ownership tracking keeps
instruction offsets and parameter indexes in tagged host-width identities rather than narrowing
them to `u32`. Nominal product/enum identities, source-order values, physical enum tags,
aggregate field indexes, and product/enum/structural bytecode table and descriptor references use
`u64` with checked host conversion. Aggregate descriptor interners pair insertion-order vectors
with hash indexes, avoiding repeated whole-table scans without making hash iteration canonical.
Native source-function, source-origin, function, block, value, local, static-bytes, product, and
frame-home identities are wide. Native layout identities separate builtin, structural, capability, resource, loan, unique, and
nominal-product domains. Native reference-layout hashing includes the typed domain, full `u64`
value, and nominal digest instead of collision-prone arithmetic tags. Native frame ordinals and trap, heap, and structural runtime-site identities remain `u64` across
generated code and runtime dispatch; an explicit presence word carries optional trap sites without
reserving a numeric value. Executable installation fallibly builds a sorted wide source-ID to
installed-entry mapping before publishing W^X state. Invocation entry counters are indexed only by
checked local ordinals, and invocation-local active-frame vectors grow under explicit execution
policy instead of imposing a native source-count or depth ceiling. Unrestricted native invocation
has no aggregate or per-frame project-selected byte ceiling: each generated prologue uses checked
frame arithmetic and the current thread's discovered stack extent and guard. Limited execution
forwards its explicit remaining frame and VM-value policy to native active-frame and value-home
accounting; native stack bytes are not a hidden policy category. Automatic recursive call groups
remain on the heap-framed VM route because retrying a partially executed recursive native group
could duplicate effects. For an acyclic automatic group, JIT planning computes the checked longest
generated call-stack requirement and executable invocation checks it before the first frame enters.
A discovered OS-stack/guard boundary or native stack-representation failure then returns a typed,
retry-safe optimization decline, invalidates the code object, and resumes the unchanged VM call;
no native poll, effect, frame publication, fuel debit, or deadline reset has occurred. A boundary
discovered after a forced native frame has entered is reported explicitly and is never retried.
Machine-code offsets, x86 frame sizes/displacements, registers, opcodes, and closed ABI tags remain
checked narrow machine boundaries. The native planner retains compact calling-convention aggregate
eligibility where required; checked preflight declines unsupported shapes and automatic execution
uses validated VM bytecode.

Trusted package loading uses the same resource separation: manifests and canonical lock files are
read to EOF with checked byte accounting and fallible chunk growth, without a category-specific or
aggregate local byte quota. Open-file and path metadata are compared before publication to reject
growth, shrinkage, or replacement during a read. Package graph construction is deterministic
iterative postorder DFS. A hash index owns each canonical path's visiting or completed state, an
explicit frame stack owns pending dependencies, and the final package vector is sorted by content
identity before canonical lock encoding. There is no dependency count or depth admission rule;
allocation, host-width, and I/O failures remain typed host failures. Package schema, digest,
ordering, cycle, path containment, exact module declarations, and symlink checks remain semantic or
security validation.

The intended cutover is described in [`source-model.md`](source-model.md): text becomes an importer
and renderer around an immutable semantic snapshot, and compiler analysis consumes that snapshot
directly.

## Execution

The default CLI mode is `auto`. It always has a validated generic VM route and may install native
code for eligible groups. A high arity, local count, or cleanup shape that the current native machine plan would expand
quadratically does not reject generic compilation: native preflight marks the function ineligible,
automatic execution stays in the VM, and a forced native diagnostic reports the unsupported
shape. Prepared identity records native
transport specialization as optional and does not equate semantic SSA identity with generated-code
support. Baseline and optimizing selections remain available as diagnostic modes while the reset
measures their representative value. They are not separate language definitions.
The SSA evaluator remains useful as a semantic test oracle but is not the product source authority.
`ExecutionPolicy` is deliberately non-defaultable. Ordinary local `lkjscript run` selects
`Unrestricted`; isolated process manifests and workers must select `Limited` explicitly. Limited
policy contains only coarse fuel, VM value/frame, heap/allocation, handle, output, wall-deadline,
hard-deadline, and cleanup-report-retention resources. Unrestricted checks are absent rather than
represented by integer sentinels. Automatic VM/native transitions preserve the policy and forward
remaining fuel, wall time, VM frame capacity, and value capacity. A retry-safe native optimization
decline consumes none of those resources before VM continuation. Cleanup retention controls only
reported host output and never stops
cleanup work. The independent logical-aggregate-construction count is removed.
List equality uses explicit work stacks in evaluator, VM, baseline-native, and structural-native
services; allocation failure is resource exhaustion, not a comparison-count trap. Unique byte
storage preflights `ExecutionPolicy`/`EvalConfig` heap and allocation policy, reserves fallibly, and
rechecks actual retained capacity at publication. Bulk file/socket byte views are processed with
private syscall chunks. Read/write preserve their partial-count contract, socket string send loops
for its full-transfer contract, and output, wall-deadline, and cancellation policy is checked at
chunk boundaries. Structural region, sealed-region, root, unique, segmented-list, region-product,
value-image, semantic-DAG, and returned-snapshot stores grow without independent ordinary count or
local-return ceilings. Checked allocation-event, retained-byte, and export observations feed the
same coarse `ExecutionPolicy` in VM and native execution; policy failure occurs before outcome
publication and cleanup retains no owner. Structural validation, snapshot conversion/codecs, graph
walks, and destruction use explicit work stacks. Structural runtime slots/generations, semantic-DAG
IDs/counts, region-product records, segmented-list coordinates, native aggregate fields/projection
paths, and witness locators are wide. Narrow host indexing is checked before access. Public runtime
values retain a closed one-word ABI: owner, borrow, view, destination, adapter, resource, unique, and
list handles are opaque nonzero `u64` tokens resolved through runtime-owned maps to wide identities.
Tokens are never decoded arithmetically, and a consumed token remains stale after slot reuse.
Outcome and process fields use canonical little-endian `u64` lengths and IDs. Outcome decoding has
only explicit total-wire-byte policy; process framing remains a separate explicit transport-byte
boundary. Construction reserves and validates before publishing token mappings, so policy,
allocation, and malformed-input failure leave prior state intact.

No optimization result is allowed to reinterpret source semantics. Forced native modes must enter
a synchronous generated entry or fail explicitly.

## Package ownership

Cargo metadata is the authority for workspace membership and dependency edges. Conceptual
ownership is:

- `lkjscript-core`: bytecode, values, execution/resource policy types, validation, and shared
  runtime contracts;
- `lkjscript-compiler`: package/source loading, analysis, typed HIR, memory planning, SSA lowering,
  bytecode generation, and the bootstrap Semantic Source service;
- `lkjscript-ir`: typed SSA model, verification, normalization, and evaluation;
- `lkjscript-vm`: validated-bytecode execution and runtime value/storage machinery;
- `lkjscript-native`, `lkjscript-jit`, and `lkjscript-executable`: native planning, tiering/code
  generation, and the executable-memory mechanism boundary;
- `lkjscript-runtime` and `lkjscript-resource`: daemon/process execution, scheduling, control,
  retained runtime state, and operational resources;
- `lkjscript-host`, `lkjscript-sys`, `lkjscript-linux-host`, and `lkjscript-database`: safe host
  interfaces and narrow operating-system/SQLite mechanisms;
- `lkjscript-contracts`: boundary schemas and content identities still used by packages, process
  messages, prepared programs, and executable artifacts; it owns the safe incremental SHA-256
  state re-exported by core, and canonical SSA, bytecode, and prepared identity construction feeds
  that state without buffering complete descriptors; compiler resource-category/profile
  descriptors are not part of the registry;
- `lkjscript-app`: CLI, daemon binaries, provider wiring, diagnostics, and integration tests.

The removed `lkjscript-xtask` crate had no product consumer. Formatting, Clippy, tests, release
builds, and application smokes now run directly.

## Trust boundaries

Validation remains fail-closed where data or authority crosses:

- source, package, manifest, semantic-operation, and path-containment boundaries;
- persisted package locks, prepared programs, runtime-control stores, and serialized messages;
- daemon/process-cell framing and provenance;
- capability grants and host providers;
- bytecode and executable-IR deserialization;
- relocation, W^X code installation, native entry, FFI, and SQLite/OS calls.

Semantic Source request bytes, response bytes, session frame/cumulative bytes, request count, and
cancellation are boundary-local host policy. A caller accepting untrusted bytecode may likewise
apply one explicit total-artifact-byte policy; malformed-bytecode validation is identical under
limited and unrestricted modes. Semantic node, hole, transaction, HIR, SSA, bytecode-table,
metadata, and constant counts do not grant language validity.

Within one synchronous typed compiler pipeline, ordinary Rust ownership and opaque verified
wrappers should replace repeated governance identities as later slices reach them.

## Unsafe code

Workspace crates forbid unsafe code by default. The `lkjscript-sys`, `lkjscript-linux-host`, and
`lkjscript-executable` mechanism crates explicitly own FFI, Linux host observation, executable
mapping, relocation, and generated-entry calls. Their public callers are safe APIs with focused
boundary tests. Future work should narrow lint allowances further as these crates are
consolidated; unsafe code must not spread into semantic compiler or product-runtime logic.

## Persistence and processes

Package locks, prepared descriptors, control-store snapshots, and process frames are validated
serialized boundaries. Compiler HIR, verified SSA, bytecode under construction, and JIT plans are
in-process structures. The future semantic snapshot will begin in memory; persistence will be
added only after edit-latency, retained-memory, concurrency, or crash-recovery measurements justify
it.
