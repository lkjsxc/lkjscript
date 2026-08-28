# Verification and evidence

Status: normative.

## Independent correctness mechanisms

Graph 5 full reconstruction and validation is the complete semantic oracle. Sparse repository
reads, witness-maintained changes, package interfaces, compiler selection, and query projection
must agree with it. Physical pack/catalog layout, persistent-map partitioning, derived cache state,
and request spelling are normalized away only when they are not semantic.

Production execution uses normalized bytecode and dense indexes. The canonical reference
interpreter independently reads accepted semantic owners and evaluates their typed structures.
Every pure command and graph-owned test used for acceptance requires production/reference equality
before comparing the expected value. Skipped, unavailable, cancelled, exhausted, or unrun work is
not a pass. Live effects are not duplicated for differential evidence.

Migration evidence for maintained consumers compares a sorted generation-neutral projection of
predecessor and Graph 5 meaning, including identity continuity, declarations/members, type and
expression ownership, relations, dependencies/interfaces, components/ports/targets, tests,
documentation/annotations, retirements, counts, and digest. Migration tooling is temporary and
must be deleted after materialization. The retained projection is evidence, never authority.

Built-in evidence has two independent byte owners: maintained standard package generation and the
embedded executable assets. Product verification exports both package transport and artifact and
requires exact byte equality with the generated maintained files.

## Command lifecycle requirements

A release executable copied to an isolated directory must complete:

```text
capabilities -> new command -> status / inspect / query
             -> reviewed change plan/apply -> check
             -> deterministic build -> pure run
```

The environment must not supply Cargo, a checkout-relative asset, network dependency, external
template, or predecessor repository. The recipe must contain an exact standard dependency and
graph-owned test.

Verification must prove:

- check, build, run, query, and every failure path leave semantic `HEAD` unchanged;
- two clean/equivalent builds and exact-current-cache builds produce identical artifact bytes;
- representative post-publication incremental compilation equals a clean rebuild;
- missing cache clean-builds and corrupt/stale/foreign cache never selects wrong meaning;
- a cache failure after accepted publication is reported separately from acceptance;
- output conflict, file/directory/symlink targets, invalid parents, interruption, and exhaustion
  do not publish partial output or modify existing paths;
- malformed, truncated, duplicate, noncanonical, overflowing, foreign-identity, and trailing
  transport/artifact/argument inputs reject before execution or visible output;
- stale and competing semantic publication accepts at most one exact base result; and
- predecessor markers and removed commands never enter alternate dispatch.

Test names and retained evidence must map these properties to exact mechanisms. An internal unit
fixture alone is not copied-binary or maintained-consumer completion.

## Maintained consumers and service boundary

The standard package and `lkjournal` must open as Graph 5, check through normalized differential
execution, build deterministic artifact 10, and match their checked-in generated assets. Exact
package, target, test, dependency, and public-interface inventories must be retained in migration
or lifecycle evidence.

Service verification freshly builds `lkjournal` through the public binary, requires byte equality
with the checked-in artifact-10 bundle, and copies the bundle plus deployment descriptors to an
isolated run. It validates exact bundle/manifest/root/revision/state identity, launches
`serve`/`worker`, and exercises the maintained external workflow. It must audit that no project
marker or repository path is opened and that canonical Graph authority is unchanged before/after.

If PostgreSQL/container/environment prerequisites are absent, service is `unavailable` with an
exact reason. Unavailable is never rewritten as pass or silently omitted.

## Exact release-target admission

Source-wide verification and exact release-target admission are separate evidence domains. A host
build or a source test cannot stand in for the distributed candidate. Target admission binds the
exact source commit, pinned toolchain and native inputs, canonical target-policy digest, candidate
bytes and mode, and every required observation in one strict receipt.

The sole current release target is `x86_64-unknown-linux-musl`. Its exact candidate must be a regular
non-symlink little-endian ELF64 x86-64 executable with no `PT_INTERP` header, no runtime
`DT_NEEDED` entry, and no GLIBC symbol-version requirement. The first-party inspector must reject
malformed, truncated, trailing, foreign-machine, dynamic, contradictory, and target-mismatched
inputs without using `ldd` or trusting the target name.

The candidate must complete the command lifecycle in two independently pinned Linux/amd64
userlands: one musl-based and one older-glibc-based. Candidate execution has no network and no host
library bind mount. Admission also requires the same bytes to pass the maintained distributed HTTP,
stateful HTTP, and standalone service oracles. The receipt records distinct userland, application,
static-inspection, cleanup, and resource classifications. Static linkage and those two userlands do
not imply a minimum kernel, universal Linux portability, another architecture, or hostile-code
isolation.

## Transferable application evidence

