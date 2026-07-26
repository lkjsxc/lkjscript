# Architecture: Runtime Flow

[Authority](../architecture.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Runtime Flow

```text
ValidatedChunk main
  -> explicit ExecutionConfig budgets and monotonic deadline
  -> install internal immutable function closures
  -> execute the source main body
  -> dense opcode dispatch with fuel/stack/frame/heap/allocation metering
  -> stack frames and return-adjacent tail reuse
  -> tagged immediate values or arena objects
  -> bounded handle/output accounting
  -> host operation dispatch
  -> lkjscript-sys Linux FFI
  -> owned Returned value or structured terminal outcome
  -> drop resources, restore terminal, flush, then CLI status translation
```

The VM is synchronous and single-threaded. It never terminates the Rust process;
exit, traps, limits, deadlines, and host failures stop only the current VM.
Returned heap values own a private reachable-object snapshot, and later VM
instances have fresh globals, arenas, handles, counters, and deadlines.
Process-global stdin/stdout and the terminal guard still prevent parallel VM
supervision. Cooperative deadlines can overrun inside current filesystem and
write/send wrappers; hard-deadline mode rejects those operations before effects
rather than claiming cancellation.

The current native flow is:

```text
forced main or hot scalar VM function entry
  -> verified scalar or host-independent reference eligibility and reachable SCC group
  -> synchronous typed-SSA lowering at a safepoint
  -> bounded W^X callable canonical native contract baseline code object
  -> one invocation-time pthread stack-bounds query
  -> cached descriptor/budget/bounds frame reservation before each stack subtraction
  -> initialized registered frame and verifier-certified exact scalar or typed-reference call map
  -> unboxed direct call or canonical-fact verified-home HeapDispatchV1 safe runtime service
  -> GcHeap collection/allocation with root writeback, argument re-materialization,
     transactional mutation, and transitive owned return snapshot
  -> PollV1/CollectReferenceV1 and structured return/trap/exit/deadline/resource/host status
  -> exactly one unregister on every registered outcome
```

Forced baseline and optimizing modes enter generated main and never fall back.
The optimizing mode verifies the bounded complete proof before source effects,
lowers only opaque `VerifiedOptimizedProgram`, installs only optimizing objects,
and retains certificate/accounting metadata. Auto compiles at one
eligible scalar-adapter function entry and uses the baseline object only on later calls;
reference-signature helpers may be generated direct callees but remain
ineligible VM/native entries. Unsupported code stays VM-correct with same-epoch
retry suppression. The old observation-only hook is
removed. Closed plans retain exact Buf-reference collection. Forced SSA/source execution
also supports Str, legacy Buf, Product, List, Option, and Result allocation and
direct/mutual recursion. Auto intentionally keeps reference-typed functions in
VM because reference transitions remain absent. Loop OSR, automatic optimizing promotion, broader
proof passes, background
compilation, speculative tiers, persistent profiles, and persistent code caches
are absent. The selected but unimplemented automatic flow is:

```text
VM root entries --64--> synchronous baseline install; triggering call stays VM
later scalar root entry -> exact Baseline(function, object, tier) token
  -> count exact baseline entries of that root
  --N--> capture current baseline token/object
          -> synchronous bounded proof/check/lower/W^X install
          -> OptimizingPending; invoke captured baseline object
  -> later root entry validates/publishes pending token -> OptimizingNative
```

N is CLI-opt-in and candidate-controlled at 64/256/1,024/4,096; optimizing is
disabled by default until retained adoption. The process-local session owns
coexisting baseline/optimizing objects, one current and optional pending
selection, and bounded stale mappings until drop. Epoch changes invalidate
optimized selection back to baseline; stale tokens cannot be selected. One
attempt per epoch, a bounded total, same-epoch suppression, and structured tier
failure are architectural boundaries, not optimizer hints. Source main and all
reference VM/native entries remain VM-only, while generated reference helpers
may call and allocate internally. There is no OSR, background compile,
deoptimization, guard, or speculation.
## Source Layout Rule

The current the removed legacy source contract language rule limits each lkjscript source directory to
16 immediate entries, counting files and subdirectories together. Rust crates,
documentation, metadata, `.git`, and build output are not language source and
are outside this rule.

The repository gate checks the complete in-tree language corpus. The compiler
also rejects an entry or imported source directory that violates the rule, so
an external project receives the same contract. The accepted destination is an
AI-maintainability lint, but this check is not weakened until aggregate source
closure/import/byte/node safety bounds are Current. See [Resource Budget
Profiles](../../decisions/platform/resource-budget-profiles.md).
## the canonical source contract Accepted Flow

the canonical source contract is an [Accepted Target, not
Current](../../decisions/semantics/semantic-core.md). Its exact path remains the one
validated Semantic Source tree through resolved HIR and verified SSA. Match is
verified then lowered to SSA CFG; evaluators and backends implement only ADT,
numeric, layout, charge, and terminator primitives. Acceptance requires actual
generated calls in forced engines with no fallback and exact roots; no current
runtime behavior is changed by that contract.

## Change Guide

- Change source semantics/projection: Semantic Source schema/validator, edition adapter/formatter,
  language docs, complete mechanical corpus migration, semantic transaction tests, and negative
  fixtures; backends never interpret spelling.
- Change types: language docs, type prelude/inference, lowering, VM behavior, and conformance tests.
- Add an opcode: core ABI, code generation, dispatch, disassembly, and malformed-bytecode validation.
- Add host capability: accepted decision, sys safety wrapper, VM resource boundary, typed prelude,
  script policy wrapper, and failure tests.
- Change limits: language decision, shared core constant, compiler enforcement, repository gate, and boundary tests.
- Change packaging: imports decision, resolver, installed layout, Docker/native bundle, and external-project smoke.
## Accepted Redesign Direction

Explicit main, effect-free imported libraries, local-only mutation,
product-threaded editor/terminal/Brainfuck state, whole-chunk validation,
structured process-safe outcomes, bounded VM execution, deterministic
fixed-point effects, resolved typed HIR, verified typed SSA, independent
evaluation, baseline normalization, reference bytecode, exact roots, owned
x86-64/W^X code, callable baseline execution, and forced proof-checked
optimizing execution are Current.

The accepted Target architecture is:

```text
goal/specification
  -> versioned Semantic Source with typed holes
  -> opaque validated source graph and deterministic Edition adapter
  -> resolved typed Core HIR
  -> ownership/effect/capture/capability analysis
  -> verified semantic SSA
  -> verified memory/region/drop lowering
  -> deterministic baseline normalization
  -> optional proof-checked optimization
  -> target-neutral verified machine plan
      +-> deterministic evaluators
      +-> validated portable VM artifact
      +-> baseline native compiler
      +-> optimizing JIT
      +-> AOT/cache
      +-> Wasm/components
```

[Semantic Source And Agent
Protocol](../../decisions/platform/semantic-source-and-agent-protocol.md) now
has Current Schema: one validated source-tree authority, exact 132-file
tracked source roundtrip (121 under `src/`), exact revision and stable hole/
declaration identities, dense nodes, structural diagnostics, checker-derived
hole context, bounded legal actions, and atomic transactions. Existing
HIR/SSA/VM/JIT behavior remains unchanged through that cutover, and no sibling
parser/tree path independently feeds a backend. Bounded topology, repository
graph/context, agent work state, atomic semantic edits, resolved-reference
facts, structured compiler diagnostics, typed holes, and bounded one-shot and
session transport are Current bounded slices.

[AI-Native Language And Platform](../../decisions/platform/ai-native-platform.md) owns the
long-term dependency order. [Resource Budget
Profiles](../../decisions/platform/resource-budget-profiles.md) prevents weakening tiny
Current limits before aggregate replacements exist. [Measured Execution
Portfolio](../../decisions/execution/execution-portfolio.md) accepts later AOT, cache,
optional local PGO, and Wasm measurement without making them Current. The
process-local automatic proof-promotion contract remains selected and disabled
by default, but no longer pre-empts the Semantic Source foundation.

The containing host-independent allocation slice based on `0daa7a0` passed the
focused cross-crate tests, strict affected Clippy, docs/tree/source checks,
`quiet verify` (182 unit/integration tests plus one compile-fail doctest), locked
release build, scalar/hello/Brainfuck smokes, and forced allocation-graph smoke
described in [Current State](../../current-state.md). Docker and performance were
not run.
