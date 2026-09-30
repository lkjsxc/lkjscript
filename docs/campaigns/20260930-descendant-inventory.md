# Contributor descendant inventory correction — 2026-09-30

## Selection and entry state

The owner requested continued development of `lkjsxc/lkjscript`, delegated design
choices and normal mainline integration, and prioritized the strongest long-term
direction over preservation of incidental styles. This continuation chooses a
demonstrated delivery blocker before expanding the already-unreleased language work.
It does not replace the owned-region/scoped-view direction or add a new language feature.

Entry main and origin were `8c7b36f0fec79f2ef287bddafea22aee0351b1ad`.
The existing unrelated untracked campaign and `change_review.rs`, the old history
stash, other worktrees and running applications remain outside this change.
Rust 1.98.0 and Cargo 1.98.0 were observed in the existing Linux Coder workspace.

## Original failed candidate

The original v0.1.60 producer [36605595167/1](https://github.com/lkjsxc/lkjscript/actions/runs/36605595167)
used source `89241cd33c88d27ef1b6f774e24ab9b0726ff093`. It ended in failure on
2026-09-30 at 03:23:21 JST, rather than remaining in progress.
Its source profile passed 19 of 20 selected gates freshly, with no reuse.
`workspace_tests` failed with `owned descendant traversal exhausted`; publication
and public verification were skipped and the terminal remained `incomplete`.
The verifier interrupted the workspace command during public CLI tests. This is
not evidence that every interrupted application test passed or that the candidate
was accepted.

Authenticated diagnostics artifact `11052529513` and its original receipt/logs
are retained under `.artifacts/20260930-descendant-inventory/failed-candidate-complete/`.
The receipt is below
`lkjscript/lkjscript/.artifacts/lkjscript-dev/check/1790703593360944789-6523-0/`.
An earlier interrupted download remains separately in `failed-candidate/`; it was
not overwritten or used as a complete artifact. Terminal artifact `11052909885`
was observed separately. Both artifacts were unexpired at that observation.

## Demonstrated mechanism and correction

The previous Linux sampler retained every historically observed child identity.
It also enqueued an already-known child again when reaching the same child through
its parent, charging repeated discovery paths against traversal capacity.
A long sequence of small process trees could therefore exhaust the verifier's
inventory without thousands of simultaneously live children.

Refresh the known inventory before each discovery. Remove only confirmed absent,
terminated or replaced PID/start identities. A failed observation retains that
identity for cleanup and returns the error; it is never converted into absence or
successful completion. A replacement process is not adopted merely because it has
a formerly known PID. Live sampled branches remain roots after reparenting.

The traversal owner counts each queued/visited PID once. It exhausts a connected
tree depth-first before falling back to retained roots, so a previously known child
does not silently reset its actual connected depth to zero. The existing
4,096 distinct per-sample process and depth-64 bounds remain; thread/read bounds,
exact PID/start checks, pidfd signaling, group ownership, EOF handling and joined
cleanup are unchanged. Confirmed historical departures are not a lifetime quota.

These are contributor-verifier changes only. Product implementation, language
semantics, application data, dependency locks and running processes are unchanged.
Sampling still cannot account for a separate-session descendant created and
reparented entirely between observations; this is not hostile-process containment.

## Focused evidence

Before changing production code, three new regressions were run against the
predecessor implementation. All three failed: 4,096 confirmed-absent retained
identities exhausted admission, a recycled unrelated identity was not retired,
and a reaped child remained alongside the live owned branch. The literal original
output is retained as `predecessor-tests.log` in the campaign artifact directory.

After correction, `cargo test --locked -p lkjscript-dev process:: -- --nocapture`
passed all 32 selected process tests, with no failed or ignored tests, in 0.79 s
(test execution only). `process-tests.log` retains the output.
The nine new tests additionally cover 8,192 simulated serial departures, failed
observations and recovery, duplicate edges at the exact process bound, one-over
process/depth rejection, and disconnected roots with later children.
The existing real separate-group/reparenting, output/EOF, cancellation, timeout,
reaping and healthy-recovery fixtures remain active.

The large-count inventory and traversal cases are deterministic controlled states,
not measurements of 4,096 concurrent real processes. Small real child fixtures
retain finite fallbacks and a kill/reap guard. No runtime speedup or memory-use
measurement is claimed.

## Frozen-source acceptance and mainline delivery

Implementation `2962c43f0617bda2c5726a96249c8fe53f571747`, tree
`4b7db15df18c5d6692f011e477c703e2f7387691`, passed
`lkjscript-dev check full --fresh --jobs 2 --machine`: 26 selected gates,
26 freshly passed, zero reused, zero unrun, no failure, `input_stable=true`.
Elapsed verification time was 709.792214972 seconds. Cargo used two build jobs;
fresh gate execution does not mean compilation caches were disabled.
The initial and final source input digest both equal
`verification_066e83fa6eca83a3332e7611061a8a451c137a2517489d2e04338e5575f75fa4`.
The authoritative receipt is
`.artifacts/lkjscript-dev/check/1790708563039022847-3099556-0/receipt.json`.
The checker-owned copied harness and original gate logs remain beside that receipt.
The campaign directory additionally retains the verifier build log, full summary,
and a separate byte-identical copy of the running verifier used for the probe below.

All language/product code, dependency locks, product tests and workflows are unchanged
from entry. After refreshing remote main and its branch rules, the implementation
was pushed normally as lkjsxc. An independent GitHub ref read confirmed remote main
at exact `2962c43f`. Later reporting edits do not replace this tested-source identity.

## Supplementary traversal and real-process observations

A standalone Rust probe included the actual `traversal.rs` implementation and used
independent parent chains as its depth oracle. With seed `d712847a1f0b3915`, all
8,192 generated root-connected trees passed: 525,857 distinct node observations,
no missing or duplicate visits, and matching depths despite repeated child edges
and varying retained-root selections/order. Each tree had at most 128 nodes.
`traversal-oracle.rs`, its executable and `traversal-oracle.log` remain in the
campaign artifact directory. Its simplified error carrier does not model procfs
or process identity, and these finite cases are not an exhaustive proof.

The retained corrected verifier's public `measure` route then ran
`serial-processes.sh` from `/tmp`. The script successfully started and joined
5,000 serial `/bin/sleep 0.02` children and printed exactly `completed=5000`.
The verifier recorded `passed`, exit code 0, no error, complete output, and
102.576210683 seconds elapsed. Original `serial-observation/observation.json`,
stdout/stderr and the script remain alongside `serial-summary.json`.
Only a small live tree was needed; this is not a 5,000-concurrent-process test,
an assertion that the sampler observed every child, or a performance comparison.

## Corrected successor candidate

After exact-source integration and refreshing release runs and tag occupancy,
one new non-publishing candidate was dispatched:
[36617982924/1](https://github.com/lkjsxc/lkjscript/actions/runs/36617982924), created
2026-09-29T19:15:47Z (2026-09-30 04:15:47 JST). Independent GitHub reading confirms
`workflow_dispatch`, main, original attempt 1, and the same tested product/controller
source `2962c43f`. It was observed queued and then in progress; neither state is
acceptance or publication. No v0.1.60 tag, scoped-selection change or promotion
was created.

The failed original producer `36605595167/1` remains failed. Its demonstrated
verifier defect, not the mere passage of time, requires this new source/candidate.
Public/latest remains immutable v0.1.55. A successful local full profile is not
hosted source acceptance or final-archive acceptance. Resume `36617982924/1` at
its actual observed boundary; do not duplicate it just because it is pending.
Before promotion, run the original final-archive cases specified in the
[consolidated selection](20260930-byte-buffer-reuse.md#revised-publication-selection-consolidate-v0160):
`native_generic_resources`, `native_byte_reuse`, `native_map_keys`,
`native_terminal_values`, and `shared_runtime`. Those final-archive cases have not
been executed by this continuation. All local verification jobs here have completed;
only the separate hosted candidate/publication boundary remains outstanding.

The subsequent reporting-only edits affect this campaign, status and the release
procedure. Generated-reference comparison, no-Python policy and product-surface
policy passed, with zero policy violations. They do not claim a new full-profile
run or alter the already-frozen candidate source.

## Candidate acceptance and publication continuation

The later byte-range continuation first completed development v0.1.61 source
integration on main at `9ca3b15c807ff53f7dd1d7a2af92e7ff432c0814`. That does not change
the v0.1.60 product source or silently add the new byte-range intrinsics to its archive.
The separate existing v0.1.60 producer was then reconciled rather than rebuilt.

Producer `36617982924/1` completed successfully at 2026-09-30 05:11:11 JST. The
GitHub API independently identifies repository and head repository `1307238071`,
`workflow_dispatch`, workflow `.github/workflows/release.yml`, main and exact source
`2962c43f0617bda2c5726a96249c8fe53f571747`. The candidate job and each required
source/build/final-admission/upload step succeeded; the terminal job also succeeded.
Original terminal and essential acceptance artifacts both report `candidate_accepted`.
The acceptance handoff binds 20 source gates, six final-target owners, two pinned
userlands, installation/recovery and original readers, with complete cleanup and
no failure. This supersedes the earlier pending observation without relabeling
the failed first producer `36605595167/1`.

The downloaded GitHub service archives match their independently retrieved digests:
terminal artifact `11058554768`, acceptance artifact `11058984250` and asset artifact
`11058974165`. Their exact inventories contain one terminal JSON, one acceptance
JSON and the three ordinary release assets respectively. The release archive,
checksum file and bootstrap match the acceptance handoff. The archive contains only
its expected directory, executable, license, third-party notices and manifest.
The extracted executable is 26,836,352 bytes with SHA-256
`f0365f4eebf6de18d3730a0c17099f258cc133c9deb20bf537a03a88a50079e2`; its manifest
matches the accepted manifest digest. No reconstructed or rebuilt asset substitutes
for these original bytes.

Original API metadata, ZIPs, assets and extracted product are retained under
`.artifacts/20260930-byte-slices/release-0.1.60/`. All five producer artifacts were
unexpired when read; the earliest service expiry is 2026-10-13T20:10:43Z. Only the
three artifacts needed for this supplementary product check were downloaded;
the large diagnostic and verifier archives remain available at their original owners.
The promotion controller must still perform its own authenticated original-reader,
transfer and publication admissions.

The prior release-only selector was read as annotated object
`033f7563e1bf79a320729368b41259ac7284acb2`, which resolves to v0.1.55/source
`320dacc051a25e35d99b00852adc1285c8a322a7`. Its promotion `36442849973/1` completed
successfully, and current public/latest was independently read as immutable release
`398379899` with its original three assets. The v0.1.60 tag was unoccupied. No
credential, permission, branch protection or immutability change is part of this
continuation.

The complete selected final-archive filter set passed **40 tests, zero failed,
zero ignored**, 131 unrelated tests filtered, in **67.48 seconds**, with process
exit zero. The filters were `native_generic_resources`, `native_byte_reuse`,
`native_map_keys`, `native_terminal_values` and `shared_runtime`, exactly as selected
before candidate production. Eight test threads were used. Execution started in
`/tmp` with an empty environment/PATH except the explicit candidate selector; no
publishing credential was supplied. The test executable was copied from Cargo's
reported public-CLI artifact at source `4a8ac5e5`. Its selected test bodies, native
fixtures and common helper bodies are unchanged from source `2962c43f`; changes in
the newer harness are unrelated new byte-range tests and inventory expectations in
unselected tests. This is supplementary black-box proof, not a relabeling of the
producer's verifier or source acceptance.

`final-archive-native-joined.log` and `.exit` retain the complete result. The first
all-filter observation lost its tool connection and has 38 per-case passes but no
terminal; it is retained as incomplete, not accepted. A separate two-case shared-host
completion passed in 25.23 seconds. The complete successful rerun redirected output
to retained logs, avoiding dependence on the tool response connection. No surviving
candidate-test process was observed before resumption; unrelated running applications
were left untouched. The completed 40-case result, not the partial log, satisfies the
supplementary final-archive requirement.

After that complete success, the ordinary annotated v0.1.60 tag was created as
lkjsxc at accepted source `2962c43f`. Its object is
`1ed78aaead92e41c053aa735a1e55fee13137953`. `--cleanup=verbatim` preserved the
selected release notes; both local tag payload and independently fetched remote
annotation were byte-compared with those unchanged notes. The source remains
reachable from refreshed main. No existing tag was moved.

The existing release-only `LKJSCRIPT_IMMUTABLE_RELEASE_TAG_OBJECT_SHA` selector
was compared with its completed v0.1.55 owner immediately before changing it to
that exact annotated object, then read back successfully at 2026-09-29T22:35:18Z
(2026-09-30 07:35:18 JST). This is the scoped selection required by the documented
promotion procedure, not a change to authentication or repository protection.
At this checkpoint the tag and selection exist, but no public v0.1.60 release is
claimed; the ordinary unchanged-asset promotion still owns publication and anonymous
installed verification.

## Completed v0.1.60 publication

On 2026-10-01 the existing accepted producer, annotated tag and scoped selector were
reconciled against live GitHub reads before dispatch. Producer `36617982924/1`
remained successful, with its original unexpired acceptance, assets and verifier.
Tag object `1ed78aaead92e41c053aa735a1e55fee13137953` still targeted
`2962c43f0617bda2c5726a96249c8fe53f571747`, reachable from main. The existing
release-only selector still selected that object; no selector write was needed.
The read-only immutable-releases endpoint returned `enabled=true`. There was no
occupied v0.1.60 release before dispatch. The release workflow and trusted controller
source were unchanged between the accepted producer and integrated main `9afa7997`.

One ordinary `operation=promote` invocation selected producer `36617982924`, attempt
`1`, from main. Run `36763094941/1` binds controller source
`9afa799794ac26fa90bd4b2413e6e4d26886ccb6`; it started at 04:04:26 JST and completed
successfully at 04:12:24 JST. Selection, fresh publication authority/occupancy,
transferred installed lifecycle, isolated publication, anonymous acquisition,
attestation verification, actual public installed lifecycle and terminal decision
all completed successfully. Candidate production was correctly skipped, not rerun.
The retained terminal reports `immutable_published_and_public_verified`, with
`latest=selected`, `latest_tag=v0.1.60` and exact latest source `2962c43f`.

Immutable release `400357606` was published at **2026-10-01 04:10:53 JST**.
Independent tag, exact release-ID and latest reads agreed; it is neither a draft
nor a prerelease. The original candidate acceptance inventory was downloaded anew
and compared programmatically with the exact public release inventory by asset
name, length and SHA-256. All three entries matched:

| Asset | Public asset ID | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `lkjscript-x86_64-unknown-linux-musl.tar.gz` | `601619022` | 12,646,980 | `9bb3c156363671646f119abcf5389a469494dd790e2282357fff3a82e11efdd9` |
| `SHA256SUMS` | `601619144` | 109 | `a35ca67a8646970fca790151b74dd74305ed58b444cdee1ec64a0c662edb726f` |
| `install.sh` | `601619255` | 3,566 | `4806c70613c7fb016649ec9dca7bd89f2001d75056bb686213004ef5764a47eb` |

The producer's final executable remains 26,836,352 bytes, SHA-256
`f0365f4eebf6de18d3730a0c17099f258cc133c9deb20bf537a03a88a50079e2`.
No asset was rebuilt, no earlier immutable asset or tag was rewritten, and no
credential, repository-protection or immutability setting was changed. The original
annotation and selected notes remain unchanged. Existing services, application
HEADs, dependency selections and data remain untouched.

Promotion terminal artifact `11120195698` is 355 service-archive bytes with digest
`sha256:418df41714771459e8fad4cfd6058037fa5b5ced9f4de2092a11fda6258b3405`.
The public diagnostic artifact is `11120300581`, publication `11119653367`,
selection `11119398176` and trusted controller `11119383195`, all bound to this
controller run/attempt. Downloaded terminal, original acceptance, public release-ID
response and exact-inventory comparison are retained in the owned-generics checkout
under `.artifacts/20261001-buffer-admission/`. This release does not include later
v0.1.61 byte ranges, ByteBuffer, Owned generics or the [new storage-admission correction](20261001-owned-storage-admission.md).
