# Release procedure

GitHub Releases are the sole public binary distribution path. A release is derived from one exact
source commit. Its tag, target, candidate, archive, manifest, checksum, asset digest, attestation,
and verification receipts are distribution identities and evidence; none can select or edit
accepted program meaning.

Immutable `v0.1.32`, release `387465958`, is the last verified supported release, from source
`67baaf0b081842e0e2e3745e8d5503e22cc791e4`. Dry run `34626667719/1` and tag run
`34671245722/1` passed. Both anonymous public routes independently acquired and admitted the
candidate and ran their small lifecycles; one fresh suite was bound within that pair. EFF-7 and
the first hosted pair are closed in the [iteration reconciliation](campaigns/202609121214.md).

The [foreground campaign](campaigns/202609121842.md) selects one combined task-iteration and
foreground-command release. Current offline-package receipt 8 requires the foreground child
at source, target, transferred and public acceptance. Existing exact-source dry-run, annotated-tag,
immutability control and public-pair gates still apply. A descriptor may omit command execution
and runtime policy only on a matching foreground-capable executable; retain numeric descriptors
and the previous executable with immutable bundles for recovery. No operational-data migration
is part of publication.

Immutable `v0.1.31`, release `386791231`, remains a retained recursive-data predecessor from
`7ea9c99a31ecd0e5317129d2bd81ab4f8f51e02e`, with its original readers and assets unchanged.

The previous immutable `v0.1.30`, release `386193815`, remains preserved. Annotated tag
`b45b3f3dc0ab5e52f28c7dd0910f8f1faa3d1e5c` selects source
`35269961fc66fc308785913190a10843ee175aaa`. Dry run `34454845131` and tag run `34461031325`
passed on attempt 1. Each ran 26 fresh source gates, static inspection, both pinned userlands,
all six target oracles, strict packaging and all five checkout-free transferred owners. Only the
tag run published. Its anonymous exact-version and latest downloads independently passed integrity,
release/asset attestations, strict extraction and all five behavioral owners before byte comparison.
Exact identities and bounded proof are in the
[public nominal evidence](evidence/202609101130-parametric-nominal-public-milestone.json).

This release distributes the implemented parametric nominal-data foundation. Immutable v0.1.29
passed exact-download behavior but failed latest acceptance when verifier process observation raced
with task exit; its same-input recovery failed on the same error. A narrow verifier correction
required additive v0.1.30 and renewed proof. Both v0.1.29 and every earlier tag and asset remain
unchanged. The campaign made two authorized writes to the existing release-control variable,
one for each annotated tag, with enabled-immutability administrator reads and prior-value checks.
The immutable-release setting itself was not changed; read/check/write is not atomic.

Upgrading the executable does not migrate repositories, artifacts or operational data. Older
incompatible Graph 12/Artifact 16 inputs reject without an automatic converter. Retain backups/exports
and the matching predecessor executable for unsupported inputs. Operational data is not disposable
derived state, and unchanged operational formats do not guarantee universal application compatibility.
No deployment or live-data migration was performed. Later reporting commits do not replace the tagged
release-source identity. Historical release evidence remains retained in its original owners.

## Identity and authority

The root `Cargo.toml` package version owns the human-facing release snapshot and its exact annotated
`vMAJOR.MINOR.PATCH` tag. It is the only version presented by current public product metadata.
Internal storage, compiler, artifact, deployment, runtime, adapter, repository, and contributor-tool
compatibility identities remain independently owned as described by
[the release and contract version decision](decisions/20260829-release-contract-version-authority.md).

`lkjscript-dev release target` is the sole executable owner of the current release target triple,
archive name, static-linkage policy, native build inputs, and pinned test userlands. The current
policy selects one asset:

```text
lkjscript-x86_64-unknown-linux-musl.tar.gz
```

Workflow shell consumes the emitted policy; it does not maintain a second target catalog. Another
architecture, operating system, or dynamic compatibility asset requires separate admission and is
not part of the current release matrix.

Before release work, fetch and inspect remote state without rewriting it:

```sh
git fetch --prune origin
git status --short
git rev-parse HEAD origin/main
git tag --list --sort=version:refname
gh release list --repo lkjsxc/lkjscript
gh api repos/lkjsxc/lkjscript/immutable-releases
gh run list --repo lkjsxc/lkjscript --workflow Release --limit 20
```

The immutable-release setting must report `enabled: true`. GitHub's settings endpoint requires
repository administration authority, which is deliberately unavailable to the publication job.
Immediately before tag push, an administrator binds the observed setting to the exact annotated
tag-object SHA in the non-secret repository variable
`LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA`. The isolated publish job checks that value and uses
only its ephemeral workflow token.

