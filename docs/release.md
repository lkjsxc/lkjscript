# Release procedure

Authenticate accepted source and exact producer run/attempt; promote finalized
archive/checksum/bootstrap unchanged. Content, behavior and authority differ.
[Status](status.md) owns availability; root Cargo owns opaque vA.B.C identity.
Preserve immutable publications/outcomes. Before freeze, native-check maintained
projects and prove tracked-source cold reproduction of required packs; missing
inputs/source drift cannot pass through exclusions or raised limits.

## Coverage and admission

[Verification](spec/verification.md) owns all gates, inventories and original-reader
requirements: standalone full is 29 gates; release is fresh 23-gate release-source
plus exact-final-byte six owners/public harness/static ELF/two userlands/installation.
Transfer and anonymous exact/latest require small installed lifecycles without broad
replay or rebuild. Every mandatory case passes once; uploads/skipped stages prove no
terminal. [Bootstrap](../tools/bootstrap-verifier.sh) retains immutable tools;
--with-controller includes controller. Honor producer dependencies/output copies
and separate Cargo/checker concurrency. release target owns pinned musl policy;
static linkage grants no hostile-code sandbox.

## Build and accept a candidate

Inspect/reuse healthy work before Release workflow_dispatch on main. Tag/PR/workflow-run
events trigger no automatic build/publication. Record exact event source and attempt.

```sh
gh workflow run release.yml --repo lkjsxc/lkjscript --ref main -f operation=candidate
gh run view --repo lkjsxc/lkjscript RUN_ID --json headSha,status,conclusion,jobs
gh api repos/lkjsxc/lkjscript/actions/runs/RUN_ID --jq '{id,run_attempt,head_sha,status,conclusion}'
```

For local execution, use a standalone clean committed clone with a real `.git`
directory, complete history, canonical origin and config-free ancestry/Cargo home.
Bootstrap in that clone: a check command remains bound to its compiled checkout.
Retain the exact 0755 verifier handoff before changed/full/source checks, preserving
the original 0555 bootstrap executable and evidence. Source admission later compares
verifier bytes/mode and the approved environment, including `CARGO_TARGET_DIR`.
Use absent owned outputs and keep source, environment and tools stable:

```sh
BOOTSTRAP_VERIFIER=/absolute/immutable/lkjscript-dev
SOURCE_SHA=$(git rev-parse HEAD)
TAG=vA.B.C
"$BOOTSTRAP_VERIFIER" release verifier prepare --executable "$BOOTSTRAP_VERIFIER" \
  --output /absolute/new/verifier --tag "$TAG" --commit "$SOURCE_SHA"
VERIFIER=/absolute/new/verifier/lkjscript-dev
CARGO_TARGET_DIR="$PWD/target" "$VERIFIER" check release-source --fresh --jobs 1 --machine \
  > /absolute/new/source-summary.json
SOURCE_RECEIPT="$PWD/$(jq -er '.receipt' /absolute/new/source-summary.json)"
env -u CARGO_TARGET_DIR "$VERIFIER" release build \
  --output /absolute/new/lkjscript --receipt /absolute/new/build.json
"$VERIFIER" release prepare --candidate /absolute/new/lkjscript \
  --cargo-about /absolute/pinned/cargo-about \
  --cargo-about-archive /absolute/pinned/cargo-about.tar.gz \
  --output /absolute/new/assets --tag "$TAG"
CARGO_TARGET_DIR="$PWD/target" "$VERIFIER" release candidate accept \
  --assets /absolute/new/assets --source-receipt "$SOURCE_RECEIPT" \
  --build-receipt /absolute/new/build.json \
  --verifier-identity /absolute/new/verifier/verifier-identity.json \
  --evidence-root /absolute/new/acceptance --output /absolute/new/release-receipt.json
```

Fixed-target construction rejects unbound overrides and checks pinned cargo-about.
Prepare repeats notices/finalizes packaging; constructed is not accepted. Accept
extracts unchanged bytes, compares packaging without rebuild, runs owners/readers
and joins cleanup before candidate_accepted. Keep failures; never rewrite bytes to
pass. Local proof grants no hosted provenance/publication authority; fabricated
GitHub context cannot supply it. Release separately authenticates mainline source,
controller and actual run/attempt.

