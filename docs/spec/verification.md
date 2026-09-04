# Verification and evidence

Status: normative.

## Independent correctness mechanisms

Complete typed meaning graph reconstruction and validation is the semantic oracle. Sparse
repository reads, witness-maintained changes, package interfaces, compiler selection, and query projection
must agree with it. Physical pack/catalog layout, persistent-map partitioning, derived cache state,
and request spelling are normalized away only when they are not semantic.

Production execution uses normalized bytecode and dense indexes. The canonical reference
interpreter independently reads accepted semantic owners and evaluates their typed structures.
Every pure command and graph-owned test used for acceptance requires production/reference equality
before comparing the expected value. Skipped, unavailable, cancelled, exhausted, or unrun work is
not a pass. Live effects are not duplicated for differential evidence.

Affine resource acceptance additionally uses a finite implementation-disjoint flow oracle. It may
share bounded snapshot decoding but cannot share production provenance, transfer, consume,
resource-call graph, or branch-merge logic or encoded expected results. It must agree on both
maintained graphs and on fabrication, unrestricted resource parameters, borrow/consume, duplicate
and post-consume use, exact requirement/interface binding, one-level and nested direct handoff,
left-to-right argument commitment, caller reuse, branch mismatch, function escape, self/mutual
recursion, and forbidden signature or containment mutations.

Function-definition projection acceptance uses complete typed reconstruction as a disjoint oracle.
The oracle may decode the same canonical authority, but it cannot call production point traversal,
structural ordering, rendering, paging, continuation, or expected-result helpers. It independently
derives the selected function contract, structural owner preorder, owner-bound validation facts,
exact semantic relations, and referenced capability operations. Production output must agree for
representative pure, task, generic, transaction, nominal-match, capability, and affine forms plus
the maintained worker and the largest maintained function. Corrupt ownership and exact-fit/one-over
admission cases remain finite implementation tests rather than repaired oracle results.

Function-extraction acceptance uses a second derivation over that independently reconstructed
definition. It cannot call production extraction traversal, capture mapping, identity allocation,
rewrite, or logical-plan helpers. For each admitted root it must agree on the movable owner set and
digest, unique parent, free and escaping locals, canonical capture order and names, exact types and
uses, result, least effect and caller-ordered requirement subset, affine provenance, body counts,
and preserved/changed/generated owner classes. Finite negative agreement includes whole or foreign
roots, generic and recursive targets, resource results and containers, ambiguous provenance, and
unsafe affine boundaries.

Structured-session acceptance independently reconstructs the exact
`(Option<State>, SessionEvent) -> SessionDecision<State>` relation from nominal layouts and type
structure without calling the production relation validator. It must agree at accepted graph,
package, compiler, artifact, and deployment boundaries and reject a different repeated state plus
every retained live/secret/callable/unresolved type. Runtime tests use controlled time, fault
points, accounting peaks, and join/permit observations to prove one transition, complete output
reservation, event ordering, coalesced ticks, item/byte backpressure, cancellation, phase failures,
and zero resources after parent completion.

HTTP route-pattern acceptance uses a separate route-language oracle. It may read bounded public
route, parameter, port, and function projections, but it cannot call production selector parsing,
overlap, specificity, route-set digest, compiler, artifact, preparation, matcher, or capture code.
The oracle independently parses public whole-segment patterns, reconstructs match languages and
specificity order, validates capture names against the ordered unrestricted `Text` parameter
suffix, selects routes, and extracts raw segment spelling. It must agree on exact precedence,
comparable nesting, two-capture order, route-set identity, malformed and exhausted patterns,
duplicate languages, incomparable overlap, signature drift, shared-port disagreement, and
order-invariance. A raw TCP HTTP/1.1 client observes dispatch and effects without importing Axum or
production HTTP helpers; live effects execute only once through production.

Migration evidence for maintained consumers compares a sorted generation-neutral projection of
predecessor and current typed meaning, including identity continuity, declarations/members, type and
expression ownership, relations, dependencies/interfaces, components/ports/targets, tests,
documentation/annotations, retirements, counts, and digest. Migration tooling is temporary and
must be deleted after materialization. The retained projection is evidence, never authority.

