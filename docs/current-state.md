# Current State

## Purpose

State implemented behavior, evidence boundaries, known defects, and accepted
next work without mixing them with long-term vision.

## Status

**Current** for the implementation section. Repairs and future products are
explicitly labeled **Accepted Target**, **Placeholder**, **Deferred**, or
**Rejected**.

## Resolved AI-Native Redesign Baseline

The redesign started from clean `main` at
`f6410a22bdcf8e516e5f2a771428f74edf4fcfa1`, one commit after the observed
forced-optimizer evidence commit `ccf53f1937951fc2535b2359fe704c66be0c1010`.
That additional commit selected synchronous automatic proof promotion but did
not implement it. The baseline host was Ubuntu 24.04.4 LTS, Linux
7.0.0-27-generic x86-64, Rust `1.96.0 (ac68faa20 2026-05-25)`, Cargo `1.96.0
(30a34c682 2026-05-25)`, with 96 GiB free on the 512 GiB artifact filesystem.
The workspace had nine Rust packages, zero third-party Cargo packages, and a
clean tree tracking `origin/main`.

Deterministic SHA-256 manifests hash lines of `<file SHA-256><two spaces><tracked
path><LF>` in byte-sorted tracked-path order:

| Baseline set | Files | Bytes | Manifest SHA-256 |
| --- | ---: | ---: | --- |
| canonical `src` `.lkjscript` corpus | 109 | 67,520 | `84fbbac1ba744ed9376f8e98dcf3389d2d914fd2b22d88fcda39dcda022c87d3` |
| all tracked `.lkjscript` sources/fixtures/workloads | 113 | 69,351 | `78d8469b697c4b6672f28bf47cfb7f96373151ef42c24b85321e6e571df3b737` |
| `meta/bench`, `meta/benchmarks`, and `meta/scripts` evidence | 31 | 2,814,103 | `4779093114cfe083bd35a3331f5df502687bcf2a14cdc390b9e5a4816ce81af3` |
| `AGENTS.md` and tracked `docs` | 57 | 457,145 | `36a81144ed96508764f1ce435713ee4f0a86d33a942d6dc30865036e23ba9e5e` |
| complete tracked tree | 287 | 5,050,416 | `1c07d270a667373edac03d5dd224e0deae8f8b2b3b7e0099624547f3b0b0ae34` |

The immutable Brainfuck reference source hash was
`af6250f93ef18b35e35788958e6c1feed1a20155011e7208546940661dbedf1d`;
the benchmark driver hash was
`bd9d6e7f237834592941d53fde484a66cf99c3f240b22c2141003176e92bb220`.

Baseline commands actually run on that clean commit:

| Command/gate | Result |
| --- | --- |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | exited 0 in 7 s; canonical format/Clippy/docs/tree/source and workspace tests passed |
| `cargo build --workspace --release --locked` | exited 0 in 15 s |
| default/VM/forced-baseline/threshold-2-auto scalar, forced optimizing, VM hello, and VM Mandelbrot runs | all exited 0; hello was `3628800`; Mandelbrot retained its canonical output |
| `python3 meta/benchmarks/brainfuck/benchmark.py --mode smoke --no-build` | exited 0; direct/run-folded correctness and failure checks passed |
| lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smoke scripts with the release binary | all exited 0 |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | exited 0 in 29 s with `result=ok` |

No retained performance sampling, full Brainfuck Mandelbrot, Miri, sanitizer,
fuzzer, non-Linux target, AArch64, or Wasm/component acceptance was run for this
baseline. The baseline runs establish current health; they do not establish any
new redesign Target as Current.

## Current Implementation

- Repository: `https://github.com/lkjsxc/lkjscript`
- Canonical source: `.lkjscript`; other extensions are rejected without shims
- Corpus: all canonical language files under `src` have executable roots that
  cover the exact corpus closure
- Physical format: one column-one marker/atom per line with matched markers and
  raw `str/`, `name/`, and `import/` blocks
- Source limits: depth 8, form children 16, tokens 384, top-level forms 8,
  product fields 15, and 16 combined immediate files/directories per source
  directory
- Source-tree scope: the width rule applies to language source directories,
  not Rust, docs, metadata, `.git`, or generated Cargo output
- Imports: contained `std/`, `lib/`, `examples/`, and `./` paths with installed
  fallback through `LKJSCRIPT_ROOT`; absolute, parent, wrong-extension, cycle,
  and canonicalized symlink escapes fail
- Compiler boundary: one analysis pass collects immutable headers and produces
  owned, resolved typed HIR with explicit Main and Functions, BindingIds,
  local-slot references, MutableLocal/SetLocal, ProductIds, numeric field
  identities, dense TraitIds/ImplIds, resolved marker bounds and witnesses,
  source origins, exact operation/type facts, and deterministic fixed-point
  function effect summaries; HIR lowers once into verified typed
  SSA, deterministic baseline normalization, and then reference bytecode
- Typed SSA: dependency-free `lkjscript-ir` owns dense function/block/value
  identities, exact types, nominal product metadata, dense trait/impl metadata,
  generic signature bounds, canonical substitutions and erased marker witness
  identities, explicit block parameters and terminators, direct/indirect/runtime calls, effects,
  safepoints, frame states, source origins, verification, an independent
  bounded evaluator, deterministic isolated baseline passes, and bytecode link
  metadata; SSA conversion renames local mutation and uses stable BindingId-
  ordered block parameters at branch and loop joins
- Proof optimization: `lkjscript-ir` provides bounded deterministic discovery
  and separate verification for ordered stable-ID certificates, with opaque
  `VerifiedOptimizedProgram` authority. Current edits cover exact I64 xor/or
  zero, and/all-ones, idempotent and/or, Bool double-not, and same-block or
  dominating exact scalar GVN/CSE. Duplicate checked I64 arithmetic/division is
  legal only behind an earlier identical dominating successful check. The
  checker builds private immutable semantic and CFG indexes without calling
  discovery legality or dominance helpers, independently checks the complete
  record sequence, reconstructs a private candidate, requires exact bitwise
  equality, verifies edit and cleanup stages, and rejects stale, forged,
  non-dominating, effectful, oversized, or aggregate-over-budget proofs
- Host implementation: nine Rust workspace crates with no third-party Rust
  dependencies; unsafe Rust is confined to `lkjscript-sys`
- Quality gate: the complete Rust workspace is rustfmt-clean and passes strict
  Clippy for all targets/features; docs status/links, explicit `PLACEHOLDER`
  labels, and exact source-closure coverage are machine-checked
- Runtime: dense bytecode lowered only from normalized SSA, contiguous stacks,
  pure session-owned stable-index `GcHeap` in `lkjscript-core`, precise
  non-moving mark-sweep shared as the VM/JIT heap implementation, monotonic
  non-reused session indices preventing stable-handle ABA, traced immutable
  products, transactional mutation with rollback and checked deterministic
  estimated-object-byte deltas, transitive-only returned snapshots, bounded
  allocation/estimated-byte/collection counters and stress policy, explicit
  validated `Trap`, and return-adjacent tail-frame reuse
