# Current status

Date: 2026-08-14

Reset base: `1f4da233367e0cd282c1e5e1c35b6f73a19880ad` on `main`

## Milestone

The active tree is a direct replacement product, not an adaptation of the previous implementation.
The old eleven-crate workspace, source programs, parser/importer, semantic workspace, HIR, SSA,
bytecode, VM, JIT, native, executable, host, package artifacts, old tests, Docker/meta verification,
CI workflow, documentation authority, and prompt archive are deleted. One Rust package now builds
`lkjscript` and `lkjscriptd`.

The dependency-closed source-free scalar vertical is implemented:

- daemon-created durable workspace and immutable revision 0;
- one typed batch constructs package, module, `main`, region, block, `40`, `2`, `add_i64`, and
  `return` with stable Node IDs;
- deterministic validation, staged allocation, dry-run, no-op rejection, semantic diff, rename,
  compatible constant replacement, blocked deletion, tombstones, and old snapshots;
- canonical deterministic `.lkjscript` artifacts and atomic retained revision publication;
- exclusive daemon ownership, private Unix IPC, typed errors, request correlation, and persisted
  one-record idempotency;
- compact workspace/node queries and structured completeness blockers;
- direct SPG lowering to private verified Core IR and typed interpretation;
- restart reopens the same IDs and executes revision 1 as `42`;
- one transaction renames the function and changes `2` to `3`, revision 2 executes as `43`, and
  revision 1 still executes as `42`;
- a Boolean wired to `add_i64` rejects with exact expected/actual types while revision, HEAD bytes,
  revision files, and future allocation remain unchanged;
- explicit holes remain queryable and reject execution.

## Evidence

`tests/semantic_vertical.rs` starts the real daemon binary, uses the production client protocol,
invokes the real client binary, restarts the daemon twice, tests competing-daemon rejection, and
covers the 42/43 retained-snapshot scenario. Unit tests cover schema rejection, identity rollback,
dry-run, stale revisions, wrong workspaces, tombstones, deletion, deterministic artifact bytes,
corruption, duplicate IDs, invalid roots, history resurrection, policy limits, HEAD integrity,
staging recovery, bounded file reads, protocol unknown/truncated/oversized input, disconnected
clients, Core IR verification, interpreter traps, and injected durable publication failures. The
current focused boundary contains 29 passing tests plus one ignored manual performance baseline. A 10,000-operation subtree test exercises
iterative validation and deletion without native-stack recursion.

Required local verification commands are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

## Exact limitations

This milestone does not complete every requirement in the campaign catalogue. It has no property
sequence generator, fuzz target, or generated machine-readable schema output yet. Queries have no direct-use index, body pagination,
continuations, legal-constructor results, or context packs. The operation set excludes calls,
subtraction, multiplication, comparison, and control regions. Entry invocation accepts no
parameters. There are no host effects, capabilities, ownership-bearing values, aggregates,
generics, package dependencies, incrementality, compile cache, native tier, runtime cells,
concurrent service, debugger, or cross-platform contract.

The idempotency retention policy keeps one keyed outcome per workspace. Snapshot history is retained
without pruning. Full graph clones and full artifact rewrites are intentional measured baselines.
No performance leadership claim is made.
