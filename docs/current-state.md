# Current State
## Status
<!-- LKJ-STATUS id=affine-resource-handles status=superseded -->
<!-- LKJ-STATUS id=agent-work-state status=current -->
<!-- LKJ-STATUS id=byte-text-ownership status=accepted-contract -->
<!-- LKJ-STATUS id=canonical-lowercase-vocabulary status=accepted-contract -->
<!-- LKJ-STATUS id=collector-free-deterministic-memory status=accepted-contract -->
<!-- LKJ-STATUS id=enum-declarations status=current -->
<!-- LKJ-STATUS id=generation-safe-resources status=accepted-contract -->
<!-- LKJ-STATUS id=jit-auto-promotion status=accepted-selection -->
<!-- LKJ-STATUS id=jit-proof-forced status=current -->
<!-- LKJ-STATUS id=memory-obligations status=current -->
<!-- LKJ-STATUS id=memory-plan status=current -->
<!-- LKJ-STATUS id=memory-tracing-ratchet status=superseded -->
<!-- LKJ-STATUS id=no-tracing-runtime status=current -->
<!-- LKJ-STATUS id=modules-and-packages status=current -->
<!-- LKJ-STATUS id=native-byte-vector-island status=current -->
<!-- LKJ-STATUS id=native-bytes-island status=current -->
<!-- LKJ-STATUS id=never-control status=current -->
<!-- LKJ-STATUS id=numeric-conversions status=current -->
<!-- LKJ-STATUS id=opaque-paths status=current -->
<!-- LKJ-STATUS id=os-resident-runtime-foundation status=current -->
<!-- LKJ-STATUS id=repository-graph-context status=current -->
<!-- LKJ-STATUS id=repository-topology status=current -->
<!-- LKJ-STATUS id=resource-profile-compiler status=current -->
<!-- LKJ-STATUS id=resource-profile-preallocation status=current -->
<!-- LKJ-STATUS id=resource-profile-shared-ledger status=accepted-target -->
<!-- LKJ-STATUS id=semantic-core-target status=accepted-target -->
<!-- LKJ-STATUS id=semantic-resource-plane status=accepted-contract -->
<!-- LKJ-STATUS id=semantic-resource-runtime status=current -->
<!-- LKJ-STATUS id=semantic-session status=current -->
<!-- LKJ-STATUS id=semantic-source status=current -->
<!-- LKJ-STATUS id=typed-holes status=current -->
<!-- LKJ-STATUS id=typed-capabilities status=current -->
<!-- LKJ-STATUS id=typed-resources status=accepted-contract -->
<!-- LKJ-STATUS id=typed-vm-scalars status=current -->
This file is the concise Current authority. Historical source generations,
protocol experiments, candidate resource profiles, rejected performance
results, and immutable AI-authorability records live only under
`docs/history/`, `docs/vision/experiments/`, retained benchmark result trees, or
Git history. They do not provide aliases or acceptance fallbacks.
## Language and source
- `.lkjscript` is the only accepted suffix. Source is marker-free with one exact
  contract digest; unknown or removed marker forms are ordinary syntax errors.
- All language-owned and user-defined names use exact lowercase ASCII
  kebab-case. Word operations, structured signatures/imports, and
  `string-literal/` are the only accepted source projections; removed spellings are rejected from one typed registry.
- Generic enums, exhaustive `match`, `never`, structured control, and explicit
  numeric conversions run through evaluator, VM, baseline JIT, and proof JIT
  where each engine supports the relevant operation set.
- Source, declaration, node, revision, and Semantic Source identities frame the Current source/semantic contract digest.
- Semantic Source requests, sessions, diagnostics, typed holes, transactions,
  and publications require stable schema names plus exact full contract
  digests. No generation-numbered envelope is accepted.
- `capability/ kind /capability` values carry one of eight closed provider
  authorities. Capability-bearing main and library APIs pass them explicitly;
  provider acquisition and ambient host services have no zero-argument form.
- Packages declare a sorted capability union. Each target receives exact typed
  main requirements before effects; capability values are unforgeable, copyable, process-local, and never serialized.
- Linux runtime pathnames use immutable byte-preserving `path` values. Explicit
  constructors reject empty, relative, NUL-containing, and oversized paths;
  observation is either exact bytes or strict UTF-8. Filesystem and SQLite operations reject `string` pathname operands.
## Modules and local packages
- Every source file is a module identified only by its package-root-relative UTF-8 path.
- Declarations are private by default. The `public` field is explicit; each
  `import/` records exact module paths and sorted declaration lists. Wildcards,
  transitive visibility, ambient root search, dot-relative paths, collisions,
  private names, and path/symlink escape are rejected.