- Execution boundary: mutable `Chunk` is builder-only for malformed-bytecode
  construction; one whole-chunk validator produces opaque immutable
  `ValidatedChunk`, and VM, disassembly, and the JIT observation seam accept
  only validated input; compiler `ExecutableProgram` retains verified
  normalized SSA, deterministic function/prototype/main and SSA/bytecode link
  metadata, and validated bytecode through an explicit accessor
- Outcomes: VM execution distinguishes returned, exited, trapped, deadline,
  resource-limit, and host-failure outcomes; the core does not terminate the
  process, returned heap values own their reachable storage, and cleanup occurs
  before CLI exit-status translation
- Runtime budgets: explicit configuration bounds fuel, stack values, frames,
  estimated live heap, aggregate allocations, handles, output, and cooperative
  wall time; hard-deadline mode rejects host wrappers that cannot guarantee
  cancellation
- Semantics: executable roots have exactly one no-parameter typed main;
  imports contain declarations only; top-level `do` and runtime value defs are
  removed; `var` introduces one exactly typed mutable local and local-only
  `set` returns Unit; Unit, typed empty-list, and Option none have distinct
  singleton tags, while Option some is traced; `nil`, `Nil`, `nil?`, and
  `null?` are removed; `arg` returns `Option Str`; universal `eq`/`ne` are
  removed in favor of exact value, object-identity, bounded structural-list,
  and F64-bit equality families; nominal products have ordered named fields,
  exact construction, access, and immutable replacement
- Ownership safe island: exact `Owned Buf`, `Ref Buf`, and `RefMut Buf` types;
  fresh `owned-buf-new`; whole-local `move`/`borrow`/`borrow-mut`; a 16,384-node
  aggregate ownership-analysis budget; lexical place initialization/end;
  same-block last-use loans; exact branch ownership joins; and evaluator plus
  reference-bytecode execution using the safe arena handle representation.
  Public SSA independently verifies explicit place initialization/end,
  canonical current owners, affine transfer, owner block arguments, bounded
  forward CFG state plus a 131,072-cell retained-state cap, exact joins,
  same-block loan uses, and global LoanId uniqueness after every pass. General
  SSA CFG validation requires dense block order, at most 4,096 blocks per
  function, bitset dominators, and at most 4,194,304 charged word operations.
  Affine cross-block values require explicit typed block arguments. `Owned Buf` is
  affine, shared references are
  Copy, exclusive references are affine, and all three are
  worker-local/non-Send/non-Sync. Legacy `Buf` semantics are unchanged.
  Borrow is accepted only as an exact direct reference argument or direct let
  initializer; temporary loans cover the full call/runtime-operation.
  Ownership/reference generic instantiation and direct/nested product or
  collection storage are rejected. References cannot escape, Borrow results
  cannot cross SSA blocks, loop cycles reject Move/Borrow and cannot carry
  changed owner/loan state, `RefMut` user-call forwarding is rejected, and
  cleanup is not deterministic user `Drop`
- Marker traits: declaration-only top-level traits and exact nominal-product
  impls are Current across imports; generic `bounds/` are solved at concrete
  calls with exact explicit ImplId or structural auto-trait witnesses. Core
  `Copy`/`Clone`/`Drop`/`Send`/`Sync` identities are reserved and no core-trait
  implementation may be asserted by source in this slice. Unit/Bool/I64/F64,
  Str/Symbol, and structurally eligible List/Option/Result/product composition
  derive `Copy`; only Unit/Bool/I64/F64 currently derive `Send`/`Sync` because
  every heap reference is worker-local. Buf/Handle/function types derive none
  of those facts. Solver and verifier depth/work are bounded,
  recursive product cycles are deterministic errors, and the exact loaded
  source closure is the temporary coherence domain. Bounded generics require a
  concrete direct call; generic-context forwarding and first-class bounded
  function values are explicitly rejected in this slice. Methods,
  associated items, generic/blanket impls, specialization, dynamic dispatch,
  and package orphan rules are not Current
- Numerics: canonical I64/F64 only; complete I64 uses signed 61-bit immediates
  plus boxed wide values, F64 remains distinct, arithmetic/comparison is
  checked or IEEE as declared, and narrower host domains reject truncation
- CLI: `run`, real bytecode `disasm`, help, and version; the unlabeled REPL stub
  was removed
- Workloads: hello, native lkjscript Mandelbrot, Brainfuck interpreted by
  lkjscript, lkjedit, one-shot HTTP, and Leibniz comparison; Brainfuck,
  terminal, and editor state is passed explicitly in immutable nominal products
  and evolved through local vars
- Resource handles: integers are rejected, stdin uses a reserved borrowed token,
  owned file/socket tokens are monotonic, and closed tokens are never reused
- Terminal ABI: arbitrary ioctl is absent; fixed `sys-tty-get`/`sys-tty-set`
  operations validate the exact 60-byte Linux state before FFI and return Results
- System Results: open, path existence, close/read/write, `isatty`, time,
  socket, poll, terminal, and terminal-guard failures return operation-qualified
  `ResultErr` values; standard wrappers unwrap explicitly
- Lossless bulk bytes: bounded `Buf` UTF-8 conversion and offset/length-checked
  file/socket partial-progress reads and writes are Current; legacy Str socket
  operations remain only for old examples
- Durable files and entropy: append/create-new/directory handles, sync,
  truncate, same-filesystem rename, and Linux `getrandom` buffer fill are
  Current; application framing/recovery policy remains in language code
- SHA-256: fixed bounded-buffer digest is Current for verifier/integrity
  consumers; HMAC, password KDF, encryption, and WebAuthn remain absent
- SQLite: generic owned connection/statement handles, prepared operations,
  exact bounded text/blob copies, and online backup are Current through the
  Linux `libsqlite3.so.0` system library; schema and storage policy stay in
  language consumers
- Canonical resource names: `stdin-handle`, `sys-close`, `sys-read-byte`,
  `sys-write-byte`, and `sys-isatty`; descriptor-era aliases are absent
- Send behavior: successful `sys-send` reports its byte count and uses Linux
  `MSG_NOSIGNAL` instead of risking process termination on a broken peer
- SSA evaluator: independent of bytecode, VM, native, and host helpers; it
  covers exact scalar/control semantics, calls and recursion, SSA-converted
  local mutation, products, Option/Result, lists, strings, deterministic args,
  host-independent buffers, traps, exits, and explicit fuel/frame/allocation/
  buffer/list bounds; console, filesystem, sockets, terminal, time, and handle
  operations return explicit unsupported-evaluator outcomes
- Callable baseline JIT: `lkjscript-jit` consumes only `VerifiedProgram`,
  lowers scalar Unit/Bool/I64/F64 plus host-independent Str, legacy Buf,
  Product, List, Option, and Result semantics and direct recursive SCC groups to
  `lkjscript-native`, installs bounded owned non-Send code objects through
  `lkjscript-sys`, and actually invokes generated System V AMD64 entries;
  scalar/direct native behavior stays unboxed and unchanged
