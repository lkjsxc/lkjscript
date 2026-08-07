# Roadmap

## Now

1. **Remove remaining source-representation, HIR, ownership, and executable ceilings.** Flat
   lexer-token, children-per-form, top-level-form, source-nesting, per-file source-byte, aggregate
   source-byte, and source-unit quotas are removed. Parsing, source projection/identity/formatting,
   recursive source and HIR destruction, and the ordinary deep-expression analysis/lowering path
   are stack-safe through 20,000 nested forms and 20,001 HIR expressions on a 256 KiB native stack.
   Ownership's aggregate expression pre-scan and memory planning's expression admission are
   removed; checked expression work remains telemetry. Compiler/SSA type depth and work fuel,
   recursive enum graph fuel, and HIR memory-plan type-node, type-edge, witness/group/dependency,
   semantic-descriptor, structural metadata table, and type-SCC work ceilings are removed.
   Canonical memoized auto-trait solving handles cyclic nominal obligations coinductively, and
   generated coverage crosses 512 nested source types and 8,192 nested raw SSA types on a 256 KiB
   stack. HIR memory-plan function/use/loan/call/obligation, destination, borrow-scope, drop-path,
   witness parameter/argument, SSA region-product, and executable structural
   destination/operation-reference admission quotas are now removed end to end. Producer telemetry
   and independent verifier totals are checked observational `u64` equality. Generated coverage
   crosses 4,096 functions, 65,536 uses, 16,384 loans/calls/borrow scopes, 32,768 obligations/drop
   paths, 16 witness parameters/arguments, 16,384 region products, and 16,384 structural
   destinations/operation references. Source positions/spans and revision-scoped node indexes,
   Semantic Source wire relations, all HIR and memory-plan dense IDs, SSA
   function/block/value/binding/trait/implementation/place/loan identities and origin/link metadata,
   core structural-image node/range identities, and native source/plan links now use `u64` with
   checked host conversion. Synthetic/unresolved states and native layout domains no longer use
   collision-prone numeric sentinels or arithmetic tags.
   Trusted package loading no longer rejects manifests above 1 MiB, canonical locks above 16 MiB,
   or dependency graphs by native recursion depth. Checked fallible EOF reads use metadata only as
   an allocation and file-change hint, while iterative indexed DFS owns deterministic dependency
   completion and cycle detection without count or depth admission. Generated coverage checks and
   compiles a package whose manifest and lock exceed 17 MiB, resolves 512 nested packages on a
   256 KiB native stack, resolves 512 direct dependencies, and reports a terminal cycle
   deterministically. Trusted bytecode validation no longer has total encoded-byte, table-entry, metadata-byte,
   constant-data-byte, or cleanup-node/range count admission; a boundary-local limited validator
   checks only total artifact bytes and has no finite default. Byte-sized function arity, call
   argument, local, cleanup-local, and executable-place widths are
   removed: one fixed-`u64` index/pair format now executes 300 parameters, arguments, live lexical
   locals, and direct VM place 299, with checked decode and automatic VM fallback; a larger stress
   case executes 1,024 parameters, arguments, and lexical locals. Branch targets use the same fixed
   `u64` layout, and the separate 65,535-byte per-function validity limit is removed. Production
   coverage executes a 115,754-byte main with 1,024 owned arguments and a generated function whose
   branch target exceeds 65,535 bytes; automatic mode retains VM fallback when native expansion is
   ineligible. SSA ownership work and retained state-cell quotas are removed; generated coverage
   verifies 44,000 owned parameters and places,
   132,000 state cells per block, 264,000 cells under the former aggregate retained-state
   accounting across propagation, and exact cleanup. SSA CFG verification no longer rejects at
   4,096 blocks or a fixed CFG-work total: indexed adjacency, iterative DFS/SCC traversal,
   reverse-postorder immediate dominators, and dominator-tree intervals verify 10,000-block linear,
   branch/merge, and cyclic generated CFGs. SSA and bytecode cleanup now use wide,
   backward-only hash-consed node arenas with segmented roots; a 300-owned-parameter/argument source
   publishes and runs with 315,450 logical actions represented by 1,200 physical nodes. Constants,
   globals, and prototype references now use fixed-`u64` operands and metadata with checked host
   indexing; focused chunks execute index 65,536 for all three tables, and a release stress source
   crosses 65,535 distinct HIR scalar constants. HIR memory-plan entry, constant, and verifier-work
   admission is removed in that production vertical. The nominal aggregate vertical now removes
   the 15-field product and 255-variant/field enum admission rules; product/enum/structural tables,
   descriptors, field indexes, source orders, physical tags, and enum substitutions use `u64` with
   checked host conversion. Generated 300-field products and 300-variant enums with a 300-field
   payload execute through validated VM bytecode, and private compact native shapes decline to the
   VM in automatic mode. Total memory witness/group/dependency and structural type/layout/
   representation table ceilings are removed; generated coverage verifies 32,769 HIR declarations
   with more than 65,536 SCC steps, 16,385 executable witness groups, and semantic descriptors past
   the former declaration, type-node, edge, and 16 MiB byte boundaries. Match usefulness
   specialization-work and witness-byte reservations are removed; deterministic memoized matrix
   specialization and explicit continuation stacks now produce complete exhaustiveness witnesses
   through 2,048 nested product patterns on a 256 KiB native stack, while a 300-field witness
   crosses the former computed byte reservation. Per-call witness arity, recursive-group ordinals,
   region products, structural destination/operation-reference counts,
   and related witness transport now use unrestricted vectors plus wide checked identities where
   identities cross a boundary. Semantic-DAG snapshot IDs and structural runtime slot/generation
   keys are now wide; remaining narrow representation work is concentrated in scheduler IDs and
   checked machine/OS/ABI boundaries. Phase 5 still replaces revision-scoped dense `NodeId` with
   edit-stable logical semantic identity.
   Bytecode links, call-witness offsets, cleanup range
   offsets, and cleanup roots are already `u64`, while physical cleanup-node/range counts remain
   unrestricted by the removed general bytecode table and metadata admission. Completion requires
   just-beyond-old-boundary and substantially larger positive programs, checked growth, and
   successful execution through the retained generic path.
