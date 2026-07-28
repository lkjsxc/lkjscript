# Current State

## Status

<!-- LKJ-STATUS id=affine-resource-handles status=superseded -->
<!-- LKJ-STATUS id=agent-work-state status=current -->
<!-- LKJ-STATUS id=byte-text-ownership status=accepted-contract -->
<!-- LKJ-STATUS id=canonical-lowercase-vocabulary status=accepted-contract -->
<!-- LKJ-STATUS id=collector-free-deterministic-memory status=accepted-contract -->
<!-- LKJ-STATUS id=enum-declarations status=current -->
<!-- LKJ-STATUS id=jit-auto-promotion status=accepted-selection -->
<!-- LKJ-STATUS id=jit-proof-forced status=current -->
<!-- LKJ-STATUS id=memory-obligations status=current -->
<!-- LKJ-STATUS id=memory-plan status=current -->
<!-- LKJ-STATUS id=memory-tracing-ratchet status=current -->
<!-- LKJ-STATUS id=modules-and-packages status=current -->
<!-- LKJ-STATUS id=never-control status=current -->
<!-- LKJ-STATUS id=numeric-conversions status=current -->
<!-- LKJ-STATUS id=opaque-paths status=current -->
<!-- LKJ-STATUS id=repository-graph-context status=current -->
<!-- LKJ-STATUS id=repository-topology status=current -->
<!-- LKJ-STATUS id=resource-profile-compiler status=current -->
<!-- LKJ-STATUS id=resource-profile-preallocation status=current -->
<!-- LKJ-STATUS id=resource-profile-shared-ledger status=accepted-target -->
<!-- LKJ-STATUS id=semantic-core-target status=accepted-target -->
<!-- LKJ-STATUS id=semantic-session status=current -->
<!-- LKJ-STATUS id=semantic-source status=current -->
<!-- LKJ-STATUS id=typed-holes status=current -->
<!-- LKJ-STATUS id=typed-capabilities status=current -->
<!-- LKJ-STATUS id=typed-resources status=accepted-contract -->

This file is the concise Current authority. Historical source generations,
protocol experiments, candidate resource profiles, rejected performance
results, and immutable AI-authorability records live only under
`docs/history/`, `docs/vision/experiments/`, retained benchmark result trees, or
Git history. They do not provide aliases or acceptance fallbacks.

## Language and source

- `.lkjscript` is the only accepted source suffix.
- Source is marker-free and has one exact contract digest. Unknown or removed
  marker forms are ordinary syntax errors.
- All language-owned and user-defined names use exact lowercase ASCII
  kebab-case. Word operations, structured signatures/imports, and
  `string-literal/` are the only accepted source projections; removed spellings
  are rejected from one typed registry.
- Generic enums, exhaustive `match`, `never`, structured control, and explicit
  numeric conversions run through evaluator, VM, baseline JIT, and proof JIT
  where each engine supports the relevant operation set.
- Source, declaration, node, revision, and Semantic Source identities frame the
  full current source/semantic contract digest.
- Semantic Source requests, sessions, diagnostics, typed holes, transactions,
  and publications require stable schema names plus exact full contract
  digests. No generation-numbered envelope is accepted.
- `capability/ kind /capability` values carry one of eight closed provider
  authorities. Capability-bearing main and library APIs pass them explicitly;
  provider acquisition and ambient host services have no zero-argument form.
- Packages declare a sorted capability union. Each target receives only its
  exact typed main requirements, validated before any source effect. Capability
  values are unforgeable, copyable, process-local, and never serialized.
- Linux runtime pathnames use immutable byte-preserving `path` values. Explicit
  constructors reject empty, relative, NUL-containing, and oversized paths;
  observation is either exact bytes or strict UTF-8. Filesystem and SQLite
  operations reject `string` pathname operands.

## Modules and local packages

- Every source file is a module identified only by its package-root-relative
  UTF-8 path.
- Declarations are private by default. The `public` field is explicit; each
  `import/` records exact module paths and sorted declaration lists. Wildcards,
  transitive visibility, ambient root search, dot-relative paths, collisions,
  private names, and path/symlink escape are rejected.
- Equal declaration spellings coexist in distinct module scopes. Qualification
  occurs once before HIR; runtime metrics retain source-visible names.
- `lkjscript.package.json` and canonical `lkjscript.lock.json` bind exact
  modules, exports, local dependency hashes, targets, package content, and full
  language/source/module/package contract identities.
- `lkjscript package lock` writes atomically. `package check`, `run`, and
  `disasm` reject missing, noncanonical, stale, or mismatched locks. Registry,
  network, home-directory, and environment fallback resolution do not exist.

## Ownership and resources

- `byte-vector`, whole-place `move`, `byte-slice`, and `byte-slice-mut` expose
  the existing bounded ownership foundation without `owned buf`, `ref buf`, or
  `ref-mut buf` source aliases.
- The universal source type `handle` is removed. Eleven exact resource kinds
  flow through source typing, HIR, verified SSA, bytecode validation, and VM
  resource-kind checks. Resources cannot use value/object equality, escape from
  `main`, or enter unsupported aggregates.
