# Active architecture

## Source and compiler flow

The current compiler authority starts at package files and the provisional line-oriented text
projection. The source loader validates paths and imports, the analyzer creates resolved typed
HIR, ownership/effect/memory passes derive executable obligations, SSA lowering creates a typed
control-flow program, and the IR verifier publishes an opaque verified program. Bytecode lowering
and validation provide the generic executable representation used by the runtime. This trusted
pipeline has no compiler resource profile or cross-phase budget ledger; source-file counts, phase
durations, and HIR memory-plan expression work are observation only. Ownership analysis does not
perform a separate aggregate-expression admission scan. Source parsing and source-tree operations
use explicit work stacks. Recursive expression mechanisms whose control and mutable compiler state
make immediate continuation-stack rewrites disproportionately invasive use the localized `stacker` boundary,
which repeatedly adds heap-backed stack segments and therefore defines tuning geometry rather than
a finite accepted depth. Recursive source and HIR ownership is dismantled by custom non-recursive
destruction. SSA ownership verification admits wide semantic state without work or retained-cell
budgets. Single-source worklist states move into block processing; join-capable states share
copy-on-write ordered fact sets and maps, so unchanged CFG propagation does not copy whole states.
Join comparison remains exact. Failure-cleanup derivation iterates active owners rather than every
declared place, avoiding repeated whole-place-table scans as cleanup progresses.

Executable-width ownership is now explicit at each boundary: analyzer/HIR arity and local storage,
codegen colors, and bytecode prototypes use host indexes; SSA frame-state slots use `u64` identity
values. Bytecode operand metadata selects one of no operand, retained `u16`, fixed `u64` index, or
two independent fixed `u64` place/local indexes. The decoder checks complete operands and host
conversion before publishing `DecodedInstruction`; validators and the VM consume the typed decoded
shape or the same fixed layout. Constants, globals, jumps, function offsets, cleanup ranges, and
product/enum/structural descriptors remain a separate `u16`/`u8` cutover.

The intended cutover is described in [`source-model.md`](source-model.md): text becomes an importer
and renderer around an immutable semantic snapshot, and compiler analysis consumes that snapshot
directly.

## Execution

The default CLI mode is `auto`. It always has a validated generic VM route and may install native
code for eligible groups. A high arity or local count does not reject generic compilation: native
preflight marks an unsupported signature ineligible, automatic execution stays in the VM, and a
forced native diagnostic reports the unsupported signature. Prepared identity records native
transport specialization as optional and does not equate semantic SSA identity with generated-code
support. Baseline and optimizing selections remain available as diagnostic modes while the reset
measures their representative value. They are not separate language definitions.
The SSA evaluator remains useful as a semantic test oracle but is not the product source authority.

No optimization result is allowed to reinterpret source semantics. Forced native modes must enter
a synchronous generated entry or fail explicitly.

## Package ownership

Cargo metadata is the authority for workspace membership and dependency edges. Conceptual
ownership is:

- `lkjscript-core`: bytecode, values, limits/resource types, validation, and shared runtime
  contracts;
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
cancellation are boundary-local host policy. Semantic node, hole, transaction, HIR, and SSA counts
do not grant language validity.

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