2. **Complete resource-policy separation.** Trusted compiler profile/ledger and source-byte
   admission are removed. The untrusted Semantic Source boundary now supplies only an aggregate
   source-byte loader policy and applies it to staged transactions before publication. Continue
   replacing remaining validity-changing implementation ceilings with unrestricted local
   compilation and explicit coarse input, memory, output, deadline, cancellation, and parallelism
   policy at untrusted request boundaries. Runtime list equality no longer has an independent
   million-step trap, byte storage no longer has a per-buffer 1 MiB rejection, and file/socket bulk
   views no longer have a 64 KiB validity rule. Explicit heap/allocation/output/deadline/cancellation
   policy now governs those paths. `ExecutionPolicy` has no finite default: ordinary local run
   selects `Unrestricted`, isolated process work selects `Limited`, VM/native checks are optional,
   cleanup retention does not stop cleanup, and the logical-aggregate-construction count is gone.
   The same bytecode exhausts under low limited fuel and succeeds unchanged unrestricted. Ordinary
   structural store and local-return snapshot ceilings are removed across region, sealed-region,
   root, unique, segmented-list, region-product, structural-value, semantic-DAG, and process-outcome
   paths. Checked structural allocation/retained-byte/export observations feed the coarse
   `ExecutionPolicy`; low-policy failures are typed and cleanup-atomic, while unrestricted execution
   crosses the former node, field, payload, dependency, record, root, and loan boundaries.
   Structural graph/snapshot operations and deep destruction are iterative. Structural slots,
   generations, DAG IDs/counts, region-product records, list positions, native aggregate fields,
   projection paths, and witness locators now use wide identities. Runtime ABI handles remain one
   word but are opaque `u64` tokens backed by wide identity maps. Process and outcome lengths and IDs
   are canonical `u64`; only explicit total-wire-byte and transport-frame policy remains. The next
   dependency-closed runtime work is representative execution-path measurement and selection.

## Next

1. Measure VM-only, automatic baseline-native, and optimizing configurations on scalar, call,
   aggregate, byte, cleanup, error, and host workloads. Select one production path and delete the
   losing product-tier surface.
2. Remove duplicated in-process witness and digest reconstruction while retaining artifact,
   process, capability, path, and executable-memory validation.
3. Implement the first semantic-program vertical described in [`source-model.md`](source-model.md),
   including direct compilation without text rendering and reparsing.
4. Add dependency-aware incremental name, type, effect, ownership, and lowering queries after
   measuring a small custom cache against a mature query framework.

## Later

1. Integrate immutable semantic snapshots, compilation caches, and warm runtime state with the
   daemon through narrow interfaces.
2. Profile representative applications and add native specialization, tiering, OSR, or
   deoptimization only where measured benefit exceeds implementation cost.
3. Expand package, service, database, network, GUI, web, and game capabilities through the semantic
   model, capability system, and selected production runtime.
