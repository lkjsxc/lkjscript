# Current status

Snapshot: 2026-10-10. Native library delivery, source acceptance, exact executable
proof, finalized publication and running applications remain separate boundaries.
Product identifiers are opaque.

## Accepted source and mainline delivery

[PR #11](https://github.com/lkjsxc/lkjscript/pull/11) is normally merged. Exact
accepted source `92330dc65151687ff2d23a07263ce1ce8ebfe2b0` is the second parent of
mainline integration `eb421a6990f752b9a093df2d2fa8212a718fd1c1`. Both have tree
`53da0f9d0df7d3b23cd902ad9c71067a524876e5`; independent Git and GitHub reads agree.
The first parent is preceding main `7fd13b5f4c0c22357df6f7fdc2d6c66e1cbf6dd9`.
Later documentation descendants report this source; they are not newly tested
runtime implementations. No force update or branch/protection change was used.

The [native fold library](../examples/map-entry-projection/consumer-fold/README.md)
exports ordinary pure generic `fold-four<Item, State>` and `fold-eight<Item, State>`.
Independent consumers test order-sensitive scalar state and nested persistent
histories retaining every earlier list prefix. Empty input preserves the supplied
initial state. Full raw admission, exact imported contracts, callback failure,
no partial output, recovery and joined cleanup remain required.

All seven maintained fold tests passed in focused and whole-source execution and
against the exact host Release executable. The history matrix alone has 180
complete results: fifteen lengths, two initial states, three implementations and
attached/source-deleted phases. Complete prefixes are checked by an independent
prefix-slicing oracle, not by replaying the implementation's recurrence.

This is a native library alternative, not a new intrinsic, unchecked cursor,
affine fold, JIT or unconditional replacement of the standard fold. Relative to
preceding main, no kernel/runtime, standard supplier, manifests, product identity
or semantic encoding changes were introduced. The existing bulk-List optimization
is retained. README and [direction](direction.md) now explicitly describe eventual
self-reliance and complete self-hosting without claiming that they are complete.

## Fresh source verification

The original `check full --fresh --jobs 1 --machine` returned **exit 0**:
**26/26 fresh gates passed**, zero reused or unrun, no failed gate. Every gate
process returned zero and initial/final source input digests match. Source stayed
clean at the exact accepted HEAD/tree. Cargo jobs were independently set to two,
with CPU affinity 0–7 and the prescribed immutable Release-profile verifier.

Workspace top-level suites passed **2,275**, failed **0**, and retained **29 existing
ignored cases**. The two successful filtered child probes are separate. Included
counts are 1,703 library tests, 244 public CLI tests and 262 contributor-tooling
tests. The standard/application artifacts and generated surfaces also passed
their separate maintained comparison gates.

Original run `.artifacts/lkjscript-dev/check/1791583803157171502-2605146-0/` is in
`/home/coder/workspace/lkjscript-native-fold-study-20261009`.
Receipt digest:
`verification_7fa73433aa5cd714d23749c003ec026d2130c2b8be4bd16afb14b3f6c9f4af3f`.
Stable input digest:
`verification_3f7d5902722a669bac334b11667c16404274966ec90d1112d76a44fc83ebd875`.
Every original file is also archived under the evidence root's
`full-source-archive/`; complete original/archive SHA256 inventories are equal.
Fourteen reusable-evidence cache writes failed, but all actual fresh processes
and gates passed. No cache-reuse success or cause of those bookkeeping failures
is inferred; original evidence remains available independently of that cache.

## Exact executable acceptance

The updated public owner independently completed against the exact host Release
executable: **76/76 required cases passed**, zero failed/ignored, complete inventory,
unchanged inputs and joined cleanup. It selects 76 of 245 enumerated cases, including
all seven native fold tests and the complete Map execution matrix.

Host executable: 33,246,488 bytes, SHA256
`eb49f70d8822e45a8b3942add79c5a635f6398bc4b05b563a594f26179ebfbdb`.
Source-matched copied harness: 595,874,192 bytes, SHA256
`4fabf4993412b8ab7fa653434bbe21d19a75cff2278ef2dcc80e4a1f9984d429`.
Verifier: 42,918,088 bytes, SHA256
`3cb2998cffdd1eeed8f22065bbd49f309dc8d7f077293570470c7ae8290e73e3`.
The original owner receipt/logs are in `host-public-owner/` at the evidence root.

A separate negative candidate-selection control ran the five exact native behavior
cases against a deliberately failing non-product sentinel. All five reached that
sentinel and failed with original test exit 101; none silently used a development
binary. A subsequent real public-product case passed. Harness, product and sentinel
identities remained unchanged. These expected negatives are not failed product
acceptance. Originals are in `candidate-binding-proof/`.

The same current-source harness separately completed against the anonymously
acquired exact public-v0.1.89 executable: **76/76 passed**, zero failed/ignored,
unchanged identities and joined cleanup. All five native behavior witnesses and
both oracles passed on this older public runtime too. The original terminal exited
zero; evidence is in `public-release-owner/`. This proves source-example
compatibility with those public bytes, not that the later List optimization or
new verification generation was retroactively included in the old release.

## Matched consumer observations

The [new paired host study](../examples/map-entry-projection/consumer-fold/experiment/host-comparison.md)
completed **400 executions** under the same native program, inputs and artifact:
two host Release executables, four sizes, five strategies and ten outer runs.
An independent reader checked every result, complete Cartesian inventory, phase
clock, joined cleanup and unchanged input; forty warm-ups remain retained.
Both the original measurement and its separate reader returned zero.

Within the current executable, the large standard fold and blocked-eight medians
are 494.868583ms and 366.408958ms, about 26% less invocation time for the latter.
Across old/new executables, results are mixed and the blocked-eight route is slower
in the successor. Empty inputs also favor the standard fold. Do not equate lower
construction counts with a uniform runtime speedup or these scalar timings with
a nested-history/whole-application benefit. The full table retains adverse cases,
preparation, accounting and RSS limits at their original measurement owner.

## Distribution and compatibility

The independently observed immutable public/latest release remains **v0.1.89**,
selecting Map source `1b7e95badb13e5ddcc9d557027c1977fd4f2a271`. It does not contain
the later bulk-List optimization or install this source example into the standard
package. Public executable SHA256:
`e24c054f9b3e44a71a8875aba5385aad82dc391a1eae88a6c115fca75bc5c9a9`.
Candidate `37910899478/1` and promotion `37925038733/1` retain their authentic
`immutable_published_and_public_verified` generation-4 history. Prior complete
publication reporting remains at `7fd13b5f:docs/status.md` and its original evidence
checkout `lkjscript-map-release-final-report-20261009/.artifacts/final-report/`.

Candidate acceptance/workload **generation 5** now requires five exact fold behavior
witnesses and every present/future case of `native_declarations::native_fold::`, in
addition to unchanged Map/other native families. Three tests first failed on the
unfixed selector; the corrected complete tooling suite passes. Predecessor and mixed
acceptance generations reject without relabelling earlier genuine publications.
A future runtime distribution requires an unoccupied product identity, integrated
source, new finalized-byte acceptance and unchanged-asset promotion. No producer,
publication-control, installer, runtime-selection or application deployment was
performed in this continuation.

## Known issue and next engineering boundary

[Issue #9](https://github.com/lkjsxc/lkjscript/issues/9) remains **unfixed**. Five
fresh public-release controls establish that a component with ports reproduces
`compilation_incremental_owner_domain` without any Command target. Function-only
creation updates the cache; one/two-port cases fall back to clean checking. Every
semantic apply and all 97 imported/template tests pass; new targets return exactly
`1`. The counts are consistent with one extra planned unit per port, but are not
proof of the exact internal leaking OwnerKey. The compiler guard was not weakened.
See the issue's retained observation table and `cache-shapes/RESULTS.md`.

Next isolate the compiler impact-owner projection before changing cache admission,
and use matched consumer observations to select traversal/callback work rather
than extrapolating speed from node counts. Small inputs, larger retained states,
prepared-call costs and region/view contracts remain separate questions. Historical
scalar timings retain their original workload and executable bindings.

## Evidence and preserved originals

Evidence root:
`/home/coder/workspace/lkjscript-native-fold-study-20261009/.artifacts/fold-completion-20261010/`.
`SOURCE-ACCEPTED.md`, `MAIN-INTEGRATED.md`, full originals, host/public owners,
negative controls and literal requests retain their own identities and outcomes.
The verification checkout stays frozen at `92330dc6`; the separate
`lkjscript-native-fold-report-20261010` worktree owns reporting descendants.

The earlier full fold attempt remains a failure: exit 1, 20 fresh passes of 26,
workspace-tests nonzero, run `1791533872978449672-1656264-0`, receipt digest
`verification_8acbd254f323c3f362e7291956342ad870095a944708891760633da931297f33`.
Scoped denied inspections recorded in `scoped-read-limitations.md` were not
repeated through another route, and their unavailable details are not invented.
The original native study, Map/List proof, failed candidates, immutable publication
history, frozen root at `b6a262f2`, other worktrees, stash and applications remain.
