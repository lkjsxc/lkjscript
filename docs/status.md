# Current status

Date: 2026-08-14

Campaign base: `a503c0b1269ed3e149f83bb0f8ad8d4f75550cbc` on `main`

## Implemented product path

The repository contains one Rust package and one source-free product route:

```text
strict generic JSON CLI (optional) -> private protocol-v2 Unix IPC -> synchronous daemon
-> durable workspace -> typed staged transaction -> immutable SPG snapshot
-> scan-based revision query or direct Core IR lowering -> verifier -> interpreter
```

The agent-native semantic repair campaign is implemented:

- one code-owned static descriptor defines the closed `const_i64`, `const_bool`, `add_i64`, typed
  `hole`, and `return` contracts used by validation, querying, codecs, lowering, and schema output;
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
- compact checksummed `LKJHEAD2` retains head identity and at most one compact keyed replay receipt;
  old HEAD1 and protocol v1 bytes reject directly;
- query batches bind all items to one retained revision, preserve query IDs/order, allow independent
  item errors, and bound page/batch/context budgets;
- implemented query families are workspace/node views, blockers, owner chains, body slices,
  incoming uses and definition references, dependencies, visible values, exact legal constructors,
  arbitrary retained-revision semantic diffs, and hole/operand repair contexts;
- all query facts use deterministic full scans and bounded composition; there is no reverse index,
  query cache, or query framework;
- strict version-2 JSON envelopes cover every public request/response through the generic CLI,
  including `DescribeSchema`; canonical IDs, unknown/trailing rejection, input/nesting bounds,
  streaming output bound, request correlation, one-value stdout, and exit 0/2/3/4 are tested;
- the real CLI/daemon integration discovers a hole, inspects context, rejects an invalid edit,
  refines without identity churn, paginates exact diff, executes `42`, restarts, preserves history
  and IDs, then performs an operand repair without a workspace dump;
- direct SPG lowering to one private verified Core IR and interpreter remains the only executable
  route.

## Evidence

The locked full test boundary currently reports 76 active passing tests and four ignored manual
measurement/smoke tests. Library/unit coverage includes schema/tag rejection, graph/history
continuity, stable refinement, allocator rollback, deterministic detailed diffs, selected bindings,
receipt/HEAD bounds, validate-only parity, idempotent replay/conflict, publication failure injection,
query pagination/cursor binding/budgets, maximum batch shape, exact uses/constructors/repair context,
protocol framing, strict JSON round trips and rejection, Core IR verification, interpreter traps,
generated transaction sequences, durable rejection/restart invariants, canonical durable path
spelling, byte-derived artifact count bounds, exact one-frame connection decoding, and deterministic
boundary mutation. A 10,000-node subtree test exercises iterative validation/deletion without
native-stack recursion.

Integration tests use the real `lkjscriptd`, production binary IPC, and real generic `lkjscript rpc`
CLI. The retained scalar integration validates durable 42/43 history, restart, competing-daemon
rejection, and injected corruption behavior. The manual real-CLI cost and scan-query tests assert
typed result 42 while printing exact bytes/latencies. The seed-1, 10,000-case release mutation smoke
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

The language still has only unit/bool/i64 scalars, constants, checked integer addition, typed holes,
and return. Entry calls accept no parameters. There are no calls, branches, loops, recursion,
aggregates, sums, matching, generics, package dependencies, effects, capabilities, host I/O,
ownership-bearing values, native execution, runtime cells, debugger, optimizer tiers, concurrent
service, cross-platform contract, or source parser/projection.

The current agent evidence measures JSON/binary bytes, CLI invocations, round trips, rejected edits,
and elapsed time. No model was invoked, so model tokens and controlled model success rates remain
unmeasured and no token-savings claim is made. No performance leadership claim is made.