## Pinned build and verification inputs

`rust-toolchain.toml` pins Rust/Cargo 1.98.0, rustfmt, clippy, and the musl x86-64 Rust target. The
typed target policy pins the Ubuntu musl compiler packages by exact version, URL, and SHA-256 and
pins one Linux/amd64 musl userland and one Linux/amd64 older-glibc userland by platform-manifest
digest. `cargo-about` 0.9.2 remains independently pinned by its downloaded archive
and executable SHA-256 in the first-party release owner and workflow.

The first-party data cutover has a separate contributor-only PostgreSQL 16.15 differential/resource
receipt. It must be fresh for that source campaign, but PostgreSQL is deliberately absent from the
product dependency graph, target policy, service gate, transferred verifier, release handoff, and
publication/anonymous-download jobs.

Inspect the canonical policy before installing its exact native inputs:

```sh
cargo build --release --locked -p lkjscript-dev
target/release/lkjscript-dev release target
```

The host verifier is a normal host executable. The product candidate is built separately through
the repository-owned target command, which records Cargo/rustc/musl compiler identities, the exact
command, process resource observation, source commit, target-policy digest, candidate mode, bytes,
SHA-256, and static ELF inspection:

```sh
evidence_parent=/absolute/private/evidence-parent
mkdir -m 0700 "$evidence_parent"
target/release/lkjscript-dev release build \
  --output "$evidence_parent/lkjscript" \
  --receipt "$evidence_parent/build-receipt.json"
target/release/lkjscript-dev release admit \
  --candidate "$evidence_parent/lkjscript" \
  --build-receipt "$evidence_parent/build-receipt.json" \
  --evidence-root "$evidence_parent/target-admission"
```

Target admission independently parses the exact candidate as ELF64 little-endian x86-64. It rejects
an interpreter program header, any runtime `DT_NEEDED` entry, a GLIBC version requirement, a foreign
machine, malformed or trailing linkage input, and a target-policy mismatch. It then runs the complete
copied-binary command lifecycle with network unavailable during candidate execution in both pinned
userlands. Finally it runs the exact candidate through the maintained distributed HTTP, transferred
stateful HTTP, transferred outbound HTTP, transferred offline-package composition, transferred
pure-tail execution, and standalone
service oracles. The offline-package oracle reconstructs retained source containers independently,
checks the fixed 11/12 diamond and standalone HTTP body without producer directories, and binds
its complete command/file inventories to target-admission receipt 4. Offline receipt 6 additionally
requires the finite-recursive tree library and consumers, complete shape/ordered results, exact
dependency replacement, public recursive sessions, restarted typed storage and fault recovery.
It also requires the effect-parametric task library, pure bound factory, complete ordered
configuration/data results, activation/grant and stopping observations, transaction/restart recovery
and exact dependency/artifact replacement. The public pair strictly admits both routes and executes
each small command lifecycle; its exact route freshly executes the full five-owner suite once and
latest binds that within-pair evidence after equality checks. It does not fabricate fresh latest
child receipts or repeat the full suite for latest.
The pure-tail oracle binds
the exact candidate's long copied-public executions, bounded-stack resource probes, and isolated
transactional HTTP success, rollback, cancellation, and cleanup. The outbound oracle uses
only isolated loopback HTTP/TLS/DNS fixtures and contacts no live relay. Required unavailable,
stale, foreign, reused, skipped,
failed, or unrun evidence cannot produce a passing target-admission receipt.

The userland observations establish only the named tested userland boundary. Static linkage does not
prove compatibility with every Linux kernel, CPU, container runtime, filesystem, or host policy.

## Fresh source proof and deterministic package

After every implementation, workflow, normative, generated, target, or release-procedure change is
committed, run one fresh source profile and rebuild/re-admit the exact candidate from that commit:

```sh
cargo run --release --locked -p lkjscript-dev -- check full --fresh --machine
target/release/lkjscript-dev release build \
  --output /absolute/absent/path/lkjscript \
  --receipt /absolute/absent/path/build-receipt.json
target/release/lkjscript-dev release admit \
  --candidate /absolute/path/lkjscript \
  --build-receipt /absolute/path/build-receipt.json \
  --evidence-root /absolute/absent/path/target-admission
```

The full-check driver uses Cargo's release profile so its frozen executable fits the existing
verifier byte bound. This does not change the full profile's selected gates, their Cargo/test
options, freshness policy, or required outcomes.

Prepare the release with both receipts:

