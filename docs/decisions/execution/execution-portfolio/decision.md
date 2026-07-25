# Measured Execution Portfolio: Decision

[Authority](../execution-portfolio.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Decision

All execution modes consume the same resolved typed HIR and verified semantic
SSA family. They may differ in placement, linking, representation, profile
collection, optimization budget, and installation. They may not reinterpret
source spelling or change values, evaluation order, traps, effects,
capabilities, ownership/drop, roots, logical resource charges, cancellation,
or structured outcomes.

The accepted portfolio is:

1. deterministic reference evaluator;
2. validated typed/register or measured reference bytecode VM;
3. ultra-low-latency baseline native compiler;
4. proof-checked optimizing JIT;
5. reproducible native AOT;
6. content-addressed persistent native cache;
7. optional explicit local PGO/AOT;
8. loop OSR;
9. guarded runtime specialization and deoptimization only after measured need
   and exact state reconstruction; and
10. direct Wasm 3.0 and component output.

Order in this list is not permission to skip prerequisites. Runtime JIT remains
the primary adaptive path. AOT/cache/Wasm are deployment strategies sharing the
same semantics, not sibling compilers.
## Preserved Execution Authority

- Optimizer discovery never grants execution authority. An independent bounded
  proof checker or translation validator yields opaque verified optimized IR.
- Every native backend consumes a closed verified machine plan or equivalent
  validated target IR, not arbitrary bytes.
- Installation validates ABI, target, CPU features, limits, relocations,
  metadata, roots, permissions, and W^X state.
- Forced engine modes fail rather than silently downgrade.
- Emission, disassembly, counters, or installed code do not establish execution;
  synchronous verified entry and exact differential results do.
- The evaluator, SSA evaluator, and VM remain independent enough to detect
  production-lowering defects.
- Unsupported targets and unsupported operations fail explicitly.
## Workload Policies

Policies are measured and versioned:

| Workload | Initial policy target |
| --- | --- |
| Tiny CLI | cached validated bytecode or AOT with minimal startup |
| Interactive tool | VM plus selective low-latency baseline compilation |
| Long-running server | baseline plus proof optimizing JIT; later loop OSR |
| Deterministic sandbox | exact logical metering and restricted adaptive policy |
| Deployment artifact | reproducible AOT and optional signed cache |
| Embedded/minimal runtime | selected VM/memory plan with explicit unsupported features |
| Browser/component | direct Wasm and canonical component interfaces |

No default changes until same-base end-to-end measurements include startup,
compilation, steady state, RSS, code/metadata, tails, and break-even.
## Baseline Compiler Candidates

The owned x86-64 encoder remains Current and retains its evidence. Its
replacement boundary stays explicit. Complete candidates may include:

- the current direct encoder;
- generated stencil/copy-and-patch lowering;
- generated baseline lowering from declarative machine rules; and
- external maintained backends as differential or optional production
  candidates.

Selection measures compile latency, generated performance, metadata/root
support, RSS, code size, dependency/TCB cost, architecture reach, and
maintenance. Maturity alone neither adopts nor rejects a backend.
## VM Portfolio Experiment

A validated typed-register or typed-slot VM may replace or supplement the
Current tagged stack VM only after a complete candidate exists. It must provide
raw full-range integer slots, exact floating bits, reference/capability slots,
typed block/register metadata, deterministic validation/metering, and direct
lowering from semantic SSA. Stack, register, and hybrid candidates are compared
on code size, dispatch, startup, compile cost, memory, diagnostics, and oracle
independence.
## Reproducible AOT

AOT consumes verified semantic SSA and the shared verified machine/backend
boundary. Artifact identity includes every semantic input, compiler/runtime and
ABI version, edition/schema version, target/CPU features, provider/component
ABI, optimization policy, and relevant resource policy. The first AOT surface
is a minimal executable/object acceptance path, not a claim of production
linker, package, or cross-platform completeness.

AOT must pass evaluator/VM/native differential tests and exact malformed image,
relocation, root-map, trap/outcome, and forced-entry checks.
## Persistent Native Cache

The cache is implemented only after complete package, semantic, native,
provider, target, and resource-policy identities are Current. Loading verifies:

- content hash and optional configured signature;
- semantic/native/runtime/provider ABI;
- edition/schema and complete source/artifact identity;
- target and CPU feature requirements;
- code, proof, root, relocation, and metadata limits; and
- W^X-safe installation without unsafe sealed-code patching.

Partial key matches and stale/corrupt entries are misses or structured failures,
never fallback authority. Eviction is bounded and cannot change semantics. No
uploaded telemetry is required.
## Optional Local PGO

Offline PGO is reclassified from permanently rejected to **Deferred Optional
Target**. It is considered only after common SSA/AOT/artifact identity exists.
Any implementation is:

- explicitly user controlled and local by default;
- workload/profile identified and reproducible;
- privacy preserving and bounded;
- optional for ordinary builds;
- semantically powerless without the same proof/validation boundaries; and
- measured against no-PGO AOT and JIT including training/build cost.

Local profiles are not telemetry. There is no plan to upload user observations.
An implementation can still be rejected if its benefit does not repay
complexity, privacy, code-size, or build cost.
## Automatic Promotion, OSR, And Deoptimization

The existing selected synchronous automatic baseline-to-proof promotion remains
a valid candidate experiment, disabled by default until its retained threshold
gate passes. The Semantic Source foundation pre-empts it only in repository
priority, not in technical validity.

OSR proceeds only in this order:

1. loop-backedge observations;
2. exact eligible loop-header state maps;
3. baseline loop OSR;
4. optimizing loop OSR;
5. guarded specialization; and
6. deoptimization only for actual guards.

State maps include VM locals/stack or registers, SSA loop parameters,
ownership/drop state, capabilities, exact roots, logical charges, deadlines,
and pending outcomes. Unsupported loops remain correct in the lower tier.
There is no deoptimization abstraction before a guard and reconstructible state
exist.
## Wasm And Portability

Target-specific registers, red zones, stack rules, calling conventions, and
instruction semantics remain below target-neutral SSA. Linux AArch64 is the
second native architecture target; direct Wasm 3.0 and component interfaces are
separate target paths through the same semantic IR. Wasm does not define native
semantics. A second backend cannot become an independent source interpreter.
## Measurement Gate

Every candidate predeclares:

- baseline/candidate commits and exact workload/source hashes;
- target, environment, CPU features, and toolchain;
- correctness oracle and forced-mode entry evidence;
- warmup, randomized/interleaved sample plan, and retained samples;
- startup, compile, steady-state, p50/p95/p99 where relevant, RSS, code,
  metadata, proof, cache, and energy metrics where available;
- adoption/rejection thresholds and break-even count; and
- cleanup and negative-evidence retention.

Feature-local same-base A/B attribution is separate from whole-main regression
sentinels. A sentinel may block release without proving causation. Existing
scalar, allocation, Mandelbrot, editor, HTTP, durable-file, SHA-256, SQLite, and
Brainfuck identities remain unchanged regression evidence.
