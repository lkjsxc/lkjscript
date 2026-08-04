# Historical Snapshot Before The Canonical Cutover
## Status
**Historical snapshot.** Present-tense and Current wording below records only
the pre-cutover baseline and grants no active fact, alias, decoder, or fallback.
## Recorded Baseline Implementation
- Repository: `https://github.com/lkjsxc/lkjscript`
- Canonical source: `.lkjscript`; other extensions are rejected without shims
- Corpus: all canonical language files under `src` have executable roots covering the exact corpus closure
- Physical format: one column-one marker/atom per line; exact the canonical source contract first
  removed leading source-contract marker; retained leading trivia; homogeneous loaded
  closures; marker inside the token but outside the declaration limit. All 125
  canonical sources are the canonical source contract; the removed legacy source contract
  is validation/migration input only
- Semantic Source contract: public identity `lkjscript.semantic-source` version 2; version 1 input is historical and
  rejected. One opaque immutable `ValidatedSourceTree` is parser/load authority. The
  canonical contract preserves every record from the removed prior source contract
  node/value/type/built-in/declaration/trivia/transaction-expression/diagnostic/correlation record, typed-hole facts,
  and generic enum declaration nodes. Subtrees roundtrip with exact spans/origins/revisions, stable keys, dense nodes,
  and canonical formatting. Unknown schema kinds, fields, operations, versions, duplicates, and trailing input fail.
  The bounded one-shot endpoint provides snapshot/entity/node/diagnostic/hole/legal action queries plus atomic rename,
  expression replacement, and all four hole transactions. resource profile categories reserve candidate, action,
  transaction, impact, and staged publication work before allocation. Agent Foundation
  baseline is historical. The local
  `semantic serve --stdio` session uses exact 8-byte framing, pins one profile/root/revision, rejects stale/external
  changes, refreshes explicitly, intersects resource profile session ceilings, and publishes
  through the same typed engine
  without an inner codec roundtrip. One outer ledger spans each one-shot request and the full session lifetime, with
  bounded journal segments between requests. Nonzero query caching, a whole-platform ledger, and unavailable exact
  downstream correlations remain non-Current. Schema also represents the canonical source
  contract without another version: identities
  are edition-separated, snapshots expose marker/number nodes, source/tree identity facts, stable generic enum
  declaration/variant/field/type identities, Never type nodes, and closed loop/return/break/continue/trap/exit
  expressions. The canonical source contract hole context and legal actions expose
  checker-valid available control forms, function/nearest-loop requirements, and exact Never admissibility
- Migration: exact compiler-owned check/diff/publish pins closure revision,
  source/tree/declaration/node identities and reports exact old/new identities
  and bytes. resource profile reservation precedes staging. Migration inserts only the
  marker and resolved required `f64-from-i64-rounded`. Locked whole-closure
  no-replace publication provides rollback, recovery, and conflict rejection
- Source limits: depth 8, form children 16, tokens 384, top-level forms 8,
  product fields 15, and 16 combined immediate files/directories per source
  directory. Foundation implementation maxima additionally reject a source file
  over 16 MiB, a loaded closure over 256 MiB exact input bytes, more than
  65,536 source units, or more than 65,536 entries in a complete source-tree
  traversal; opened regular-file reads are bounded by the smaller remaining
  per-file/aggregate allowance plus one sentinel byte and reject metadata/read
  size changes before parsing; iterative dependency-first import and source-tree
  traversal avoid native stack growth, a directory rejects on entry 17 without
  collecting the remainder, and all immediate entries count
- Source-tree scope: the width rule applies to language source directories,
  not Rust, docs, metadata, `.git`, or generated Cargo output
- Imports: contained `std/`, `lib/`, `examples/`, and `./` paths with installed
  fallback through `LKJSCRIPT_ROOT`; absolute, parent, wrong-extension, cycle,
  non-regular source, non-UTF-8 host logical path, and containment failures
  fail. Ordinary compile/run require the canonical source contract without inference. On Current
  Linux, containment and identity use the canonical path resolved from the stable opened descriptor through
  `/proc/self/fd` before reading; changed/deleted/unresolvable descriptor paths
  fail closed. Non-Linux host-path loading is not accepted and has only a
  fail-closed compilation fallback. Public in-memory compile/validate APIs
  require the same canonical relative non-dot `.lkjscript` logical paths as the
  Semantic Source validator, without compatibility aliases
- Compiler boundary: one analysis pass collects immutable headers and produces
  owned, resolved typed HIR with explicit Main and Functions, BindingIds,
  local-slot references, MutableLocal/SetLocal, ProductIds, stable EnumIds/VariantIds/
  VariantFieldIds, invariant enum substitutions, dense TraitIds/ImplIds, marker
  witnesses, source origins, exact type facts, and fixed-point function effects;
  HIR lowers once into verified typed SSA, deterministic baseline normalization, and then reference bytecode
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
- Resource profiles: `lkjscript.resource-profile` uses exact category and profile digests and
  preserves the adopted category names/order with 29 closed
  categories across five positive monotonic bounded profiles. Core provides closed authorities, fixed 16-entry paths,
  lower-only grants, move-only reservations, and a fixed nonallocating 256-record journal; Drop commits unused units
  unless returned. Public compiler and Semantic Source `_with_ledger` APIs borrow an outer-owned ledger. Semantic
  requests reuse it through protocol bytes, tree/query/hole/action work, transaction/migration staging, and response
  preflight; local sessions retain it across direct typed requests. Validated source shape reserves enum/match HIR work,
  immutable HIR reserves exact charged input shape before SSA, and immutable normalized SSA reserves exact charged
  input shape before bytecode. Failures preserve the typed deterministic prefix. Parser/source allocation and exact
  bytecode-output categories remain gaps; compiler, semantic, proof, artifact, and runtime are not yet joined by one
  application request ledger