- Native runtime ABI: semantic/runtime versions remain 1 and native ABI 2 is
  required. Enum-identified `EnterFunctionV1` and `PollV1` calls record entries
  and enforce cooperative fuel/deadlines; generated ABI-2 prologues call the
  encoder-owned `ReserveFrameV1` after only minimal ABI setup and before frame
  subtraction/initialization. Sys validates descriptor bytes, configured
  aggregate/per-frame limits, active-frame capacity, the exact configured
  active value/home/root budget, and guarded current pthread stack bounds. The
  sys invocation caches immutable current-thread stack bounds once, then checks
  each generated reservation without repeating pthread attribute queries.
  Registration itself records source-function entry and consumes the mandatory
  entry poll before body effects, avoiding two duplicate runtime calls; backedge
  polls remain explicit. Verified transitive may-collect summaries suppress caller
  publication calls for non-collecting scalar closures while retaining exact
  empty maps. Sys tracks exact reservation/release across nested frames. Collecting calls publish
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
  Buf/Str/List/Option/Result/product layout identities and verified frame homes,
  not raw object pointers; zero is accepted only for EmptyList/None; the Copy runtime-adapter token is non-Send/non-Sync.
  Bounded verifier-owned backward-CFG liveness charges every retained root
  before allocation and certificates sorted/deduplicated typed requirements for
  every direct/runtime call. The encoder consumes the certificate, and private
  structural image requirements prevent omitted/stale public maps from
  validating. `CollectReferenceV1` exercises exact non-empty Buf roots while
  Poll/Enter stay non-collecting. `lkjscript-sys` alone retains raw active-frame
  addresses, validates the installed image/chain/maps, grows root capacity
  dynamically under an aggregate cap, copies typed roots to safe runtime
  services, writes back handles, and reports exact stack/frame/root outcomes.
  Runtime-service limits are distinct from materialization limits. Generic
  `HeapDispatchV1` sites retain canonical operation-specific
  input/result/layout/allocation/store facts, including nominal product field
  and List/Option/Result payload identities, plus arbitrary bounded typed
  arguments/result homes, source identity, and safepoint; sys copies
  values/roots into safe `GcHeap` services, writes roots back, re-materializes
  moved arguments, and writes exact results. Caller/callee chains, dead-root
  exclusion, bounds,
  structured failures/outcomes, W^X, and repeated installation are tested
- Native source limits: the callable SSA adapter rejects indirect calls,
  polymorphic/unsupported signatures, Symbol, Handle/host IO, and lexical
  Owned/Ref/RefMut. Scalar ABI-2 maps remain exactly empty; supported
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

## SQLite Evidence

The SQLite implementation tree was verified on Linux x86-64 with the system
`libsqlite3.so.0` using:

- `cargo run --locked -p lkjscript-xtask -- quiet verify` (passed);
- `cargo build --workspace --release --locked` plus HTTP, bulk-byte, durable,
  SHA-256, and SQLite smokes (passed);
- `docker compose -f meta/docker-compose.yml --profile verify run --build --rm
  verify` (passed; `sqlite-smoke ok`).

These are VM and generic host-boundary results. They are not JIT evidence and
do not establish application durability or migration behavior.

## Accepted AI-Native Platform Direction

[AI-Native Language And Platform](decisions/ai-native-platform.md) supersedes
implementation-era permanent assumptions while preserving the Current typed
HIR/verified SSA, opaque verifier/proof authority, exact roots, structured
outcomes, deterministic budgets, W^X, and fallback-free forced native evidence.
The accepted destination is versioned Semantic Source, semantic agent edits,
Edition 2 ADTs and exact control/numerics, explicit capabilities, hybrid safe
memory, reproducible packages/components, one semantic IR family, and a
measured evaluator/VM/JIT/AOT/cache/Wasm portfolio. None of those absent layers
is Current by adoption of the direction.

The immediate repository-wide implementation priority is [Semantic Source And
Agent Protocol](decisions/semantic-source-and-agent-protocol.md), followed by
[Resource Budget Profiles](decisions/resource-budget-profiles.md). All Current
source limits remain enforced until aggregate replacements are Current. The
[Measured Execution Portfolio](decisions/execution-portfolio.md) reclassifies
AOT, content-addressed native caches, and optional explicit local PGO as later
measured Targets; it does not add an engine or change Current runtime policy.

The marker-trait foundation, initial `Owned Buf` ownership safe island, exact
ABI-2 frames/roots, host-independent source allocation/recursive SCC slice, and
forced first optimizing pipeline remain Current. General ownership, full static
trait methods/associated items, Handle/host native calls, native/VM reference
transitions, and broader proof optimization remain absent. Their existing
accepted contracts remain compatible foundations unless explicitly superseded.

## Accepted Target: Automatic Baseline-To-Proof Promotion

The next automatic promotion slice has an **Accepted Implementation Selection**,
not a Current implementation. It preserves the existing 64-VM-entry automatic
baseline threshold and keeps proof promotion CLI-opt-in and disabled by default
until retained adoption. Candidate optimizing thresholds count exactly 64,
256, 1,024, or 4,096 baseline entries of one promotion root. The Nth entry
synchronously proves, lowers, and W^X-installs an optimizing object but must
invoke the captured baseline object; only a later entry can publish the pending
optimized object.

Opaque entry tokens bind function, object, and tier. One process-local session
exactly owns coexisting baseline/optimizing objects, one current selection, and
at most one pending selection. Bounded stale invalidated objects remain owned
until drop but are never selectable. The states are `BaselineCandidate`,
`BaselineCompiling`, `BaselineNative`, `OptimizingCandidate`,
`OptimizingCompiling`, `OptimizingPending`, `OptimizingNative`, and `Disabled`.
There is one attempt per explicit epoch, a bounded total, same-epoch
suppression, structured tier failure with baseline retained, and epoch-driven
optimized invalidation/retry back to baseline.

Auto entry remains scalar-only; source main remains in the VM. Generated
reference helpers may call and allocate inside their native group but cannot
transfer references at VM/native entry or become auto roots. Forced tiers remain
unchanged and fallback-free. Required evidence records thresholds/enables,
epochs, attempts/failures/suppressions, transitions, exact tier entries and
objects/code bytes, baseline entries and time before first optimized entry,
proof/W^X, stale invalidations, and bounded mappings/work/certificates/metadata.

The predeclared clean locked release gate randomizes at least four warmups and
31 samples for auto baseline-only, thresholds 64/256/1,024/4,096, unchanged
forced sentinels, and allocation/reference correctness. Adoption requires at
least 1.10x median process speedup, improvement greater than twice combined
MAD, p95 no more than 5% worse, compile cost repaid, exact oracle/stream/state/
proof/W^X results, historical scalar native and process medians within 5%, and
no repeated attempt or fallback. The largest passing candidate within twice
combined MAD of the fastest passing median is selected. Otherwise optimizing
stays disabled and the complete rejection is retained. See [Proof-Based
Optimizing JIT](decisions/proof-based-optimizing-jit.md) for the authoritative
contract.

