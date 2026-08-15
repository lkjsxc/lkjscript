# Current status

Date: 2026-08-15

Campaign base: `dc541eb3ebb7a54006e8057d0f76b0596cf012e4` on `main`

## Implemented product path

The repository contains one Rust package and one source-free product route:

```text
strict generic JSON CLI (optional) -> private protocol-v3 Unix IPC -> synchronous daemon
-> durable workspace -> typed staged transaction -> immutable SPG snapshot
-> scan-based revision query or direct Core IR lowering -> verifier -> interpreter
```

The agent-native semantic repair and structured pure-program campaigns are implemented:

- one code-owned static descriptor defines the closed scalar, comparison, identity-targeted call,
  structured `if`/`for_i64`, hole, yield, and return contracts used by graph validation, queries,
  codecs, schema output, and lowering;
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
  artifact format 2 and protocol/JSON version 3 are the only readers; older bytes reject directly;
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
- strict version-3 JSON envelopes cover every public request/response through the generic CLI,
  including `DescribeSchema`; the generated descriptor exhaustively identifies unit/newtype/record
  payloads, exact required/optional field names, stable type expressions, envelopes, tags, and
  limits; canonical IDs, unknown/trailing rejection, input/nesting bounds, streaming output bound,
  request correlation, one-value stdout, and exit 0/2/3/4 are tested;
- the principal real generic-CLI/daemon integration creates `range_sum`, `normalize_and_sum`, and
  `main` in one structured transaction, requests four selected bindings, derives loop arguments from
  bounded nested repair context, proves an invalid bool repair publishes nothing, refines the hole
  without identity churn, queries `OperationRefined`, executes `5050`/`0`/`55`, restarts, and proves
  both incomplete and repaired retained revisions preserve identities and behavior;
- direct SPG lowering iteratively discovers only the complete direct-call closure, deterministically
  lowers multi-function structured control to one private typed CFG, verifies it independently, and
  executes arguments, calls, recursion, branches, and loops through one explicit-frame interpreter;
- run requests require bounded ordered arguments plus positive bounded fuel and frame policy; an
  aggregate 65,536 live frame-value-slot policy rejects before allocation and releases on return;
  exhaustion categories and arithmetic traps are exact and leave the daemon usable;
- inline diagnostics retain at most 64 deterministically sorted related identities while exact
  completeness blockers remain available through paginated queries.

## Evidence

The locked full test boundary currently reports 121 active passing tests and six ignored manual
measurement/smoke tests. The retained fresh-target measurement predates the final focused review
fixes and remains labelled separately in `docs/performance.md`. Library/unit coverage includes schema/tag rejection, graph/history
continuity, stable refinement, allocator rollback, deterministic detailed diffs, selected bindings,
receipt/HEAD bounds, validate-only parity, idempotent replay/conflict, publication failure injection,
query pagination/cursor binding/budgets, maximum batch shape, exact uses/constructors/repair context,
protocol framing, strict JSON round trips and rejection, Core IR verification, interpreter traps,
generated transaction sequences, durable rejection/restart invariants, canonical durable path
spelling, byte-derived artifact count bounds, exact one-frame connection decoding, and deterministic
boundary mutation. A 10,000-node subtree test exercises iterative validation/deletion without
native-stack recursion.

Integration tests use the real `lkjscriptd`, production binary IPC, and real generic `lkjscript rpc`
CLI. The retained scalar integration validates durable 42/43 history and restart. The structured
integration covers the representative nested repair workflow, competing-daemon rejection, retained
revisions, and corrupt structured-revision startup rejection; the retained example covers the same
public authoring/repair/run/restart path. Ignored reproducible harnesses
print exact structured JSON/binary bytes, CLI round trips, artifact sizes, repeated runtime latency,
old scalar repair evidence, and scan-query cost. The seed-1, 10,000-case release mutation smoke
passed; it is deterministic mutation testing, not coverage-guided fuzzing. Exact commands and
observations are retained in `docs/performance.md`.

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
yield, and return. Only the selected direct-call closure must be complete; unrelated incomplete
functions remain non-blocking. There are no aggregates, sums, matching, generics, package
dependencies, effects, capabilities, host I/O,
ownership-bearing values, native execution, runtime cells, debugger, optimizer tiers, concurrent
service, cross-platform contract, or source parser/projection.

The current agent evidence measures JSON/binary bytes, CLI invocations, round trips, rejected edits,
and elapsed time. No model was invoked, so model tokens and controlled model success rates remain
unmeasured and no token-savings claim is made. No performance leadership claim is made.