Object-catalog acceptance has a separate footer oracle. It enumerates and strictly decodes
immutable pack footers, deterministically resolves duplicate physical objects, and computes entry
count and packing-independent logical commitment without calling the current catalog manifest reader,
segment lookup, merge, or commitment implementation. It must agree with the selected manifest after
healthy incremental construction and recovery. Repeated-process open, lookup, plan, apply, and seal
fixtures separately require zero complete footer scans and zero full reconstructions; recovery and
deep verification must report their scans rather than relabel them as healthy work. Missing,
predecessor, malformed, stale, canonical-corruption, and every segment/manifest/cleanup
interruption fixture must preserve old-or-new `HEAD` visibility and exact canonical bytes.

The lightweight million-owner capacity admission uses a copied release-mode executable and only
the unchanged public `change plan` / `change apply` path. It constructs exactly 1,000,000
independent modules in 1,000-operation batches, then runs status, exact inspection, bounded owner,
exact-name, and bounded-context queries plus typed semantic and independent catalog oracles. The
gate admits at most 7,200 wall seconds and 68,719,476,736 run bytes and requires at least
8,589,934,592 available memory bytes and 68,719,476,736 available filesystem bytes at preflight.
It is capacity-only evidence: it does not run or claim million-owner check, compilation, build,
release, deployment, service, operational-data, or production admission.

Recipe lowering additionally requires generation-neutral projections for `minimal`, `command`,
`http`, and `nostr-relay-info` to agree with their predecessor results while excluding deliberately
fresh repository, package, revision, and owner identities. The oracle must use an independent graph
reader or explicit expected records, not the recipe operation list. It compares owner kinds/names,
parentage, types, expressions, dependency bindings, requirements, ports, targets, tests, relations,
and deployment inventory. Each nonempty recipe must also prove that every operation is owned by the
public compact descriptor and uses ordinary authored preparation and full validation.

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

A copied executable must also discover and inspect exact-interface resource types, parameter-use
modes, and parameter requirement binding; normalize direct and input-file affine requests
identically; author nested private task helpers and a caller solely through compact plan/apply;
inspect their complete definitions; check/build the accepted direct handoff; and reject stale,
predecessor, missing/wrong binding, unsupported signature, caller-reuse, duplicate-transfer,
branch-mismatch, indirect-call, and recursive forms with an unchanged complete authority inventory.

A copied executable in an empty temporary `minimal` project must discover `interactive`, export and
stage the exact built-in transport, author the canonical session types, handler, component,
requirement, function-backed port, and target solely through ordinary compact plan/apply, then
inspect, check, build, and prepare/serve it. It must reject stale, predecessor, foreign, malformed,
and relationally invalid inputs without a checkout-only authoring helper or a semantic-authority
change during runtime.

A copied executable in a separate empty `minimal` project must discover the `add.http-route` and
`set.http-route` selector fields and limits, author both exact and pattern routes through ordinary
compact plan/apply, and inspect their selector segments, captures, handler signatures, route-set
digest, and maximum specificity chain. It must change and delete pattern routes while preserving
stable route identity, reject altered commitments and stale bases without publication, then check,
build, prepare, and serve the result. Raw requests must prove exact-over-pattern selection, two
ordered captures, raw spelling, query independence, fixed no-effect 404 behavior, bounded matcher
work, restart equality, and complete cleanup.

The copied executable must discover function-definition inspection offline, force multiple pages
with changed resume budgets, and independently recompute the complete digest from raw compact
records. In a fresh isolated HTTP project it locates and projects `response-text`, constructs one
ordinary compact `replace.body` request, proves direct/file plan equality, applies against the exact
base, reinspects the child revision, and observes only the intended response literal change with an
unchanged function contract. It then checks, deterministically builds, serves on loopback, and
observes that response. Stale continuation/base/plan, mutated token, and projection-as-change input
must reject without advancing authority, and the temporary project, service root, request files,
artifacts, and copied candidate must be removed.

The copied executable must also discover the sole `extract.function` operation, normalize its
record and direct forms identically, export and strictly decode the complete logical review, and
apply it once at an exact base. Pure fixtures execute equally before and after in production and
the canonical reference tier. Task fixtures prove least-effect closure and the existing affine
handoff rules without replaying live effects. Selected subtree identities must move intact; stale,
conflicting, malformed, predecessor, bounded-exhaustion, cancellation, pre-visibility interruption,
and derived-cache-failure cases must leave either the complete old or complete new authority.

Test names and retained evidence must map these properties to exact mechanisms. An internal unit
fixture alone is not copied-binary or maintained-consumer completion.

## Maintained consumers and service boundary

