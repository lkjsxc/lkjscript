# Verification
## Purpose
Define Current evidence gates and predeclare accepted repository-intelligence
gates without claiming unimplemented checks.

## Status
<!-- LKJ-F public-fact-foundation current FRKXqrrhVO3Z0TXrTcxVFmUz5S3sChntrDlE3ykQzFM -->

Current formatting, Clippy, workspace tests, source closure/tree, documentation,
registered public-fact status, exclusion, and projection consistency, retained-result,
exact unsafe-boundary registry, `structure`, repository graph/context,
agent-state, Semantic Source with typed holes, legal actions, and transactions,
the implemented canonical-language enum construction, exhaustive match, Never/control, and explicit
numeric conversion through evaluator, VM, forced baseline, and forced proof,
its one-shot protocol, bounded local stdio sessions,
resource profile core reservation tests, and compiler resource checks are described
in the capsules. Cross-engine and JIT acceptance for implemented canonical-language
slices, the completed atomic migration and canonical corpus cutover, and ordinary
rejection of removed marker forms are Current. Nonzero query
caching, whole-pipeline pre-allocation, and logical metering beyond enum
construction remain **Accepted Targets**, not Current commands.

## Current Documentation Gate

The existing command is:

```sh
cargo run --locked -p lkjscript-xtask -- check-docs
```

It checks entry documents, local links, the strict sharded [Public Fact
Authority](status-authority.md), statuses, interfaces, exclusions, contained
paths, content-derived digests, projection markers, cycles, bounds, focused
coherence rules, platform revision, and removed generation names. Deterministic
inventory and expected markers go under `target/lkjscript/documentation/`.
Example execution, evidence freshness, architecture derivation, and arbitrary
prose equivalence are not Current.

`structure check` is the hard repository topology gate. It checks every tracked
authored text file for at most 200 physical lines, 32 KiB, and ordinary lines
at most 120 Unicode scalars; every authored directory for at most 16 immediate
tracked entries; path depth; local links; and stale old paths.

## Current Unsafe Boundary Gate

The implemented command is:

```sh
cargo run --locked -p lkjscript-xtask -- check-unsafe
```

`LKJ-UNSAFE-BOUNDARY` scans authored Rust code for exact `unsafe` tokens while
ignoring comments and string/character literals. It enforces both directions:
every matching file occurs once in `meta/unsafe/registry.json`, and every
registered path exists and matches. The registry has at most 16 stable boundary
entries, each with a reviewed safe caller contract and at most 16 sorted unique
files. The Current registry includes the host local-control peer-identity mechanism
and inherited sys boundaries. The command runs in `quiet verify`.

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

The Semantic Source contract gate covers all 102 tracked files with the current
canonical base representation, closed node/value/type/built-in/declaration/trivia/
expression/correlation records, enum/match/pattern nodes and expressions,
typed holes including match expected/scope facts, schema/source roundtrip,
`snapshot`, `read-entity`, `query-node`, `diagnostics`, atomic
`rename`, and atomic `replace-expression`. It includes strict
schema/version/field/variant rejection; exact pinned serde boundary;
duplicate/trailing/malformed Unicode JSON; aggregate budgets; deterministic
ordering; stale identities/preconditions; rename collisions and complete
references; expression type/effect/ownership constraints; bounded response
encoding before publish; publication failure; byte-identical rollback; and
prepared-journal crash recovery before the next read; descriptor-anchored
ancestor-swap rejection; and preservation of an externally created leaf at the
no-replace install boundary. Focused hole tests cover strict stale and
mismatched contract rejection;
malformed and duplicate identity; expected type and scope ambiguity;
deterministic ranking; snippets and blockers; ownership/effect validation;
roundtrip; legal actions; exact/+1 pre-allocation; stale preconditions; all four
hole transactions; release rejection; and local sessions. Session tests cover
8-byte framing, clean EOF,
partial/oversized/cumulative boundaries, strict envelopes, pinned profile/root,
stale revisions, external edits, refresh, publication revision advance,
lifetime/request/revision limits, deterministic responses, shutdown, and CLI
stdout isolation.

