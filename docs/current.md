# Current implementation

This document describes implemented behavior in the checkout. It is not a promise of backward
compatibility.

## User-visible capability

- `.lkjscript` is the only accepted source suffix. Its current one-marker-per-line notation is a
  bootstrap projection, not the permanent semantic schema.
- Packages and modules use checked manifests, lock data, deterministic import resolution, path
  containment, and cycle rejection.
- The implemented language includes typed functions and calls, local bindings and mutation,
  conditionals and loops, products, enums, exhaustive matching, generic `Option` and `Result`,
  numeric conversions, bytes, byte vectors, lists, typed errors, and explicit capabilities.
- The command-line runtime supports a default automatic path and diagnostic `vm`, `baseline-jit`,
  and `optimizing-jit` selections. Forced native selections preflight support and report failure
  rather than claiming generated execution after fallback. Native entry accounting uses a
  fallibly installed wide source-ID mapping and invocation-local counts keyed by installed entry,
  so a hot eligible function above source ID 63 can enter native code. Active native frames grow
  dynamically under an explicit `ExecutionPolicy`. Unrestricted invocation has no project-selected
  aggregate or per-frame native stack-byte ceiling; generated prologues use checked arithmetic and
  the current thread's discovered stack extent and guard. Automatic recursive groups stay in the
  heap-framed VM. Acyclic automatic groups precompute and check their complete native stack
  requirement before entry, so a typed stack/bookkeeping decline can invalidate the optimization
  and retry in the VM without duplicating effects. Forced native mode reports an actual external
  stack boundary explicitly.
- Host adapters cover standard I/O and selected filesystem, TCP, hashing, terminal, and SQLite
  operations behind typed capability checks.
- Ordinary runtime memory is collector-free. Unique storage, invocation regions, structural
  values, and explicit host-resource ownership are implemented for the supported value families.

The executable examples under `src/examples/` and compiler/runtime tests own the exact supported
surface.

## Compiler and runtime foundations

Current text compilation follows this path:

```text
line-oriented text and package files
    -> source tree and resolved typed HIR
    -> ownership, effect, and memory-plan checks
    -> independently verified and normalized SSA
    -> validated bytecode
    -> default VM/native execution path
```