Longer-term accepted sequences for [staged self-hosting](decisions/self-hosted-platform-roadmap.md),
[modules and reproducible packages](decisions/modules-and-packages.md),
[isolates and structured concurrency](decisions/isolates-and-structured-concurrency.md),
[the Web platform](decisions/web-platform-roadmap.md), and [the first-party
relational database](decisions/relational-database-roadmap.md) are explicitly
not Current implementation claims.

## Known Defects

The source identity cutover does not make the runtime semantically complete.
The highest-priority defects are:

1. some library file operations remain per-byte or quadratic; application-level
   storage recovery is a language-consumer responsibility;
2. source/import aggregate bytes and counts are not comprehensively bounded;
   bytecode tables/data/code/metadata and VM execution resources are bounded;
3. cooperative deadlines can overrun inside filesystem, console-write,
   send/write, terminal-cleanup, or other non-cancellable wrappers;
   hard-deadline mode reports those operations as unsupported `HostFailure`
   before effects; live-heap accounting is estimated at VM instruction
   boundaries, and `print` builds its host-format string before the output check;
4. stdin/stdout and the terminal guard remain process-global, so concurrent VM
   supervision is unsupported; handle metadata is VM-local and bounded but
   monotonically allocated until that VM ends.

## Evidence

The retained optimizing-JIT benchmark-only work at clean commit
`cc967ff7e6f57a3225ae974d64ced6039ed8e9ae` was run on Linux
7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX, with Rust/Cargo 1.96.0. It changes
no engine policy and makes only a forced first-tier performance measurement;
automatic promotion remains disabled and unmeasured.

| Retained optimizing benchmark command or check | Result |
| --- | --- |
| `python3 meta/benchmarks/jit/benchmark.py` | clean locked workspace release build passed; exact silent reference-VM I64 `3333` oracle and all forced outcomes passed; all 4 warmups and 31 measured samples per case were retained in deterministic randomized interleaving with monotonic process wall and polled peak RSS |
| same-commit optimizing workload | baseline native median/MAD exactly 1,999,889/10,469 ns; optimizing 670,029/2,174 ns; 2.984780x speedup; 1,329,860 ns improvement versus 25,286 ns twice-combined-MAD threshold; process wall 3,565,363 to 2,411,023 ns, 1.478776x; 72 checked proof records, 2,424 optimizing versus 13,656 baseline code bytes, 10,001 optimizing entries, zero baseline entry/fallback, and verified W^X |
| retained scalar sentinel comparison | native median 7,982,586 ns versus retained 7,647,935 ns, ratio 1.043757: passed the 1.05 ceiling; process median 9,207,038 versus 9,372,036 ns, ratio 0.982395: passed. This cross-commit sentinel includes compiler/runtime evolution and does not attribute recovery to optimizing passes |
| allocation-graph correctness metrics, once | exact I64 `1`; 3 optimizing entries, 7 allocations, 6 collections, 14 attempted/14 successful heap calls, maximum 3 roots, 6 barriers, zero baseline entry/fallback, and W^X |
| mechanical verdict | **Adopted** for forced first proof-optimizing performance because every exact criterion passed; automatic promotion remains disabled and unmeasured, with no OSR/deoptimization/speculation claim |
| prior retained run | `063668e` remains **Rejected** in `optimizing-jit-linux-x86_64-rejected-scalar-regression.json`: optimizer-local 2.930761x passed, scalar native 8,182,742/7,647,935 ns = 1.069928 failed, and scalar process 9,340,049/9,372,036 ns = 0.996587 passed; its record truthfully lists dirty benchmark README/harness/cache paths. Folding the mandatory entry poll into frame registration removed one runtime transition before the clean adopted rerun without weakening polls or proofs |
| retained JSON validation; `cargo run --locked -p lkjscript-xtask -- check-docs`; `git diff --check` | passed; both JSON files parsed, commit/verdict/SHA-256 identities matched, all 10 adopted criteria were true, exact medians and 1.478776 process ratio matched, docs links passed, and the diff had no whitespace errors |
| Not tested | benchmark rerun, full canonical workspace verification, Docker, automatic optimizing promotion, OSR, deoptimization, speculation, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The final forced-optimizer hardening in this document's containing commit,
based on `114196422fb41b8c1b1dab6304c1680000cf67ed`, was checked in the
primary Linux 7.0.0-27-generic x86-64 checkout with Rust/Cargo 1.96.0. It closes
aggregate cleanup/preflight/pass-accounting and structured pre-entry evidence,
and replaces per-entry pthread stack queries with one invocation-bound query.

| Final forced-optimizer command or check | Result |
| --- | --- |
| focused IR/JIT/sys/app tests | passed; type-parameter-vector preflight, aggregate worst-case cleanup charging, unreachable-before-copy cleanup, validation-inclusive pass totals, and nonzero optimizing entry evidence for zero stack/frame structured outcomes plus prior proof/root/allocation coverage |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | passed |
| docs/tree/source checks and `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; rustfmt, strict Clippy, exact source closure, 213 unit/integration tests, and one non-Send compile-fail doctest |
| `cargo build --workspace --release --locked`; default hello, forced scalar/allocation/optimizing JIT, Mandelbrot, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed; declared optimizer workload returned I64 `3333`, retained 72 checked-I64 proof records, emitted 2,724 optimizing bytes, entered optimizing code 10,001 times, and recorded zero baseline entries/fallback; allocation optimization returned I64 `1` with 3 optimizing entries and zero downgrade |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok`, 213 tests plus the compile-fail doctest, and all configured smokes |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | retained performance sampling, automatic promotion, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The adversarial proof-optimizer repair in this document's containing commit,
based on `1f9999854d91e3abc033c555bd465f8ce1be36c1`, was checked in an
isolated Linux 7.0.0-27-generic x86-64 worktree with Rust/Cargo 1.96.0 and 96
GiB free in the shared artifact filesystem.

| Adversarial proof-optimizer command or check | Result |
| --- | --- |
| focused IR and app optimizer/JIT/CLI tests | passed; independent forged-proof rejection, exact checked trap identity from source, public oversized-candidate/growth rejection, charged duplicate-expression width, unreachable diamond/loop cleanup, optimizing recursive live roots/maps, help, metrics fields, and retained prior optimizer/JIT coverage |
| `cargo test --locked --workspace` | passed; 213 unit/integration tests plus the non-Send compile-fail doctest |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`; `cargo fmt --all -- --check` | passed |
| separate `check-docs`, `check-tree`, and `check-sources`; `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; the canonical gate reran formatting, strict Clippy, docs/tree/source closure, all 213 tests, and the compile-fail doctest |
| `cargo build --locked --workspace --release`; forced release baseline scalar, optimizing scalar, baseline allocation graph, and optimizing allocation graph | passed; all four smokes exited zero with empty stdout/stderr and no forced downgrade |
| `git diff --check` | passed |
| Not tested | Docker, performance sampling, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The forced first proof-optimizing implementation in this document's containing
commit, based on `cd4eee2d9381decf98ef89f6dc9f8526cbea3aa8`, was checked in an
isolated Linux x86-64 worktree with Rust/Cargo 1.96.0. It makes only the forced
first pipeline Current; it does not select automatic promotion or establish the
1.20x aspirational performance gate.

