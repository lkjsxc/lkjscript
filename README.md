# lkjscript

`lkjscript` is an experimental, source-free programming system designed primarily for programs built
and maintained by AI coding agents. Its canonical program is a closed, strongly typed Semantic
Program Graph owned by one local daemon—not a source file, syntax tree, or arbitrary property graph.

The project exists to test a different programming interface: an agent discovers an exact machine
schema, proposes bounded typed changes, receives deterministic validation and repair facts, and runs
immutable retained revisions without generating or round-tripping source text. Humans remain the
users at the level of intent, review, governance, and operation; they are not expected to hand-author
the canonical graph.

## What is unusual

```text
human intent
    -> AI coding agent
    -> typed versioned requests
    -> lkjscriptd (sole graph writer)
    -> immutable .lkjscript revisions
    -> verified Core IR -> explicit-frame interpreter
```

Names and JSON are presentation or transport. Stable graph identities survive rename and
identity-preserving hole refinement. Calls, structured conditionals, and counted loops lower directly
from an immutable graph snapshot to one private verified Core IR. The interpreter uses explicit call
frames plus bounded fuel, frame count, and aggregate live value slots rather than user-depth native
recursion.

A human typically asks an AI coding agent to create or change a program, reviews the compact receipt
and semantic diff, and runs the selected revision. The agent can query bounded repair context instead
of reconstructing a workspace from source files or requesting a whole-graph dump.

The following is **explanatory pseudocode, not lkjscript source syntax and not canonical data**:

```text
range_sum(n):
  carried = 0
  for index in 0 .. n:
    carried = carried + index
  return carried

normalize_and_sum(n):
  if n < 0: return 0
  return range_sum(n)
```

The retained [structured pure program example](examples/structured-pure/) creates this meaning through
typed structured transactions, discovers a nested typed hole, rejects an invalid repair, refines it
without changing identity, executes `5050`, `0`, and `55`, and verifies retained revisions after a
daemon restart.

## How agents interact

`lkjscriptd` is a private local daemon and the only live writer of durable workspace state. The
production `lkjscript` CLI accepts one strict version-3 JSON envelope, sends the corresponding closed
binary request over local Unix IPC, and emits one typed JSON response. JSON is transport only; it is
never persisted as a second program representation.

From the repository root, agents should begin with runtime schema discovery:

```sh
cargo run --quiet --locked --bin lkjscript -- schema --pretty
```

An installed or otherwise absolute `lkjscript` binary path can be used from elsewhere.

The generated description includes operation and transaction variants, structured draft fields,
Run arguments and policy, query/error vocabularies, stable tags, ID formats, and active boundary
limits. Agents can then create structured functions, query exact revision-bound context, refine holes,
query paginated semantic diffs, and run entries with ordered `unit`/`bool`/`i64` arguments.

## What is a `.lkjscript` file?

A `.lkjscript` file is a canonical, checksummed immutable Semantic Program Graph snapshot for one
workspace revision. It records program meaning, stable identities, ownership, ordered child slots,
typed operations, and explicit holes. It is not source code, a JSON document, bytecode, or a mutable
compiler cache. Private dense Core IR is derived again from a selected complete revision and is not
serialized into the semantic artifact.

## Current implementation

The current Linux x86-64 bootstrap implements:

- durable workspaces with immutable revisions, stable IDs, tombstones, strict artifact format 2, and
  compact `LKJHEAD3` publication;
- direct structured authoring for functions, parameters, calls, `if`, `for_i64`, constants, checked
  `add_i64`, `lt_i64`, typed holes, yields, and returns;
- atomic commit and validate-only transactions, bounded receipts, idempotent committed retry, and
  identity-preserving scalar hole refinement;
- revision-bound paginated queries, semantic diff, legal constructors, visible values, incoming uses,
  and bounded nested repair context;
- deterministic multi-function Core IR lowering and verification;
- ordered invocation arguments, calls, finite or bounded recursive execution, lazy branches, loops,
  checked overflow traps, and exact fuel/frame/live-value-slot exhaustion;
- strict generic JSON CLI projection over private version-3 local IPC, persistence, and restart.

It does **not** currently provide a source language or parser, public network service, sandbox,
native backend or JIT, optimizer tiers, package ecosystem, effects or host capabilities, aggregates,
nominal products or sums, pattern matching, generics, ownership-bearing values, debugger, daemon
request concurrency, or a production-ready platform. The daemon relies on local filesystem/socket
permissions; executed pure programs have no ambient host authority. The supported bootstrap platform
is Linux x86-64 with a current stable Rust toolchain.

## Try the real product path

From the repository root, run the retained end-to-end example:

```sh
./examples/structured-pure/run.sh
```

From another directory, invoke `run.sh` by its absolute repository path.

It builds production release binaries, uses a private temporary state directory, drives typed JSON
through the generic CLI, prints typed results, shuts down and restarts the daemon, and cleans only its
own state. It requires a POSIX shell and Python 3 standard library in addition to Rust.

## Project documentation

- [Language semantics](docs/spec/language.md)
- [Semantic graph, identity, transactions, and artifacts](docs/spec/semantic-graph.md)
- [Daemon and machine protocol](docs/spec/protocol.md)
- [Architecture and trust boundaries](docs/architecture.md)
- [Implemented status and limitations](docs/status.md)
- [Performance and interaction-cost evidence](docs/performance.md)
- [Evidence-gated roadmap](docs/roadmap.md)

## Development verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
```

The larger deterministic malformed-boundary smoke is an ignored release test and is explicitly not
coverage-guided fuzzing:

```sh
LKJSCRIPT_MUTATION_SEED=1 LKJSCRIPT_MUTATION_CASES=10000 \
  cargo test --release --lib campaign_tests::boundary_mutation_smoke --locked -- \
  --ignored --nocapture --test-threads=1
```

## License

Licensed under the [Apache License 2.0](LICENSE).