Trusted compiler entry points compile directly without selecting a compiler resource profile or
charging source, HIR, or SSA shape to a budget ledger. The lexer-token, children-per-form,
top-level-form, source nesting, 16 MiB per-file source, 256 MiB aggregate-source, and 65,536
source-unit validity ceilings have been removed. The parser uses fallibly grown explicit frames;
source projection, identity flattening, formatting, module rewriting, clone, and destruction use
iterative work stacks. Remaining recursive expression analysis, HIR memory planning, and SSA
lowering call sites use localized repeatable heap-backed stack segments rather than a finite depth
admission rule. Trusted validation, loading, package analysis, and compilation select an explicit
unrestricted source-byte policy. Source files are read to EOF in checked, fallibly reserved chunks;
metadata is a capacity and change-detection hint, not admission. Ownership analysis no longer
pre-scans HIR merely to reject an aggregate expression count. HIR memory-plan expression work is
checked observational `u64` telemetry rather than admission. SSA ownership verification likewise
has no ownership-work or retained-state-cell admission ceiling. Its worklist moves single-source
states and shares copy-on-write state across joins, while exact predecessor-state equality and all
ownership cleanup rules remain enforced. SSA CFG verification has no per-function block or
verification-work admission ceiling. It indexes deterministic successor and predecessor lists once,
uses explicit-stack DFS and SCC traversal, derives immediate dominators in reverse-postorder with
the Cooper-Harvey-Kennedy algorithm, and answers dominance through dominator-tree intervals rather
than dense per-block bitsets. Active-enum provenance uses the same predecessor index plus
preindexed values and explicit visited worklists; its checked step count is observation only.
Generated raw SSA coverage verifies 10,000-block linear, branch/merge, and cyclic functions and
rejects malformed high-block dominance and target edges. An ignored release stress test compiles
source into more than 4,096 SSA blocks, publishes bytecode, and executes the result through the VM.
Generated SSA coverage verifies 44,000 owned resource
parameters and places with 132,000 active-place, owner, and affine state cells per block, 264,000
cells under the former aggregate retained-state accounting across a two-block propagation, one
44,000-argument consuming call, and exact cleanup of every place. Match usefulness and
exhaustiveness no longer use computed specialization-work or witness-byte reservations. The
checker interns patterns and deterministic matrix rows, memoizes default and constructor
specializations, and evaluates continuations on fallibly grown explicit stacks. Canonical witnesses
use a flat arena and a fallible iterative renderer, so compiler diagnostics remain complete rather
than being rejected or truncated by a preallocation estimate. Generated coverage executes a
32-product-deep exhaustive match through validated VM bytecode, diagnoses complete and malformed
2,048-product-deep patterns on a 256 KiB native stack, and emits a complete 300-field missing-variant
witness beyond the former 32,832-byte single-pattern reservation. The same broad fixture preserves
useless-arm detection. Match lowering's placement-fact walk is iterative, while producer and
verifier structural placement estimates use repeatable heap-backed stack segments for nested
nominal types. Compiler and SSA type validation
no longer use depth or work admission fuel. Their type shape, substitution, ownership, identity,
formatting, clone, equality, hashing, and destruction paths use explicit worklists or repeatable
heap-backed stack segments. Auto-trait solvers canonicalize type nodes, memoize obligations, and
compute a deterministic coinductive greatest fixed point, including nominal cycles; enum recursion
validation likewise uses graph worklists rather than depth fuel. HIR memory planning and executable
witness validation no longer cap functions, uses, loans, calls, obligations, witness parameters or
arguments, destinations, borrow scopes, drop paths, type facts, type-graph edges, semantic
descriptor nodes/bytes, witnesses, witness groups/dependencies, structural metadata tables,
region-product metadata, or deterministic type-SCC work. Producer accounting and independent
verifier reconstruction use checked observational `u64` totals and require exact equality; they do
not compare a program count with an admission maximum. Source bytes, lines, columns, spans, revision-scoped node indexes, declaration/source order, and
Semantic Source JSON node relations use `u64`; source slicing and node lookup convert to host
`usize` before access. HIR source, binding, trait, implementation, place, loan, loop, match-plan,
match-arm/path, and every memory-plan dense identity use `u64`. Memory witness parameters,
bindings, group ordinals, local dependency targets, HIR destination/borrow-scope/drop-path/drop-glue
IDs, and bytecode structural type/layout/representation/destination IDs use the same checked
host-index boundary. SSA
function, block, value, binding, trait, implementation, place, loan, origin, frame-state bytecode
position, and bytecode-link identities are canonical `u64`; synthetic origins and unresolved
witness group membership are typed states rather than numeric sentinels. Residual generic lowering
preserves every independently owned dynamic parameter in declaration order; multi-witness generic
source executes through the validated VM path without owner alias or local-reuse loss. The two
retained seal thresholds select a total generic placement fallback and cannot reject a program. Uses, loan ends,
direct calls, expression entries, source places, destination children, borrow scopes, drop paths,
declaration metadata, and structural destination metadata are preindexed instead of repeatedly
scanning complete tables. Generated HIR coverage derives and independently verifies 65,537 uses
and loans, more than 32,768 obligations, and 32,769 destinations/drop paths. A 16,385-destination
structural geometry reaches verified SSA and unrestricted validated bytecode; separate unrestricted
validator coverage accepts 65,537 destination-field operation references. Generated region-product coverage
verifies 16,385 records. Contract coverage validates 16,385 independent executable witness groups
and semantic descriptors with 16,385 reachable declarations, 65,537 fields/type edges, and more
than 16 MiB of canonical input. Full-source production tests compile 4,097 helper functions through
HIR, SSA, validated bytecode, and VM execution; compile and execute 16,385 direct calls with 16,385
inferred borrow scopes; and transport 17 hidden witness parameters and arguments through the same
path. The VM remains valid when compact native tables decline a large group. On the local
20-logical-CPU AMD Ryzen 9 9955HX host with 32 GiB RAM and `rustc 1.96.0`, release test bodies took
3.60 s for 4,097 functions, 333.24 s for 16,385 calls/scopes, 9.91 s for the HIR use/loan/obligation
and destination/drop-path geometry, 27.24 s for 16,385 structural destinations through validated
bytecode, 0.01 s for 65,537 validated operation references, and 0.02 s for 16,385 verified SSA
region products. The call/scope run peaked at approximately 30.4 GiB process-tree RSS. Its compiler
metrics attributed 140 ms to memory planning, 59.44 s to bytecode validation, and 261.05 s to
preparation, so those downstream phases and peak memory remain measured scale problems rather than
reasons to restore an admission quota.

