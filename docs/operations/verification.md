# Verification

## Purpose

Define Current evidence gates and predeclare accepted repository-intelligence
gates without claiming unimplemented checks.

## Status

Current formatting, Clippy, workspace tests, source closure/tree, documentation,
placeholder, retained-result, `structure`, repository graph/context,
agent-state, one-shot Agent Foundation V1, and compiler resource-profile checks
are described in the capsules. Complete future Schema V1 and shared-ledger
gates remain **Accepted Targets**, not Current commands.

## Current Documentation Gate

The existing command is:

```sh
cargo run --locked -p lkjscript-xtask -- check-docs
```

It checks a hardcoded required-document set and repository-local links. Any
document move must update that required set in the integration change; moved
paths must not be retained as aliases merely to satisfy the old checker.

`structure check` is the hard repository topology gate. It checks every tracked
authored text file for at most 200 physical lines, 32 KiB, and ordinary lines
at most 120 Unicode scalars; every authored directory for at most 16 immediate
tracked entries; path depth; local links; and stale old paths.

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
- generated locations and the absence of a permanent exemption ratchet; and
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
validation; ancestor-symlink rejection; bounded quarantine; crashed-lock
recovery; compaction retention/idempotence; and explicit context truncation.
Generated snapshots stay under `target/lkjscript/agent-state/`.

## Current Semantic Operation And Resource-Profile Gates

The first protocol gate covers `snapshot`, `read_entity`, `query_node`,
`diagnostics`, atomic `rename`, and atomic `replace_expression`. It includes
strict schema/version/field/variant rejection; exact pinned serde boundary;
duplicate/trailing/malformed Unicode JSON; aggregate budgets; deterministic
ordering; stale identities/preconditions; rename collisions and complete
references; expression type/effect/ownership constraints; bounded response
encoding before publish; publication failure; byte-identical rollback; and
prepared-journal crash recovery before the next read; descriptor-anchored
ancestor-swap rejection; and preservation of an externally created leaf at the
no-replace install boundary.

Compiler resource-profile tests cover all five profiles, exact/lowered/+1/
overflow boundaries, corpus roots, deterministic diagnostics, identity, and
post-phase source/HIR/SSA publication guards. The broader pre-allocation and
cross-authority ledger gates remain Accepted.

## Evidence Rule

A command that did not run did not pass. Historical success is evidence only for
the named commit/environment/command. Documentation-only commits explicitly
record code/build/runtime gates as not tested.

## Strict Capsule Manifest

- [Current local, runtime, Docker, and performance gates](verification/current-local-gates.md)
- [Accepted automatic proof-promotion gate](verification/accepted-automatic-promotion-gate.md)
