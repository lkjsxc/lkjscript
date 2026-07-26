# Verification

## Purpose

Define Current evidence gates and predeclare accepted repository-intelligence
gates without claiming unimplemented checks.

## Status

Current formatting, Clippy, workspace tests, source closure/tree, documentation,
registered capability-status consistency, placeholder, retained-result,
`structure`, repository graph/context, agent-state, complete Semantic Source
Schema V2 with its exact V1 base, typed holes/legal actions and transactions,
Edition 2 enum construction, exhaustive match, Never/control, and explicit
numeric conversion through evaluator, VM, forced baseline, and forced proof,
its one-shot protocol, bounded local stdio sessions,
Profile V2 core reservation tests, and compiler resource checks are described
in the capsules. Edition 2 cross-engine/JIT acceptance, exact atomic migration, the canonical
corpus cutover, and ordinary Edition 1 rejection are Current. Nonzero query
caching, whole-pipeline pre-allocation, and logical metering beyond enum
construction remain **Accepted Targets**, not Current commands.

## Current Documentation Gate

The existing command is:

```sh
cargo run --locked -p lkjscript-xtask -- check-docs
```

It checks a hardcoded required-document set, repository-local links, and the
registered cross-authority capability claims defined by [Capability Status
Authority](status-authority.md). Any document move must update those authorities
in the integration change; moved paths must not be retained as aliases merely
to satisfy the old checker.

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
validation; closed bounded semantic session/entity/transaction/diagnostic/hole
references; ancestor-symlink rejection; bounded quarantine; crashed-lock
recovery; compaction retention/idempotence; and explicit context truncation.
Generated snapshots stay under `target/lkjscript/agent-state/`.

## Current Semantic Operation And Resource-Profile Gates

The Semantic Source V2 gate covers all 125 tracked files with the unchanged V1
base representation, closed node/value/type/built-in/declaration/trivia/
expression/correlation records, enum/match/pattern nodes and expressions,
typed holes including match expected/scope facts, schema/source roundtrip,
`snapshot`, `read_entity`, `query_node`, `diagnostics`, atomic
`rename`, and atomic `replace_expression`. It includes strict
schema/version/field/variant rejection; exact pinned serde boundary;
duplicate/trailing/malformed Unicode JSON; aggregate budgets; deterministic
ordering; stale identities/preconditions; rename collisions and complete
references; expression type/effect/ownership constraints; bounded response
encoding before publish; publication failure; byte-identical rollback; and
prepared-journal crash recovery before the next read; descriptor-anchored
ancestor-swap rejection; and preservation of an externally created leaf at the
no-replace install boundary. Focused hole tests cover strict V1 rejection;
malformed and duplicate identity; expected type and scope ambiguity;
deterministic ranking; snippets and blockers; ownership/effect validation;
roundtrip; legal actions; exact/+1 pre-allocation; stale preconditions; all four
hole transactions; release rejection; and local sessions. Session tests cover
8-byte framing, clean EOF,
partial/oversized/cumulative boundaries, strict envelopes, pinned profile/root,
stale revisions, external edits, refresh, publication revision advance,
lifetime/request/revision limits, deterministic responses, shutdown, and CLI
stdout isolation.

Focused enum tests cover exact Schema V2 declaration nodes and roundtrip,
stable nominal/member/layout identities, source order, generic substitutions,
invariance, duplicate/empty/malformed rejection, nested ownership rejection,
recursion exact/+1 bounds, exact constructor field order/arity/type rejection,
SSA metadata/provenance/layout/substitution rejection, inactive projection
rejection, stable physical tags, boxed active-payload tracing, validated
bytecode descriptors, evaluator/VM/native/proof differential values and logical
exhaustion, exact generated entries/runtime calls/roots, malformed native
metadata/tag/projection rejection, and zero forced fallback. Numeric conversion tests add stable `NumericError`
identity/cases, exact bit/exponent boundaries, F64-bit outcomes, malformed SSA,
four-engine differentials, exact generated heap-runtime calls, and zero
fallback. Match tests add
nested usefulness/witness, stale plan, active projection, resource, source-order,
single-evaluation, and four-engine zero-fallback coverage. Automatic/host-native
enum transitions remain part of the Accepted Edition 2 gate.

Core Profile V2 tests cover all five profiles, exact/lowered/+1/overflow
boundaries, category order, positive monotonic ceilings, parent-child
oversubscription, consume/return/Drop reservation behavior, authority depth,
missing authority, deterministic prefix equality and nested order, exact
journal capacity plus one, no-mutation rejection, and identity. Compiler tests
retain corpus-root and legacy post-phase source/HIR/SSA publication coverage.
Whole-pipeline pre-allocation and shared cross-authority ledger gates remain
Accepted.

## Current Edition 2 Gate

The [Edition 2 acceptance contract](../decisions/semantics/edition-2/execution-and-acceptance.md)
is Current for all 125 tracked sources (121 under `src/`), exact old/new
identity and byte reports, check/diff/publish idempotence, stale/mixed/conflict
rejection, atomic all-file rollback and crash recovery, compiler-resolved
conversion, ordinary markerless rejection, evaluator/VM/forced-JIT
value/outcome/charge differentials, actual generated calls without fallback,
malformed-metadata rejection, exact roots, and Profile V2 boundaries. Runtime
smokes and Docker remain separately recorded gates.

## Evidence Rule

A command that did not run did not pass. Historical success is evidence only for
the named commit/environment/command. Documentation-only commits explicitly
record code/build/runtime gates as not tested.

## Strict Capsule Manifest

- [Current local, runtime, Docker, and performance gates](verification/current-local-gates.md)
- [Accepted automatic proof-promotion gate](verification/accepted-automatic-promotion-gate.md)