Generated coverage compiles, creates
a verified HIR memory plan, creates verified and normalized SSA, validates bytecode, executes
through the VM, and destroys a program with 20,000 nested `do` expressions on a 256 KiB native
thread stack. That fixture records exactly 20,001 memory-plan expressions, 20,003 entries, and
40,045 verifier steps. A 512-deep nested list type crosses source analysis, HIR memory planning,
verified SSA, validated bytecode, VM execution, deterministic malformed-type diagnostics, and
destruction on the same small stack. Raw SSA verification covers valid and malformed 8,192-deep
types, while generated 300-wide and cyclic nominal type graphs exercise public compiler paths and
coinductive trait tests. A nested product-match fixture reaches a physical marker depth above 50,
and malformed 8,192-deep mismatched and unclosed input produces deterministic diagnostics and
drops partial trees on the same small stack. Other generated coverage compiles and executes a
source 1,024 bytes beyond the former 16 MiB boundary and exercises the source authority with 65,537
in-memory units. Checked accounting crosses 256 MiB; the exact 258 MiB compile-and-execute geometry
is retained as an opt-in stress test and has not been run as part of normal verification. Compile
metrics are observational phase timings and source-file counts only. Trusted local package
verification reads manifests and canonical locks to EOF in checked, fallibly reserved chunks. File
metadata is only an initial allocation hint and a growth, shrinkage, and replacement consistency
check; it is not admission. The former 1 MiB manifest and 16 MiB lock limits are removed. Package
dependency traversal uses an explicit DFS frame stack plus indexed visiting/completed states, so
cycle detection and shared dependency reuse do not consume the native stack or repeatedly scan all
packages. Generated ordinary check-and-compile coverage uses a valid manifest and canonical lock
larger than 17 MiB, a 512-package chain on a 256 KiB thread stack, and 512 direct dependencies;
the deep fixture also reports a real terminal cycle identically on repeated builds. Package
manifests and prepared-program identities likewise do not carry compiler-profile identity. Trusted compiler
output and prepared-identity rebinding now validate bytecode with an explicit unrestricted mode:
there is no finite default, total encoded-byte ceiling, physical-table count ceiling, metadata-byte
ceiling, or per-constant data ceiling. The boundary-local `ValidationPolicy::Limited` variant checks
only one checked total-byte observation and reports a distinct bytecode-policy failure; the same
chunk remains valid under `Unrestricted`. Bytes-literal decoding checks exact lowercase hexadecimal
syntax and reserves decoded storage fallibly without a constant-size admission rule. Generated
coverage validates a chunk carrying more than 16 MiB of constant data under unrestricted mode,
rejects it under a deliberately low byte policy, and compiles and executes `bytes-length` through
the VM for a literal one byte beyond the former 1 MiB constant limit. Canonical
verified-SSA, validated-bytecode, and prepared-program identity bytes stream directly into one
incremental SHA-256 implementation owned by `lkjscript-contracts` and re-exported by core. Identity
construction has no canonical-byte, append-count, prepared-descriptor-byte, or ordered-closure-entry
admission ceiling and retains a fixed-size hashing working set. Prepared closures still require a
nonempty, nonzero, strictly ordered unique sequence, and canonical sequence lengths remain checked
against their `u64` wire representation.