| First proof-optimizing command or check | Result |
| --- | --- |
| `cargo test --locked --workspace` | passed; 209 unit/integration tests plus the non-Send compile-fail doctest, including deterministic certificates, same-block/dominator checked GVN, forged proof rejection, 64 randomized scalar differentials, evaluator/VM/baseline/optimizing exact outcomes, allocation graphs, traps/exits/deadline/fuel, unsupported/budget no-downgrade, W^X, and entry/tier facts |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | passed |
| separate `check-docs`, `check-tree`, `check-sources`; `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; canonical source closure includes the new optimizing workload and the gate reran formatting, strict Clippy, all 209 tests, and the compile-fail doctest |
| `cargo build --workspace --release --locked`; forced scalar baseline, allocation baseline, explicit-VM optimizing workload, forced optimizing scalar/allocation smokes | passed with silent normal streams; optimizing workload returned exact I64 `0`, installed one optimizing object, entered optimizing code 10,001 times, retained 4 records (3 algebraic, 1 GVN, 1 checked-I64 subset), emitted 2,788 versus baseline 3,405 code bytes, and had zero baseline entries/objects or VM fallback; optimizing allocation returned exact I64 `1` with 7 allocations, 6 collections, 14 attempted/14 successful heap calls, and zero downgrade |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | Docker, 1.20x performance sampling, automatic promotion, broader optimization passes, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The final allocation-baseline hardening in this document's containing commit,
based on `7942d4e0d57e863b9ffe071cf07dc3ad252c1e23`, was checked in the
primary Linux 7.0.0-27-generic x86-64 checkout with Rust/Cargo 1.96.0. It closes
remaining exact ABI, evaluator accounting, trap identity, stable-index, and
structural-layout boundaries without changing canonical language sources.

| Final allocation-hardening command or check | Result |
| --- | --- |
| focused core/IR/native/sys/JIT/VM/app tests | passed; exact heap-site ABI identity, incremental list equality and error propagation, evaluator buffer payload/wrapper allocation limits, full-u32 explicit trap sites, stable-handle ID exhaustion, and collision-free interned nested layouts plus prior allocation/native coverage |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | passed |
| docs/tree/source checks and `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; rustfmt, strict Clippy, exact source closure, 202 unit/integration tests, and one non-Send compile-fail doctest |
| `cargo build --workspace --release --locked`; default hello, forced scalar/allocation JIT, Mandelbrot, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed; allocation graph returned I64 `1` with 3 native entries, 7 allocations, 6 collections, maximum 3 roots, 14 attempted/14 successful heap calls, 6 barriers, and zero fallback; Mandelbrot remained 1,176 bytes with SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok`, 202 tests plus the compile-fail doctest, and all configured smokes |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | performance sampling, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The adversarial allocation-baseline repair in this document's containing
commit, based on `3467137b3e2ad9cf15ff55cd4cf38a134126e373`, was checked in
an isolated worktree on Linux x86-64 with Rust/Cargo 1.96.0. It repairs the
Current host-independent slice; Handle/host calls, native/VM reference
transitions, the complete allocation-capable decision, and collection-pause
measurement remain outside this evidence.

| Adversarial repair command or check | Result |
| --- | --- |
| focused core/native/sys/JIT/VM/app tests | passed; auto reference-helper entry gating, non-reused same-layout stale handles, canonical malformed heap descriptors, moving-service argument re-materialization, buffer Result boundaries, MAX/MAX+1 list equality, selected callee trap identity, zero/tiny native active values, transactional mutation rollback/limits, reachable-only snapshots, and attempted/successful heap-call metrics plus retained prior coverage |
| strict workspace Clippy, all targets/features | passed with `-D warnings` |
| separate docs/tree/source checks and `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict Clippy, exact source closure, 193 unit/integration tests, and one compile-fail doctest |
| locked workspace release build; default/VM/forced/threshold-2-auto scalar, VM hello, forced allocation-graph metrics, and Brainfuck smoke | passed; scalar streams were empty, hello was exact `3628800`, allocation graph returned I64 `1` with 14 attempted/14 successful heap calls, 7 allocations, estimated-byte keys, and zero fallback, and Brainfuck direct/run-folded correctness/failure boundaries passed |
| metrics parser correction | the first local parser invocation failed because the metrics file intentionally begins with `LKJSCRIPT_METRICS `; the generated program had exited successfully. A corrected prefix-aware parser was run and passed |
| Not tested | Docker, performance sampling, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

The host-independent source allocation/recursion slice in this document's
containing commit, based on `0daa7a0d3064ad487cee2154d91f9db0a0fc0c82`,
was checked in isolated worktree
`/tmp/pi-agent-d9f4b948-568f-497-2a12ad4f` on Linux 7.0.0-27-generic x86-64
with Rust/Cargo 1.96.0. Canonical Brainfuck source was unchanged.

