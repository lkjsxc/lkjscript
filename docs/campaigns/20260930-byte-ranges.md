# Immutable byte ranges: implementation and verification

Date: 2026-09-30 (Asia/Tokyo). Entry main: `53198b30b8f8b946c0887b448a016a594b58f529`.
Development version: **0.1.61**. This record does not publish a binary release.

## Selected boundary

Add two ordinary pure `Bytes` operations: `bytes-slice(bytes, start, end)` selects
`[start, end)` with strict `0 <= start <= end <= length`; `bytes-copy(bytes)` produces
an independent backing containing only the visible bytes. Byte offsets need not
align with UTF-8 characters. Invalid bounds fail rather than clamp or return a prefix.

The implementation is deliberately narrower than a new owned/borrowed type or region
system. A byte view is an immutable value. Its backing may remain much larger than its
visible range. An explicit copy lets callers end that retention when other aliases
no longer need the parent. No speed, RSS, live-memory quota, secure erasure or
whole-program zero-copy claim is made.

Production checked slicing uses a flat window over either immutable shared bytes or
a concatenation vector. A window never owns another window. Unique descriptors may
shrink in place; strong and weak aliases prevent metadata mutation. Full nonempty
ranges retain their payload; empty ranges do not retain the original backing. New
window descriptors are admitted before allocation. Copying admits its descriptor
and payload before allocating, checks cancellation between reservations and during
64 KiB chunks, and moves the resulting vector without another payload copy.

The reference evaluator and explicit Core hosts materialize equal values through
independent range checks. They do not call the optimized window algorithm. Allocated
work can therefore differ between evaluators. Logical value admission, content-based
ordering, map keys, captures and encoded bytes remain unchanged. Existing byte-copy
work counters describe concatenation only; they are not extended into a global metric.

The closed external signatures are exactly `(Bytes, I64, I64) -> Bytes` and
`(Bytes) -> Bytes`, both pure. The `closed_external_signatures` validator feature
advances from 3 to 4 so older validation witnesses do not silently authorize a changed
set of accepted external declarations. No graph, type, artifact or data encoding is
added. The source forbids unsafe Rust as before.

## Recorded experiments

Logs are retained under `.artifacts/20260930-byte-slices/` on the development machine.
They are observations, not regenerated historical fixtures.

- **Predecessor:** the new literal slice fixture failed on entry implementation with
  `intrinsic_unknown` for `core.bytes.slice`; one failed, zero passed. The test-only
  patch and completed log are retained as `predecessor.patch` and
  `predecessor-completed.log`. This was an expected absence-of-capability result.
- **Private storage:** seven tests passed. They cover both backing forms, full/empty
  ranges, 10,000 nested slices with one descriptor and flat backing ownership, strong
  and weak aliases, release of a 1,000,000-byte parent, concatenation, ordering,
  reservation refusal and cancellation. `focused-storage.log` records the run.
- **Byte-related unit selection:** 67 passed, zero failed, in 3.80 seconds after
  compilation. `focused-bytes.log` includes all 969 valid ranges over lengths 0..16,
  invalid/extreme bounds, independent values, pointer identity, exact allocation
  boundaries and joined cleanup. At the failure boundary, the external call has
  actually started; zero remaining frames, transactions and handles are asserted.
- **Native integration, first attempt:** one passed and two failed because the new
  consumer example lacked one closing parenthesis. The source diagnostic was
  `change_block_parenthesis`. The fixture was corrected; the failed log is retained
  as `native-ranges-first.log`, not rewritten as a successful run.

- **Corrected native integration:** all three tests passed in 27.39 seconds;
  `native-ranges-second.log` retains the result. This includes source-free artifact
  execution, package/generic/capture/map/codec composition and negative public inputs.
- **Clippy:** the complete workspace and all targets passed with warnings denied
  (`cargo clippy --locked --workspace --all-targets -- -D warnings`, 24.60 seconds).
- **Generated reference:** `capabilities --generate-docs docs/generated` completed
  using the rebuilt executable with the new exact embedded standard.

- **Retained optimized executable:** all three native integration tests also passed
  in 25.05 seconds against the copied release-profile v0.1.61 executable, from
  `/tmp` with an empty environment/PATH. Its capability digest is
  `42e79f7e666ec665c523fb265fdd729fca14c425ffe40cfb6710d2e6b75faf04`.
  `installed-native.log` records the run; a byte comparison confirmed that the
  retained executable is the release build used for this source.