Executable function arity and local slots now remain `usize` through HIR and code generation; SSA
frame-state slots use `u64`, and bytecode prototypes, owner-place metadata, and failure-cleanup
locals/places use host indexes. The one active bytecode format encodes branch targets, constant and
global indexes, closure operands, local indexes, call argument counts, memory-witness ordinals, and
single place indexes as fixed little-endian `u64`; instructions that name both a place and local
encode two separate `u64` operands. `ConstId`, `GlobalId`, prototype constants, global-prototype
metadata, runtime function/symbol/static-constant references, call-witness prototype references,
and bytecode-link prototype references carry `u64`, with checked conversion before host indexing.
Constant and global interning use hash-backed lookup with insertion-order vectors; floating-point
keys use exact bits, and string/bytes keys own their data, so canonical order never depends on hash
iteration. Bytecode function, block, and instruction links, call-witness instruction offsets,
cleanup node identities, and cleanup range offsets are also `u64`. Nominal product IDs, product
and enum field indexes, enum variant source orders and physical tags, enum substitutions, and
product/enum/structural table and descriptor references use `u64`; every host index conversion is
checked. The 15-field product and 255-variant/field enum admission rules are removed, as is the
aggregate structural-layout-field admission total. Product, enum, and structural descriptor
interners use insertion-ordered vectors plus hash indexes rather than repeated linear interning
scans. Decoding classifies operands as no operand, retained `u16`, checked host index, or checked
place/local pair,
proves the complete operand is present, and rejects host-width overflow before validation or
indexing. The VM
uses checked frame arithmetic and fallible reservations, rejects a low stack host policy before
wide frame allocation, tracks instruction starts independently of encoded length, and tail-forwards high local slots without two- or three-byte instruction assumptions.
Focused generated chunks intern, validate, and execute 65,537 distinct constants and globals,
load/store index 65,536, and construct and call prototype 65,536. Malformed truncated and
out-of-range fixed-width constant operands fail before indexing. An opt-in release production test
compiles source containing exactly 65,537 distinct scalar constants through HIR memory planning,
verified normalized SSA, validated/prepared bytecode, and VM execution; memory-plan entries,
constants, and verifier work are checked observational telemetry rather than admission. Baseline
normalization legally removes overwritten pure scalar stores before bytecode lowering, while the
focused chunk test owns direct wide-operand execution evidence. Generated production coverage also
compiles and executes 300 parameters, 300 arguments, and more than 255 simultaneously live lexical
locals, reads slot 299, agrees with the SSA evaluator, and proves
that automatic execution remains on the generic VM route while forced native mode reports an
unsupported signature. Generated nominal aggregate coverage constructs and updates a 300-field
product, projects field 299, and exhaustively matches a 300-variant enum whose final variant has
300 fields. It verifies a physical tag above 255, executes the high payload field and high tag
through validated VM bytecode, rejects malformed high field/tag references before indexing, and
confirms that compact native aggregate ineligibility falls back to the VM in automatic mode. A
larger generated stress case executes 1,024 parameters, arguments, and lexical locals through the
VM and reads slot 1,023. A separate 1,024-owned-parameter and argument
source emits 115,754 bytes in main, validates and prepares that executable, executes exact cleanup
through the VM, and confirms that automatic execution retains the generic VM fallback when native
cleanup expansion is ineligible. A generated owned parameter in slot 299 also executes and cleans
up. Another generated source emits a function larger than 65,535 bytes with a branch target beyond
that former boundary; calls taking both branch paths execute through the VM. Malformed fixed-width
jumps are rejected when truncated, aimed into operand bytes, outside the function, or unrepresentable
as a host index. Failure cleanup is represented in SSA and bytecode by deterministic
hash-consed backward-only node chains. Ordinary cleanup has separate loan, unplaced-owner, and
place roots so independently changing segments do not copy one another; call-unentered cleanup has
its own root. The production 300-owned-byte-vector parameter/argument fixture publishes and runs in
the VM with 315,450 logically expanded cleanup actions represented by 1,200 physical nodes, while
exercising local and place indexes above 255. Validation rejects duplicate nodes, empty root sets,
self, forward, and out-of-range links before indexing and checks aligned, sorted, nonoverlapping
ranges plus exact live-owner coverage whenever cleanup metadata is present. The native machine-plan
backend still materializes per-instruction cleanup calls, so preflight explicitly declines a shared
shape whose expansion exceeds its private 65,535-call eligibility envelope; automatic execution
keeps the validated generic VM path rather than failing publication. Direct
validated-bytecode coverage executes unique local and place 299 and rejects equal-to-count,
truncated, misaligned-jump, and malformed cleanup references. Prepared identity now records native
transport specialization as an optional identity rather than substituting the semantic SSA identity
when specialization is unavailable.