- Host implementation: nine Rust workspace crates. Exact locked `serde` and
  `serde_json` dependencies are confined to strict JSON protocol/tooling
  boundaries; unsafe Rust is confined to `lkjscript-sys`
- Quality gate: the complete Rust workspace is rustfmt-clean and passes strict
  Clippy for all targets/features; docs status/links, explicit `PLACEHOLDER`
  labels, and exact source-closure coverage are machine-checked
- AI-authorability bootstrap: one replayable raw-text function-rename task and a
  strict retained-result validator are Current. `gpt-5.6-sol` completed the
  exact two-file change in 43,421 ms with 10 tool calls, one compiler run, zero
  failed mutations/repairs, and no unrelated paths; `gpt-5.4-mini` reached a
  correct branch but failed the benchmark because it transiently violated
  requested worktree isolation after two failed mutations and two repair loops;
  the available Qwen 3.5 9B request timed out before any tool call. This is
  narrow raw-text evidence, not a general model/interface claim; comparative
  semantic-versus-hole benchmark variants are not Current
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
  `ValidatedChunk`, and VM, disassembly, and runtime tiering accept
  only validated input; compiler `ExecutableProgram` retains verified
  normalized SSA, deterministic function/prototype/main and SSA/bytecode link
  metadata, and validated bytecode through an explicit accessor
- Outcomes: VM execution distinguishes returned, exited, trapped, deadline,
  resource-limit, and host-failure outcomes; the core does not terminate the
  process, returned heap values own their reachable storage, and cleanup occurs before CLI exit-status translation
- Runtime budgets: explicit configuration bounds fuel, stack values, frames,
  estimated live heap, aggregate allocations, handles, output, and cooperative
  wall time; hard-deadline mode rejects host wrappers that cannot guarantee cancellation
- Semantics: executable roots have exactly one no-parameter typed main;
  imports contain declarations only; top-level `do` and runtime value defs are
  removed; `var` introduces one exactly typed mutable local and local-only
  `set` returns Unit; Unit, typed empty-list, and generic `Option.None` have
  distinct singleton tags, while `Option.Some` uses ordinary enum tracing; `nil`, `Nil`, `nil?`, and
  `null?` are removed; `arg` returns `Option Str`; universal `eq`/`ne` are
  removed in favor of exact value, object-identity, bounded structural-list,
  and F64-bit equality families; nominal products have ordered named fields,
  exact construction, access, and immutable replacement. the canonical source contract enum
  declarations/type facts and exact `variant-value` construction are Current
  through HIR, verified SSA, evaluator, bytecode/VM, boxed active-payload GC,
  and forced Linux x86-64 baseline/proof JIT. Exhaustive match is Current through bounded plans and all four engines.
  the canonical source contract `Never` is a join-only HIR type with no SSA/runtime/storage/ABI
  value; typed loop/while block parameters, early return, nearest break and
  continue, dynamic Str trap values, and structured exit are Current through
  evaluator, validated bytecode/reference VM, and forced baseline/proof JIT
  with zero fallback. The four explicit the canonical source contract numeric conversions, stable
  `NumericError`, bit-exact evaluator/VM behavior, and generated baseline/proof
  runtime sites and generic prelude typed-error native transitions are Current; mixed numerics are rejected
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
  Affine cross-block values require explicit typed block arguments. `Owned Buf` is affine, shared references are
  Copy, exclusive references are affine, and all three are
  worker-local/non-Send/non-Sync. Legacy `Buf` semantics are unchanged.
  Borrow is accepted only as an exact direct reference argument or direct let
  initializer; temporary loans cover the full call/runtime-operation.
  Ownership/reference generic instantiation and direct/nested product or
  collection storage are rejected. References cannot escape, Borrow results
  cannot cross SSA blocks, loop cycles reject Move/Borrow and cannot carry
  changed owner/loan state, `RefMut` user-call forwarding is rejected, and cleanup is not deterministic user `Drop`
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
  associated items, generic/blanket impls, specialization, dynamic dispatch, and package orphan rules are not Current
- Numerics: canonical I64/F64 only; complete I64 uses signed 61-bit immediates
  plus boxed wide values, F64 remains distinct, arithmetic/comparison is
  checked or IEEE as declared, and narrower host domains reject truncation
- CLI: `run`, real bytecode `disasm`, help, and version; the unlabeled REPL stub was removed
- Workloads: hello, native lkjscript Mandelbrot, Brainfuck interpreted by
  lkjscript, lkjedit, one-shot HTTP, and Leibniz comparison; Brainfuck,
  terminal, and editor state is passed explicitly in immutable nominal products
  and evolved through local vars
- Resource handles: integers are rejected, stdin uses a reserved borrowed token,
  owned file/socket tokens are monotonic, and closed tokens are never reused
- Terminal ABI: arbitrary ioctl is absent; fixed `sys-tty-get`/`sys-tty-set`
  operations validate the exact 60-byte Linux state before FFI and return Results
- System Results: open, path existence, close/read/write, `isatty`, time,
  socket, poll, terminal, and terminal-guard failures return ordinary `Result.Err`
  values carrying closed `SystemError`; UTF-8 failures carry closed `Utf8Error`; wrappers unwrap explicitly