- **Two-way predecessor boundary:** the retained entry v0.1.60 executable refused
  the new standard transport with `intrinsic_unknown` for `core.bytes.copy`, exit 2,
  without changing the receiving HEAD. Conversely, a command project authored by
  that v0.1.60 executable passed all 76 tests in the v0.1.61 runtime with equal
  evaluators and unchanged HEAD; its old exact standard was not implicitly upgraded.
  See `predecessor-stage.log` and `old-command-new-runtime.log`.
- **First fresh full run:** source `2b2b5743d2b3fac3bfdd4fb86bdedf04da69d816`,
  tree `dee6c903a02fd1c1e07d42aa88365a2020c50df6`, failed after 597.586 seconds:
  6 fresh gates passed of 26 selected. Core tests reported 946 passed, 3 failed,
  8 ignored. The three failures were fixed pre-addition expectations: standard
  inventory 1,454 rather than 1,558, tests 75 rather than 85, and the preceding
  closed-signature validator digest. The exact assertions were updated, not removed.
  The failed receipt is retained at
  `.artifacts/lkjscript-dev/check/1790713995944561729-3265302-0/receipt.json`,
  digest `verification_1e7d9ee8cfbd5b1b9b29794fb5748f080b3ddc79d4ed7d98c293f282da8799f1`.
  A follow-up audit updated current-standard contributions in fresh native web,
  policy, command, text and package probes. Fixed historical fixtures retain their
  original counts and dependencies; the newly expected ten tests are not inferred
  from a passing runtime report.

## Maintained standard

The request `packages/standard/requests/20260930-byte-ranges.lkjc` was planned and
applied through the public native executable. It creates two external declarations
and ten graph tests: 104 new owners, no updated/deleted owners, no new type records,
no dependencies or retirements. The preceding 1,454 owners remain intact.

`check` passed **85 tests**, zero failed, `differential=equal`, with 3,642 production
instructions and 2,161 reference expressions. The resulting standard has 1,558 live
owners and 222 compiler units. Public export produced a 561,791-byte transport and
an 848,866-byte artifact. Generated assets are copied from those public outputs,
not written by a privileged semantic builder.

Exact result:

- semantic revision: `rev_7a13416f398fceb35d246500dd02b0f02ab7931347bb21cb7f6841ad8b65556a`;
- package revision: `package_revision_97275d57a6addc7a574b6114868c5dd736ad471e66b1fa61b2f5a7530b981603`;
- transport: `package_transport_1f2b61a31ae16e186014beb8eb30a5609d6ed77176d8063de62d130691d70b78`;
- artifact: `artifact_bundle_fce7ce9c39b0b899ca12768fedf47c566eede9d8a09b620dd20042254a9161be`.

## Native composition scope

Literal fixtures live in `tests/fixtures/byte-ranges-library.lkjc` and
`tests/fixtures/byte-ranges-consumer.lkjc`. The library implements generic
`map-range<Output>`, a captured-value function and a one-byte-length packet parser.
The consumer checks captures, explicit copies, text callbacks, concatenation,
content-based map lookup, nested ranges and typed-data round trips.

The public test harness copies the executable to an unrelated directory with an
empty environment/PATH. It authors and checks the library, exports its transport,
stages an exact dependency in a separate consumer, drafts accepted meaning and builds
an artifact. The detached phase removes both projects, requests, exported transport
and draft before running the artifact. Five binary range cases and three packet cases
are compared with independently constructed expected outputs. Rejected ranges and
inputs must publish no result and must not change the accepted project revision.

## Acceptance and delivery

The focused implementation and corrected native tests have passed. Full acceptance
is the next validation step. No main commit, fresh full-profile success or installed
optimized result is claimed by this intermediate record.

A batch expansion of the manually maintained status/specification/guide pages was
refused by the tool service and was not applied. This record and the generated
capability reference identify the new development surface; the earlier status page
must not be interpreted as a complete inventory of this change.

The separate frozen v0.1.60 candidate `36617982924/1` at `2962c43f` completed its
workflow successfully at 05:11:11 JST. That is not a claim of v0.1.61 acceptance,
public promotion or anonymous installed acquisition. Existing releases and running
applications are not changed by this implementation.