The standard package and `lkjournal` must open as typed meaning graph repositories, check through
normalized differential execution, build deterministic artifact bundles, and match their checked-in generated assets. Exact
package, target, test, dependency, and public-interface inventories must be retained in migration
or lifecycle evidence.

Service verification freshly builds `lkjournal` through the public binary, requires byte equality
with the checked-in artifact bundle, and copies the bundle plus deployment descriptors to an
isolated run. It initializes one first-party data root shared by separately validated service and
worker grants, validates exact bundle/manifest/root/revision/state identity, launches `serve` and
two `worker` processes, and exercises login, actor isolation, resource/history/object
reconciliation, claim/info/renew/complete, retry/fail, expired-lease replacement, restart, backup,
absent-root restore, failed startup without readiness, cancellation, and cleanup. A bounded
independent operational observer decodes the unchanged queue-data format without calling the queue
engine and records attempt advancement, final state, cleared raw transition fields, productive
iterations, and clean task shutdown. It must audit that no project marker or repository path is
opened and that canonical typed meaning authority is unchanged before/after. This required
product/service gate has no database server, container, connection secret, or host database-library
prerequisite.

The same copied candidate must project the maintained affine worker entry, its private helper, and
the independently selected largest maintained function from an isolated full authority copy before
running live effects. The comparison derives the entry's `jobs` acquisition, absent/live match,
single consume transfer, and no post-transfer use; then derives the helper's exact parameter-
requirement relation, `lease-info` borrow, renewed-state match, heartbeat consume, and complete/fail
consume from generic definition records, exact references, and the disjoint typed oracle. Entry and
helper must each remain at or below 40 body records. Small and changed page budgets must cover each
exact logical inventory once, with matching complete digest/counts and unchanged repository tree,
semantic `HEAD`, generated application bundle, descriptor, data, queue, and object authority.

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
stateful HTTP, outbound HTTP, and standalone service oracles using isolated authorities. The receipt records distinct userland, application,
static-inspection, cleanup, and resource classifications. Static linkage and those two userlands do
not imply a minimum kernel, universal Linux portability, another architecture, or hostile-code
isolation.

## Transferable application evidence

`distributed-http`, `stateful-http`, and `outbound-http` each have contributor and transferred execution contexts.
Transferred mode requires an explicit absolute lexically canonical create-new evidence root outside
any checkout. It must not discover a repository root, use a compile-time checkout path, invoke
Cargo, read a source/generated/template file, or use an ambient application helper. The receipt
binds the verifier, source candidate, private copied candidate, execution context, optional checkout,
result, bounded logs, and complete cleanup.

The stateful owner creates a dependency-free `minimal` project, discovers and exports the exact
built-in transport through the copied candidate, stages it without changing semantic `HEAD`, and
constructs one bounded request adding the dependency, component, requirements, function-backed HTTP
ports, `serve` target, exact and pattern routes, indexed capture parameters, and BBS policy. It
retains one application definition across contributor,
target-admission, pre-publication, and anonymous public verification. It may use public parsing
utilities but no recipe owner builder, source, fixture topology, or `http` recipe. Passing evidence
requires direct/input normalization equality, reviewed plan/apply, idempotent reprepare,
clean/incremental artifact equality, a complete pattern add/set/delete lifecycle, exact precedence,
two ordered captures, raw spelling and query independence, the named reducer/function value/standard
fold construction, real HTTP create/read/update/delete, and missing/nonmatching/repeated/reordered
header admission,
strict malformed input, expectation rollback, schema divergence failure, restart persistence,
logical backup/absent-root restore, corrupt/absent-root failed startup, graceful shutdown, unchanged
accepted graph authority, and data/runner-root cleanup. Timeout, early exit, data failure, shutdown
failure, or cleanup failure remains a typed non-pass. Workflow shell cannot reclassify it.

The stateless distributed HTTP oracle remains an implementation-disjoint faster gate. None of the
three application oracles replaces another at release admission.

The maintained service owner also starts `lkjournal-live-1` against the same isolated initialized
data root as HTTP and worker coverage. Its bounded raw TCP client computes the upgrade accept value,
masks client frames, parses server frames, and emits malformed cases without importing production
handshake/frame/close helpers. Two authenticated connections prove independent subscriptions;
HTTP create/update prove tick-driven push; replace/unsubscribe and actor isolation remain graph
policy; slow-reader, fragmentation, ping/pong, close, unmasked/invalid/oversized/stalled/abrupt,
overload, cancellation, shutdown, restart, and resubscription cases retain transcript digests,
accounting peaks, unchanged semantic authority, and complete cleanup. Fixtures may own only bounded
expected transcripts, never application route, token, grammar, filter, order, or subscription
policy.

