# Release procedure

GitHub Releases are the sole public binary distribution path. A release is a derived distribution
of one exact source commit; its tag, archive, manifest, checksum, GitHub asset digest, and release
attestation do not become semantic program authority.

## Owned identity and prerequisites

The root `Cargo.toml` package version owns the release version. A published release uses the exact
annotated tag `vMAJOR.MINOR.PATCH`, and the tag commit must already be reachable from `origin/main`.
The only admitted target is `x86_64-unknown-linux-gnu`. A target is not added until its exact asset
passes copied-binary product acceptance and its runtime requirements are measured.

Before release work, fetch and inspect remote state without rewriting it:

```sh
git fetch --prune origin
git status --short
git rev-parse HEAD origin/main
git tag --list --sort=version:refname
gh release list --repo lkjsxc/lkjscript
gh api repos/lkjsxc/lkjscript/immutable-releases
```

The last response must report `enabled: true`. A repository administrator may enable the setting
once with:

```sh
gh api --method PUT repos/lkjsxc/lkjscript/immutable-releases
```

Do not put administrator credentials in the workflow. GitHub's immutable-release settings endpoint
requires repository `Administration: read`, which is not an assignable `GITHUB_TOKEN` permission.
Immediately before pushing each release tag, the administrator therefore binds the successful
setting check to that exact annotated tag-object SHA in the non-secret repository variable
`LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA`. The write-isolated publish job requires that exact
binding and otherwise stops before creating a draft. Normal publication uses only the ephemeral
workflow token and the publish job's `contents: write` permission.

## Pinned inputs and local preparation

`rust-toolchain.toml` pins Rust/Cargo 1.98.0, rustfmt, clippy, and the GNU x86-64 target. The notice
generator is `cargo-about` 0.9.2. Its selected release archive has SHA-256
`9099a59e820c38a68b9d65f300662a567d56562f9a10f6aa4c7e86c17c2566af`; the executable extracted
from that archive has SHA-256
`b06bd6a8bfd726cffb90e3e0588e3e0b1cfbb582bf6a34f4c1c2692ba8f2e7b8`. Keep both checks. Update a
pin only in a reviewed commit that regenerates notices and reruns the release tests. Hosted full
verification provisions the Linux amd64 service dependency by its platform-manifest reference
`postgres@sha256:075f7ba66bc9b3ce7d6b8b635208ff61cd7cf1a67d71ec530eec5d7ae0cbe571`
before invoking the otherwise network-independent service harness.

From a clean fast-forward checkout, build and prepare a dry-run package with absolute paths:

```sh
cargo fetch --locked
cargo build --workspace --release --locked
package_version=$(cargo metadata --locked --no-deps --format-version 1 |
  jq -er '.packages[] | select(.name == "lkjscript") | .version')
release_tag="v$package_version"
evidence_parent=/absolute/private/evidence-parent
mkdir -m 0700 "$evidence_parent"
cargo run --locked -p lkjscript-dev -- distributed-http \
  --binary "$PWD/target/release/lkjscript" \
  --evidence-root "$evidence_parent/distributed-http" --machine
cargo run --locked -p lkjscript-dev -- release prepare \
  --candidate "$PWD/target/release/lkjscript" \
  --cargo-about /absolute/path/to/cargo-about \
  --cargo-about-archive /absolute/path/to/cargo-about.tar.gz \
  --output /absolute/absent/path/release-output \
  --tag "$release_tag" \
  --publication dry-run
cargo run --locked -p lkjscript-dev -- release verify \
  --archive /absolute/path/release-output/lkjscript-x86_64-unknown-linux-gnu.tar.gz \
  --checksums /absolute/path/release-output/SHA256SUMS \
  --receipt /absolute/path/release-output/release-receipt.json \
  --extract-to /absolute/absent/path/verified-release \
  --expected-tag "$release_tag" \
  --expected-publication dry-run
```

The untargeted fetch makes every package selected by locked Cargo metadata available before the
notice generator switches Cargo to offline mode; target filtering and the production-only policy
still determine the notice contents. Preparation generates notices twice from that locked,
offline, target-filtered production closure;
runs the exact private copy of the candidate through the existing copied-binary lifecycle; creates
two archives; and compares and strictly extracts them. It refuses a dirty checkout, symlink or
nonregular input, output conflict, wrong target, malformed manifest, unknown license, nondeterministic
notice/archive, wrong inventory/order/mode, link, traversal, duplicate, corrupt checksum, or candidate
mismatch. `release verify --extract-to` publishes the already validated archive root through one
create-new directory boundary, so later jobs do not duplicate archive parsing in shell. Dry-run
output is development evidence, not a public release.

For a release-mode preparation, provide the fresh successful `check full` receipt and
`--require-full-verification`. Every gate, including independent no-Docker copied-binary HTTP
acceptance and PostgreSQL service acceptance, must be fresh and passed. The direct
`distributed-http` invocation above is a focused reusable receipt for candidate diagnosis; the
full-profile gate remains the release-preparation dependency owner.

## Hosted dry run and tag publication

The `Release` workflow uses explicit `ubuntu-24.04`. Its build job has read-only repository
permission, performs full and exact-candidate verification, and uploads two one-day transient
artifacts: the three-file release handoff and a separate verifier handoff containing the exact
release-built `lkjscript-dev` plus its byte/mode identity. A read-only no-checkout job downloads
both by artifact ID, verifies their artifact and file digests, restores verifier mode only after
its bytes agree, safely extracts the packaged candidate through `release verify`, and runs the
transferred `distributed-http` owner with an explicit private evidence root. Publication depends on
that pass. A manual dispatch is a dry run unless `publish=true` and its selected ref is the exact
existing annotated tag:

