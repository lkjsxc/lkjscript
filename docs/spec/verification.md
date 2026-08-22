# Verification and evidence

Status: normative.

## Correctness oracles

Canonical decoding and deep reconstruction ignore disposable indexes. Full semantic validation
reconstructs names, scopes, types, effects, capabilities, components, targets, tests, and retained
relations from root/module objects. Incremental/query acceleration must agree with that route.

Meaning graph 4 revisions authenticate semantic-summary contract 2 facts with a semantic
certificate. Content-addressed summaries and the revision-bound reverse-dependency index remain
disposable: missing bytes rebuild from canonical modules, while a rebuilt certificate mismatch is
canonical corruption. Focused differential tests compare the four current local preparation
classes—eligible pure-body replacement, independent empty-module creation, module rename, and
declaration rename—with complete canonicalization and validation. This is not evidence for general incremental
validation; every other transaction class keeps complete preparation.

Execution has two implementation-disjoint tiers: prepared bytecode is production; the semantic
reference interpreter walks validated operation structures. Every graph-owned package test runs
actual and expected expressions through both tiers, requires tier equality, then requires expected
value equality. A missing, skipped, unavailable, exhausted, cancelled, or unrun check is not pass.

## Profiles

`tools/check` is the executable verification owner. `focused` runs formatting, locked library and
public integration tests, and diff checks. `changed` is selection convenience and widens
uncertainty. `product` builds release and checks maintained packages, deep doctors, and artifact /
built-in reproduction. `service` runs release-build and isolated PostgreSQL service/worker
acceptance. `full` adds Clippy, all workspace targets, checker self-tests, every product gate, and
service acceptance, and never treats a prior receipt as a fresh pass.

Independent ready nodes may run in parallel only when they do not mutate an executable another
node can launch. The DAG serializes debug executable producers before public/workspace tests and
requires formatting before compiler-producing gates. Checker self-test owns this dependency
contract so an `ETXTBSY` race cannot be normalized as flakiness.

Successful checks emit one aggregate JSON line and a receipt locator. Each gate retains bounded
stdout/stderr separately under `.artifacts/check`. Failure returns bounded excerpts and exact log
locations. Passing test names and child logs are not printed by default.

## Receipt rules

A transaction receipt canonically binds repository, base/result, transaction digest, semantic diff
digest, affected owners, validation profile and facts, optional idempotency key, and bounded
nonsemantic intent. It is part of accepted history.

Build, test, service, backup, restore, review projection, doctor, and verification receipts are
evidence or derived outputs, not accepted program meaning. They must bind exact semantic revision,
tool/contract identity, exact relevant inputs, status, counts, output/log locators, and honest
limitations. Volatile elapsed time, platform facts, and operator labels never enter semantic
revision identity.

Pass reuse is permitted only when every semantic and operational input is proven identical and the
active profile permits discovery. Final publication verification remains fresh. Current `full`
does not reuse a prior pass.

## Required adversarial coverage

Tests cover unknown contracts, foreign identity domains, duplicate/trailing/excess input, checked
allocation, corrupt object bytes, missing/corrupt derived indexes, stale base, no-change,
precondition failure, idempotent replay, interrupted publication boundaries, two-parent history,
draft separation/rebase, deterministic backup/restore, read-only retention-preview reachability and
plan stability, predecessor rejection, and public output bounds. Retention-preview tests assert
`destructive_ready: false`; they are not garbage-collection or compaction evidence.

Scale evidence must name exact generated topology, revision, toolchain, platform, command, cold or
warm cache state, wall/CPU/memory where available, storage growth, output bytes, semantic work
counts, and limitations. Limit increases may not substitute for correcting a superlinear
algorithm.

## Claims

Performance, security, portability, provider-token, and monetary claims require retained evidence.
Output bytes are not token counts. When provider input/cached/output/request/retry/cost telemetry is
unavailable, evidence says unavailable and makes no savings claim. Fresh-checkout evidence is
platform-specific and does not imply portability.
