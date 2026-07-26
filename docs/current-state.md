# Current State

## Status

<!-- LKJ-STATUS id=affine-resource-handles status=current -->
<!-- LKJ-STATUS id=agent-work-state status=current -->
<!-- LKJ-STATUS id=enum-declarations status=current -->
<!-- LKJ-STATUS id=jit-auto-promotion status=accepted-selection -->
<!-- LKJ-STATUS id=jit-proof-forced status=current -->
<!-- LKJ-STATUS id=modules-and-packages status=current -->
<!-- LKJ-STATUS id=never-control status=current -->
<!-- LKJ-STATUS id=numeric-conversions status=current -->
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

This file is the concise Current authority. Historical source generations,
protocol experiments, candidate resource profiles, rejected performance
results, and immutable AI-authorability records live only under
`docs/history/`, `docs/vision/experiments/`, retained benchmark result trees, or
Git history. They do not provide aliases or acceptance fallbacks.

## Language and source

- `.lkjscript` is the only accepted source suffix.
- Source is marker-free and has one exact contract digest. Unknown or removed
  marker forms are ordinary syntax errors.
- Generic enums, exhaustive `match`, `Never`, structured control, and explicit
  numeric conversions run through evaluator, VM, baseline JIT, and proof JIT
  where each engine supports the relevant operation set.
- Source, declaration, node, revision, and Semantic Source identities frame the
  full current source/semantic contract digest.
- Semantic Source requests, sessions, diagnostics, typed holes, transactions,
  and publications require stable schema names plus exact full contract
  digests. No generation-numbered envelope is accepted.
- `Capability/ Kind /Capability` values carry one of eight closed provider
  authorities. Capability-bearing main and library APIs pass them explicitly;
  provider acquisition and ambient host services have no zero-argument form.
- Packages declare a sorted capability union. Each target receives only its
  exact typed main requirements, validated before any source effect. Capability
  values are unforgeable, copyable, process-local, and never serialized.

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

- Fresh `Owned Buf`, whole-place `move`, and bounded lexical `borrow` /
  `borrow-mut` remain the Current ownership island.
- Owned `Handle` locals are affine. They must be returned, explicitly moved, or
  cleaned up with `drop`; leak, double-drop, borrowed-handle drop, and use after
  move/drop are compile errors.
- Generic `drop`, SQLite close, and SQLite finalize consume ownership through
  HIR and verified SSA. VM resource-table teardown is a deterministic safety
  net for host failure, not an implicit source cleanup policy.
- Opaque monotonic handle tokens remain stale-safe and disjoint from integers
  and borrowed standard streams.

## Compiler and execution

- One validated source tree feeds module resolution, typed HIR, ownership and
  effect analysis, verified SSA, bytecode, evaluator, VM, and both JIT tiers.
- Linux x86-64 baseline acceptance requires real synchronous native calls. The
  forced proof JIT accepts only proof-checked optimized SSA and has no VM
  fallback.
- Native image compatibility is the exact tuple of language, verified-SSA,
  runtime-call, and native-layout contract digests. Runtime calls and public
  metrics use stable unnumbered names.
- Metrics use schema `lkjscript.metrics` and the full metrics contract digest.
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

- complete region/borrow/drop semantics for resources nested in products and
  collections;
- an opaque byte-preserving path type and portable path policy;
- persistent verified compilation/artifact caching and measured cache adoption;
- component interfaces, Wasm, AOT, native caches, and remote distribution;
- automatic baseline-to-proof promotion acceptance beyond its selected
  measured candidate;
- portability acceptance beyond Linux x86-64.

These are not placeholders and expose no inert endpoints.

## Verification authority

The canonical local gate is:

```sh
cargo run --locked -p lkjscript-xtask -- quiet verify
```

Runtime, forced-tier, Docker, retained-result validation, Miri, sanitizers,
fuzzing, and performance gates are separate evidence classes. Exact commands,
commit, environment, result, and untested gates must be reported; an unrun gate
is never implied.
