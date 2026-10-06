# Release procedure

GitHub Releases are the public binary distribution path. Accept one finalized candidate, nominate
its exact producer run and attempt, and promote its unchanged archive, checksum and bootstrap.
Content identity, behavioral acceptance and publication authority are separate decisions.

Product tags are exact `vA.B.C` identifiers, not Semantic Versioning promises.
All three decimal components have the same role: none denotes compatibility,
stability, change size or a feature milestone. The root Cargo package owns the
whole version; do not introduce a second version file or synchronize unrelated
contract generations. Preserve canonical spelling and the 64-byte tag bound at
the shared release-container validator. Match the entire product/tag identity.
Select public/latest explicitly; do not infer compatibility or publication from
numeric ordering. Existing published tags, candidate bytes and source bindings
remain unchanged. See the [product identity contract](spec/product-surface.md#opaque-three-component-identifiers).

Before freezing a source selection, complete maintained native `check` operations
for `packages/standard`, `applications/lkjournal`, `tools/native-guides/project`
and `tools/native-policy/project` with the selected executable. Inspect newly
materialized packs through their native owner and include required deterministic
inputs in the reviewed source; do not depend on warmed, untracked local files.
A tracked-source copy must reproduce the maintained inputs without adding missing
native packs. Keep the source-stability assertion: do not ignore all packs, raise
limits or relabel `worktree_changed_during_run` as acceptance. The
[2026-10-01 cold-source control](campaigns/20261001-version-policy.md#source-only-maintained-input-reproduction)
records the concrete four-pack inventory correction. This does not freeze general
compiler caches into source or make generated artifacts editable semantic authority.

Current public availability belongs to [status](status.md#public-binary).
The completed v0.1.64 publication is recorded in the
[owned-choice continuation](campaigns/20261002-owned-choices.md#completed-v0164-publication).
Historical selections remain in their original archives and the linked immutable
procedure revision; they are not instructions to restart completed releases.

## Published structured-parallel v0.1.68

The [0.1.68 notes](releases/v0.1.68.md) consolidate same-task owned calls and methods,
runtime mailbox custody and lexical parallel owned tasks. The
[current status](status.md) records tested source, mainline integration and the
completed unchanged-asset publication and anonymous verification. Original early
development evidence remains in the [parallel archive](campaigns/20261002-structured-parallel.md).
The development 0.1.69 successor follows the same procedure: after fresh full-source
acceptance and copied-host native package evidence, dispatch one normal non-publishing
candidate from integrated source. Preserve every source, finalized-archive,
installed-recovery, pinned-userland and original-reader obligation.

Final candidate acceptance now runs the source-matched public harness against the
exact executable extracted from the finalized archive. It selects every enumerated
case matching `native_owned_`, `native_byte_buffer_`, `native_byte_ranges_`,
`resident_policy`, `native_parallel` and `native_refresh`, plus the exact
`copied_binary_authors_builds_and_serves_interactive_topology_from_minimal` case.
Every family must be present. The native owner rejects missing, duplicate, ignored,
failed or unexpectedly substituted cases even when the harness exits successfully.
It records the source-selected Cargo executable, copied harness and candidate
identities, exact inventory, bounded process logs and joined cleanup. The harness
runs outside the compiler checkout with a closed environment; candidate and harness
bytes must remain unchanged. No additional manual supplement is needed for a new
candidate accepted under this contract.

For focused diagnosis against explicitly supplied candidate bytes, the same owner is
available from a clean source-matched checkout (ordinary or linked worktree) through the immutable verifier:

```sh
lkjscript-dev release public-harness --candidate /absolute/path/lkjscript \
  --evidence-root /absolute/absent/native-public-evidence
```

This standalone command binds the current harness source and supplied executable
bytes, not their provenance. The enclosing candidate owner separately authenticates
the finalized archive, source and build receipt. Development-host proof does not
substitute for final-byte acceptance. Normal promotion and anonymous acquisition
still use the unchanged assets and existing authority; no credentials, protections,
immutable tags or prior assets are changed.

`lkjscript-final-candidate-acceptance-3` requires the native-public proof in addition
to all previous source, six target-owner, two userland and installation boundaries.
The current controller intentionally rejects predecessor acceptance terminals rather
than relabelling them. Their original contracts and evidence remain historical facts;
a corrected successor requires a fresh producer. In particular, producer 37046478616
accepted the earlier 0.1.68 source without the joined-deadline repair and is superseded,
not a candidate to promote after the repair.

[Current status](status.md) owns the exact public/latest selection. A pending
successor producer is release-only waiting; accepted source can proceed to main.

## Content and compatibility

The owned-result successor, development v0.1.69, extends the same mandatory
`native_parallel` public-harness family with separate supplier/worker/consumer
packages, full returned payloads, reviewed edits and source-deleted execution.
All enumerated matching cases remain required against the finalized archive's
exact executable; there is no separate manual supplement. Its compiler 20,
bytecode 16 and artifact 27 assets require their own source and candidate
acceptance. The v0.1.68 producer remains a separate immutable selection and cannot
serve as evidence for these successor bytes.

Development v0.1.70 extends the same family with concrete generic child types and
nominal implementation operands, including generic forwarding and whole owned
aggregate returns across separate packages. Compiler 21, bytecode 17 and artifact
28 require fresh source and finalized-byte acceptance for these changes; neither
v0.1.68 nor v0.1.69 evidence is relabelled as v0.1.70 proof.

Development v0.1.71 extends that required family with generic group builders checked
before concrete callers exist, both mixed result-pair orientations, intermediate
pairs, twice-forwarded implementation parameters and reviewed identity-preserving
edits. The exact published guide also runs as a public-harness test. Graph 22,
validator 27, compiler 22 and artifact 29 require their own fresh acceptance;
predecessor proofs and immutable assets remain separate. The final-byte harness
selects every matching native parallel case automatically.

Development v0.1.72 adds reusable structured workers and the maintained three-package
transformation/reduction workload to that same required family. Final-byte cases
exercise complete serial/parallel/nested results after source deletion, an independent
HTTP sibling, repeated dispatch and joined shutdown. CLI observation 36 and
shared-runtime observation 2 distinguish invocation dispatch/fallback from physical
worker lifetime; semantic graph and artifact generations remain unchanged. Fresh
source and finalized-byte acceptance remain required for this implementation.

Public v0.1.73 adds explicit effect and requirement applications to Owned
generic witness calls. The mandatory `native_owned_` family includes its maintained
three-package example, exact authority/witness selection, canonical edits and
source-deleted execution. Graph 23, validator 28, compiler 23, bytecode 18 and
artifact 30 require fresh source and finalized-byte acceptance. Supported semantic
predecessors remain readable; derived bundles must be rebuilt. Package interface
13 and type-object 10 retain their layouts. Public projection 13 exposes the new
operands and rejects predecessor continuations.

Development v0.1.81 adds synchronous borrowed graph tasks and exact task methods,
plus recursive scoped reads spanning joined parallel children. Shareable is
independent of Transferable: synchronous task borrowing requires Owned, shared
child reads require Owned and Shareable, and consumed inputs or owning results
retain transfer admission. Graph/owner 30, validator 35, package interface 18,
compiler unit 30, bytecode 25 and artifact 37 select the new admission contract.
Rebuild derived bundles and maintained inputs through supported product owners;
predecessor receipts cannot attest the successor source or finalized bytes.
The same mandatory `native_owned_` and `native_parallel` public-harness families
own its detached three-package witnesses. The [release notes](releases/v0.1.81.md)
describe the capability; [status](status.md) owns actual acceptance and publication.

The canonical manifest discriminator is `format: "lkjscript-release-content-1"`. It binds the product
version/intended tag, exact product commit, repository, target/build policy and command, pinned Rust
and Cargo, lockfile, static ELF executable, license/notices and deterministic packaging. It contains
no publication mode, remote tag object, promotion permission, consuming run or reporting commit.
This encoding identity does not add a human-facing language/runtime version. Graph, compiler,
program-artifact and application-data generations do not change.

The public assets are exactly `lkjscript-x86_64-unknown-linux-musl.tar.gz`, `SHA256SUMS` and
`install.sh`. The archive contains only the ordered `lkjscript/` directory, executable, license,
third-party notices and manifest. The checksum has one exact archive entry. The existing bootstrap
binds exact immutable URLs and lengths/hashes, extracts the new manager and delegates native
installation to it. It requires no Cargo, Python or checkout. Installation grants no application
authority and performs no operational-data migration.

New native readers also strictly admit authentic supported legacy manifests and installed receipts,
retaining their original canonical encoding. Legacy declared publication remains unverified;
neutral content claims no publication provenance. Neither implies authenticity. Unknown fields,
ambiguous formats, unsafe members, links/traversal, extra entries, wrong modes, checksum/ELF/version
mismatches and conflicting immutable slots still reject. Existing slots and pinned runtime paths
remain intact. Older managers may reject neutral content before changing an installation; the exact
new bootstrap is the upgrade/recovery route. Do not edit old manifests, receipts, tags or assets.

## Coverage and admission

`check full --fresh` remains an honest standalone complete profile with its existing 26 gates.
Release acceptance instead requires the dependency-complete union below; a profile label alone is
never equivalent proof. The [verification specification](spec/verification.md) owns the full mapping.

| Previous claim | Current required owner |
| --- | --- |
| Source/reference, safe Rust, lint, tooling/checker correctness, generated assets, maintained consumers | Fresh `check release-source`: 20 gates, including retained default/all-feature distinctions |
| External service, distributed HTTP, outbound HTTP, offline packages, pure tail, stateful HTTP | Six target owners once against the executable extracted from the final archive |
| Static linkage and both pinned userlands | Native ELF admission and the existing exact target admission |
| Installed lifecycle, upgrade, retained runtime/selection and two-version recovery | Candidate installation tier once; real bootstrap/native installation |
| Transfer and anonymous exact/latest acquisition | Authenticated inventory/byte identity, strict native admission and small installed create/edit/build/run lifecycles |
| Original-reader admission | Producing CI context before the candidate terminal is accepted |

Source-only probes embedded in behavioral owners remain owned by those original readers; the target
verifier is source-built. Necessary reference/unit work and host-verifier configurations are not
second distributable product builds. Workloads, assertions and individual owner deadlines remain
unchanged. The candidate supervisor has a finite outer allowance covering the existing target-owner
allowances; no completed broad owner is replayed merely because bytes crossed a job boundary.

The checker retains immutable output copies for its gates before later Cargo configurations can
replace shared target paths. Its original reader checks exact source inputs, registry dependency
closure, verifier/runtime/environment, commands, logs, retained outputs and fresh terminal outcomes.
The target and installation readers admit their original evidence in the same owning environment.
A forged summary, omitted/skipped/failed stage, cancellation or incomplete cleanup cannot yield
candidate acceptance. Diagnostic originals need not be relocatable; portability applies to the
admitted terminal decision under authenticated service provenance, not arbitrary filesystem replay.

## Build and accept a candidate

The maintained `Release` workflow uses explicit `workflow_dispatch` on main. It has no tag-triggered
build or automatic PR/workflow-run artifact execution. Inspect existing sources/runs before dispatch,
reuse healthy accepted work, and record the exact resulting event SHA and attempt.

```sh
gh workflow run release.yml --repo lkjsxc/lkjscript --ref main -f operation=candidate
gh run view --repo lkjsxc/lkjscript RUN_ID --json headSha,status,conclusion,jobs
gh api repos/lkjsxc/lkjscript/actions/runs/RUN_ID --jq '{id,run_attempt,head_sha,status,conclusion}'
```

The workflow installs Rust/Cargo 1.98.0, pinned native musl packages and both pinned userland images
from the maintained `release target` policy. It verifies cargo-about 0.9.2 by archive and executable
digests. Fixed target production requires a config-free `CARGO_HOME` and rejects unbound compiler,
linker, target or code-generation overrides. Ordinary source tests may retain their own toolchain
configuration. The selected verifier is copied once and stays immutable across other Cargo builds.

The corresponding owner operations, when local execution is justified, are:

```sh
VERIFIER=/absolute/immutable/lkjscript-dev
"$VERIFIER" check release-source --fresh --machine
"$VERIFIER" release build --output /absolute/new/lkjscript --receipt /absolute/new/build.json
"$VERIFIER" release prepare \
  --candidate /absolute/new/lkjscript \
  --cargo-about /absolute/pinned/cargo-about \
  --cargo-about-archive /absolute/pinned/cargo-about.tar.gz \
  --output /absolute/new/assets --tag vA.B.C
"$VERIFIER" release verifier prepare --executable "$VERIFIER" \
  --output /absolute/new/verifier --tag vA.B.C --commit EXACT_PRODUCT_SHA
/absolute/new/verifier/lkjscript-dev release candidate accept \
  --assets /absolute/new/assets \
  --source-receipt /absolute/repository/.artifacts/lkjscript-dev/check/RUN/receipt.json \
  --build-receipt /absolute/new/build.json \
  --verifier-identity /absolute/new/verifier/verifier-identity.json \
  --evidence-root /absolute/new/acceptance --output /absolute/new/release-receipt.json
```

All paths shown as new must be absent and owned. Keep source-check and acceptance process inputs
stable. `prepare` generates notices twice, constructs final neutral content and renders the final
bootstrap. Its result is `constructed`, never accepted/promotable. `candidate accept` extracts the
final executable, independently compares deterministic packaging without rebuilding it, admits
source proof, runs the six owners and pinned environments, runs installation/recovery, invokes the
original readers and joins owned resources before emitting `candidate_accepted`. Failures retain
incomplete/failed/cancelled/unavailable state and their stage logs. No asset is rewritten to obtain
acceptance or later permission.

## Select, promote and resume

Select an exact producer, including its original attempt. `promote` includes selection and transfer
admission; a separate read-only `consume` invocation is optional diagnosis, not a required rehearsal:

```sh
gh workflow run release.yml --repo lkjsxc/lkjscript --ref main \
  -f operation=consume -f producer_run=RUN_ID -f producer_attempt=ATTEMPT
```

The controller comes from that invocation's integrated mainline workflow source; its revision is
recorded separately from product source. It authenticates repository/head repository, allowed event
and workflow, exact run/attempt, successful mandatory job/steps, terminal contract, artifact IDs,
service ZIP digests, exact inventories and expected asset/verifier bytes. It requires product source
to be reachable from freshly resolved main. Artifact names or caller-supplied hashes are insufficient.
It then runs the small transfer lifecycle without product builds or broad application replay.
Read-only publication preflight reports an independent authorized/rejected result. For the private
0.1.38 cutover candidate, the existing tag selects different content and must reject.
Successful consumption requires both a completed authority decision and successful transfer/installed
lifecycle with joined cleanup. A zero-exit authority process must report `promotion_authorized`;
a failed process is tolerable only for `consume` with a `rejected` result from `operation=authority`
and a nonempty reason. Missing, malformed, contradictory, unavailable, cancelled or incomplete
results fail the invocation, even if its controller job otherwise reports success. Diagnostic states
remain retained where CI permits; cancellation does not guarantee that a terminal upload ran.
An accepted rejection grants no publication permission. `promote` and `resume-publication` still
require authorization; `resume-public` retains its separate read-only public check.

For a selected release with separate publication authorization, first prepare meaningful notes and an
ordinary annotated tag selecting the accepted product commit. Refresh remote main, exact tag/release
occupancy, relevant jobs and enabled immutability. The existing repository control
`LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA` must bind that exact annotated object. Only an explicitly
authorized operator may change this scoped control or create/push the tag. Reconcile its prior owner
and terminal release state, compare the prior value immediately before any authorized write, and
read it back. That read/check/write is not atomic compare-and-swap. Preserve prior immutable objects
and genuine failures; never change immutability, credentials, access or protections to make admission
pass. A tag push alone starts no build or publication.

Preserve Markdown notes explicitly when creating an annotation:

```sh
git tag --annotate --cleanup=verbatim TAG SOURCE_COMMIT --file /absolute/release-notes.md
```

Git's default tag cleanup removes comment lines, including Markdown headings.
Compare the actual local and remote annotation with the intended notes before selecting it.
Complete and read back the scoped selection before dispatch, and check each step's result
before starting its dependent action. A running workflow can retain the prior variable value;
an update after dispatch does not establish that job's publication authority. If that boundary
rejects before transfer, a fresh `promote` invocation with the same accepted producer must still
complete transfer admission. `resume-publication` is for the subsequent publication boundary.

The publisher requires that annotated tag to exist before release creation. The create request
omits `target_commitish`, which GitHub documents as unused for an existing tag. Supplying a frozen
source whose workflow files differ from current main can unnecessarily require workflow-write
permission, which the publication job's token does not have. Release draft target metadata is not
the source authority: admission binds the actual annotation and commit, exact producer ownership
marker, bot author, notes and asset inventory. These checks continue before uploads and publication;
omitting tag-creation metadata neither selects main as the product nor authorizes a missing tag.
See the [GitHub create-release contract](https://docs.github.com/en/rest/releases/releases#create-a-release).

With those prerequisites established, explicitly dispatch:

```sh
gh workflow run release.yml --repo lkjsxc/lkjscript --ref main \
  -f operation=promote -f producer_run=RUN_ID -f producer_attempt=ATTEMPT
```

The publisher receives only the trusted controller and selected bytes. It executes no candidate,
transferred verifier or handoff script, has no Cargo build path, and uses bounded pinned operations.
Only its job receives write permission. It rechecks source/main, annotated object/source/version,
scoped authorization, occupancy and exact bytes before publication. It promotes the already accepted
three assets unchanged and verifies actual immutable state. Attestations establish provenance, not
semantic correctness or permission; supported GitHub release/asset verification checks actual subjects.

A failed publication/API boundary resumes with `operation=resume-publication` and the same exact
producer selectors. A matching immutable release is idempotent. An owned partial draft may receive
only missing matching assets after source/owner/inventory checks. Foreign/conflicting drafts and
published mismatches reject without deletion, replacement or retagging. Fresh authority is always
required. Successful product construction and broad acceptance do not rerun.

A failed public boundary uses `operation=resume-public`, again selecting the original producer.
Public exact acquisition resolves the tag response to its nonzero immutable release ID, then reads
that ID and admits the complete accepted asset inventory. Both responses must identify the same
ordinary published immutable release and selected tag. A tag response may contain only a valid
subset of those assets; conflicting entries still reject. An incomplete or conflicting response
from the release ID rejects before attestations or downloads. This handles an incomplete tag
lookup without replacing public bytes or accepting an incomplete final inventory.
Exact and latest acquisition have distinct identities. Selected publication requires latest to be
this candidate; a moved alias cannot silently pass. A later read-only recheck may record the actual
new latest identity as superseded/not applicable and still check immutable exact content. No different
bytes are executed or certified as the candidate. Reproducible product/installer defects require a
real corrected candidate; transient acquisition failure only repeats its failed boundary.

GitHub reruns preserve the original event SHA/ref. To use a fixed controller from newer main, dispatch
a new consumer instead of assuming a rerun changes workflow source. Producer attempt and consumer
attempt always remain separate. Serialize promotion and public/latest checks through the workflow's
shared publication concurrency group; independent read-only candidate work has separate concurrency.

## Retention and terminal results

Essential candidate assets, verifier and terminal acceptance handoffs request **14 days** through
GitHub artifacts. Selection records actual artifact IDs, digests and service expiry; repository/service
limits may shorten retention. Missing, ambiguous or expired trusted material reports unavailable,
with a new-candidate or separately authorized trusted-recovery action. It never silently rebuilds and
labels the result reuse. Preserve originals needed for a real failure diagnosis; do not create an
unlimited evidence service or execute them with publishing credentials.

Changed product source/version, required configuration, executable/assets, verifier/workload,
environment or trust inputs invalidate affected proof. A later reporting or controller-only revision
receives its own appropriate checks and does not relabel the candidate source. Public rechecking with
immutable public bytes and a separately trusted compatible verifier cannot manufacture lost historical
acceptance; the ordinary selected resume path requires its retained trusted handoffs.

CI's terminal result distinguishes candidate accepted, promotion authorized/rejected, immutable assets
published, and public verification passed/failed/unavailable. Mandatory skipped stages cannot produce
whole-operation success. An upload, green child or main push alone is insufficient. Fixture writes
prove controller behavior, not GitHub publication or live attestation issuance.

Main integration and public closure remain separate. Honor actual protections, integrate independently
main-ready work normally and verify remote ancestry. If hosted acceptance is still pending, report
that release gate as incomplete. At meaningful work boundaries inspect the exact run; if external
completion is the sole dependency, return its observed state, retained identities, missing gate and
next concrete action. Do not start a duplicate run or promise unattended monitoring.

## Historical selections

Completed and superseded release selections are preserved in the
[immutable prior procedure](https://github.com/lkjsxc/lkjscript/blob/d9a6ba5e7a8ab6de94a3ee5f5b54e526c42e2ab3/docs/release.md)
and its original evidence links. Consult an exact predecessor only when recovering
that release or investigating a relevant failure. Current work needs no historical
publication reconstruction. The current public binary and active successor belong
to [status](status.md).

### Withheld v0.1.49 and the v0.1.50 successor

[Original selection and resolution](https://github.com/lkjsxc/lkjscript/blob/d9a6ba5e7a8ab6de94a3ee5f5b54e526c42e2ab3/docs/release.md#withheld-v0149-and-the-v0150-successor).

### Superseded v0.1.61 candidate

[Original failed producer](https://github.com/lkjsxc/lkjscript/blob/d9a6ba5e7a8ab6de94a3ee5f5b54e526c42e2ab3/docs/release.md#superseded-v0161-candidate).

### Superseded consolidated 0.1.63 candidate

[Original selection and diagnosis](https://github.com/lkjsxc/lkjscript/blob/d9a6ba5e7a8ab6de94a3ee5f5b54e526c42e2ab3/docs/release.md#superseded-consolidated-0163-candidate).

### Selected consolidated 0.1.64 successor

[Original selection](https://github.com/lkjsxc/lkjscript/blob/d9a6ba5e7a8ab6de94a3ee5f5b54e526c42e2ab3/docs/release.md#selected-consolidated-0164-successor);
[completed publication](campaigns/20261002-owned-choices.md#completed-v0164-publication).
