# Current status

## Accepted source and mainline delivery

Published **v0.1.89** selects the Map predecessor described below. Direct immutable
List construction is accepted at source
`418cb56fef2fec63477f118e17f6a8458a0089a2`, tree
`a2201001c22c5d3d3036aa86e929cd8c8abacb03`.
Remote main was normally fast-forwarded to this exact source and independently
reread through both Git and the GitHub Git-ref API. A later status-only descendant
is a report, not a newly tested runtime source.

This source combines bulk-list implementation `d33760e9` with main `1b7e95ba`,
including the serialized release-source workflow. It retains the checked Map
projection and complete final-byte Map coverage from PRs #7 and #8. The merge
changed only workflow/status files relative to the bulk implementation; runtime,
tests, manifests, native inputs and maintained generated assets were unchanged.

## Behavior and compatibility

Private `List::from_items` now assembles each final immutable node once instead
of repeatedly copying paths for unpublished intermediate prefixes. The existing
32-way topology, persistent append, retained payload sharing, indexing, traversal,
stack-safe cleanup and complete independent raw admission remain in place.
No public signature, semantic encoding, generated discovery, storage-height or
logical-length limit changes. No data migration is required for this mechanism.

Reservations still precede modeled growth, including the final cancellation
checkpoint on empty and nonempty inputs. Cumulative construction charges decrease;
a budget sufficient for the final tree may now succeed where redundant prefix
construction exhausted it. This intentional resource-cost change does not relax
configured limits or grant type, ownership, capture or effect authority.
See the [decision](decisions/bulk-list-construction.md).