- Equal declaration spellings coexist in distinct module scopes; qualification
  occurs once before HIR and runtime metrics retain source-visible names.
- Package manifests and locks bind exact modules, dependencies, targets, content, and contract identities.
  Experimental generic locks add interface and transport-requirement digests without promotion.
- `lkjscript package lock` writes atomically. `package check`, `run`, and
  `disasm` reject missing, noncanonical, stale, or mismatched locks. Registry,
  network, home-directory, and environment fallback resolution do not exist.
## Ownership and resources
- `byte-vector`, whole-place `move`, `byte-slice`, and `byte-slice-mut` expose
  the existing bounded ownership foundation without `owned buf`, `ref buf`, or `ref-mut buf` source aliases.
- The universal source type `handle` is removed. Eleven exact resource kinds
  flow through source typing, HIR, verified SSA, bytecode validation, and VM
  resource-kind checks. Resources cannot use value/object equality, escape from `main`, or enter unsupported aggregates.
- Exact drop glue for deterministic `byte-vector` and all eleven resource kinds
  reaches affine SSA place metadata. Verified SSA has explicit loan-end and
  whole-place-drop events, rejects owner-erasing `place-end`, proves
  static/dead/conditional discharge at joins and explicit terminators, and pairs
  explicit `drop`, SQLite close, and SQLite finalize with exact resource-drop events.
- Byte-vector and owned typed-resource cleanup is elaborated on normal lexical
  and source-level return, break, continue, trap, and exit paths. Each live SSA
  failure site names an exact interned cleanup plan independently reconstructed
  by the ownership verifier. The evaluator and reference VM execute those plans
  for instruction failure, fuel/deadline exhaustion, and propagated callee
  outcomes; failed pre-entry calls clean transferred arguments separately.
  Forced baseline and proof JITs execute the same verified plans for byte and
  structural owners without fallback. Statically decidable conditional owners
  execute through all four engines, and typed resources execute dedicated
  explicit/implicit close in the VM. Bounded ordered cleanup failures retain the
  unchanged primary outcome. Native owned-resource execution remains fail-closed.
- The VM uses checked generation-bearing core resource-table tokens. Evaluator
  fake providers perform no ambient I/O and dispatch borrowed standard input,
  terminal detection, file/directory acquisition and close, and SQLite
  connection/statement acquisition and close/finalize. One exact kind can fail
  acquisition or close deterministically. Native tiers still support only
  borrowed `standard-input`; complete evaluator host and owned-native operations remain incomplete.
- Core provides deterministic byte/byte-vector storage and a bounded flat-image
  runtime for strings, paths, deterministic aggregates, regular recursive enums,
  copy-leaf and recursively nested copy-list segments, results, destinations,
  and views. They execute in all four engines with bounded tables and cleanup.
- Immutable bytes literal/read/copy/clone/freeze/thaw operations execute through
  all four engines. Native static identities select verified image data; dynamic
  byte values remain affine. Owned resources and mixed ownership graphs remain outside.
## Compiler and execution
- One validated source tree feeds module resolution, typed HIR, ownership and
  effect analysis, verified SSA, bytecode, evaluator, VM, and both JIT tiers.
- Linux x86-64 baseline acceptance requires real synchronous native calls. The
  forced proof JIT accepts only proof-checked optimized SSA and has no VM fallback.
  `lkjscript-executable` owns W^X, entry, native bridging, and tests; JIT no longer reaches these through sys.
  `lkjscript-linux-host` owns app topology/affinity; residual sys owns only host I/O/path/socket/tty/time/SQLite.
- The reference VM uses a safe closed 16-byte value with complete inline i64
  and exact-bit f64 payloads. Scalar constants, stack/locals, operations,
  conversions, calls, returns, host adapters, and JIT transitions allocate no
  aggregate records. Deterministic aggregates use bounded structural images;
  selected lists and products use invocation-local segmented/region storage.
  Native images retain typed frame homes for call and cleanup validation but no
  liveness maps or collection services. Exact byte-vector and bytes groups use
  closed byte/u32 access, mutation, borrow, copy, clone, freeze, thaw,
  end-borrow, and drop calls with zero final live owners, loans, or release backlog.
- Native image compatibility is the exact tuple of language, verified-SSA,
  runtime-call, and native-layout contract digests; public metrics use stable unnumbered names.