| Source allocation/recursion command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-core -p lkjscript-native -p lkjscript-sys -p lkjscript-jit -p lkjscript-vm -p lkjscript-app` | passed; shared heap boundaries, malformed heap sites/classes/homes, generic three-argument frame-home dispatch, service trap/resource/host propagation, existing CollectReferenceV1 certificates, source forced collection through direct/mutual recursive live-reference frames, nested Product/Option/Result/List/Str/Buf evaluator/VM/native equality, tiny allocation/heap limits, ownership rejection, W^X and existing scalar gates |
| `cargo clippy --locked -p lkjscript-core -p lkjscript-native -p lkjscript-sys -p lkjscript-jit -p lkjscript-vm -p lkjscript-app --all-targets --all-features -- -D warnings` | passed |
| separate `check-docs`, `check-tree`, and `check-sources` | passed; canonical language sources, including Brainfuck, were unchanged |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict workspace Clippy, docs/tree/source closure, 182 unit/integration tests, and one compile-fail doctest |
| `cargo build --workspace --release --locked`; scalar default/VM/forced/threshold-2-auto, explicit-VM hello, Brainfuck smoke, and forced allocation-graph metrics smoke | passed; allocation graph returned exact I64 `1`, recorded 3 native entries, 7 allocations, 6 collections, maximum 3 roots, 14 successful heap calls, 6 barriers, zero fallback, and empty stdout |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | Docker, performance sampling, full Brainfuck Mandelbrot, Handle/host native calls, native/VM reference transitions, Miri, sanitizers, or non-Linux targets |

This evidence makes only the host-independent source allocation/recursion slice
Current. It does not establish the full allocation-capable target, an optimizing
tier, or OSR.

The exact native-root repair in this document's containing commit, based on
`cc7ad01c9365b659a8cf909c400788aadde4770a`, was checked in isolated worktree
`/tmp/pi-agent-0917730b-997b-416-8744f760` on Linux 7.0.0-27-generic x86-64
with Rust/Cargo 1.96.0. It establishes pre-touch guarded frame reservation,
verifier-certified root completeness with a private image check, bounded root
construction, exact runtime-service resource classification, and dynamic
shallow-root capacity. It does not establish source-level native allocation or
a shared VM/native heap.

| Exact native-root repair command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-native -p lkjscript-sys` | passed; verifier certificate/adversarial width, omitted-live-root corruption, 64 KiB thread stack rejection, zero-frame bound, configured byte limits, exact reservation release, runtime-service classification, and a valid shallow 1,025-root map plus existing native/sys coverage |
| `cargo clippy --locked -p lkjscript-native -p lkjscript-sys -p lkjscript-jit --all-targets --all-features -- -D warnings` | passed |
| `cargo run --locked -p lkjscript-xtask -- check-docs` | passed |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict workspace Clippy, docs/tree/source closure, 179 unit/integration tests, and one compile-fail doctest proving the Copy adapter token is non-Send |
| `cargo build --workspace --release --locked`; default hello, forced scalar JIT, Mandelbrot, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed; Mandelbrot retained its exact 1,176-byte output and SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok`, 179 tests plus the compile-fail doctest, and all configured smokes |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | source-level native allocation, shared VM/native collection, performance, full Brainfuck Mandelbrot, Miri, sanitizers, or non-Linux targets |

The closed-machine-plan native-reference/active-frame implementation in this
document's containing commit, based on HEAD
`ec54cde9b93a302c1310d2107c10b785001f184d`, was checked on Linux
7.0.0-27-generic x86-64 with Rust/Cargo 1.96.0. It establishes ABI-2 typed
stable words, exact closed-plan Buf roots, generated active frames, and actual
safe-service collection; it does not establish source-level allocation or a
shared VM/native heap.

| Native-reference/frame command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-native -p lkjscript-sys -p lkjscript-jit -p lkjscript-vm` | passed; plan/image malformed boundaries, non-empty exact maps, generated collection with dead-root exclusion, caller/callee chains, structured epilogues, frame bounds, repeated W^X installation, and existing JIT/VM tests |
| `cargo clippy --locked -p lkjscript-native -p lkjscript-sys -p lkjscript-jit --all-targets --all-features -- -D warnings` | passed |
| separate `check-docs`, `check-tree`, and `check-sources` | passed; canonical language sources were unchanged |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict workspace Clippy, docs/tree/source closure, and all 175 workspace tests |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | source-level native allocation, shared VM/native collection, Docker/release smokes, performance, Miri, sanitizers, or non-Linux targets |

The ownership implementation tree based on main HEAD `c64b3ab` was checked on
Linux 7.0.0-27-generic x86-64 with Rust/Cargo 1.96.0. Canonical Brainfuck source
was unchanged.

| Ownership correction command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-compiler -p lkjscript-ir -p lkjscript-core -p lkjscript-vm -p lkjscript-app` | passed; source/HIR/SSA malformed boundaries plus evaluator/reference-VM equivalence and existing scalar JIT app gates |
| `cargo clippy --locked -p lkjscript-ir -p lkjscript-compiler -p lkjscript-jit -p lkjscript-app --all-targets --all-features -- -D warnings` | passed |
| separate `check-docs`, `check-tree`, and `check-sources` | passed; canonical language sources were not modified |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; formatting, strict workspace Clippy, docs/tree/source closure, and all 168 workspace tests |
| `cargo build --workspace --release --locked`; default hello, forced scalar JIT, Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed; Brainfuck source remained unchanged |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok` and all configured smokes |
| `cargo fmt --all -- --check`; `git diff --check` | passed |
| Not tested | performance, full Brainfuck Mandelbrot, Miri, sanitizers, or non-Linux targets |

The marker-trait implementation tree based on `5c6ba38` was checked on Linux
7.0.0-27-generic x86-64 with Rust/Cargo 1.96.0:

| Marker-trait command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-compiler -p lkjscript-ir -p lkjscript-app` | passed; declaration/coherence/bound solving, structural auto traits, malformed SSA witnesses, and evaluator/VM marker-call equivalence |
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --locked -p lkjscript-ir -p lkjscript-compiler -p lkjscript-jit -p lkjscript-app --all-targets --all-features -- -D warnings` | passed |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs/tree/source closure, rustfmt, strict workspace Clippy, and all 151 workspace tests |
| `cargo build --workspace --release --locked` plus Brainfuck, lkjedit, HTTP, bulk-byte, durable-file, SHA-256, and SQLite smokes | passed |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed with `result=ok`; rebuilt release runtime and reran the configured gates/smokes |
| Not tested | full Brainfuck Mandelbrot, performance, Miri, sanitizers, or non-Linux targets |

This evidence establishes only marker declarations, exact nominal impls,
generic marker bounds, bounded structural Copy/Send/Sync solving, and verified
erased witness identity. It does not establish trait methods, associated items,
ownership, package coherence/orphan rules, dynamic dispatch, specialization, or
native generic monomorphization.

The lossless bulk-byte and durable-file changes in this documentation's
containing commits were checked on Linux x86-64 with Rust/Cargo 1.96.0:

| Command or check | Result |
| --- | --- |
| `cargo test --locked -p lkjscript-core -p lkjscript-compiler -p lkjscript-sys -p lkjscript-vm` | passed; focused compiler/core/sys/VM coverage including exact binary socket transfer |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; workspace check, docs/tree/source closure, rustfmt, strict Clippy, and all workspace tests |
| `cargo build --workspace --release --locked`; bulk-byte, durable-file, and HTTP smokes | passed; exact `.lkjscript` file-buffer plus append/replay consumers and legacy HTTP behavior |
| `docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify` | passed; Docker source closure and all configured runtime smokes including bulk bytes and durable files |
| Not tested | performance and application-level HTTP/storage workloads |

Phase A implementation commit
`12836da90d886c9e741a5ac9f8148a17d00f0505` and the state-threaded editor
behavior follow-up `91d7e9bb734307269eb44b2d3a0882ba55d2f5b2`, based on `e4c1d0e`, were
checked on Linux x86-64 with Rust/Cargo 1.96.0. Evidence is command-specific; Docker, full Brainfuck
Mandelbrot, and performance are not implied.

| Command or check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --locked` | passed |
| focused `lkjscript-compiler` and app HIR/numeric tests | passed; 37 compiler and 10 app integration tests |
| `cargo run --locked -p lkjscript-xtask --quiet -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 82 workspace tests |
| `check-sources` | passed for all 94 `.lkjscript` sources; the nine compiled executable closures equal the corpus exactly |
| HIR/local mutation conformance | explicit Main/Function, missing/duplicate/imported main, declaration-only imports, rejected top-level effects/value defs, stable BindingId/local-slot shadowing, initializer scope, local-only set rejection and exact typing, same-function isolation, ProductId/field resolution, and StoreLocal execution passed |
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| Brainfuck smoke | direct and run-folded correctness/failure boundaries passed |
| lkjedit smoke | passed; existing-file insert/save/reopen, missing-file creation, CRLF redraw, and command paint |
| one-shot HTTP smoke | passed |
| validated-chunk boundaries | centralized decode/CFG/metadata validation and random raw-chunk no-panic tests passed after integration |
| structured execution boundaries | return/exit/trap/deadline and configured resource categories passed; returned heap values remain owned after VM teardown |
| native-backend decision spike | 8 randomized warmups plus 31 retained pairs; exact generated calls passed; owned execution median/MAD 48.406374/0.540016 ms versus Cranelift 0.134.2 119.422902/0.566505 ms; temporary artifacts removed; no production backend implemented |
| Phase A `check-docs` and `git diff --check` | passed |