- Explicit `drop`, SQLite close, and SQLite finalize consume ownership through
  HIR and verified SSA. Exactly-once compiler-inserted cleanup and generated
  native host execution remain accepted-contract work, so typed resources are
  not claimed Current as a complete capability.
- Opaque monotonic VM tokens remain stale-safe, exact-kind checked, and
  disjoint from integers and borrowed standard input. Core now provides the
  replacement host-independent resource table with typed provider/scope-bound
  generation keys, reservations, nonwrapping LIFO reuse, parent protection,
  invalidating close, reverse cleanup, and exact obligation accounting. VM and
  native integration remain incomplete.
- Core provides a bounded deterministic unique store with opaque store-scoped,
  generation-bearing typed keys for byte-vector, dynamic bytes, and path
  layouts. It is not yet integrated into source execution or native tiers.

## Compiler and execution

- One validated source tree feeds module resolution, typed HIR, ownership and
  effect analysis, verified SSA, bytecode, evaluator, VM, and both JIT tiers.
- Linux x86-64 baseline acceptance requires real synchronous native calls. The
  forced proof JIT accepts only proof-checked optimized SSA and has no VM
  fallback. A focused scalar group with direct calls, loops, bool, i64, and f64
  proves both forced tiers have nonzero generated entries, zero fallback, no
  collector-capable runtime call, empty exact root maps, and zero allocation,
  collection, root, and barrier counters. VM scalar boxing remains separate, so
  this is not the full value-island capability.
- Native image compatibility is the exact tuple of language, verified-SSA,
  runtime-call, and native-layout contract digests. Runtime calls and public
  metrics use stable unnumbered names.
- Metrics use schema `lkjscript.metrics` and the full metrics contract digest.
- `lkjscript memory inventory` exposes 62 sorted memory-obligation records under
  `lkjscript.memory-obligations`. It truthfully reports the Current tracing heap,
  exact roots, transitional ownership island, PLACEHOLDER `bytes`, and accepted
  deterministic candidates; it is derived evidence, not semantic authority.
  Every executable program retains a content-addressed pre-backend HIR memory
  plan covering every expression result, parameter/result, place, loan,
  constant, and call. Separate exhaustive producer and verifier traversals run
  before an opaque checked-HIR wrapper can enter SSA. The direct-affine SSA
  inventory remains independently recomputed derived evidence.
- `LKJ-MEMORY-TRACING-RATCHET` fails when the exact eleven registered
  `HeapObj` families change without an accepted registry update. `lkjscript
  memory traced [--json]` exposes the same sorted Current set. This intermediate
  gate does not claim that the runtime collector is removed.
- Resource categories and profiles use full category/profile/maxima/ceiling
  digests. The selected ledger spans compiler phases; one request-owned ledger
  across every compiler/runtime authority remains an accepted target.

## Repository and agent platform

- `lkjscript describe --json` and `semantic describe` expose the deterministic
  closed contract registry.
- Capsule manifests, repository policy/provenance, graph/query outputs,
  capability status, and agent work state use stable schemas plus exact contract
  digests.
- The repository graph remains bounded and evidence-backed. Agent checkpoints
  remain revision-checked, append-only, and fail closed on stale semantic or
  repository identities.
- The machine `LKJ-DOC-GENERATION` rule rejects numbered language, schema,
  protocol, profile, ABI, and standalone generation names in Current-owned
  code, tests, fixtures, examples, config, and documentation. Immutable
  historical evidence is explicitly excluded rather than rewritten.

## Accepted targets not claimed Current

- promotion of the implemented lowercase vocabulary to Current remains blocked
  only by the atomic removal of transitional `buf` source surfaces;
- complete typed-resource provider/state facts, integration of reusable
  generation-bearing slots, compiler-inserted exactly-once cleanup, evaluator
  providers, and forced native host execution;
- immutable `bytes`, full affine `byte-vector` corpus migration, ranged lexical
  byte slices, borrowed `str`, and removal of transitional `buf` after complete
  cross-engine replacement;
- complete region/borrow/drop semantics for resources nested in products and
  collections;
- the selected collector-free deterministic cutover: inferred modes and loans,
  ordinary and sealed shared regions, pools, exact cleanup, migrated evaluator,
  VM and native storage, no-RC falsification, and deletion of all tracing paths;
- a portable path policy beyond the Current Linux absolute-byte contract;
- a replacement persistent verified artifact cache after the first complete
  candidate failed its measured adoption gate and was removed;
- component interfaces, Wasm, AOT, native caches, and remote distribution;
- automatic baseline-to-proof promotion acceptance beyond its selected
  measured candidate;
- portability acceptance beyond Linux x86-64.

The reserved immutable `bytes` parser path is explicitly reported as
`PLACEHOLDER` until implemented. Other targets expose no inert endpoint.

## Verification authority

The canonical local gate is:

```sh
cargo run --locked -p lkjscript-xtask -- quiet verify
```

Runtime, forced-tier, Docker, retained-result validation, Miri, sanitizers,
fuzzing, and performance gates are separate evidence classes. Exact commands,
commit, environment, result, and untested gates must be reported; an unrun gate
is never implied.
