# Current status

Date: 2026-08-15

Campaign base: `99d7ca5bbdac6bcf90fdd64721c13df1342ef67a` on `main`

## Implemented product path

The repository contains one Rust package and one source-free product route:

```text
strict generic JSON CLI (optional) -> private protocol-v4 Unix IPC -> synchronous daemon
-> durable workspace -> typed staged transaction -> immutable SPG snapshot
-> scan-based revision query or direct Core IR lowering -> verifier -> interpreter
```

The agent-native semantic repair, structured pure-program, and nominal immutable-data campaigns are
implemented:

- products, product fields, closed sums, and sum variants are persistent canonical nodes with
  immutable shape, stable member identity, exact owner/ordinal validation, atomic declaration
  transactions, forward `TypeDraft` resolution, typed incoming references, deletion blocking, and
  adjacent-history continuity;
- deterministic iterative by-value cycle rejection and checked derived layouts cover field offsets,
  variant discriminants, payload placement, and runtime-cell footprints without serializing layout;
- canonical product construction, field projection, variant construction, and exhaustive structured match use exact declaration/member identities; a transaction-local non-persisted catalogue supports later-authored declarations/members, identity-keyed fields and arms normalize to declaration order before implied allocation, and real arm regions/payload arguments/yields validate payload, scope, and result contracts independently;
- the paginated nominal-type query exposes declaration/member names, exact identity/type facts, and optional derived layout facts without fabricated values for unrepresentable layouts; revision/declaration-bound cursors provide continuation;
- legal constructors and repair context bound every retained requirement vector to 64 items, report exact operand/member totals and completeness, keep fitting products one-query repairable, and provide oversized product nominal-query continuation;
- nominal holes refine without identity churn only to valid regionless product, variant, or projection
  operations of the same result type; match remains ineligible.

The complete product path also retains:

- one code-owned static descriptor defines the closed scalar, comparison, identity-targeted call, structured `if`/`for_i64`, nominal operation, hole, yield, and return contracts used by graph validation, queries, codecs, and schema output; match regions use one narrow closed dynamic variant rule consumed by validation;
- `RefineHole` performs the sole one-way identity-preserving constructor transition, retaining the
  hole Node ID, owner, body position, and uses while rejecting another hole, terminator, mismatched
  result, invalid operand, and already-complete target;
- transactions use explicit commit/validate-only mode and a bounded response projection; default
  receipts contain exact count/digest and at most 64 selected handle bindings rather than full
  allocation/diff data;
- validate-only performs commit-equivalent semantic, artifact, response, and durability preflight
  without publication or identity consumption; idempotency is commit-only;
- semantic changes are deterministic and distinguish hole refinement; complete changes are queried
  through exact revision-bound pagination;
- compact checksummed `LKJHEAD3` retains head identity and at most one compact keyed replay receipt;
  artifact format 3 with semantic schema `lkjscript-spg003` and protocol/JSON version 4 are the only
  readers; format 2 and version 3 bytes reject directly;
- structured function/body/expression drafts directly replace public low-level body scaffolding;
  iterative depth-first expansion creates canonical implicit regions, blocks, block arguments, and
  terminators, allocates all identities before edits, supports forward/mutual calls, and returns
  selected explicit bindings only;
- query batches bind all items to one retained revision, preserve query IDs/order, allow independent
  item errors, and bound page/batch/context budgets;
- implemented query families are workspace/node views, blockers, owner chains, body slices,
  incoming uses and definition references, dependencies, visible values, exact legal constructors,
  arbitrary retained-revision semantic diffs, and hole/operand repair contexts;
- all query facts use deterministic full scans and bounded composition; legal constructors and owner
  chains retain only the requested page while counting exact totals, and repair context retains only
  bounded category slices; there is no reverse index, query cache, or query framework;
- strict version-4 JSON envelopes cover every public request/response through the generic CLI. The
  payload-bearing `DescribeSchema` directly replaces the old tag-only request and supports a compact
  manifest by default, seven closed repeatable sections, explicit full output, and strict optional
  lowercase digest matching. The semantic section exposes the validator-owned named-kind, nonempty,
  sibling-uniqueness, and 1 MiB artifact-name contract; errors/limits exposes the exact 64 MiB
  artifact and 1 MiB artifact-name policies. Full output describes every request, response,
  referenced DTO, exact required/optional field, and tagging convention as one closed type-expression graph; an automated
  closure check rejects undefined or multiply defined named types. One BLAKE3 domain-separated digest
  covers the complete executable descriptor through the canonical protocol schema-facts encoder;
  selected sections and full output partition that authority exactly, matching digests return only
  `unchanged`, and there is no cache or persisted schema response. Unknown, empty, duplicate, excessive, malformed, uppercase, truncated,
  and trailing forms reject; request correlation and one-value stdout are tested, exit 0 and 2 have
  focused process evidence, and the implemented exit 3/4 mappings do not yet have focused process
  tests; schema usage and boundary errors are compact bounded envelopes;