- Metrics use `lkjscript.metrics` and its full contract digest. `lkjscript
  memory inventory` exposes sorted memory-obligation records under
  `lkjscript.memory-obligations`. It reports deterministic structural, region,
  unique, resource, artifact, and host ownership; it is derived evidence, not
  semantic authority.
  Every executable program retains a platform-bound canonical HIR memory plan
  covering every result, place, loan, constant, and call without Rust formatting.
  Concrete SSA/bytecode witnesses reject zero or duplicate identities.
  Experimental residual transport and structural-owner lists execute exact
  witnesses and cleanup in evaluator/VM. Native owner lists remain blocked;
  exact specializations are bounded at 32/declaration and 1,024/package, reverify
  rewritten SSA, and retain zero fallback on the locked 4,096-operation workload.
  Independent verification still precedes SSA; inventory remains derived.
- `LKJ-RUNTIME-NO-TRACING-COLLECTOR` rejects collector directories, APIs,
  object families, services, liveness maps, configuration, and metrics across
  all crate sources. No tracing-family registry or `memory traced` command
  remains. Lists use segmented regions and flat snapshots. The
  [product decrement](history/evidence/product-tracing-removal.md) and
  [substrate](current-state/structural-memory-evidence.md) include compact
  stale-safe structural roots, destinations, views, and direct evaluator, VM,
  baseline, and proof services. HIR/SSA rejects aggregates outside structural
  or invocation-region storage. Runtime keys and witness slots cannot cross
  processes; an experimental nested copy-list crosses an isolated process only
  as a semantic [snapshot](current-state/memory/polymorphic-value-plane-evidence.md).
- Resource categories and profiles use full category/profile/maxima/ceiling
  digests. The selected compiler-phase ledger is Current; one request-owned
  compiler/runtime ledger remains an accepted target.
- The measured semantic resource runtime, Linux observation, owner homes,
  scheduled optimizer/native integrations, and selected defaults are Current.
  See [exact evidence](current-state/semantic-resource-plane-evidence.md).
- The exact Linux [OS-resident runtime foundation](current-state/os-resident-runtime-evidence.md)
  has foreground `lkjscriptd`, exclusive lease, durable control, authenticated Unix control, CLI, and service files.
  Trusted VMs and supervised Linux process cells admit arguments, stdio, and clock
  grants with private state and bounded quotas. Durable process-app install, list,
  lifecycle, invoke, and restart recovery use authenticated control. The daemon-owned
  ordered database gives each running incarnation an isolated tenant provider and
  aborts it on lifecycle release. Authenticated ephemeral session presence is Current.
  Database VM operations, interactive cells, GUI, and non-Linux transports are not Current.
## Repository and agent platform
- `lkjscript describe --json` and `semantic describe` expose the deterministic closed contract registry.
- Capsule manifests, repository policy/provenance, graph/query outputs,
  capability status, and agent work state use stable schemas plus exact contract digests.
- The repository graph remains bounded and evidence-backed. Agent checkpoints
  remain revision-checked, append-only, and fail closed on stale semantic or repository identities.
- The machine `LKJ-DOC-GENERATION` rule rejects numbered language, schema,
  protocol, profile, ABI, and standalone generation names in Current-owned
  code, tests, fixtures, examples, config, and documentation. Immutable
  historical evidence is explicitly excluded rather than rewritten.
## Accepted targets not claimed Current
- evaluator dispatch beyond the fake-provider slice and native owned resources beyond borrowed `standard-input`;
- ranged lexical byte-slice source syntax and borrowed `str`;
- complete region/borrow/drop semantics for resource-bearing aggregates;
- structural domains, regions, and typed pools are Current and sealed owner traffic is node-count invariant.
  Language `sealed`, recursive products, indirect generics, structural lists, and full no-node-RC remain targets;
- a portable path policy beyond the Current Linux absolute-byte contract;
- a replacement persistent verified artifact cache after the first complete
  candidate failed its measured adoption gate and was removed;
- component interfaces, Wasm, AOT, native caches, and remote distribution;
- automatic baseline-to-proof promotion acceptance beyond its selected measured candidate;
- the semantic resource plane beyond its Current runtime slice: elastic/adaptive
  locality, blocking pools, real multi-domain adoption, and source structured concurrency;
- portability beyond Linux x86-64; host/database build for `wasm32-wasip1`
  and a fake-storage recovery probe runs, but VM/runtime-system do not build there;
The listed `bytes` subset is Current in all four tiers; the complete island remains Accepted.
## Verification authority
The canonical local gate is:
```sh
cargo run --locked -p lkjscript-xtask -- quiet verify
```
Runtime, Docker, retained-result, safety, fuzz, and performance gates are separate.
Report exact commands, commit, environment, result, and untested gates; an unrun gate is never implied.