Phase B fixed-point effect inference in this documentation's containing commit,
based on `061f7c51c74412fcb19cd43df8385ac692a26367`, was checked on Linux x86-64
with Rust/Cargo 1.96.0. Only effect inference and its HIR facts changed; typed
SSA, native code, runtime JIT, runtime smokes, Docker, and performance were not
tested or implemented.

| Phase B command or check | Result |
| --- | --- |
| `cargo test --locked -q -p lkjscript-compiler` | passed; 44 compiler tests |
| `cargo check --workspace --all-targets --locked` | passed |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 101 workspace tests |
| fixed-point effect conformance | passed; pure leaf, direct/transitive propagation, direct and mutual recursion, recursive effects, allocation, memory read/write, local mutation, host IO, process exit, trap, declaration-order independence, generic canonical direct calls, retained argument effects, and conservative indirect calls |

Phase C typed-SSA/reference-bytecode contract commit `787d7b1` and
implementation commits `41deaef`, `0c9903b`, `d9a6917`, `47c3b83`, and
`1b7b1ce`, based on
`ec2afbb1161eff437370d1e75c9522af9a261342`, were checked on Linux x86-64 with
Rust/Cargo 1.96.0. This evidence establishes typed SSA and the reference
cutover, not native execution, JIT tiering, OSR, Docker, or performance.

| Phase C command or check | Result |
| --- | --- |
| focused crate tests | passed; 6 `lkjscript-ir`, 44 compiler, 14 core, 31 VM, and 14 app tests |
| SSA differential conformance | passed; exact focused Unit/Bool/I64/F64/control/loops/calls/recursion/local mutation/products/Option/Result/buffers/traps/exits, explicit unsupported host operations, tail-call bytecode shape, and 64 deterministic bounded randomized typed scalar programs |
| malformed SSA and pass conformance | passed; direct malformed identity/use/dominance/edge/loop/effect cases, each isolated pass, repeated determinism, post-pass verification, combined normalization, and evaluator bounds |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| `check-docs`, `check-tree`, and `check-sources` | passed; all nine executable closures cover all 94 canonical sources through SSA and validated bytecode |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs, tree, exact source closure, rustfmt, strict Clippy, and 112 workspace tests |
| `cargo build --workspace --release --locked` | passed |
| canonical hello | passed; output `3628800` |
| Mandelbrot | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| Brainfuck smoke | passed after preserving return-adjacent tail calls and liveness-allocated typed bytecode locals; direct and run-folded correctness/failure checks passed; full Brainfuck Mandelbrot was not run |
| lkjedit and one-shot HTTP smoke | passed |

The native-foundation commit based on `ec2afbb` passed six focused native/sys
unit and integration tests, strict Clippy, the then-current 106-test canonical
gate, and generated-code invocation for multi-block control, a 100-iteration
loop, direct native calls, an allowlisted runtime call, exact I64 traps, F64
bits/comparisons, structured exit, W^X permissions, limits, and 32 repeated
install/invoke/drop cycles. It did not connect source or SSA to native code and
therefore was not a JIT test.

Earlier decision-grade and diagnostic performance records remain in
[Experiment Registry](vision/experiments.md); they were not rerun for Phase A,
Phase B, or Phase C. A gate that did not run did not pass. Docker, full
Brainfuck Mandelbrot, source-to-native execution, and performance were not
tested for Phase C.

The callable scalar baseline implementation chain through
`a9d0584ad0106817c4eac5de7dbc9191e7537105`, based on current-main
`c4c96094260072323f9399fe7f0f7b4a14d1eef6`, was checked in isolated worktree
`/tmp/pi-agent-a98a8be7-b37a-422-f33e779d` on Linux
`7.0.0-27-generic` x86-64 with Rust/Cargo 1.96.0. The evidence establishes the
exact allocation-free scalar subset, not full-language native execution, OSR,
or a performance result.

| Callable baseline command or check | Result |
| --- | --- |
| focused IR/compiler/native/sys/JIT/VM/app tests | passed; the final canonical workspace gate reports 125 tests, including 7 source-engine and 1 direct verified-SSA JIT tests |
| strict workspace Clippy, all targets/features | passed with `-D warnings` |
| `check-docs`, `check-tree`, `check-sources` | passed; ten roots exactly cover all 96 canonical sources |
| `cargo run --locked -p lkjscript-xtask -- quiet verify` | passed; docs/tree/source closure, rustfmt, strict Clippy, and all 125 tests |
| `cargo build --workspace --release --locked` | passed in the shared target tree |
| scalar workload, explicit `vm` / `baseline-jit` / threshold-2 `auto` | all exited 0 with empty stdout and exact test-oracle F64 bits |
| forced scalar diagnostics | one installed W^X object; compiled `scalar-step` and `main`; 100,001 native entries, 100,000 direct native calls, 300,002 PollV1 calls, zero VM fallbacks/failures |
| auto scalar diagnostics | 99,998 later-call native entries, 99,998 PollV1 calls, exactly two initial VM calls, zero compile failures; no OSR claim |
| explicit VM and threshold-2 auto hello | both output `3628800`; auto recorded 15 native leaf entries and one retry-suppressed recursive-group failure |
| direct Mandelbrot in VM | passed; 1,176 bytes, 24 lines, SHA-256 `222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907` |
| Brainfuck smoke only | passed direct/run-folded correctness and failure checks; full Brainfuck Mandelbrot was not run |
| lkjedit and one-shot HTTP smokes | passed |
| opt-in generated binary plus external `objdump` | passed; 1,926-byte source-derived object dumped, disassembled, then removed; normal stdout stayed empty |

Docker, full Brainfuck Mandelbrot, performance sampling, OSR, background work,
optimizing/speculative tiers, native references/allocation/host IO, and
non-Linux/non-x86-64 acceptance were not run or implemented in that callable
implementation chain.

The retained measurement/default commit
`025cbb2feadbb18fbae51e68e38b9c849798d068`, following instrumentation/default
commit `56535c589998eeefa045fca622720662a2f78662`, was measured from a clean
isolated worktree on Linux 7.0.0-27-generic x86-64, AMD Ryzen 9 9955HX with 20
logical CPUs available, 32 GiB RAM, Rust/Cargo 1.96.0, and Python 3.12.3. The
release binary was 1,448,584 bytes with SHA-256
`94dec3b623f07333ed57659c67d8461c8ac30e7c13684f147700b72cefd9a638`;
the 289-byte workload SHA-256 was
`aa8acecbad8add81f7a3a79b19a69e8f503d36c8af6e1f503b572bfadd14157e`.