The unchanged scalar reporting workload checks complete values in both products.
At one million elements, constructed nodes fall from 123,940 to 32,259, existing
branch-handle copies from 1,486,250 to zero, and modeled cumulative construction
bytes from 138,703,200 to 113,032,520. The final retained topology is unchanged.
These are not elapsed speed, RSS, actual allocator traffic or application gains.
Original before/after results and limitations are in the
[performance guide](performance.md#direct-bulk-list-construction-2026-10-09).

## Fresh source acceptance

The original `check full --fresh --jobs 1 --machine` process returned **exit 0**:
**26/26 fresh passed**, zero reused, no unrun gates, no failure. Its original
receipt was separately read and every selected gate reports a fresh zero exit.
Initial and final input digests are identical:
`verification_ab565756839ef96c295981acd7887ce460a52b4d6341b40688d85d4eb9a6d4d1`.
The source stayed clean at the accepted HEAD/tree through completion.

The workspace library passed **1,703** tests with eight existing ignored cases;
public CLI passed **237** with one existing ignored case; contributor tooling
passed **259** with 19 existing ignored cases. All had zero failures. Standalone
Map tests passed 2/2. The earlier focused List group passed all **17** tests,
including nine new construction/ownership regressions. Standard and application
artifacts were also independently compared byte-for-byte with maintained outputs.

Evidence checkout: `/home/coder/workspace/lkjscript-list-bulk-20261009`.
Original full receipt:
`.artifacts/lkjscript-dev/check/1791542382380955892-2173253-0/receipt.json`.
Receipt digest:
`verification_cffc697d6b72cecc98607070221bfa1582d7c5dcb36bda8d0c5dda37f2912c15`.
Original terminal observation:
`lkjscript-bulk-final-public-tests-join-20261009-698cb4`.
The immutable Release-profile verifier is 42,917,096 bytes, SHA-256
`77cd04d68ca63c22cf8c66ea7d4fd19cda41fe7a91a747dbbc3e77a8341fae57`.
The run independently selected two Cargo build jobs, one checker job and CPU
affinity 0-7, without weakening cases, receiver bounds or deadlines.

## Exact host Release-byte proof

After full completion, the source-matched Release public harness and exact host
product were copied outside the build paths. Both Map-family tests passed with
an explicit candidate path and unchanged executable identities. The retained
matrix has **96 paired cases / 192 complete result executions**, independent
ordered-map expectations, full input admission, deleted authoring project and
transport, malformed-input/resource refusal, recovery and joined cleanup.

Host product SHA-256:
`eb49f70d8822e45a8b3942add79c5a635f6398bc4b05b563a594f26179ebfbdb`.
Copied harness SHA-256:
`e7e2aea82a519c9622b7e42fe1e6cf625e916597fbfa9eec4f9da563e980526a`.
Original commands, logs and executable copies are in `.artifacts/list-bulk/release-map/`;
the source-deleted proof remains `/tmp/lkjscript-components-1p8vOr`.
This is host Release-profile evidence, **not finalized musl archive acceptance**.
The final-candidate owner still requires all native families under acceptance
contract/workload generation 4; no family, candidate-binding or inventory check
is removed by this change. See [release acceptance](release.md).

## Published Map release and successor boundary

**v0.1.89** is the immutable ordinary public/latest release:
[release and notes](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.89).
Its accepted product source is `1b7e95badb13e5ddcc9d557027c1977fd4f2a271`,
with annotated tag object `ff39871407a911853c38ed17e38606f4655e4cee`.
Local/remote tag objects, exact source and verbatim Markdown annotation were
independently compared before the scoped publication selector was updated and
read back. Existing immutability and protection settings were preserved.

[Candidate 37910899478/1](https://github.com/lkjsxc/lkjscript/actions/runs/37910899478)
completed successfully under final-candidate acceptance contract/workload 4:
20/20 fresh source gates, six target owners, two pinned userlands, installed
recovery, original receipt readers and joined cleanup. Its final archive's
executable SHA256 is `e24c054f9b3e44a71a8875aba5385aad82dc391a1eae88a6c115fca75bc5c9a9`.
The required public harness executed all 69 selected cases once: 69 passed,
zero failed or ignored, including the exact detached Map matrix and independent
cost oracle. The complete matrix retains 192 complete result executions, deleted
sources, full raw input admission, refusal/recovery and cleanup. Its copied
candidate identity matches the executable extracted from the accepted archive.

[Promotion 37925038733/1](https://github.com/lkjsxc/lkjscript/actions/runs/37925038733)
completed with `immutable_published_and_public_verified`: authority authorized,
publication and public verification successful, latest selected v0.1.89 and its
exact accepted source. The controller came from main `1dcd76ab`; it executed zero
product builds. Public bootstrap installation and the create/edit/build/run
lifecycle passed in the separate read-only job with publishing credentials absent.
Independent anonymous exact-tag, latest and release-ID reads agree on ordinary
immutable release `407859944`; all three downloaded assets compare byte-for-byte
with the accepted candidate.

The archive, SHA256SUMS and install.sh were promoted without rebuilding. Their
respective lengths and SHA256 values are:

| Asset | Bytes | SHA256 |
| --- | ---: | --- |
| lkjscript-x86_64-unknown-linux-musl.tar.gz | 15300391 | `3b4ced16faa94142c3bcf70098be4168a462dc661d0d3095089dcfd9b0860418` |
| SHA256SUMS | 109 | `307cb99a220fe33d63be2d61e7668a733162cc8fa2abaa3c32331beb7bc96ae5` |
| install.sh | 3566 | `4399604f625b77da9e35eb2dff21a1bceb69f4daff6a4df22b4ec605eb2f4b8d` |

Original candidate/publication artifacts, anonymous acquisition and final report
checks remain in `/home/coder/workspace/lkjscript-map-release-final-report-20261009/.artifacts/final-report/`.
The accepted Map correction `4fba5833` and serialized workflow follow-up `8f02fcf1`
each have separate fresh 26/26 full-check acceptance; their retained original
receipts are respectively `1791523334550838046-1232059-0` and
`1791534915673731595-1782877-0` in the Map checkouts below. Original diagnostic
regressions returned 0 passed / 3 failed / exit 101 before correction and all
three passed afterward. A previous full attempt on `8f02fcf1` failed two existing
timing-sensitive cases under concurrent host load; its originals remain under
`.artifacts/release-report/serialized-full-failed-originals/`. The successful full
used bounded local parallelism; tests, assertions and deadlines were unchanged.
Failed candidate `37892791430/1` preceded source-gate serialization and is not the
accepted producer. Older candidate `37643217328/1` was never substituted.

The later bulk-List source described above is integrated but is **not included in
v0.1.89**. Its future distribution requires an unoccupied product identity and a
new integrated-source candidate with complete final-byte/public acceptance. Do
not relabel v0.1.89 or its predecessor proof. No application deployment was made.

## Retained failures and continuation owners

The first full attempt used an incorrectly selected 697,029,128-byte Test-profile
verifier, exceeding unchanged receiver bounds. It returned exit 1 with 21 fresh
passes, four failed gates and one unavailable gate; workspace tests passed and
source was stable. Its receipt remains
`.artifacts/lkjscript-dev/check/1791537857837514206-2011202-0/receipt.json`.
This is a failed original, not a deliberate negative control or reusable full
acceptance. The successful run above uses the prescribed Release verifier.

The regression-only predecessor, immutable test binaries, every refusal/cleanup
original and supplemental inspection limitation remain under `.artifacts/list-bulk/`.
The current completion owner is `.artifacts/list-bulk/integrated-full/COMPLETED.md`.
An unused supplemental inspector has a known receipt-schema mismatch; its scoped
edit refusal was not retried, and it supplies no acceptance evidence. Earlier
scoped log-read refusals remain recorded rather than replaced with invented detail.

Previous Map acceptance remains in `/home/coder/workspace/lkjscript-map-public-20261009`,
including original run `1791523334550838046-1232059-0`, `.artifacts/map-public-acceptance/`
and `.artifacts/map-release-20261009/`. Original frozen runtime checkout
`/home/coder/workspace/lkjscript` remains at `b6a262f2`. Failed candidate
`37892791430/1`, unrelated worktrees/stashes, native-fold work and running
applications remain preserved. Only this task's unused incremental compiler cache
was retired to recover disk capacity; source and original evidence were retained.
