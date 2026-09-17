# Release procedure

GitHub Releases are the public binary distribution path. Accept one finalized candidate, nominate
its exact producer run and attempt, and promote its unchanged archive, checksum and bootstrap.
Content identity, behavioral acceptance and publication authority are separate decisions.

Immutable [v0.1.38](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.38) already publishes source
`7083f9a6d56ed702017942e100c3696fc6f35308`. Publisher
[35117655769/1](https://github.com/lkjsxc/lkjscript/actions/runs/35117655769) passed all four hosted
jobs, including anonymous exact/latest installed application acceptance and attestations. Its separate
later manual original-reader closure was not performed by the current campaign. That historical gap
does not make the release unavailable or create a recurring reconstruction obligation. Genuine
v0.1.36/v0.1.37 failures remain in the [delivery history](campaigns/202609151412.md) and
[numerical milestone ending](campaigns/202609162154.md). The
[former procedure](https://github.com/lkjsxc/lkjscript/blob/108ca2777fc202b543f48816a99b7db519542f97/docs/release.md)
describes those frozen producers; the procedure below governs new candidates only.

The [cutover campaign](campaigns/202609180007.md) selects non-publishing candidate and read-only
consumer proof. It authorizes no new version, tag, release or release-control write. First public use
of the new encoding follows the next selected useful public capability, important product fix or
explicit release request. Choose that future version/source before candidate creation. A private
candidate built with occupied version 0.1.38 is never the public v0.1.38 release.

## Content and compatibility

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
  --output /absolute/new/assets --tag vMAJOR.MINOR.PATCH
"$VERIFIER" release verifier prepare --executable "$VERIFIER" \
  --output /absolute/new/verifier --tag vMAJOR.MINOR.PATCH --commit EXACT_PRODUCT_SHA
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

Select an exact producer, including its original attempt, with a separate read-only invocation:

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

For a future release with separate publication authorization, first prepare meaningful notes and an
ordinary annotated tag selecting the accepted product commit. Refresh remote main, exact tag/release
occupancy, relevant jobs and enabled immutability. The existing repository control
`LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA` must bind that exact annotated object. Only an explicitly
authorized operator may change this scoped control or create/push the tag. Reconcile its prior owner
and terminal release state, compare the prior value immediately before any authorized write, and
read it back. That read/check/write is not atomic compare-and-swap. Preserve prior immutable objects
and genuine failures; never change immutability, credentials, access or protections to make admission
pass. A tag push alone starts no build or publication.

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
that campaign gate as incomplete. At meaningful work boundaries inspect the exact run; if external
completion is the sole dependency, return its observed state, retained identities, missing gate and
next concrete action. Do not start a duplicate run or promise unattended monitoring.