| Retained scalar metric | Result |
| --- | --- |
| protocol/oracle | four warmups and 31 randomized samples per variant, seed `0x4c4b4a534d455452`, no removed samples; every process returned exact F64 bits `0x401af3ef5a48f5f0` with zero stdout and no unexpected stderr |
| process wall median / MAD / p95 / min / max | VM 354.533038 / 4.711766 / 362.572659 / 347.360647 / 369.390164 ms; forced 9.372036 / 0.467328 / 10.364211 / 8.711153 / 10.472645 ms; auto-64 214.482019 / 3.352331 / 226.691819 / 206.949992 / 228.798658 ms |
| generated execution | forced native median 7.647935 ms versus VM execution 352.918413 ms: **46.146x**, meeting the aspirational 5x target |
| compile/install/entry | native lowering+encoding 0.040096 ms, relocation/W^X install 0.036558 ms, 0.076654 ms combined; forced time to first native entry 0.080141 ms and first-call duration 7.647935 ms; measured whole-workload break-even one invocation |
| auto-64 | 1.653x process-wall speedup over VM; median time to first native entry 0.297720 ms; 64 expected initial VM entries, 99,936 native entries/PollV1 calls, zero compile failures; main remained VM and no OSR is claimed |
| forced counts/cache | 100,001 native entries, 100,000 direct calls, 300,002 PollV1 calls, zero fallback/failure; one object, 1,926 code bytes, 2,618 metadata bytes, 4,096 accounted allocation bytes |
| peak RSS median | VM 2,736 KiB; forced 2,724 KiB; auto 2,808 KiB, polled from `/proc` |
| threshold decision | auto process medians at thresholds 1/64/1,024 were 211.286082 / 214.482019 / 211.901028 ms with overlapping dispersion; 64 is retained as the middle conservative policy, keeping 63 cold calls in VM while avoiding the 1,024-entry trigger delay |
| pre-JIT VM diagnostic | compatible exact-oracle source: current VM 357.510855 ms versus `c4c9609` 364.419240 ms (0.981x); difference below twice larger MAD, so no regression/improvement claim; old/current binaries 1,129,440/1,448,584 bytes and median RSS 2,272/2,756 KiB |

Every sample and phase distribution is retained at
`meta/benchmarks/jit/results/callable-baseline-jit-linux-x86_64.json`,
`auto-threshold-1.json`, `auto-threshold-1024.json`, and
`pre-jit-c4-vm-comparison.json`. Temporary `c4c9609` worktree, copied binary,
and source copy were removed; the compatible source itself is retained under
`meta/benchmarks/jit/pre-jit-workload/`. Profiling/disassembly improvement was
not required because the 5x target passed. Docker, full Brainfuck Mandelbrot,
OSR, non-scalar native semantics, and non-Linux acceptance were not run.

Final-worktree inventory was recalculated rather than copied from older
records: 96 canonical `src/**/*.lkjscript` files (58,734 bytes, 8,067 physical
lines) are covered by ten executable roots; two additional compatible benchmark
sources live under `meta/benchmarks/jit/pre-jit-workload` and are not canonical
corpus members; the canonical workspace gate reports 126 tests; and `docs/`
contains 42 Markdown documents. The final release binary retains the
1,448,584-byte size and SHA-256 above. The four committed result JSON files are
293,337, 293,879, 293,535, and 29,965 bytes with the exact hashes recorded in
Experiment C4.

Final acceptance ran `cargo run --locked -q -p lkjscript-xtask -- quiet verify`
(126 tests), `cargo build --workspace --release --locked`, ordinary/default,
explicit VM, forced, and threshold-2 auto scalar runs, explicit-VM hello and
Mandelbrot, `python3 meta/benchmarks/brainfuck/benchmark.py --mode smoke
--no-build`, and the lkjedit/HTTP smoke scripts. All passed; scalar streams were
empty, hello was exactly `3628800`, and Mandelbrot remained 1,176 bytes/24 lines
with SHA-256
`222c57ba490929db28c8f122d76f3bdbf0282ffd70d7686734e98ae1a7d9c907`.
The exact final implementation-tree command
`docker compose -f meta/docker-compose.yml --profile verify run --build --rm verify`
passed with `result=ok`; the image reran the canonical gate and release hello,
Mandelbrot, lkjedit, and HTTP boundaries. Separate final commands for rustfmt,
strict workspace Clippy, docs/tree/source checks, locked release build, and
`git diff --check` also exited 0. Full Brainfuck Mandelbrot was not run. The
first aggregate smoke wrapper itself exited 1 only because its extra local assertion incorrectly
expected a newline after the canonical newline-free hello output; every wrapped
command had exited 0. The corrected complete wrapper was rerun and exited 0,
so no failed product command is hidden.

## Accepted Next Target

The first dependency-ordered slice is the complete Semantic Source foundation,
not automatic optimizing promotion. It must:

1. place the complete Current Edition 1 declaration/expression vocabulary
   behind one opaque validated source-tree authority;
2. preserve every exact corpus byte through deterministic parse/format/parse;
3. provide deterministic stable declaration keys and revision-scoped dense node
   IDs;
4. provide bounded atomic semantic transactions with stale revision and exact
   precondition rejection, no text substring replacement, and no partial file
   publication;
5. convert unmatched marker, duplicate declaration, unknown binding, arity,
   type mismatch, and stale edit into versioned structured diagnostics while
   retaining human rendering;
6. support useful expression holes and bounded exact context/candidate queries,
   while every executable/release path rejects unresolved holes; and
7. establish retained raw-text/entity-edit/hole-fill benchmark cases without
   claiming general AI superiority from the bootstrap sample.

The exact contract and gates are [Semantic Source And Agent
Protocol](decisions/semantic-source-and-agent-protocol.md). Phase 2 then adds
aggregate source/compiler bounds and named profiles before any tiny Current
source limit becomes a lint. Edition 2 and later package/capability/ownership
work follows those foundations.

The previously selected process-local synchronous automatic baseline-to-proof
promotion remains an **Accepted Implementation Selection** and valid later
experiment. Its threshold gate and default-disabled policy remain unchanged;
it is no longer the immediate repository priority. Broader proof passes, OSR,
background compilation, guards, deoptimization, AOT, caches, optional local
PGO, and non-Linux targets remain non-Current.

## Rejected

Mandatory uploaded telemetry, hidden forced-engine fallback, incomplete artifact
cache keys, separate backend semantic compilers, unbounded optimizer search,
and optimizer-dependent deterministic semantic charges are rejected.
Current-process bounded JIT counters remain local, ephemeral, and not telemetry.

## Deferred

Production AOT, content-addressed native caches, optional explicit local PGO,
automatic optimizing promotion, OSR, package installation/update/manifests/
locks/registry, supervisor/scheduler, adaptive or generational GC, background
JIT compilation, guarded runtime specialization/deoptimization, non-Linux native
backends, browser, general HTTP server/framework, and GUI runtime are later
cycles. Local PGO is optional and is considered only after common SSA/AOT and
complete artifact identity; no uploaded telemetry is accepted. These documents
are decisions or experiments, not capability claims.