A Semantic Source service already exposes snapshots, stable node queries, typed holes,
diagnostics, transactions, and a local stdio session. Its untrusted boundary applies coarse
request-, aggregate-source-, and response-byte policy; the same source-byte policy checks staged
transaction source before publication. JSON shape and nesting have no separate admission quota:
strict typed decoding and encoding use dynamically grown Serde stacks, and recursive request values
are destroyed iteratively. Semantic source-node flattening, transaction node lookup, and expression
path validation use explicit work stacks. Hole validation witnesses traverse complete finite product
shapes without a depth cutoff, including product chains beyond depth 8. Candidate and legal-action
responses remain deterministically complete when returned; if the complete response exceeds the
response-byte policy, preparation returns typed `OutputLimit` before any staged publication rather
than emitting a falsely complete partial result. The service has no source-unit, token, node, JSON
nesting, witness-depth, or work admission quota. Other boundary-local byte and request-count policies
remain for untrusted framing and persistence. It currently mirrors the text-oriented source tree and
the compiler still recompiles from text, so it is a bootstrap editing service, not yet the intended
semantic program authority.

## Tested platform

The broad local suite and native paths are exercised on Linux x86-64. Portable Rust components
may build elsewhere, but no other host or native target is currently claimed as tested.

## Known gaps