Focused native immutable-bytes tests cover exact static/dynamic owner/loan ABI
categories, immutable verified image-data identities, literal/read/length,
checked slice copy, clone independence, zero-copy freeze and dynamic thaw,
one-copy static thaw, direct calls, exact returned payload transfer, allocation
failure, stale/wrong-layout/forged identities, traps, and cleanup. Differentials
require evaluator/VM/native equality, nonzero selected-tier entries,
optimizing-only proof entries, zero fallback/transitions, and zero final owners,
loans, or release backlog.

Focused native resource-island tests cover exact capability/resource ABI
categories, wrong capability and resource kinds, whole-group rejection of
unsupported operations and reachable mixed ownership types, forced
baseline/proof `standard-input` entries, VM outcome equality, zero
fallback/transitions, no invocation-region runtime-value dispatch, and exact
borrowed reservation/install/reuse/removal with zero obligations.

Focused scalar tests cover the safe closed 16-byte `Value`, complete-range i64,
exact f64 payloads including NaNs and signed zero, inline constants/locals/calls/
returns, VM/native and detached-return transitions, SQLite f64 binding, zero
scalar aggregate allocation through evaluator/VM/baseline/proof fixtures,
synchronous generated entry, and zero forced fallback. The unconditional
`LKJ-RUNTIME-NO-TRACING-COLLECTOR` source gate rejects any collector mechanism.

Focused enum tests cover exact Schema declaration nodes and roundtrip,
stable nominal/member/layout identities, source order, generic substitutions,
invariance, duplicate/empty/malformed rejection, nested ownership rejection,
recursion exact/+1 bounds, exact constructor field order/arity/type rejection,
SSA metadata/provenance/layout/substitution rejection, inactive projection
rejection, stable physical tags, structural active payloads, validated bytecode
descriptors, evaluator/VM/native/proof differential values and logical
exhaustion, exact generated entries and structural runtime calls, malformed
native metadata/tag/projection rejection, and zero forced fallback. Numeric conversion tests add stable `NumericError`
identity/cases, exact bit/exponent boundaries, F64-bit outcomes, malformed SSA,
four-engine differentials, direct rounded I64-to-F64 native conversion with
exact result bits and no heap call, exact generated structural-runtime calls
with zero heap dispatch for the three fallible conversions, and zero fallback. Match tests add
nested usefulness/witness, stale plan, active projection, resource, source-order,
single-evaluation, and four-engine zero-fallback coverage. Automatic and
host-native enum transitions remain outside Current acceptance.

Core resource profile tests cover all five profiles, exact/lowered/+1/overflow
boundaries, category order, positive monotonic ceilings, parent-child
oversubscription, consume/return/Drop reservation behavior, authority depth,
missing authority, deterministic prefix equality and nested order, exact
journal capacity plus one, no-mutation rejection, and identity. Compiler tests
retain corpus-root coverage and add exact/+1 deterministic-prefix boundaries
for HIR, match planning, SSA construction, bytecode input, diagnostics, and
outer-ledger accumulation. Semantic tests add deterministic typed prefixes,
exact/+1 no-publication transaction response/staging boundaries, transaction
staging boundaries, direct typed session execution, and all 128 bounded request
segments. Parser-wide pre-allocation, exact bytecode-output sizing, and a
whole-platform cross-authority ledger remain Accepted.

## Current Canonical Source Contract Gate

The canonical language gate is Current for all 102 tracked sources
(89 under `src/`). Retained
[execution acceptance evidence](../history/semantics/edition/execution-and-acceptance.md)
records exact source and semantic identity and byte reports, check/diff/publish idempotence, stale/mixed/conflict
rejection, atomic all-file rollback and crash recovery, compiler-resolved
conversion, rejection of removed marker forms, evaluator/VM/forced-JIT
value/outcome/charge differentials, actual generated calls without fallback,
malformed-metadata rejection, exact roots, and resource profile boundaries. Runtime
smokes and Docker remain separately recorded gates.

## Evidence Rule

A command that did not run did not pass. Historical success is evidence only for
the named commit/environment/command. Documentation-only commits explicitly
record code/build/runtime gates as not tested.

## Strict Capsule Manifest

- [Current local, runtime, Docker, and performance gates](verification/current-local-gates.md)
- [Accepted automatic proof-promotion gate](verification/accepted-automatic-promotion-gate.md)