```sh
product_version=$(cargo metadata --locked --no-deps --format-version 1 |
  jq -er '.packages[] | select(.name == "lkjscript") | .version')
release_tag="v$product_version"
target/release/lkjscript-dev release prepare \
  --candidate /absolute/path/lkjscript \
  --cargo-about /absolute/path/cargo-about \
  --cargo-about-archive /absolute/path/cargo-about.tar.gz \
  --output /absolute/absent/path/release-output \
  --tag "$release_tag" \
  --publication dry-run \
  --full-verification-receipt /absolute/path/full/receipt.json \
  --target-admission-receipt /absolute/path/target-admission/receipt.json \
  --require-full-verification
target/release/lkjscript-dev release verify \
  --archive /absolute/path/release-output/lkjscript-x86_64-unknown-linux-musl.tar.gz \
  --checksums /absolute/path/release-output/SHA256SUMS \
  --receipt /absolute/path/release-output/release-receipt.json \
  --extract-to /absolute/absent/path/verified-release \
  --expected-tag "$release_tag" \
  --expected-publication dry-run
```

Preparation generates target-filtered third-party notices twice from the locked offline production
closure, creates two archives, and requires byte equality. The archive inventory is exactly one
`lkjscript/` directory containing the executable, root license, generated third-party notices, and
canonical release manifest. `SHA256SUMS` contains exactly the one archive entry. Strict verification
rejects nonregular inputs, links, traversal, duplicates, extras, incorrect order/mode/timestamp,
noncanonical or predecessor metadata, target/linkage contradiction, checksum corruption, extraction
conflict, and candidate mismatch. `release verify --extract-to` makes the validated directory visible
through one create-new boundary, so workflow shell never owns archive parsing.

Current public release metadata binds the product name and version, source, target policy,
executable bytes and ELF facts, opaque capabilities digest, toolchain, locked closure, notices, and
deterministic packaging. It contains no separate format or subsystem version. The private release
receipt additionally binds fresh source and target-admission evidence; that contributor evidence is
not shipped as public product metadata.

## Hosted dry run

The `Release` workflow runs on explicit `ubuntu-24.04`. Its read-only checkout job builds the host
verifier and exact musl candidate separately, runs fresh full and target admission, prepares the
deterministic package, and uploads a three-file release handoff plus a two-file application-verifier
handoff. The latter is a typed private handoff that binds the exact verifier bytes, tag, source
commit, mode, and the roles in private handoff version 4 release-verify, distributed-http, outbound-http,
offline-packages, pure-tail, and stateful-http.

A second read-only job has no checkout. It downloads both handoffs by artifact ID and digest, verifies
the verifier before restoring its executable mode, safely extracts and re-inspects the candidate,
and runs all five transferred behavioral owners through `release transferred run`. Stateful verification uses only an explicit absolute
create-new evidence root and an isolated first-party data store; it provisions no database server or
container. Outbound verification uses a separate create-new root and deterministic local raw
HTTP/TLS fixtures. All five child receipts must pass their complete current typed readers before the publication job
can run.

Dispatch a dry run against the final source commit:

```sh
gh workflow run Release --repo lkjsxc/lkjscript --ref main \
  -f publish=false -f tag="$release_tag"
gh run view --repo lkjsxc/lkjscript RUN_ID --json headSha,status,conclusion,attempt,jobs
gh run download --repo lkjsxc/lkjscript RUN_ID \
  --name release-handoff-RUN_ID-RUN_ATTEMPT \
  --dir /absolute/absent/path/hosted-handoff
gh run download --repo lkjsxc/lkjscript RUN_ID \
  --name pre-publication-application-evidence-RUN_ID-RUN_ATTEMPT \
  --dir /absolute/absent/path/hosted-application-evidence
```

Inspect matching runs before a selected dispatch. Observe once initially and once at a meaningful
work boundary; another read needs a changed state or a specific diagnostic question. Complete
independent work while the run executes. If only external waiting remains, hand off the exact
commit, run/attempt, observed state, remaining required checks and the next `gh run view` action.
Do not watch indefinitely, poll, fetch unchanged logs repeatedly or merge around pending protection.
Download artifacts only after completion, and retain the explicit failing artifact-ID/digest checks.

The dry run must freshly pass build, full, all six named target oracles, package, and the
five-owner transferred operation. Its publish and post-release jobs must be
skipped, and no tag, draft,
release, or public asset may be created. Evidence from another commit, workflow, target policy,
candidate, verifier, image, or run attempt is stale.

The bounded transferred operation runs from the already verified two-file verifier handoff. It
requires the extracted executable and canonical manifest from `release verify --extract-to`, the
expected source/tag/publication context, and externally supplied verifier bytes/hash:

