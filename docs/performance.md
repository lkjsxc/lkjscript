# Performance and economy evidence

Measurements are observations, not promises.

## Current public binary release

Release `v0.1.6` was built and published by GitHub Actions run
[`33130051176`](https://github.com/lkjsxc/lkjscript/actions/runs/33130051176) on explicit
`ubuntu-24.04` with Rust/Cargo 1.98.0. The run had three jobs: the read-only build job completed in
11 minutes 8 seconds, the no-checkout publication job in 19 seconds, and anonymous post-release
verification in 12 seconds. All required jobs passed on attempt 1.

| Observation | Value |
|---|---:|
| locked dependency prefetch | 2.703 s; 474,049,511 registry bytes |
| release compilation workflow step | about 266 s |
| pinned PostgreSQL image preparation | 4.003 s |
| fresh full verification receipt | 292.861 s; 20/20 gates fresh passed; zero reused |
| service acceptance within full verification | 3.844 s end to end; 32,364 KiB maximum service-process peak RSS |
| exact-candidate copied-binary acceptance | 0.590 s; 0.160 s CPU; 75,272 KiB peak RSS |
| complete release preparation (notice, candidate, two packages, strict verification) | 68.174 s |
| transient handoff artifact | 6,959,115 compressed bytes |
| draft upload, exact inspection, and publication job | 19 s |
| anonymous download, attestation, extraction, and smoke job | 12 s |

The exact release executable is 15,390,408 bytes. `LICENSE` is 11,336 bytes, the generated
third-party notice is 315,160 bytes, and `RELEASE-MANIFEST.json` is 3,957 bytes. Their 15,720,861
payload bytes compress to a 6,954,661-byte archive, a ratio of 0.442384 (55.76% smaller). The
108-byte checksum file and 3,906-byte external receipt complete the fixed handoff. The largest
observed gate peak RSS was 79,944 KiB for workspace tests. Exact notice-generation wall time and
peak RSS were not isolated from the packaging step.

Each of the four hosted anonymous transport requests succeeded on its first attempt and transferred
6,954,661 or 108 bytes as appropriate. GitHub draft discovery required two bounded attempts and
one five-second wait; asset-digest propagation, immutable-state observation, release verification,
and both asset verifications succeeded on their first attempts. An independent local download also
found exact-tag, latest, and release-handoff bytes equal and completed the same command lifecycle.
The release run executed no destructive retry.

These are single hosted and client observations, not latency distributions. Runner caches and
network conditions were uncontrolled. The same-input package was byte-stable within one runner;
this does not establish bit-for-bit compiler reproducibility across runner images. Provider token
use, monetary cost, and exact filesystem syscall counts are unavailable. Structured identities,
digests, classifications, failed predecessor attempts, and raw-evidence pointers are in
[`20260828-v0.1.6-public-release.json`](evidence/20260828-v0.1.6-public-release.json).

## Current normalized command lifecycle

Campaign `202608270014` measured release binaries after a warm build on Linux
`7.0.0-29-generic` x86-64 with Rust/Cargo 1.98.0. Each child ran in a minimal ordinary environment;
the lifecycle used no Cargo or network. `lkjscript-dev measure` retained bounded stdout/stderr,
monotonic wall time, Linux `/proc` CPU ticks, and `VmHWM`. CPU has 10 ms resolution and the 10 ms
poller may conservatively miss a shorter transient RSS peak. Filesystem cache state was
uncontrolled and no command was retried.

| Workflow | Wall | CPU | Peak RSS | Compiler work | Result |
|---|---:|---:|---:|---|---|
| command project creation | 80.423 ms | 10 ms | 9,664 KiB | not applicable | 10 owners, 1 dependency, 1 target, 1 test |
| first command check | 84.583 ms | 50 ms | 10,936 KiB | 4 compiled / 0 reused | 8 tests equal; 188,916-byte artifact |
| clean command build | 66.519 ms | 30 ms | 10,812 KiB | 4 / 0 | SHA-256 `bdccb1fa22509c7da7e741c3417466a8b40fa1168b8684d83c07e40d2183f0a8` |
| exact-current command build | 70.566 ms | 40 ms | 10,776 KiB | 0 / 4 | byte-equal to clean build |
| accepted presentation rename + cache handoff | 36.533 ms | below one tick | 9,648 KiB | 0 / 4 / 0 removed | accepted; cache `updated` |
| post-change exact-current build | 71.967 ms | 40 ms | 10,848 KiB | 0 / 4 | 188,949 bytes |
| post-change clean rebuild | 66.631 ms | 40 ms | 10,916 KiB | 4 / 0 | byte-equal, SHA-256 `d7a6fb977e967148fd243b9d4896cc21d6e7c853ab69b9b3e24d771e52e32aef` |
| pure command run | 60.980 ms | 40 ms | 10,368 KiB | 0 / 4 | `"hello"`; 3 VM instructions / 2 reference expressions; equal |
| standard clean check | 1,099.775 ms | 1,060 ms | 11,540 KiB | 60 / 0 | 7 tests equal; 572 objects |
| standard clean build | 1,105.521 ms | 1,040 ms | 11,980 KiB | 60 / 0 | maintained 182,596 bytes equal |
| `lkjournal` clean check | 597.420 ms | 560 ms | 17,092 KiB | 60 / 0 | 12 tests equal; 2 packages / 2,280 objects |
| `lkjournal` clean build | 555.658 ms | 510 ms | 16,300 KiB | 60 / 0 | maintained 685,766 bytes equal |

The exact-current sample did less semantic compilation work (zero units compiled versus four) but
was not faster in this single uncontrolled-cache timing; no latency improvement is claimed. The
post-change presentation edit selected zero compiler units, reused all four root units, and
produced the same compilation manifest and artifact bytes as a clean rebuild. Maintained standard
and `lkjournal` clean outputs exactly matched their checked-in owners.

The lifecycle response currently exposes semantic/compiler/link/artifact work but not aggregate
repository/cache bytes or exact synchronization syscall counts. Successful artifact receipts did
report synchronized visibility and removed owned stages. Provider token, cache, request, retry, and
monetary telemetry was unavailable and is not inferred.

Structured evidence is
[`202608270014-normalized-command-lifecycle.json`](evidence/202608270014-normalized-command-lifecycle.json).
Raw observations and bounded logs are under
`.artifacts/campaign/202608270014/performance/work-KCVcbb/`; each structured row retains its
observation SHA-256. These are single samples, not latency or topology distributions.

## Earlier evidence

The normalized-query section below is current for query
contract 3 at its named commit. Unless another row explicitly says otherwise, the remaining
numbers predate meaning graph contracts 2, 3, and 4, persistent root pages, direct CLI v7, exact-ID
imports/targets, normalized query, or explicit generics. Those rows are historical baselines and
must not be presented as current-contract performance. Two graph-4 release workflows at 10,000
empty background modules are retained below on opposite sides of the semantic-fact cutover; no
current distribution, million-owner, or complete-service performance receipt exists yet.

The historical environment was Linux `7.0.0-29-generic` x86-64, `rustc 1.96.0`, Cargo 1.96.0.
CPU time, peak RSS, provider tokens/cache/requests/retries, and monetary telemetry were unavailable.
No token or monetary result is inferred from byte counts.

## Current review-bound logical-plan export

At implementation commit `b8a1cf3bc8e8a21c8b188f4d6613ec1e4bfb81e4`, the explicit release-scale
test prepared the same public change with export disabled and enabled. Its 9,621-record request
created 500 empty background modules plus one record/field whose balanced type topology contained
119 base, 4,500 payload, and 4,499 join fragments. The resulting logical plan contained 502 owner
changes, 9,125 type additions, two added relations, 502 structural-validation owners, two
semantic-validation owners, and two reasons; it selected no tests.

| Observation | Export disabled | Export enabled |
|---|---:|---:|
| plan wall | 2.362 s | 2.386 s |
| compact stdout | 35,681 bytes / 512 records | 35,778 bytes / 513 records |
| external plan | none | 1,491,596 bytes / 10,655 records |
| repository bytes written | 0 | 0 |

The external plan crosses the compact response's 10,000-record boundary without raising that
boundary or truncating detail. Linux `VmHWM` reported 4,792 KiB for the new release test process.
The encoder renders each record once through a checked meter, BLAKE3 state, and optional
file sink; it does not retain the complete file or clone the exported fact sets. The enabled sample
adds one file synchronization and one parent-directory synchronization in the owned writer path;
exact filesystem syscall telemetry was unavailable.

A copied release binary also measured one direct rename: planning plus a 3,721-byte/20-record plan
file took 7.190 ms, apply took 15.108 ms, and the project remained exactly 9,586 bytes with an equal
path/content inventory across planning. These are single warm-cache observations, not latency or
memory distributions. CPU time was unavailable because `/usr/bin/time` is absent; filesystem cache
state was uncontrolled. Raw structured evidence, exact artifact SHA-256 values, commands, and
limitations are retained in
`docs/evidence/20260826-review-bound-logical-change-plan.json`; raw logs and files are under
`.artifacts/campaign-202608261448/`.

## Current reviewed ownership-closure deletion

Campaign `202608261834` implemented the public closure path beginning at commit
`7afa63ea3643905745986a355721aa46cc80af67`. Measurements used Linux
`7.0.0-29-generic` x86-64 with Rust/Cargo 1.98.0. The retained locality test prepared the same
13-owner closure once in a 43-owner graph and once after adding 2,000 unrelated modules. Closure
selection observed equal logical work in both repositories:

| Dimension | 43 owners | 2,043 owners |
|---|---:|---:|
| selected roots / closure owners | 1 / 13 | 1 / 13 |
| ownership steps / relation edges | 12 / 45 | 12 / 45 |
| canonical point reads / decoded records | 14 / 14 | 14 / 14 |
| witness point reads / decoded records | 26 / 28 | 26 / 28 |
| owner edits / retirement edits | 13 / 13 | 13 / 13 |
| removed / added plan relations | 15 / 0 | 15 / 0 |
| structural / semantic / test owners | 13 / 12 / 1 | 13 / 12 / 1 |
| canonical / witness map pages | 13 / 26 | 27 / 40 |

This supports the scoped claim that closure discovery depends on the selected ownership and exact
relation evidence plus persistent-map locator costs, not every unrelated owner. The page difference
is retained rather than normalized away. The test log is
`.artifacts/campaign/202608261834/scale/locality-test.log` (SHA-256
`18137641b0c7d2d3e44a1b09034d108092b38d8d9e0caa25ba5f5636507a2e0a`); its bounded metrics file
has SHA-256 `fee3e87ca3fd43f117e7569093fdb92e12081a2332c4b247f0f513de14167974`.

The final release-scale command was:

```sh
LKJSCRIPT_OWNED_CLOSURE_EVIDENCE_DIR=.artifacts/campaign/202608261834/scale \
  cargo test --locked --release --lib \
  authored_owned_closure_scale_emits_complete_plan_under_default_admission \
  -- --ignored --nocapture
```

It created a fresh normalized repository containing one module and a shallow 8-ary tree of 1,500
inline-documentation descendants, selected the module, exported and strictly decoded the complete
plan, applied it, and reopened the accepted result under default admissions. The exact result
revision was `rev_bcc8e8c9332cc23ba05da01583a254a3466148421990171f0b74e1cd05615915`.

| Observation | Value |
|---|---:|
| closure owners / ownership steps | 1,501 / 1,500 |
| plan records / bytes | 6,021 / 2,797,359 |
| canonical point reads / pages / records | 7,506 / 19,769 / 6,005 |
| witness point reads / pages / records | 13,509 / 38,705 / 13,507 |
| relation edges | 12,000 |
| owner / retirement edits | 1,501 / 1,501 |
| validation owners / selected tests | 1,501 / 0 |
| staged objects / pages / bytes | 1,766 / 259 / 700,032 |
| repository bytes before plan / after plan / after apply | 2,239,880 / 2,239,880 / 3,297,220 |
| bootstrap / plan / apply wall | 135.939 ms / 1.906293 s / 50.253 ms |
| process peak RSS | 30,880 KiB |

The complete plan's BLAKE3 digest is
`d002769ee456ca065edb4e1b3e435046da2c791560a9e62c935923feef96a2d1`; its retained file SHA-256
is `a1db4c0663a6af010ba505a3e51c54fd5ebe7e96745ff81c25b3d8b85226e21c`. The final test log and
metrics SHA-256 values are respectively
`5a8dfddbd2aa111100bfceccce794c8748b52b8e60919e34824f1724dea39ac6` and
`ad6e5076a22271993fe9551b4b8f71ad6579a1899768c314b2f9b28b651c626a`.

Filesystem cache state was uncontrolled; “fresh” describes the repository, not the host cache.
Linux `VmHWM` is process-wide. The optimized target was warm from the preceding focused release
run; Bash reported 2.264 seconds real, 1.884 user, and 0.279 system for the final test command.
Per-test CPU time and exact filesystem syscall/synchronization counts were unavailable. This is one
admitted topology, not a maximum-size, latency-distribution, wide-fanout, or million-owner claim. An
exploratory 2,001-owner form exhausted the unchanged witness-byte admission, so it is not reported
as a pass. The measured initial candidate-relation path also rescanned the complete derived delta
per endpoint; the final implementation replaces it with one bounded, once-charged deterministic
index, with a focused exact-fit/exhaustion regression test.

## Current normalized semantic query locality

At implementation commit `8ea897f7307d9726e57710c833a1596a9dd74127`, the release test
`ten_thousand_owner_and_high_fanout_relation_pages_remain_logically_local` constructed 10,000
live normalized owners and 9,999 canonical relations across persistent-map page boundaries. The
fixture uses the normalized first-party in-process builder so public request generation does not
dominate the locality measurement; an independent copied-release-binary test covers public
correctness and no-write behavior.

The command was `cargo test --release --locked --lib
ten_thousand_owner_and_high_fanout_relation_pages_remain_logically_local -- --nocapture`. It ran
on Linux `7.0.0-29-generic` x86-64 with Rust/Cargo 1.98.0, a warm release build, and a new
test process. The process completed in 0.754 seconds real, 0.461 seconds user, and 0.170 seconds
system time; Linux `VmHWM` reported 77,660 KiB peak RSS. `/usr/bin/time` was unavailable,
so Bash's process timer supplied CPU time. Filesystem cache state was uncontrolled.

| Scenario | Wall | Output bytes / records | Returned | Map pages / bytes / entries | Catalog / objects / store bytes | Canonical / witness | Continuation |
|---|---:|---:|---:|---:|---:|---:|---|
| first owner page | 707 us | 1,680 / 13 | 5 | 4 / 23,693 / 39 | 9 / 9 / 24,366 | 5 / 0 | yes |
| middle owner page | 595 us | 1,724 / 13 | 5 | 3 / 24,070 / 48 | 8 / 8 / 24,815 | 5 / 0 | yes |
| terminal owner page | 539 us | 1,472 / 12 | 5 | 3 / 23,422 / 36 | 8 / 8 / 24,167 | 5 / 0 | no |
| empty filtered owner page | 909 us | 969 / 8 | 0 | 10 / 38,171 / 295 | 10 / 10 / 38,171 | 0 / 0 | yes |
| resumed empty filtered page | 856 us | 966 / 8 | 0 | 9 / 36,982 / 275 | 9 / 9 / 36,982 | 0 / 0 | yes |
| exact namespace find | 731 us | 857 / 8 | 1 | 9 / 28,674 / 137 | 11 / 11 / 28,900 | 2 / 1 | no |
| first kind-prefixed relation page | 528 us | 2,229 / 13 | 5 | 4 / 22,543 / 39 | 5 / 5 / 22,620 | 1 / 5 | yes |
| middle kind-prefixed relation page | 527 us | 2,229 / 13 | 5 | 4 / 22,753 / 49 | 5 / 5 / 22,830 | 1 / 5 | yes |

Executable assertions bind exact find to bounded point reads, descend continuation pages from the
exclusive logical lower bound, cap entries visited by the selected scan quantum plus lookahead,
use a relation-kind map prefix, forbid the full reconstruction and query-index writer paths, and
compare full pagination with an independently sorted canonical oracle. Both the fixture counter
and repository inventory report zero repository bytes written. Raw structured evidence is
retained at `docs/evidence/20260826-normalized-query-scale-10000.json`; the full log is
`.artifacts/campaign-202608260032/query-scale-release.log`, SHA-256
`a0b809e5ee6d27485bffd3286924c7ab5c1c92f72cd1a3af86f75b59f10b4f2e`.

This is one topology and one warm-build/process-cold observation, not a latency or memory
distribution. It does not establish cold-cache, million-owner, filesystem-call, or full public
fixture-construction performance. Output bytes are not provider tokens, API requests, or monetary
cost.

## Historical source and predecessor baselines

At campaign start `6747754bdcad4dc9000c3d7891db7ef207c8ec2f`, the former source-authority
`lkjournal` workflow had retained warm p50 observations of 44.718854 ms and 1,356 output bytes
for orientation, 76.677093 ms and 311 bytes for package tests, and 84.051330 ms and 40,322 bytes for
a service-module body. Source authority has been deleted, so these figures are useful only as
historical comparison points.

An older graph-product build emitted 37,897 bytes for one exact-function query, took about 998.5 ms
for orientation, and took about 730.6 seconds for its complete verification profile. Those
observations motivated bounded exact query work; they do not describe the current product.

## Historical meaning-graph-1 command measurements

Seven warm release samples were taken per command on 2026-08-22 before the graph-2/3 and CLI-v3
cutovers. The release binary was 13,399,896 bytes with SHA-256
`f7d985142b4e019d22c5d3ab8a2bdfcdb99b7043a663edd5be2379aafefd13ab`. Fresh unique output files
were used for build, review, and backup.

| Historical command, now rejected or renamed | p50 | Output bytes | Derived file bytes |
|---|---:|---:|---:|
| `lkjournal semantic status` | 1.863 ms | 712 | — |
| `lkjournal semantic orient --limit 20` | 3.642 ms | 1,710 | — |
| `lkjournal semantic find handle --exact` | 1.684 ms | 645 | — |
| `lkjournal semantic show decl_ec993… --body` | 2.863 ms | 5,613 | — |
| `lkjournal semantic test` | 22.101 ms | 418 | — |
| `lkjournal semantic doctor --deep` | 5.217 ms | 299 | — |
| `lkjournal semantic build` | 17.505 ms | 607 | 160,195 |
| `lkjournal semantic text-project` | 14.639 ms | 630 | 1,328,003 |
| `lkjournal semantic backup` | 11.141 ms | 476 | 160,697 |

Those spellings remain here only to identify the historical evidence. Substituting current CLI-v7
names or normalized query semantics into old rows would fabricate a current measurement.

At that point canonical standard authority was 21,062 bytes and canonical `lkjournal` authority
was 160,419 bytes. Disposable query indexes added 50,858 and 790,720 bytes respectively. Those
sizes describe graph contract 1 and are not current graph-4 storage claims.

## Historical graph-1 scale topology

The retained `docs/evidence/20260822-semantic-scale.json` fixture constructed many empty modules
under the predecessor public ID-allocation, artifact-import, and transaction workflow. It did not
exercise persistent Merkle pages, explicit generics, dense declaration/call topologies, or CLI v4.

| Historical observation | 10,000 added modules | 90,000 added modules |
|---|---:|---:|
| first 10,000-module apply | 462 ms | 469 ms |
| final 10,000-module apply | 462 ms | 10.949 s |
| apply response | 5,378 bytes | 5,378 bytes each |
| cold exact local-index lookup | 73.5 ms | 630 ms |
| warm exact lookup, work = 1 | 15.8 ms | 138 ms |
| exact show | 13.7 ms | 134 ms |
| orientation | 129 ms | 1.283 s |
| deep doctor | 93.5 ms / 10,024 module versions | 3.661 s / 450,120 module versions |
| deterministic build | 132 ms / 1,330,505 bytes | 1.269 s / 10,370,515 bytes |
| backup | 98.4 ms / 2,183,176 bytes | 3.995 s / 44,315,558 bytes |
| canonical store | 1,832,489 bytes | 41,163,949 bytes |
| store with derived indexes | 6,631,953 bytes | 82,844,955 bytes |

The graph-1 final apply grew to 10.949 seconds because acceptance reconstructed, cloned, validated,
and reverified the complete candidate. Persistent-root contract 2 has replaced the flat physical
root with path-copied Merkle pages. Graph 3 additionally implements local preparation for
pure-body changes, independent module creation, module rename, and declaration rename, plus persisted authenticated
summary deltas. Other changes retain complete logical preparation, and a cold index rebuild can
still be broad. The one retained graph-4 10,000-module sample below is not a scale curve and does
not establish impact-proportional end-to-end mutation. The historical 90,000-module curve must not
be used to claim that it does.

Graph contract 1 also had a 100,000-module root ceiling, so its million-module case was not run.
Persistent-root contract 2 removes that flat-root semantic count from its root shape, but
one-million-owner creation, lookup, update, doctor, backup, restore, RSS, I/O, and interruption
behavior remain unmeasured. Persistent page unit/property tests are correctness evidence, not
complete-workflow scale evidence.

## Current persistent-root locality property

The graph-4/root-2 working tree now has an in-process differential test that constructs 10,000-
and 100,000-module persistent roots, changes one module-object binding, compares the delta root
with a complete rebuild, publishes the retained pages into the accepted base, and reconstructs the
exact changed logical root. A counting store observes physical base reads beneath overlay write
probes. At both sizes the test requires fewer than 64 physical page reads, less than one quarter of
base pages and bytes, and fewer than 32 retained pages. It also bounds 100,000-module reads to the
10,000-module observation plus eight pages, retained pages plus eight, and bytes to twice the
10,000-module observation. The focused debug test passed in 4.62 seconds on the shared warm
worktree. These are executable asymptotic bounds, not a release CLI latency or exact I/O sample.

The staged-page tests also retain an interrupted-publication counterexample in which a generated
ancestor already exists physically but one generated child does not. The overlay keeps the reused
ancestor, extraction retains its reachable child, and exhaustive reconstruction succeeds. A
separate corruption fixture proves staged and exhaustive traversal reject a parent edge that does
not match the child prefix. Million-owner complete-workflow, RSS, filesystem-call, and cold-cache
evidence remains unavailable.

One fresh-temporary-project release workflow then created 10,000 empty background modules through
one public change, created one local module, renamed it, queried it, ran deep doctor, built, and
backed up. Raw structured evidence is retained at
`docs/evidence/20260822-graph4-scale-10000.json`. The 10,000-module batch took 44.913 seconds and
returned 794,397 bytes. After that background, one-module creation took 22.781 ms and module rename
took 45.653 ms; each checked one module, but their store deltas wrote 842,666 and 843,519 derived
index bytes respectively. Exact lookup reported semantic work one in 2.397 ms for the first process
and 1.966 ms for the second. Orientation took 165.291 ms, deep doctor took 281.345 ms over four
revisions and 30,006 retained module versions, build took 199.282 ms, and segmented backup took
21.216 seconds for 3,053,899 payload bytes.

This is one warm-build, uncontrolled-filesystem-cache sample, not a distribution. It demonstrates
bounded local semantic validation and exact lookup, while exposing two prerequisites before larger
public runs: inefficient 10,000-operation bulk construction and a revision-bound semantic index
whose rewritten bytes grow with total modules.

After batched persistent-map mutation, single-use prepared root publication, and semantic-fact
contract 3 replaced those paths, the same public 10,000-module workflow was repeated from a fresh
temporary project at implementation commit `8ec09e24efc9968d900cfd3a4fa9ef63035a06d8`. Raw evidence
is retained at `docs/evidence/20260822-graph4-fact3-scale-10000.json`. The batch took 1.160 seconds
and returned the same 794,397-byte class of allocation receipt. Local module creation and rename
took 22.683 ms and 36.486 ms, checked one module each with full-oracle equality, and wrote 51,263
and 52,116 derived-index bytes. Exact lookup reported work one in 2.354 ms cold and 1.756 ms warm;
orientation took 155.361 ms, deep doctor took 316.332 ms across 30,006 retained module versions,
build took 203.360 ms, and segmented backup took 21.162 seconds for 3,052,823 payload bytes.

Compared only with the preceding single sample, the observed batch elapsed time fell by 38.7x and
local derived-index bytes fell by about 94%. These are equal-topology point observations, not a
latency distribution or a general performance guarantee. Backup remains the dominant measured
operation. A 100,000-module attempt reached backup, but its final JSON was lost when the completed
execution session exceeded the orchestration capture context and could not be reopened; it is
classified unavailable rather than pass. The one-million-owner public workflow was not run.

## Current artifact-10 resident deployment

The maintained `lkjournal` bundle remains 685,766 bytes and contains two packages, 2,280 closure
objects (465,145 object bytes), 120 compiler units, and one artifact segment. Strict reload observed
the same 2,280 objects and object bytes. Its manifest is
`artifact_manifest_97447a36407a29bb2b979ac42191d774334e661d799f65399b6eba904d593834`
and bundle identity is
`artifact_bundle_f29713ff437662d63d9b93514c97b007815c43f185a9eba4498ecb700a276501`.

The local campaign host freshly rebuilt and byte-compared that bundle, then classified live service
acceptance unavailable because Docker was absent; the preliminary run took 1.192 seconds and is not
a service latency result. The supported copied-binary HTTP/worker/PostgreSQL observations, including
readiness, per-request, worker, shutdown, CPU/RSS, backup, object, authority-inventory, and evidence
sizes, are owned by
[`202608272159-artifact10-service-cutover.json`](evidence/202608272159-artifact10-service-cutover.json).
Filesystem cache state, scheduler noise, container startup, PostgreSQL behavior, and hosted-runner
sampling remain uncontrolled point-observation limitations. No SLO or provider-cost inference is
made.

The assembled release-mode product candidate was 15,386,696 bytes, 1,118,240 bytes smaller than
the published `v0.1.4` executable observation. Its preliminary product profile passed 16/16 fresh
gates in 240.796 seconds with no reuse. This size difference is contraction evidence, not a general
binary-size trend; the exact final full profile remains the completion authority.

## Current distributed HTTP application candidate

The 0.1.7 release-mode executable measured 15,463,656 bytes with SHA-256
`033ad8a5cf52a06c67facd912d8abdca35b194ee9b9ee0804d6396cab29b352e`. It is 73,248 bytes
(0.48%) larger than the immutable v0.1.6 executable observation. The candidate was copied once to
a fresh temporary root outside the checkout and completed 23 bounded child commands plus two
resident runners in 1.146 seconds. The environment contained only `LANG`; no database, container,
Cargo invocation, checkout input, network registry, or product-side helper participated.

On this Linux x86-64 host, HTTP project creation took 97.504 ms wall / 30 ms sampled CPU with a
6,496 KiB peak RSS observation. Reviewed plan and apply took 44.609 ms / 20 ms / 7,220 KiB and
65.239 ms / 30 ms / 7,576 KiB respectively. Check took 65.188 ms / 50 ms / 8,020 KiB. The
exact-current build reused all six compiler units in 64.574 ms / 40 ms / 8,012 KiB; after deleting
the disposable cache, the clean build compiled all six in 74.921 ms / 40 ms / 8,124 KiB. Their
194,077-byte Artifact 10 outputs were byte-equal with SHA-256
`c42440341eb85e7843840baa9b10a8f16f7b685285ab4441ea9636995afd9f62`.

First and restarted readiness took 25.138 ms and 25.135 ms. Raw loopback requests took 0.445 ms
and 0.424 ms and both returned the same 30-byte body digest. Graceful shutdown observations took
5,170 ns and 1,323 ns after admission stopped, with zero remaining tasks or cleanup failures. The
largest sampled child peak was 8,464 KiB; sampled child CPU totaled 380 ms. Retained stdout/stderr
logs totaled 20,457 bytes, all stderr streams were empty, and six invalid artifact/startup forms
produced no ready event. Exact accepted authority remained 9 files / 164,287 bytes before and after
build, serving, requests, shutdown, and restart.

These are single warm-filesystem point observations with 10 ms CPU sampling resolution, not an SLO
or cross-host distribution. Exact filesystem operation counts, synchronized bytes, request VM
instruction counts, context switches, provider telemetry, and cost were unavailable without adding
an invasive measurement boundary. The complete identities and per-command observations are in
[`202608281025-distributed-http-application.json`](evidence/202608281025-distributed-http-application.json);
bounded raw logs remain under the receipt directory named there.

## Historical compiler, service, and verification receipts

Before the current cutover, graph packages passed 6 standard and 11 `lkjournal` tests with
bytecode/reference equality. The PostgreSQL service/worker acceptance completed in 4.431 seconds at
`.artifacts/service/20260822T061313.861237Z-420089/receipt.json`, covering 13 route/failure
observations, update/history, a 200,000-byte object, one worker iteration, shutdown, PostgreSQL
backup/restore, restart, and an equal restored read. It used plaintext HTTP and PostgreSQL
`NoTls`. It is one historical complete observation, not a current latency distribution.

The predecessor full profile passed 15/15 gates in 94.743 seconds at
`.artifacts/check/20260822T061143.565460Z-417922/receipt.json`, bound to worktree input SHA-256
`d822a652537d07e1859f0faea1e481b6d0e037f93f0eab3452416147074d0910`. It is not proof for the
current tree.

Current `lkjscript-dev check` models gates and retained exact inputs separately and labels fresh versus
reused evidence; the authoritative full policy requires fresh execution. A working-tree graph-4 /
CLI-v4 full profile passed 17/17 fresh gates in 6.078 seconds at
`.artifacts/check/20260822T131807.529974Z-726352/receipt.json` after a first run exposed and retained
an executable-writer DAG race. This is not yet final commit-bound or fresh-checkout evidence. No
current claim is made for provider cost, correction depth, startup, RSS, incremental compilation,
or million-owner complete-workflow performance. The one measured working-tree release binary was
15,031,768 bytes; it is an observation, not a size regression curve.