- the principal nominal generic-CLI/daemon integration and `examples/nominal-match` create the
  Reading/Input application in one structured transaction, request only selected bindings, obtain
  exact nominal repair context, prove an invalid identity-keyed product repair publishes nothing,
  refine the hole without identity churn, query `OperationRefined`, execute scalar and public nominal
  input/output oracles plus selected/unselected overflow, restart, and prove both incomplete and
  repaired revisions preserve identities and behavior;
- direct SPG lowering iteratively discovers the exact complete direct-call and transitive nominal-type
  closures. One private type table fixes primitives first and reachable nominal declarations in
  persistent Node-ID order, retains semantic origins, recomputes layouts, and omits unreachable types;
- aggregate instructions and exhaustive payload-aware switches lower through the same typed CFG and
  are independently verified; the interpreter uses flat frame cell arenas plus initialized facts;
- Run accepts exact revision-bound primitive/product/sum values, normalizes exact owned product
  fields to declaration order, validates exact owned variants and payload contracts, and emits
  semantic IDs only. Depth 24 is proven through complete strict JSON Run envelopes for worst-case nested products and sums; 4,096 nested
  items and 64 KiB encoded bytes aggregate across all arguments and also bound mandatory-result
  preflight. The 65,536-cell policy covers peak live arenas plus exact transfer/flatten scratch and
  prospective callee arenas before allocation or copy. Aggregate instructions use direct arena
  ranges, switch reads only the discriminant, and block entry performs no uncharged clearing. Fuel is
  charged before work as one base per instruction/transfer plus `max(1,cells)` for each logical copy,
  with full-sum canonicalization plus active-payload logical copies charged for variants; exhaustion and arithmetic traps are exact and
  leave the daemon usable;
- inline diagnostics retain at most 64 deterministically sorted related identities while exact
  completeness blockers remain available through paginated queries.

## Evidence

The final fresh-target locked boundary reports 166 active passing tests and eight explicitly
ignored measurement/smoke tests. Library/unit coverage
includes schema/tag rejection, graph/history
continuity, stable refinement, allocator rollback, deterministic detailed diffs, selected bindings,
receipt/HEAD bounds, validate-only parity, idempotent replay/conflict, publication failure injection,
query pagination/cursor binding/budgets, maximum batch shape, exact uses/constructors/repair context,
protocol framing, strict JSON round trips and rejection, Core IR verification, interpreter traps,
generated transaction sequences, durable rejection/restart invariants, canonical durable path
spelling, byte-derived artifact count bounds, exact one-frame connection decoding, and deterministic
boundary mutation. A 10,000-node subtree test exercises iterative validation/deletion without
native-stack recursion.

Integration tests use the real `lkjscriptd`, production binary IPC, and real generic `lkjscript rpc`
CLI. The retained scalar integration validates durable 42/43 history and restart. Structured and
nominal integrations cover nested repair, competing-daemon rejection, retained revisions, nominal
Run, and corrupt-revision startup rejection; `examples/nominal-match` covers the complete public
schema/authoring/repair/diff/run/restart path. Ignored reproducible harnesses print exact current
schema and nominal-workflow JSON/binary bytes, CLI round trips, artifact/HEAD sizes, layout,
compile/execute and repeated runtime latency, plus older separately labelled baselines. The seed-1,
10,000-case release mutation smoke is required final evidence; it is deterministic mutation testing,
not coverage-guided fuzzing. Exact commands and observations are retained in
`docs/performance.md`.

Required verification is:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

## Exact limitations

The current baseline intentionally uses full snapshot clones for mutation, full semantic
recomputation and full diff materialization during preparation/query, full canonical artifact
rewrite per revision, full retained history, and scan-based incoming-use/dependency/context queries.
It has no reverse-reference index, query cache, incremental validator/compiler/persistence, journal,
database, async runtime, or daemon request concurrency. These mechanisms require representative
measurements and exact invalidation/durability evidence before introduction.

The deterministic mutation corpus is bounded and not coverage-guided fuzzing; no coverage metric is
claimed. Machine schema is generated at runtime rather than committed as a file. JSON is only a
strict transport projection and not source or persisted authority. The daemon remains Linux x86-64
bootstrap software using private local IPC; no sandboxing or public network service is claimed.

Canonical semantic state, authoring, compiler, and interpreter now support unit/bool/i64 values,
constants, checked addition, integer comparison, identity-targeted zero/one/multi-argument calls,
structured conditionals and counted loops, entry arguments, block arguments, recursion, typed holes,
yield, return, nominal products and closed sums, aggregate construction/projection, exhaustive lazy
matching, and public nominal Run input/output. Only the selected direct-call closure must be complete;
unrelated incomplete functions and unreachable nominal declarations remain non-blocking. There are
no generics, package dependencies, effects, capabilities, host I/O, ownership-bearing values,
managed heap, native execution, debugger, optimizer tiers, concurrent service, cross-platform
contract, or source parser/projection.

The current agent evidence measures JSON/binary bytes, CLI invocations, round trips, rejected edits,
and elapsed time. No model was invoked, so model tokens and controlled model success rates remain
unmeasured and no token-savings claim is made. No performance leadership claim is made.