```sh
/absolute/verifier-handoff/lkjscript-dev release transferred run \
  --candidate /absolute/verified-release/lkjscript \
  --manifest /absolute/verified-release/RELEASE-MANIFEST.json \
  --tag "$release_tag" --commit "$release_source" --publication dry-run \
  --boundary pre-publication --evidence-root /absolute/absent/transferred-evidence \
  --verifier-identity /absolute/verifier-handoff/verifier-identity.json \
  --expected-verifier-sha256 "$verifier_sha256" --expected-verifier-bytes "$verifier_bytes"
```

`release transferred verify` takes the same arguments to re-read existing evidence without rerunning
behavior. It cannot certify a different root, boundary, source, candidate, or verifier. Retries use
new roots and retain failed attempts. The aggregate, all child receipts, and bounded command/output
files are retained; incomplete state is published before invocation. Neither a child status flag nor
obsolete receipt generation suffices. `release admission-verify` and preparation likewise re-read
each named target receipt through its existing owner.

## Exact tag and immutable publication

After the exact release source is clean, normally pushed, reachable from `origin/main`, and its
hosted dry run is fresh, recheck that the intended tag and release are unused and no relevant run
is active. Later accepted development or reporting commits may follow that source. Use an isolated
clone with its own `.git` directory when necessary; select the accepted release source as HEAD
without rewinding main. Create and push only the annotated tag:

```sh
git fetch --prune origin
git merge-base --is-ancestor HEAD origin/main
git status --short
release_tag="v$(cargo metadata --locked --no-deps --format-version 1 |
  jq -er '.packages[] | select(.name == "lkjscript") | .version')"
test -z "$(git ls-remote --tags origin "refs/tags/$release_tag" "refs/tags/$release_tag^{}")"
gh release view "$release_tag" --repo lkjsxc/lkjscript && exit 1 || true
git tag -a "$release_tag" -m "lkjscript $release_tag"
test "$(git cat-file -t "refs/tags/$release_tag")" = tag
tag_object_sha=$(git rev-parse "refs/tags/$release_tag")
test "$(git rev-parse "refs/tags/$release_tag^{}")" = "$(git rev-parse HEAD)"
test "$(gh api repos/lkjsxc/lkjscript/immutable-releases --jq '.enabled')" = true
gh variable set LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA \
  --repo lkjsxc/lkjscript --body "$tag_object_sha"
test "$(gh variable list --repo lkjsxc/lkjscript --json name,value \
  --jq '.[] | select(.name == "LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA") | .value')" \
  = "$tag_object_sha"
git push origin "refs/tags/$release_tag"
```

Before writing the variable, record its exact prior value and reconcile it to a completed release;
exclude queued, in-progress, waiting, requested, pending, or foreign publishers. Compare the prior
value again immediately before the write. This read/check/write is not atomic compare-and-swap.
Before tag push, restore the prior value only if the field still contains this attempt's tag object
and no tag/run has acquired it. After tag push retain the binding for the owning run. A release-only
grant does not authorize changing immutability or any other settings.

The tag push owns publication. The only `contents: write` job receives the verified release handoff,
performs no checkout, and executes no repository binary or script. It checks the remote annotated tag
and administrator binding, creates or resumes only the exact draft, uploads only missing exact assets
without clobber, verifies both GitHub asset digests, and publishes immutable latest state. Do not
manually create a parallel release.

## Anonymous public acceptance

The post-release job anonymously downloads separate exact-tag and `releases/latest` archive/checksum
pairs and independently accepts their release/asset authenticity and asset identities. It then invokes
one `release transferred pair-run` in mandatory release mode. Rust independently admits each strict
archive and static executable into a create-new extraction, compares complete archive/checksum/
manifest/executable bytes and admitted modes, and runs the documented capabilities/new/status/check/
build/run lifecycle through each extraction with independently expected typed text `"hello"`.
A final status confirms the accepted revision remains healthy. Each route has a private project and
supervised cleanup. The pair then freshly executes the existing five-owner suite once against exact,
with boundary `exact-download`. Latest binds that actual aggregate within this invocation; it does
not receive a fabricated fresh five-child receipt. Its small lifecycle is separately fresh.

```sh
/absolute/verifier/lkjscript-dev release transferred pair-run \
  --exact-assets /absolute/exact-assets --latest-assets /absolute/latest-assets \
  --tag "$release_tag" --commit "$release_source" --publication release \
  --evidence-root /absolute/absent/pair \
  --verifier-identity /absolute/verifier/verifier-identity.json \
  --expected-verifier-sha256 "$verifier_sha256" --expected-verifier-bytes "$verifier_bytes"
```

