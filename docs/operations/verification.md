# Verification

## Purpose

Define Current evidence gates and predeclare accepted repository-intelligence
gates without claiming unimplemented checks.

## Status

Current formatting, Clippy, workspace tests, source closure/tree, documentation,
placeholder, retained-result, `structure`, repository graph/context, and
agent-state checks are described in the capsules. Complete Semantic Source
protocol gates remain **Accepted Implementation Contracts**, not Current commands.

## Current Documentation Gate

The existing command is:

```sh
cargo run --locked -p lkjscript-xtask -- check-docs
```

It checks a hardcoded required-document set and repository-local links. Any
document move must update that required set in the integration change; moved
paths must not be retained as aliases merely to satisfy the old checker.

Until `structure check` is implemented, each migration independently checks
every tracked authored text file for at most 200 physical lines, 32 KiB, and
ordinary lines at most 120 Unicode scalars; every authored directory for at most
16 immediate tracked entries; path depth; local links; and stale old paths. The
exact script/command and result belong in commit evidence.

## Current Structure Gate

The implemented commands are:

```text
cargo run --locked -p lkjscript-xtask -- structure audit --json
cargo run --locked -p lkjscript-xtask -- structure check
cargo run --locked -p lkjscript-xtask -- structure explain <rule-or-path>
```

They implement the rule IDs, provenance classes, bounds, strict manifest,
generated-output location, and canonical audit schema in [Bounded Repository
Topology](../decisions/platform/bounded-repository-topology.md). Focused fixtures
cover every exact-limit and limit-plus-one boundary. Audit output is generated
under `target/` and is byte-deterministic for the same inputs.

The structure gate must cover at least:

- exact provenance, classification, capsule-manifest, and local-link rules;
- file line/byte/width, directory width/depth, item, fan-out, and cycle rules;
- generated location and temporary ratchet records; and
- symlink containment, malformed manifests, duplicate identities, unsupported
  analyses, and stale authority paths.

## Current Graph, Context, And Work-State Gates

Repository graph tests cover closed graph data, stable identities, deterministic
builds, stale input, aggregate charges, cycle-safe traversal, and explicit
truncation. Context tests cover section order, inclusion, omission, profile
limits, revision, and total charge.

Agent-state tests cover malformed, duplicate, unknown, and trailing JSON; stale
and concurrent preconditions; exact and limit-plus-one bounds; checked revision
overflow; deterministic quarantine; atomic failure before rename; evidence/hash
validation; compaction retention and idempotence; and explicit context
truncation. Generated snapshots stay under `target/lkjscript/agent-state/`.

## Accepted Semantic Operation Gate

The first protocol gate covers `snapshot`, `read_entity`, `query_node`,
`diagnostics`, atomic `rename`, and atomic `replace_expression`. It includes
strict schema/version/field/variant rejection; exact pinned serde boundary;
duplicate/trailing/malformed Unicode JSON; aggregate budgets; deterministic
ordering; stale identities/preconditions; rename collisions and complete
references; expression type/effect/ownership constraints; publication failure;
and byte-identical rollback.

No protocol, graph, context, task-state, or resource-profile surface becomes
Current until its focused fixtures, Current canonical semantics, and required
runtime gates pass on the containing commit.

## Evidence Rule

A command that did not run did not pass. Historical success is evidence only for
the named commit/environment/command. Documentation-only commits explicitly
record code/build/runtime gates as not tested.

## Strict Capsule Manifest

- [Current local, runtime, Docker, and performance gates](verification/current-local-gates.md)
- [Accepted automatic proof-promotion gate](verification/accepted-automatic-promotion-gate.md)
