# Opaque product numbering and release admission

## Initial owner instruction (unchanged)

[https://github.com/lkjsxc/lkjscript](https://github.com/lkjsxc/lkjscript) について進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。また、バージョンがa.b.cdみたいな感じになっているわけですが、a,bもcdと同じように特別な意味を持たないようにしてほしい。

## Startup and selected boundary

On 2026-10-01 (Asia/Tokyo), independent GitHub main and both local checkouts
matched `1ec4dd614323d8275cbd6b88a771ab0891dcd9e9`. The source already contains
structural owned products. Reuse detached
`/home/coder/workspace/lkjscript-owned-generics-20260930`; retain main's two
unrelated untracked files, the recent-history stash, other worktrees, active
services and the isolated checkout's four pre-existing derived package packs.
The pinned compiler is Rust 1.98.0. Initial host observations were approximately
54 GiB available RAM and 512 GiB free disk. Commit identity remains lkjsxc.

The owner requires identifier-only roles for all three product components. Keep
the single root Cargo version owner and exact whole-tag matching; do not invent
a second VERSION file, component-specific compatibility rules, carry threshold,
new dependency resolver or synchronized contract ladder. Cargo's external
dependency rules remain independent. Existing canonical identities are unchanged.

Select development 0.1.62 to distinguish this work from frozen v0.1.61 source
`c64f42dc66f0e7cc459c66cc83daad5d66821c1a`, producer `36828457203/1`.
That producer was observed running and is not restarted or mutated. Independent
GitHub latest still selects immutable v0.1.60, release `400357606`, with its three
original assets. No new public release follows from selecting this source version.

## Reproduced predecessor defect

Add two focused tests before changing production validators. The installer's
release-container validator admits canonical 64-byte tags and rejects 65 bytes;
the duplicate contributor producer and verifier-handoff validators accept both.
The new tests independently fix the 64/65-byte expected boundary and fail at the
two contributor acceptance assertions. No bound was raised to make them pass.

Pinned foreground command:
`cargo test --locked -j 4 -p lkjscript-dev tag_length_matches_container_admission -- --nocapture`.

The retained `.artifacts/20261001-version-policy/predecessor-tag-limits.log`
reports 0 passed, 2 failed, 224 filtered out and Cargo exit 101. This is deliberate
predecessor failure evidence, not failed corrected-source acceptance.

## Implementation and consumer closure

`src/release_container/tag.rs` is the single canonical-spelling owner. All three
positions use the same decimal rules; successful validation needs no allocation
or integer parsing. Whole identity matching remains separate from spelling.
Producer, handoff and runtime wrappers retain their corrupt/usage/runtime_tag
diagnostic roles. Tests cover all positions, exact selection, malformed components,
ranges/suffixes and 64/65-byte boundaries without integer overflow.

The executable product record declares `version-policy=opaque-triplet` on ordinary,
focused and cached discovery; its complete capability digest binds the policy.
The version query remains unchanged. The maintained public-surface verifier and
CLI expectations are updated independently. Generate reference documents through
the product's existing owner, not by editing generated Markdown.

The product specification, original version-authority decision amendment, project
direction, contributor guidance and release examples agree. README installation
is pinned to the independently observed public v0.1.60 instead of stale v0.1.48.
Shared hosting is correctly described as public. Status and roadmap no longer
describe integrated structural products as awaiting integration. Old campaign
observations, releases and the frozen candidate retain their original meanings.

This increment does not change language meaning or graph/compiler/artifact/data
encodings. The discovery-field addition changes capability digests and requires
strict clients to admit the field; retained review requests must be re-planned
under the selected executable. It does not waive any semantic admission checks.

## Execution boundaries and preserved unsuccessful launches

An initial background command incorrectly attempted to launch shell `export`
as an executable and failed before testing. A shell-wrapper request was blocked
before execution; no protection or permission was changed. A foreground test
connection closed while its compiler continued; completion was not inferred from
the transport error. The later pinned foreground rerun produced the retained
predecessor failure above.

A direct manifest-path test launch resolved the home directory's Rust 1.97.0
instead of the repository pin. Its owned Cargo process was interrupted and joined;
`tag-tests.log` remains an unaccepted/cancelled launch, not pinned evidence.
Subsequent direct Cargo commands explicitly select `+1.98.0`. No dependencies were
updated; the lockfile change is only the root product identifier.

## Focused corrected-source observations

Pinned `cargo +1.98.0 test --locked -j 4 -p lkjscript -p lkjscript-dev --lib tag`
completed with 27 product tests passed, two pre-existing one-time migration tests
ignored, and 12 contributor tests passed. Both predecessor controls now pass.
`tag-tests-pinned.log` retains the result. There are no newly ignored tests.
The focused public CLI discovery/export/cache test and the new exact runtime
selection test each passed. The latter exercises six canonical identities and
seven invalid selectors, preserving an absent installation prefix in every case.

A pinned release-profile build completed. The executable reports `lkjscript 0.1.62`
and product `version-policy=opaque-triplet`; capabilities digest is
`e0ad4af6eb11e09578b189d8015c118656d54ea04954a758029bf604d6ff04a8`.
The product regenerated all eight tracked reference pages through its native owner.
A copy under `/tmp/lkjscript-version-policy-20261001.A1cHd9` was exercised from
outside the checkout with only PATH and an isolated HOME. It generated the same
eight document sizes/digests, returned the policy in an unchanged-cache response,
and treated `v1.0.0` as an exact missing installation (`runtime_not_installed`,
exit 2), not a stability or prefix selector. Original and copied executables shared
SHA-256 `135a4c7acb3d2fa896f400fe4865db2d0156da38b8bbbf18698378a3031a9af5`.
This is a local Linux host-binary probe, not finalized static-archive acceptance or
a cross-platform reproducibility/hostile-isolation claim.

## Acceptance and publication boundary

Corrected-source acceptance and exact mainline delivery are recorded below when
observed. Source checks do not accept a finalized archive or publish a release.
Keep the existing v0.1.61 candidate independent. A new v0.1.62 distribution is
deferred until the next selected product milestone or an explicit release request,
with its own exact-source and finalized-archive acceptance; do not rebuild or
relabel the frozen candidate merely to change numbering policy.