`pair-verify` accepts the same arguments and strictly rereads the pair, both admissions, lifecycles
and every actual child receipt without executing applications. Input directories must be separate
canonical absolute directories containing exactly the target-owned archive and `SHA256SUMS`;
symlinks, aliases, hard-linked inputs and pre-existing output roots reject. Both routes, the verifier
and its handoff are reobserved around each expensive child and before terminal acceptance. Failed,
cancelled, unavailable, output-exhausted, changed or unclean attempts remain non-passing; retries
use new roots. Evidence retains route receipts/logs and the small built artifacts required by their
reader, plus the one actual aggregate and all child records. Generated projects are removed.

`--publication dry-run` is explicitly a **local pair rehearsal**. Prepare from a clean source commit
using fresh full and exact target-admission receipts, the unchanged product-version tag string and
the pinned notice tool; create a matching verifier handoff and two independent local input copies.
The pair's real child aggregate uses `pre-publication` for dry-run manifests. This requires no new
tag or release and proves no anonymous acquisition or GitHub authenticity. Release and rehearsal
evidence cannot substitute for each other. Source-full, six-owner target and independent fresh
pre-publication acceptance remain mandatory for a selected publication. The contributor command
owns admission/execution/equality/cleanup; the hosted workflow owns public acquisition/authenticity
and successful overall completion. The [decision](decisions/20260911-bound-public-pair.md) records
the policy change and its trusted-runner limits.

For owner development, run the release/process Cargo filters and list exact ignored test names.
With genuinely prepared inputs under `LKJSCRIPT_PAIR_REHEARSAL_ROOT` (`verifier`, `exact-assets`,
`latest-assets` and the actual preparation `release-receipt.json`), explicitly run
`release::transferred::pair::tests::live_pair_interruptions_preserve_failures_before_fresh_recovery`
with `--ignored --exact`, then perform the genuine `pair-run` into `pair`. Explicitly run
`release::transferred::pair::tests::live_pair_receipt_fault_matrix` with
`LKJSCRIPT_PAIR_FIXTURE_ROOT` naming that completed pair and
`release::transferred::fault_tests::live_receipt_fault_matrix` with
`LKJSCRIPT_TRANSFERRED_FIXTURE_ROOT` naming its `full-suite`. These source-bound fault tests restore
original evidence; they are not fresh product behavioral observations or default full rehearsals.
The next selected user-visible release milestone must exercise this public workflow. No new
publication or deployment is selected merely to integrate this tooling change.

An independent token-free transport check may repeat:

```sh
curl --fail --location --output /absolute/path/exact.tar.gz \
  "https://github.com/lkjsxc/lkjscript/releases/download/$release_tag/lkjscript-x86_64-unknown-linux-musl.tar.gz"
curl --fail --location --output /absolute/path/latest.tar.gz \
  https://github.com/lkjsxc/lkjscript/releases/latest/download/lkjscript-x86_64-unknown-linux-musl.tar.gz
cmp /absolute/path/exact.tar.gz /absolute/path/latest.tar.gz
gh release verify "$release_tag" --repo lkjsxc/lkjscript
gh release verify-asset "$release_tag" /absolute/path/exact.tar.gz \
  --repo lkjsxc/lkjscript
```

Current-public README and status claims advance only after exact and latest anonymous acceptance has
passed. A published release without that evidence is externally committed but not closed.

## Recovery and maintenance

Before tag push, correct the exact source or workflow defect, create a new final source commit, and
rerun every invalidated fresh boundary. An unpushed local tag may be removed only after proving it
never reached the remote.

Once a tag is pushed, never move or delete it. An unchanged exact tag with only a transient,
idempotent orchestration failure may use the bounded repository recovery dispatch after proving it
cannot create contradictory state:

```sh
gh workflow run Release --repo lkjsxc/lkjscript --ref main \
  -f publish=true -f tag="$release_tag"
```

Any source, workflow, target, verifier, package, or candidate change after tag push requires the
smallest unused additive patch. Once a release is published, never edit, unpublish, replace, relabel,
or delete it or its assets. Read-only propagation and verification may retry within the bounded
workflow policy; a content defect recovers only through a new patch identity.

Official actions remain pinned to full commit SHAs. Review changes to action SHAs, toolchain,
native-package digests, userland images, cargo-about digests, runner labels, and resource
measurements as explicit release inputs. Static linkage is directly inspected evidence, not build
provenance, binary signing, hostile-code isolation, or universal Linux portability.
