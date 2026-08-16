# lkjscript

`lkjscript` is an experimental programming system built primarily for coding agents.

Instead of treating source files as the program's source of truth, it stores each program as a
typed, versioned model managed by a local background service (`lkjscriptd`). An agent discovers the
operations the service accepts, submits a bounded typed change, receives deterministic validation
facts, and runs an immutable saved revision.

Textual source, diagrams, and other views may exist in the future. They would remain views or import
formats: program meaning has one authoritative stored form, and normal editing never depends on
rendering and reparsing text.

The formal name of this model is the **Semantic Program Graph** (SPG). “Graph” describes semantic
entities and relations such as containment, ordered children, value uses, and direct references. It
does not require a pointer graph or graph database as the physical storage layout.

## Why build this?

Coding agents usually edit text designed for humans, reconstruct compiler context, and infer whether
a patch preserves identity and type rules. `lkjscript` tests a different product boundary: the
service owns stable identities and accepted program state, while agents work through exact typed
operations and bounded queries.

Humans remain first-class users. They state intent, choose goals, review changes and explanations,
govern permissions, operate applications, and own product decisions. Coding agents discover the
machine contract, construct and revise programs, inspect focused context, submit proposals, and
receive deterministic acceptance or rejection. Removing hand-authored source as the authority does
not remove human agency.

## The current product path

```text
human intent and governance
    -> coding agent
    -> bounded typed operations and queries
    -> local service (`lkjscriptd`)
    -> validated immutable program revisions
    -> verified Core IR -> explicit-frame interpreter
```

Stable Node IDs do not depend on names, source positions, hashes, compiler indexes, or addresses.
Renaming preserves identity. A typed placeholder can be filled by a contract-defined operation while
preserving its identity, owner, body position, output, and existing uses. Rejected and validate-only
proposals publish nothing and consume no persistent IDs.

The following is **explanatory pseudocode, not lkjscript source syntax**:

```text
record Resources { cpu: i64, memory: i64, trusted: bool }
variant Decision { accept(i64), reject(RejectReason) }

decide(job, limits):
    if job exceeds limits: reject(the_reason)
    otherwise: accept(a_deterministic_score)
```

The retained [job-admission policy](examples/job-policy/) creates this broader application through
the public service path. It saves an incomplete revision, rejects an invalid repair, fills a typed
placeholder without identity churn, runs accepted and rejected outcomes, renames a field, restarts,
and checks old and current revisions.

The focused [named-data example](examples/named-data/) demonstrates immutable records, variants with
a fixed alternative set, complete lazy handling, named runtime input/output, and placeholder repair.

The retained [release-channel replay](examples/release-channel/) preserves the controlled campaign
task through the same public path. A separate isolated coding-agent trial authored that task without
opening implementation sources; it is evidence about this interface, not a production-readiness or
model-quality benchmark.

The [release-manifest classifier](examples/release-manifest/) is the first managed-value consumer.
It classifies an exact binary manifest using immutable bytes, checked indexing and slicing, content
equality, and a bounded payload scan. Byte values behave like ordinary immutable values; the runtime
may share their storage, and callers never see an address or allocation identity.

## How coding agents interact

`lkjscriptd` is the only live writer of durable workspace state. For RPC commands, the generic
`lkjscript` CLI accepts one strict version-7 JSON envelope, sends the same closed typed JSON request
in a bounded length frame over private local Unix IPC, and writes one typed JSON response. The separate `schema`
command derives the machine contract locally from the same executable definitions. JSON is
transport, not a second program representation.

For repeated requests, `lkjscript --state DIRECTORY session` accepts one compact version-7 envelope
per bounded line and flushes one compact response per line. It reuses the CLI process while retaining
the daemon's existing one-request-per-connection publication boundary and the same request vocabulary.

Begin with the compact machine-contract manifest:

```sh
cargo run --quiet --locked --bin lkjscript -- schema
```

Request exact contract roots with their transitive dependency closure, ask for the full description
explicitly, or reuse a known machine-contract fingerprint:

```sh
cargo run --quiet --locked --bin lkjscript -- schema \
  --root create_workspace --root apply_transaction --root query_repair_context --root run
cargo run --quiet --locked --bin lkjscript -- schema --full --pretty
cargo run --quiet --locked --bin lkjscript -- schema --known-digest DIGEST
```

Endpoint roots include the exact JSON-envelope request, success, typed-error, and boundary-error layers
plus applicable IDs and limits. The protocol specification separately owns the local frame grammar. The local command and service response derive them from the same
executable contract. A matching digest returns the compact `unchanged` result. Exact request, response, strictness, and limit contracts are
owned by the [protocol specification](docs/spec/protocol.md).