Exactly lkjscript-x86_64-unknown-linux-musl.tar.gz, SHA256SUMS and install.sh are
published. Native readers enforce exact ordered inventory/modes/hashes/ELF/identity/
slots, rejecting unsafe/extra/ambiguous/link/traversal/conflicting content. Content
binds source/build/license, never permission/tag object/consumer attempt. Bootstrap
binds immutable URL/length/hash and runs extracted manager without Cargo/Python/source.
Installation grants no authority, migration or implicit runtime fallback.

## Select, promote and resume

Optional read-only consume selects the original run/attempt:

```sh
gh workflow run release.yml --repo lkjsxc/lkjscript --ref main \
  -f operation=consume -f producer_run=RUN_ID -f producer_attempt=ATTEMPT
```

Current integrated controller independently authenticates repository/head repository,
event/workflow/source/run/attempt/mandatory jobs, terminal identities, artifact IDs,
service ZIP digests/inventories and exact asset/verifier bytes. Product must reach
freshly resolved main; controller source is separate. Names/hashes/self-declared
passed supply no authority. Transfer requires joined small lifecycles. Only consume
may finish with classified rejected authority; malformed/contradictory/incomplete
results cannot pass. Rejection grants no publication permission.

Prepare notes and annotated tag on accepted product source:
`git tag --annotate --cleanup=verbatim TAG SOURCE_COMMIT --file /absolute/notes.md`.
Compare local/remote annotation to intended Markdown. Refresh main/protections,
jobs/occupancy/immutability. Existing explicit operator authority is required to
push tags or bind LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA to that exact object.
Reconcile prior owner/value, compare before write/read back before dispatch; this
is not atomic CAS, and running jobs may retain earlier values. Never alter access,
credentials, protections or immutable history to pass. Then dispatch:

```sh
gh workflow run release.yml --repo lkjsxc/lkjscript --ref main \
  -f operation=promote -f producer_run=RUN_ID -f producer_attempt=ATTEMPT
```

Publisher independently checks tag/source/version, scoped authority, ownership/bot,
notes/inventory/occupancy and exact bytes before upload/publication. Existing-tag
creation omits target_commitish. Publisher alone receives write permission; it runs
no candidate/verifier/handoff, builds nothing and verifies unchanged immutable
publication. Attestations prove provenance, never behavior or permission.

resume-publication retries only failed publication/API with same producer/fresh
authority. Matching immutable publication is idempotent; owned partial drafts may
receive only missing matching assets. Foreign/conflicting/published mismatches
reject without deletion/replacement/retagging. Pre-transfer failures need fresh promote.
resume-public retries only public acquisition/check. Exact tag resolves nonzero
immutable release ID; independent ID lookup admits complete inventory. Conflicting
tag/incomplete ID responses reject. Selected publication requires candidate as latest;
later exact recheck may label independently superseded latest inapplicable, never
execute other bytes as this candidate. Defects require corrected candidates;
transient acquisition repeats only its boundary.

Reruns preserve event source/ref: newer fixed controllers require new consumers.
Separate producer/consumer attempts; serialize promotion/public-latest through shared
workflow concurrency. Candidate jobs have independent concurrency.

## Retention and terminal results

Request 14-day asset/verifier/handoff retention; record actual IDs/digests/expiry.
Missing/ambiguous/expired originals are unavailable: obtain new candidate or explicit
trusted recovery, never silent rebuild/relabelled reuse. Preserve failures; execute
no transferred bytes with publishing credentials. Changed source/version/config/bytes/
verifier/workload/environment/trust invalidates affected proof. Descendants receive
scoped checks; public rechecks cannot manufacture lost acceptance.

Candidate/authority/publication/public-check terminals remain distinct. Integrate and
verify remote ancestry separately from release waiting. External waits report exact
source/run/attempt/gate/state/originals/action and reuse healthy jobs, without promises
of unattended monitoring. Git/campaigns retain history, not recurring task diaries.