`distributed-http` and `stateful-http` each have contributor and transferred execution contexts.
Transferred mode requires an explicit absolute lexically canonical create-new evidence root outside
any checkout. It must not discover a repository root, use a compile-time checkout path, invoke
Cargo, read a source/generated/template file, or use an ambient application helper. The receipt
binds the verifier, source candidate, private copied candidate, execution context, optional checkout,
result, bounded logs, and complete cleanup.

The stateful owner constructs its 1,010-record BBS change from exact candidate discovery. It retains
one application definition across contributor, target-admission, pre-publication, and anonymous
public verification. Passing evidence requires reviewed plan/apply, idempotent reprepare,
clean/incremental artifact equality, real HTTP create/read/update/delete, strict malformed input,
statement rollback, migration divergence failure, restart persistence, failed-startup behavior,
graceful shutdown, unchanged accepted Graph authority, secret redaction, database cleanup, and
runner-root cleanup. PostgreSQL unavailability, timeout, early exit, migration/statement failure,
shutdown failure, or cleanup failure remains a typed non-pass. Workflow shell cannot reclassify it.

The no-database distributed HTTP oracle remains an implementation-disjoint faster gate. Neither
application oracle replaces the other at release admission.

## Package and public-release evidence

Release preparation requires fresh successful source-full and target-admission receipts bound to
the same commit and candidate. Manifest and receipt schemas explicitly represent static linkage and
must reject predecessor dynamic-only metadata. Two notice generations and two packages must be
byte-equal. Strict verification owns archive inventory, order, mode, timestamp, link/traversal,
canonical metadata, checksum, target/linkage, candidate, and extraction-conflict rejection.

Before publication, a read-only no-checkout job verifies exact artifact and verifier handoffs,
re-inspects the extracted candidate, and freshly passes transferred distributed and stateful HTTP.
The publication job depends on both receipts, has the only release-write permission, performs no
checkout, and executes no repository binary or script.

After immutable publication, exact-tag and `releases/latest` assets are downloaded anonymously and
verified independently. Each path requires checksum, GitHub asset digest, release/asset attestation,
strict extraction, source/manifest/candidate equality, static inspection, transferred distributed
HTTP, transferred stateful HTTP against a fresh isolated database, and cleanup. Exact/latest byte
equality is required but cannot replace either behavioral run. A required stale, reused, skipped,
unavailable, failed, cancelled, or unrun observation is not public acceptance.

## Verification profiles and receipts

The contributor owner is:

```sh
cargo run --locked -p lkjscript-dev -- check PROFILE
```

`focused` runs narrow format/library/public checks. `changed` selects by exact changed inputs and
widening rules. `product` builds release and verifies copied-binary workflows, maintained Graph 5
consumers, generated docs, and built-in/generated assets. `service` owns isolated standalone
artifact-10 service acceptance. `full` owns formatting, lints, workspace targets, all tests,
release/product/service classification, and diff checks; final full evidence must be fresh.

The harness owns gate dependencies, exact fingerprints, bounded child logs, required outputs,
timeouts, and fresh/reused/skipped/unavailable/failed classification. Reuse is valid only when the
harness proves every semantic and operational input identical and the profile permits it. Final
product and full verification run after final code, generated assets, evidence, and documentation.

Receipts and logs are derived evidence. They bind command, exact inputs, toolchain/platform,
dependencies, classification, output paths/digests, and limitations. They never enter semantic
revision identity. Large logs remain under `.artifacts`; tracked evidence contains concise
structured summaries and digests.

## Performance evidence

Release measurements cover copied command creation, first check, clean build, exact-current build,
post-change incremental plus equal clean rebuild, pure run, standard clean check/build, and
`lkjournal` clean check/build.

Measurements keep wall time, CPU time, peak RSS, semantic inventory, compiler units compiled/
reused/removed, linked packages, artifact objects/bytes, repository/cache I/O when observed,
output records/bytes, synchronization/visibility operations when observed, retries, and unavailable
dimensions separate. Cache state, build warmth, filesystem, toolchain, and environment are named.
Bytes and time are not provider-token, request, retry, cache-hit, or monetary telemetry.

Exact-current reuse must perform less semantic compilation work than clean build. Incremental work
must remain bounded by prepared compiler impact and persistent-map locator costs. No absolute
latency target is implied, and retained regressions require explanation rather than speculative
optimization.

## Claims and handoff

Completion reports exact focused/product/full/service commands and classification, commit SHAs,
receipt/log/artifact paths and SHA-256 digests, deviations and evidence, measurements, known
limitations, working-tree state, and push status. Stale, reused when fresh was required, skipped,
unavailable, or failed evidence cannot be described as passed.

Security, portability, scale, artifact provenance, provider-token, and monetary claims require
direct retained evidence. The current verified environment does not imply portability or hostile-
code isolation.
