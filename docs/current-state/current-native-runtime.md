# Current State: Current Host Capabilities And Native Runtime

[Authority](../current-state.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

- Lossless bulk bytes: immutable `bytes`, affine `byte-vector`, strict UTF-8
  conversion, and checked-slice file partial-progress reads/writes are Current.
- Durable files and entropy: typed `file-writer`, `file-appender`, and
  `directory` runtime slots enforce exact sync/truncate/write access. Linux
  `getrandom` fill remains Current.
- SHA-256: fixed checked-slice digest returning immutable bytes is Current for verifier/integrity
  consumers; HMAC, password KDF, encryption, and WebAuthn remain absent.
- SQLite: `sqlite-connection` and `sqlite-statement` are statically and
  dynamically disjoint; prepared operations, bounded text/blob copies, and
  online backup use Linux `libsqlite3.so.0`.
- Public names include `standard-input`, `drop`, `read-resource-byte`,
  `write-resource-byte`, and `is-terminal`; `sys-*` names are internal stable
  runtime identities.
- Text socket operations are truthfully named `receive-string` and
  `send-string`. Send reports its byte count and uses Linux `MSG_NOSIGNAL`.
- SSA evaluator: independent of bytecode, VM, native, and host helpers; it
  covers exact scalar/control semantics, calls and recursion, SSA-converted
  local mutation, products, Option/Result, lists, strings, deterministic args,
  host-independent bytes and byte vectors, traps, exits, and explicit
  fuel/frame/allocation/byte/list bounds; console, filesystem, sockets, terminal, time, and typed
  resource operations return explicit unsupported-evaluator outcomes
- Callable baseline JIT: `lkjscript-jit` consumes only `VerifiedProgram`,
  lowers scalar `unit`/`bool`/`i64`/`f64` plus host-independent `string`,
  bytes, byte vectors, product, list, option, result, and monomorphic host-independent enum
  semantics and direct recursive SCC groups to `lkjscript-native`, installs
  bounded owned non-Send code objects through
  `lkjscript-executable`, and actually invokes generated System V AMD64 entries;
  scalar/direct native behavior stays unboxed and unchanged
- Native runtime ABI: semantic/runtime versions remain 1 and native canonical native contract is
  required. Enum-identified `EnterFunctionV1` and `PollV1` calls record entries
  and enforce cooperative fuel/deadlines; generated canonical native contract prologues call the
  encoder-owned `ReserveFrameV1` after only minimal ABI setup and before frame
  subtraction/initialization. The executable crate validates descriptor bytes, configured
  aggregate/per-frame limits, active-frame capacity, the exact configured
  active value/home/root budget, and guarded current pthread stack bounds. The
  executable invocation caches immutable current-thread stack bounds once, then checks
  each generated reservation without repeating pthread attribute queries.
  Registration itself records source-function entry and consumes the mandatory
  entry poll before body effects, avoiding two duplicate runtime calls; backedge
  polls remain explicit. Verified transitive may-collect summaries suppress caller
  publication calls for non-collecting scalar closures while retaining exact
  empty maps. The executable crate tracks exact reservation/release across nested frames. Collecting calls publish
  a dense safepoint, and every structured return/trap/exit/deadline/resource/host
  edge unregisters before status returns to the execution owner
- Engine modes: explicit `vm`, `baseline-jit`, `optimizing-jit`, and `auto` work; ordinary `run`
  defaults to `auto` at the conservative 64-entry threshold, explicit `vm`
  remains deterministic, forced baseline compiles the complete reachable
  supported SCC group before main effects and never falls back, and auto
  compiles scalar-adapter hot entries for later calls while conservatively
  retaining reference-signature and unsupported VM entries; compiled groups
  may contain reference helpers only as direct generated callees. Forced
  `optimizing-jit` proof-optimizes before effects, compiles the complete required
  reachable supported group, installs only `Tier::Optimizing`, enters optimized
  main, and returns an engine error rather than baseline/VM downgrade on proof,
  budget, support, install, or invocation failure. `auto` is baseline-only
- Tier/code ownership: the former observation hook is removed. Per-function
  states additionally distinguish `OptimizingCandidate`, `OptimizingCompiling`,
  and `OptimizedNative`; code-object tier is `Baseline` or `Optimizing`. Records
  retain saturating calls, bounded attempts, epoch/failure/object facts,
  and native entries. Code objects retain ABI/tier/group, size/accounting,
  relocation/runtime/safepoint/source/outcome, compile/install, invalidation,
  W^X, entry metadata, and bounded optimizing certificate/stat accounting under
  synchronous session ownership. Metrics separately count baseline/optimizing
  objects and entries, actual executed optimization passes, certificate
  byte estimates/records, explicit phase counters, and exact rewrite families
- Retained JIT evidence: opt-in low-overhead JSON metrics are separate from full
  diagnostics and never use stdout; allocation/object byte fields are labeled
  deterministic estimates, heap operation attempts and successes are distinct,
  and no collection-pause distribution is claimed. The standard-library harness
  polls process RSS, checks exact results and stream silence, randomizes at
  least four warmups plus 31 samples, and retains every sample and distribution
  under `meta/benchmarks/jit/results/`. The first `063668e` run is retained as
  **Rejected**: its 2.930761x optimizer-local result passed, but its historical
  scalar native sentinel ratio was 1.069928, above the 1.05 ceiling. After
  generated entry-poll transition recovery, the clean `cc967ff` run passed
  every criterion at 2.984780x and is **Adopted** for forced proof-optimizing
  performance. Automatic optimizing promotion remains disabled and unmeasured;
  no OSR, deoptimization, or speculation claim is made
- Native references and heap sites: typed opaque stable-handle words use exact
  Str/List/Option/Result/product/concrete-enum layout identities and verified
  frame homes, not raw object pointers; zero is accepted only for EmptyList/None;
  the Copy runtime-adapter token
  is non-Send/non-Sync.
  Bounded verifier-owned backward-CFG liveness charges every retained root
  before allocation and certificates sorted/deduplicated typed requirements for
  every direct/runtime call. The encoder consumes the certificate, and private
  structural image requirements prevent omitted/stale public maps from
  validating. `CollectReferenceV1` exercises exact non-empty traced roots while
  Poll/Enter stay non-collecting. `lkjscript-executable` alone retains raw active-frame
  addresses, validates the installed image/chain/maps, grows root capacity
  dynamically under an aggregate cap, copies typed roots to safe runtime
  services, writes back handles, and reports exact stack/frame/root outcomes.
  Runtime-service limits are distinct from materialization limits. Generic
  `HeapDispatchV1` sites retain canonical operation-specific
  input/result/layout/allocation/store facts, including nominal product field,
  List/Option/Result payload, and enum/variant/field/layout/tag/substitution
  identities, plus arbitrary bounded typed arguments/result homes, source
  identity, and safepoint; the executable crate copies
  values/roots into safe `GcHeap` services, writes roots back, re-materializes
  moved arguments, and writes exact results. Caller/callee chains, dead-root
  exclusion, bounds,
  structured failures/outcomes, W^X, and repeated installation are tested
- Native source limits: the callable SSA adapter rejects indirect calls,
  polymorphic/unsupported signatures and enum substitutions, Symbol,
  Handle/host IO, and lexical Owned/Ref/RefMut. Scalar canonical native contract maps remain exactly empty; supported
  host-independent reference operations have exact non-empty maps. Native/VM
  reference transitions are absent, so per-function auto-entry eligibility
  prevents a compiled reference helper from ever labeling a direct VM call
  native. Explicit trap sites carry deterministic selected message identity
  through lowering, image metadata, sys outcome, and JIT lookup
- Absent selected/later tiers and surfaces: automatic optimizing promotion has
  an **Accepted Implementation Selection** but is not Current; broader optimizing
  passes, loop OSR, background compilation, speculative tiers, deoptimization,
  Handle/host native allocation, native/VM reference transitions, persistent
  profiles, and persistent code caches remain absent
## Semantic Source foundation Evidence

The Foundation legacy contract implementation tree based on `e71c976` was checked on Ubuntu
24.04.4 LTS, Linux 7.0.0-27-generic x86-64, with Rust/Cargo 1.96.0. It changes