The outbound owner creates `nostr-relay-info` only through copied-candidate discovery and project
creation, checks it, compares clean and exact-current artifact bytes, and serves it from an isolated
root. Its implementation-disjoint raw HTTP/1.1 and TLS oracle records exact request line, Host,
headers, connection count, and bounded response bytes without sharing production endpoint parsing,
HTTP parsing, response generation, or application assertions. Fixed deterministic certificate
fixtures exercise trusted, untrusted, expired, and hostname-mismatched chains without retaining a
private key or root-secret value.

Passing aggregate outbound proof combines the copied-candidate receipt with focused adapter tests.
The receipt covers exact HTTPS success and byte preservation, explicit loopback HTTP,
public-versus-loopback admission, redirect non-following, non-200 and wrong media type, response
header/body exhaustion, total timeout, inbound-client cancellation, malformed protocol, invalid
startup trust, recovery, restart, shutdown during an active request, repeated clean shutdown,
unchanged accepted graph authority, and complete process/socket/secret/project/artifact/root
cleanup. Focused adapter tests with independently enumerated expected outcomes cover
mixed/forbidden scripted DNS and graph-supplied forbidden headers. No required case contacts a live
relay or automatically replays a GET.

Contributor-only migration evidence runs an exact PostgreSQL 16.15 image pinned by manifest and
config digests. It creates deterministic representation-neutral BBS and `lkjournal` fixtures in an
isolated database, exports sorted canonical facts without using the first-party codec or transaction
model, imports them into an absent data root, verifies backup/restore equality and the copied public
workflow receipts, then removes the temporary container and data roots. After one warm-up, three
fresh samples per workload retain median wall time, CPU observation, peak RSS, durable logical-data
bytes, synchronization/publication counts, and operation counts. Ratios above 5x wall, 2x RSS, or 4x
durable bytes block admission. PostgreSQL is an oracle only: it is not a product dependency,
release-candidate prerequisite, alternate adapter, dual reader/writer, or public import path.

## Package and public-release evidence

Release preparation requires fresh successful source-full and target-admission receipts bound to
the same commit and candidate. Manifest and receipt schemas explicitly represent static linkage and
must reject predecessor dynamic-only metadata. Two notice generations and two packages must be
byte-equal. Strict verification owns archive inventory, order, mode, timestamp, link/traversal,
canonical metadata, checksum, target/linkage, candidate, and extraction-conflict rejection.

Before publication, a read-only no-checkout job verifies exact artifact and verifier handoffs,
re-inspects the extracted candidate, and freshly passes transferred distributed, stateful, and
outbound HTTP.
The publication job depends on both receipts, has the only release-write permission, performs no
checkout, and executes no repository binary or script.

After immutable publication, exact-tag and `releases/latest` assets are downloaded anonymously and
verified independently. Each path requires checksum, GitHub asset digest, release/asset attestation,
strict extraction, source/manifest/candidate equality, static inspection, transferred distributed
HTTP, transferred stateful HTTP against a fresh isolated first-party data root, transferred
outbound HTTP against fresh local HTTP/TLS fixtures, and cleanup. Exact/latest
release-asset, manifest, and candidate byte equality is required but cannot replace either
behavioral run. Clean/incremental artifact equality is required within each independently created
application; artifacts from the two fresh applications have independently allocated semantic
identities and are not required to have the same digest. A required stale, reused, skipped,
unavailable, failed, cancelled, or unrun observation is not public acceptance.

## Verification profiles and receipts

The contributor owner is:

```sh
cargo run --locked -p lkjscript-dev -- check PROFILE
```

`focused` runs narrow format/library/public checks. `changed` selects by exact changed inputs and
widening rules. `product` builds release and verifies copied-binary workflows, maintained typed
meaning graph consumers, generated docs, and built-in/generated assets. `service` owns isolated
standalone artifact bundle HTTP/interactive/worker service acceptance. The data cutover additionally requires the
contributor PostgreSQL differential/resource receipt. `full` owns formatting, lints, workspace
targets, all tests, release/product/service classification, and diff checks; final full evidence
must be fresh.

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