## What is a `.lkjscript` file?

A `.lkjscript` file is an immutable saved revision of the typed program model. It records program
meaning, stable identities, containment, ordered child slots, typed operations, direct references,
and typed placeholders. It is not source text, JSON, bytecode, or a mutable compiler cache.

The current artifact has deterministic checked bytes. Private dense Core IR is derived again from a
selected complete revision and is never stored as program authority.

## Current implementation

The current Linux x86-64 implementation provides:

- durable workspaces, immutable revisions, stable IDs, deletion history, atomic publication, restart,
  and strict corruption rejection;
- named immutable record types and variant types with a fixed set of alternatives, stable field and
  variant identity, acyclic by-value layout, field projection, construction, and handling of every
  variant;
- structured functions, parameters, identity-targeted calls, conditions, counted loops, constants,
  checked integer addition and comparison, typed placeholders, yields, and returns;
- immutable `bytes` values with canonical public encoding, checked length/index/slice/equality
  operations, byte literals, and composition inside records and variants;
- atomic commit and validate-only transactions, bounded transaction-local symbolic labels, anonymous
  one-use inline value expressions, selected returned bindings, compact receipts,
  identity-preserving placeholder repair, paginated semantic diffs, and bounded repair context;
- direct deterministic lowering from one immutable revision to one private Core IR, independent IR
  verification, and an explicit-frame interpreter;
- exact public `unit`, `bool`, `i64`, `bytes`, record, and variant values identified by semantic Node IDs;
- strict generic JSON projection over private synchronous local IPC;
- a bounded invocation-scoped byte arena whose validated opaque handles occupy fixed runtime cells,
  with deterministic cleanup and separately checked cells, visible bytes, retained backing, views,
  objects, and result materialization;
- package-wide `unsafe_code = "forbid"` for this Rust package, checked untrusted boundaries, and
  explicit resource policies.

These are current verified implementation choices, not universal architecture mandates: one Rust
package, synchronous requests, maps and vectors, full snapshot cloning, full scans, full artifact
rewrites, flat runtime cells, and interpretation. Future storage, indexing, concurrency, memory
strategies beyond the current invocation arena, frontends, or acceleration require a real consumer and evidence while preserving one
program authority and one semantic execution route.

## Current limitations

There is no source frontend, public network service, sandbox, package ecosystem, effect system or
host operation, permission-value system, resource-owning value, general managed heap, debugger, native
backend, optimizer tier, daemon request concurrency, or cross-platform support. Programs currently
operate only on pure primitives and acyclic immutable named values; managed bytes cannot escape one
`Run`. The local access boundary relies
on operating-system directory and socket permissions.

The package forbids local unsafe Rust, but this is not a formal proof or a claim that every dependency
contains no unsafe implementation. Memory safety still trusts the Rust toolchain, standard library,
operating system, and resolved dependencies. Resource exhaustion is handled by explicit operational
limits and is distinct from memory unsafety. See [architecture and trust
boundaries](docs/architecture.md) for the claim boundary.

`lkjscript` is bootstrap research software, not a production-ready platform. Performance and agent
interaction claims are limited to reproduced observations in [performance
evidence](docs/performance.md).

## Try the real service path

From the repository root:

```sh
./examples/job-policy/run.sh
./examples/named-data/run.sh
./examples/release-channel/run.sh
./examples/release-manifest/run.sh
```

All four scripts build production release binaries, create private temporary state, communicate only
through the production CLI and service, perform typed shutdown and restart, and remove only state
they created. They require a current stable Rust toolchain, a POSIX shell, and Python 3.

## Documentation

- [Language semantics](docs/spec/language.md)
- [Program model, identity, transactions, history, and artifacts](docs/spec/semantic-graph.md)
- [Local service and machine protocol](docs/spec/protocol.md)
- [Components, trusted computing base, and trust boundaries](docs/architecture.md)
- [Implemented status and exact limitations](docs/status.md)
- [Performance and interaction-cost evidence](docs/performance.md)
- [Evidence-gated roadmap](docs/roadmap.md)

## Development verification

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
```

The larger deterministic malformed-boundary smoke is an ignored release test. It is deterministic
mutation testing, not coverage-guided fuzzing:

```sh
LKJSCRIPT_MUTATION_SEED=1 LKJSCRIPT_MUTATION_CASES=10000 \
  cargo test --release --lib campaign_tests::boundary_mutation_smoke --locked -- \
  --ignored --nocapture --test-threads=1
```

## License

Licensed under the [Apache License 2.0](LICENSE).
