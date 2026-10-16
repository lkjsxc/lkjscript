# Release procedure

Authenticate accepted source and the exact producer run/attempt. Promote finalized
archive/checksum/bootstrap unchanged. Content, behavior and publication authority
are separate claims. [Status](status.md) owns availability; root Cargo owns product
identity. Preserve immutable publications and unrelated resources.

## Source and candidate acceptance

Use the primary clean committed checkout and its single `$PWD/target`. Bootstrap
and checks lease shared outputs; do not create another clone or Cargo target.
Before packaging, check maintained projects and reproduce required assets through
supported product owners. Missing inputs and source drift cannot pass by exclusion.

[Verification](spec/verification.md) owns coverage: full has 29 gates; release needs
25 fresh source gates and final acceptance of six target owners, seven native
smokes, ELF policy, two pinned userlands and installation recovery. Complete source
semantic outcomes and actual final processes remain distinct. A retained fresh
full receipt can supply the required source subset under identical context; re-admit
all original gates without rewriting their profile or bytes.

Default compact check receipts are insufficient for transfer. Explicitly request
`--retain-archive` for source acceptance. The release verifier handoff is a deliberate
publication boundary, not a per-gate executable copy. Keep selected source,
environment and verifier stable through acceptance. Set Cargo/checker concurrency
independently and join readers before any shared-target writer.

```sh
verifier_root=$(bash tools/bootstrap-verifier.sh --with-controller --retain-archive)
BOOTSTRAP_VERIFIER="$verifier_root/lkjscript-dev"
SOURCE_SHA=$(git rev-parse HEAD)
TAG=vA.B.C
"$BOOTSTRAP_VERIFIER" release verifier prepare --executable "$BOOTSTRAP_VERIFIER" \
  --output /absolute/new/verifier --tag "$TAG" --commit "$SOURCE_SHA"
VERIFIER=/absolute/new/verifier/lkjscript-dev
"$VERIFIER" check release-source --fresh --retain-archive --machine \
  > /absolute/new/source-summary.json
SOURCE_RECEIPT="$PWD/$(jq -er '.receipt' /absolute/new/source-summary.json)"
"$VERIFIER" release build \
  --output /absolute/new/lkjscript --receipt /absolute/new/build.json
"$VERIFIER" release prepare --candidate /absolute/new/lkjscript \
  --cargo-about /absolute/pinned/cargo-about \
  --cargo-about-archive /absolute/pinned/cargo-about.tar.gz \
  --output /absolute/new/assets --tag "$TAG"
"$VERIFIER" release candidate accept \
  --assets /absolute/new/assets --source-receipt "$SOURCE_RECEIPT" \
  --build-receipt /absolute/new/build.json \
  --verifier-identity /absolute/new/verifier/verifier-identity.json \
  --evidence-root /absolute/new/acceptance --output /absolute/new/release-receipt.json
```

Use absent owned output paths. Fixed-target construction rejects unsupported
configuration and checks pinned cargo-about. Preparation finalizes notices and
packaging; construction alone is not acceptance. Acceptance extracts unchanged
bytes, independently checks packaging, runs boundary owners and joins cleanup.
Source semantics cannot replace final processes. Keep explicit handoff originals
until their consumer completes; delete incidental scratch after joined verification.

Exactly `lkjscript-x86_64-unknown-linux-musl.tar.gz`, `SHA256SUMS` and `install.sh`
are published. Readers enforce complete ordered inventories, modes, hashes, ELF and
identity. Reject unsafe links, traversal, extra/ambiguous/conflicting content.
Bootstrap binds immutable URL, length and hash and runs the extracted manager
without Cargo, Python or source. Static linkage grants no hostile-code sandbox;
installation grants no authority, migration or implicit runtime fallback.

## Hosted workflow and authority

Inspect and reuse healthy jobs before dispatch. Tags, PRs and workflow-run events
do not automatically build or publish. Record exact event source and attempt.

```sh
gh workflow run release.yml --repo lkjsxc/lkjscript --ref main -f operation=candidate
gh run view --repo lkjsxc/lkjscript RUN_ID --json headSha,status,conclusion,jobs
gh api repos/lkjsxc/lkjscript/actions/runs/RUN_ID --jq '{id,run_attempt,head_sha,status,conclusion}'
```

Optional read-only consume selects `operation=consume`, `producer_run=RUN_ID` and
`producer_attempt=ATTEMPT`. The integrated controller independently authenticates
repository/head repository, event/workflow/source/run/attempt, mandatory jobs,
terminal identities, artifact IDs, service ZIP digests and exact verifier/assets.
Product source must reach current main; controller source is separately bound.
Names, hashes and self-declared success grant no authority. Transfer includes joined
small installed lifecycles. Malformed, contradictory or incomplete results reject.
Only consume may terminate with a classified authority rejection.

Local proof cannot supply hosted provenance. Never fabricate GitHub context or
execute a candidate/verifier with publication credentials. Publisher alone receives
write authority and runs no candidate, verifier or build.

Prepare release notes and an annotated tag on accepted product source. Compare
intended annotation and local/remote identity. Refresh main, branch protections,
active jobs, occupancy and immutability before promotion. Explicit owner authority
is required for tag publication or binding the immutable tag-object variable; this
must not alter credentials, access or protections. Reconcile/read back any authorized
binding and account for jobs retaining earlier values. Then dispatch `operation=promote`
with the same producer run and attempt.

Publisher rechecks tag/source/version, scoped authority, ownership, notes, complete
inventory and exact bytes before publishing. Existing-tag creation omits
`target_commitish`. Existing immutable content is never replaced, deleted or retagged.
Attestations establish provenance, not behavior or permission.

## Recovery, retention and terminals

`resume-publication` retries only publication/API failure against the same producer
and fresh authority. A matching publication is idempotent. An owned partial draft
may receive only missing matching assets; foreign or conflicting content rejects.
Pre-transfer failures require a new promote attempt.

`resume-public` retries acquisition/check only. Exact tag resolves a nonzero immutable
release ID; independent ID lookup must admit the complete inventory. Conflicting tag
or incomplete responses reject. Selected publication requires the candidate as
latest. Later rechecks may classify a superseded latest check as inapplicable; never
execute different bytes as the selected candidate. Defects need corrected candidates.

Request 14-day retention for explicit assets/verifier/handoff, recording actual IDs,
digests and expiry. Missing/ambiguous/expired originals are unavailable; obtain a new
candidate or explicitly trusted recovery, never silently rebuild or relabel reuse.
Changes to source, bytes, configuration, verifier, workloads, environment or trust
invalidate affected evidence. Descendants receive scoped checks; public rechecks
cannot manufacture lost acceptance.

Reruns preserve original event source/ref, so new controllers need new consumer
runs. Keep producer and consumer attempts distinct. Serialize promotion/public-latest
through workflow concurrency; candidate jobs have independent concurrency.
Candidate, authority, publication and public-check terminals remain distinct.
Integrate and independently verify remote ancestry separately from release waiting.
Report exact source/run/attempt/gate/state and next action at external waits.