- Source, HIR, memory-plan, and SSA dense identities are `u64` and use checked host indexing.
  Revision-scoped `NodeId` is deliberately still a bootstrap opaque dense identity: widening its
  wire/index representation prevents aliasing and saturation, but does not provide the edit-stable
  logical semantic identity required in Phase 5. Core structural-image and semantic-DAG node IDs,
  fields, ranges, counts, structural runtime slots/generations, region-product records, native
  aggregate indexes, projection paths, and witness locators are canonical `u64` values with checked
  host indexing. Scheduler IDs, machine-code offsets, register numbers, opcode bytes, OS CPU
  numbers, and SQLite fields retain narrow representations at private or external boundaries.
  Native frame-function ordinals, trap and runtime-site identities, and source/native function
  links remain `u64` through the generated-code runtime ABI. Native invocation discovers the actual
  thread stack extent and guard and has no fixed 4 MiB aggregate or 1 MiB per-frame ceiling.
  Generated executable coverage runs more than 4 MiB of checked aggregate frames, runs one frame
  beyond 1 MiB on a sufficiently large thread stack, and declines that same frame safely on a
  256 KiB thread. Automatic recursive groups retain the validated generic VM route; acyclic groups
  receive a checked pre-entry stack requirement, and synthetic representation-boundary coverage
  proves the decline occurs before any native entry. Automatic execution also retains the VM route
  when native planning declines a shape for another genuine backend reason.
  Bytecode validation has no project-selected encoded-size, physical-table, metadata-size,
  constant-size, cleanup-node/range, witness, region-product, or structural-operation table
  admission. `ValidationPolicy::Limited` remains an explicit untrusted total-byte policy, while
  trusted compilation and prepared binding use unrestricted validation. Ordinary local
  `lkjscript run` explicitly selects `ExecutionPolicy::Unrestricted`; the type has no `Default`.
  Isolated process manifests and workers require `ExecutionPolicy::Limited`, whose current
  conservative values preserve the prior process behavior. Fuel, VM values/frames, heap bytes,
  allocations, handles, output bytes, wall deadline/hard-deadline requirement, and cleanup-report
  retention are absent under unrestricted execution rather than encoded as maximum-value
  sentinels. VM and native checks are conditional, and automatic VM-to-native transitions forward
  remaining fuel, wall time, frame capacity, and value capacity without replacing the selected
  policy. Native stack bytes have no hidden policy default. Cleanup-report retention is
  host output policy only; cleanup attempts continue after retention is exhausted. The former
  logical-aggregate-construction work count and resource outcome are removed from VM, evaluator,
  native services, and process encoding.
  Structural list, region-product, unique, ordinary/sealed region, root-table, structural-value,
  semantic-DAG snapshot, and returned structural snapshot stores no longer carry project-selected
  object, byte, node, field, depth, dependency, drop, owner, release-work, segment, record, root,
  loan, or local-return snapshot ceilings. They grow fallibly with checked arithmetic. Structural
  image/snapshot conversion, validation, encoding, decoding, equality, rehydration, and destruction
  use explicit work stacks. VM, baseline-native, and optimizing-native structural allocation,
  retained-byte, and return-export observations are charged to the selected coarse
  `ExecutionPolicy`; a low policy returns a typed resource outcome and cleans unpublished owners,
  while the same program succeeds under sufficient or unrestricted policy. Runtime diagnostics are
  retained opportunistically so allocation failure cannot prevent ownership cleanup.
  Structural runtime slots and generations, unique-store slots/generations, region-product arena
  and record identities, and semantic-DAG node/edge/count fields are wide. One-word runtime ABI
  handles are opaque nonzero `u64` tokens resolved through wide identity maps; stale tokens remain
  invalid after slot reuse. Segmented-list keys are also opaque tokens over wide segment/entry
  coordinates rather than packed positions. Outcome and process codecs use canonical `u64` lengths
  and IDs and reject host-width overflow before indexing. Outcome decoding retains only explicit
  total-wire-byte policy; process framing retains its separate `MAX_FRAME_BYTES` transport boundary.
  Generated tests cross the former 65,535 list/field/node boundaries, inject identities above
  `u32::MAX`, reject malformed high DAG references, and carry a full VM structural return through
  encode/decode and fresh-runtime authenticated rehydration.
  Structural list equality in the VM, evaluator, and native runtime is iterative and complete for
  the acyclic segmented-list representation; there is no independent comparison-step quota.
  Dynamic byte vectors and static-byte clones have no per-buffer size rule: checked, fallible
  allocation is charged to explicit heap/allocation policy. File and socket byte-view operations
  accept views beyond the former 64 KiB boundary. Private 256 KiB syscall chunks are tuning only;
  partial-count operations remain partial, full socket send iterates, and output, deadline, and
  cancellation policy is checked across chunks.
- Type and SSA type ownership, identity, verification, trait solving, and the ordinary memory-plan
  and lowering path have deep-stack coverage. Other recursive semantic-operation, transaction,
  runtime structural-value, and specialization paths still need explicit work-stack conversion or
  equivalent evidence. Some analyses retain poor large-input complexity.
- The compiler cannot yet consume a syntax-independent semantic snapshot directly.
- Semantic edits still publish text files, and stable identity remains coupled to the current
  source representation in important paths.
- The evaluator, VM, baseline native path, and proof-oriented optimizing path still multiply the
  implementation surface. Their long-term roles have not yet been selected by representative
  measurement.
- Some host-resource cleanup obligations remain explicit rather than compiler-inserted on every
  implemented outcome.
- The daemon, process-cell, scheduler, and database foundations exist, but they are not required
  to validate the local language and compiler architecture.
