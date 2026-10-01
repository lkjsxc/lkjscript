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

## Initial full-source result and test correction

Source `44d0871c629024ef302c8226d5eb072da20b8206`, tree
`475785c860cc8a013489298f6f9548355ad6fee7`, completed the fresh full profile with
25 of 26 gates passed, zero reuse and `input_stable=true`. It failed only
`workspace_tests`: 1030 product-library tests passed, eight existing migration
cases were ignored and the canonical public-capabilities test still expected the
old two-field product record. Receipt
`.artifacts/lkjscript-dev/check/1790842621715822541-368567-0/receipt.json` is 68177
bytes, digest `verification_a6b4f9c3eefb3cfa397d32c9b354ed07d77367752058f9f273fce8146576c8ef`.
This is preserved failed acceptance, not a successful source cut.

Correct the independent test's literal header, explicit policy assertion and
independently assembled digest preimage to include `version-policy=opaque-triplet`.
Do not remove its complete-digest assertion, hide a public contract or ignore the
test. Production output and the already generated reference pages are unchanged.
The subsequent full profile must bind the corrected source after it is committed.

## Authenticated frozen candidate failure

Producer `36828457203/1`, exact source `c64f42dc66f0e7cc459c66cc83daad5d66821c1a`,
finished with failure on 2026-10-01 at 16:48:51 JST. Independently read job
`110259357705` shows all 20 source gates passed freshly, but final input admission
failed with `worktree_changed_during_run` and `input_stable=false`. Candidate
production/final-archive acceptance did not run; the terminal job's success merely
reports the failure, not acceptance. Original source receipt is 63481 bytes,
digest `verification_30f786203cdb8c0f087ef1f1a5c4cf90b85c9a8b9cf26a0bf5de76e6a58ecbe`.

Authenticated diagnostic artifact `11147471503`,
`candidate-diagnostics-36828457203-1`, contains 52315240 bytes and service SHA-256
`5ba31590a65f816f7d754d8d6a81af70ce8e4d559ed34412f97254ba2ff3858c`. The downloaded
archive matches that digest. Observed service expiry is 2026-10-15T07:48:38Z;
a retained conversation copy is `/mnt/data/candidate-diagnostics-36828457203-1.zip`.
Only diagnostic data was read, not executed. The initial snapshot has no untracked
inputs. Its receipt retains different initial/final input digests, but the archive
does not retain a final path-by-path input manifest. Therefore this evidence alone
does not identify the exact historical file delta; the independent reproduction
below establishes the missing-input behavior without inventing that delta.

## Source-only maintained-input reproduction

Export only tracked projects from `44d0871c` into the independent copied-binary
directory using `git archive`: `packages/standard`, `applications/lkjournal`,
`tools/native-guides/project` and `tools/native-policy/project`. The four original
untracked packs are absent. With only PATH and an isolated HOME, run each project's
public `check` through the copied 0.1.62 executable. No compiler checkout, external
semantic generator or direct accepted-graph edit is used. All four clean compilation
passes succeed: standard 89, lkjournal 44, guides 79 and policy 62 tests, 274 total,
zero failures and equal production/reference results. The original accepted semantic
revisions remain unchanged.

The following packs are newly produced from that tracked-source copy and match
the originally retained local files byte for byte (SHA-256 shown):

| Project | Native pack | SHA-256 |
| --- | --- | --- |
| `packages/standard` | `pack_fb17e4f7c24f6864c60ff9d33733f75e0fc842b1fe4e85e9ce317b1f7117b13d.lkjp` | `1733355f556cd6d60bccb477039108492cc7b546621173c40ecf165b203fc4e3` |
| `applications/lkjournal` | `pack_ceb46052f3d0d9c6d10f889da7cdc23ee9f08fcc560bef6429ddbfa3e6b79032.lkjp` | `1c7d812c1240e2c973375888152327e297ef77fc2db086bc66f44536d0236bdf` |
| `tools/native-guides/project` | `pack_dccd0a25eac5a09c12d5314c0d98295544dd26c0b338ebcd91af616730aafce2.lkjp` | `854b28f205e002ded9ad774a082956067d5a1f1b48ca15a13ee2b810ad3dda02` |
| `tools/native-policy/project` | `pack_17a92be1b9216cb1c3213f2e4d2eb4b1b8858e9ff740ac27fa2867d5cf2abd5e.lkjp` | `d79813181b78268b6590b9bd7133f0f4b7ac737cbd2a782ba6f1d48e663d258c` |

Reclassify only those four proven native-generated files as missing maintained
inputs and include them unchanged in the corrective source. This explicitly revises
the initial decision to leave them untracked; it does not delete or overwrite them.
No blanket staging, pack ignore rule, snapshot filtering change or relaxed gate is
used. The first full run's warmed untracked files were stable, but that was not
proof of a complete tracked cold-checkout input inventory. Corrected acceptance
starts with these native inputs tracked and no untracked source files. A second
tracked-source copy checks that this inventory no longer grows on first use.

## Acceptance and publication boundary

Corrected-source acceptance and exact mainline delivery are recorded below when
observed. Source checks do not accept a finalized archive or publish a release.
Keep the existing v0.1.61 candidate independent. A new v0.1.62 distribution is
deferred until the next selected product milestone or an explicit release request,
with its own exact-source and finalized-archive acceptance; do not rebuild or
relabel the frozen candidate merely to change numbering policy.