```sh
gh workflow run Release --repo lkjsxc/lkjscript --ref main \
  -f publish=false -f tag="$release_tag"
gh run watch --repo lkjsxc/lkjscript RUN_ID --exit-status
gh run download --repo lkjsxc/lkjscript RUN_ID \
  --name release-handoff-RUN_ID-RUN_ATTEMPT \
  --dir /absolute/absent/path/hosted-handoff
gh run download --repo lkjsxc/lkjscript RUN_ID \
  --name pre-publication-http-evidence-RUN_ID-RUN_ATTEMPT \
  --dir /absolute/absent/path/hosted-http-evidence
```

Independently run `release verify` against the downloaded handoff and inspect the transferred HTTP
receipt digest. A dry run creates no tag, draft, or release; the publish and post-release jobs are
skipped.

After the exact implementation commit is clean, verified, normally pushed, and equal to
`origin/main`, create and push only an annotated tag:

```sh
git fetch --prune origin
test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"
package_version=$(cargo metadata --locked --no-deps --format-version 1 |
  jq -er '.packages[] | select(.name == "lkjscript") | .version')
release_tag="v$package_version"
git tag -a "$release_tag" -m "lkjscript $release_tag"
test "$(git cat-file -t "refs/tags/$release_tag")" = tag
tag_object_sha=$(git rev-parse "refs/tags/$release_tag")
test "$(gh api repos/lkjsxc/lkjscript/immutable-releases --jq '.enabled')" = true
gh variable set LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA \
  --repo lkjsxc/lkjscript --body "$tag_object_sha"
test "$(gh variable list --repo lkjsxc/lkjscript \
  --json name,value \
  --jq '.[] | select(.name == "LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA") | .value')" \
  = "$tag_object_sha"
git push origin "refs/tags/$release_tag"
```

The tag push owns publication. Do not create a release manually in parallel. The publish job does
not checkout or execute repository code. It downloads only the verified handoff, checks fixed
names and digests with runner tools, checks the remote annotated tag and its administrator-recorded
immutable-setting confirmation, obtains the tag message through the Git API as its release notes,
finds an exact draft through a bounded authenticated release listing, creates or resumes that
draft, uploads only missing assets through its release-ID endpoint without `--clobber`, verifies
both remote asset digests, and then publishes it as latest. Published exact immutable state is
idempotent success; extra, mismatched, duplicate, or listing-bound-exhausted state fails.

If a tag-triggered run fails before publication because the workflow itself needs a later repair,
leave the tag untouched. After the repair is on `main`, recover with the next unused patch release.
For an unchanged exact tag whose source and workflow need no repair, an idempotent manual retry may
use `publish=true`; the workflow definition comes from `main` but checkout and all executable work
come from the requested annotated tag:

```sh
gh workflow run Release --repo lkjsxc/lkjscript --ref main \
  -f publish=true -f tag="$release_tag"
```

## Public verification

Verify anonymous exact and latest transport separately:

```sh
release_tag=$(gh api repos/lkjsxc/lkjscript/releases/latest --jq '.tag_name')
mkdir -p /tmp/lkjscript-release-check/exact /tmp/lkjscript-release-check/latest
curl --fail --location --output /tmp/lkjscript-release-check/exact/archive.tar.gz \
  "https://github.com/lkjsxc/lkjscript/releases/download/$release_tag/lkjscript-x86_64-unknown-linux-gnu.tar.gz"
curl --fail --location --output /tmp/lkjscript-release-check/exact/SHA256SUMS \
  "https://github.com/lkjsxc/lkjscript/releases/download/$release_tag/SHA256SUMS"
curl --fail --location --output /tmp/lkjscript-release-check/latest/archive.tar.gz \
  https://github.com/lkjsxc/lkjscript/releases/latest/download/lkjscript-x86_64-unknown-linux-gnu.tar.gz
sha256sum /tmp/lkjscript-release-check/exact/archive.tar.gz \
  /tmp/lkjscript-release-check/latest/archive.tar.gz
gh release verify "$release_tag" --repo lkjsxc/lkjscript
gh release verify-asset "$release_tag" /tmp/lkjscript-release-check/exact/archive.tar.gz \
  --repo lkjsxc/lkjscript
```

The post-release job compares exact/latest archive and checksum bytes, checks both GitHub asset
digests, and uses only `actions: read`, `contents: read`, and `attestations: read` for artifact,
release, and asset verification. It downloads the same verifier handoff used before publication,
checks its bytes before mode restoration, and uses first-party `release verify --extract-to` for
each anonymous archive. With credentials removed from the executable environment, it then runs the
transferred `distributed-http` acceptance owner independently against the exact-tag and latest
binaries. Separate retained receipts prove creation, reviewed mutation, check, deterministic build,
serve, restart, startup failures, shutdown cleanup, and unchanged Graph authority. Immutable
`v0.1.7` is the first release admitted by this complete mechanism. The historical `v0.1.6` release
predates it and is not credited with the public HTTP recipe.

## Recovery and maintenance

An exact draft may receive only a missing exact asset before it is published. Never use
`--clobber`; never delete or replace a conflicting asset. Inspect any failed draft before rerunning.
Once published, the tag, release, and assets are immutable. A content defect is recovered through
the smallest unused patch version, a new commit, and a new annotated tag—never by moving, deleting,
or force-updating published state. A post-release transport failure does not authorize rollback;
first distinguish propagation from incorrect content.

Official actions are pinned to full commit SHAs in the workflow, with their readable release tags
in comments. Review and update the action SHA, cargo-about archive/executable digest, toolchain,
runner label, and runtime measurements as explicit inputs. Broad CI, another platform, a musl/static
asset, package manager, installer, updater, mirror, signing, or build provenance requires a separate
campaign and independent admission evidence.
