# Owned storage admission — 2026-10-01

## Initial mandate (unchanged)

[https://github.com/lkjsxc/lkjscript](https://github.com/lkjsxc/lkjscript) について進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。

## Reconciliation and selected boundary

Both the main checkout and the independently fetched GitHub main started at
`9afa799794ac26fa90bd4b2413e6e4d26886ccb6`, tree
`3f190685967112d9df230fff5872ebf9e4a6e228`. The existing detached
`lkjscript-owned-generics-20260930` checkout has the same source and its own
build/evidence directory. It is reused for this continuation; main's two unrelated
untracked files, the historical recent-history stash, other worktrees and running
services are preserved. Rust is pinned to 1.98.0; builds use four Cargo jobs.

The predecessor's final full-profile receipt was present but not reflected in
the status summary. Its exact source, stable-input flag, 26 fresh passes, zero
reuse and empty unrun list were checked directly. This completes that source's
acceptance and mainline integration; it is not acceptance of later modifications,
a new binary publication or a service upgrade. The predecessor campaign retains
the full observation and all earlier failed runs.

The next long-term boundary remains useful, composable owned data and structured
transfer, not a second program-meaning authority. Before extending it, source
inspection found that ByteBuffer creation's allocation reservation counted the
token and Vec descriptor but omitted synchronized storage, loan bookkeeping and
shared-control metadata. The neighboring scalar carrier already includes those
costs in its modeled reservation. This continuation first closes that actual
runtime-accounting defect and its pre-allocation cancellation boundary.

No language syntax, ownership rule, canonical encoding, validator generation,
accepted application/standard meaning or deployment authority is changed.
Allocation-policy observations are modeled cumulative admission, not allocator
size classes, peak RSS, live-heap quotas or hostile-code containment.

## Failure sensitivity

The first long launcher disconnected after compilation and left no trustworthy
terminal status; its `creation-red.log` and empty `creation-red.exit` are retained,
not counted as executed test evidence. A direct cached invocation then executed
the regression and failed with an observed 48-byte VM creation charge versus
the independently modeled 80 bytes on this Linux x86-64 host. A separate retained
invocation (`creation-red2.log`, exit 101) confirms that failure.

The test locates the first quota that actually constructs storage, not merely the
first quota that completes execution. It independently models the token,
`Mutex<(Option<Vec<u8>>, usize)>` and two shared counters. It checks one-byte-short
refusal before construction, exact admission, whole-invocation success and zero
remaining owners/loans on every exit. The follow-up observes both the production
VM and source-derived evaluator before comparing their charges.

Evidence remains in the reused checkout's
`.artifacts/20261001-buffer-admission/`. Subsequent results are recorded only
after their terminal states are observed.

`creation-red3.log` completed with exit 101 and independently observed both
creation charges as `[48, 48]`, against `[80, 80]` for the VM and source-reference
respectively. Both engine observations execute before the final comparison, so the
second result is not inferred from the first. This is 32 bytes of undercounted
modeled creation storage per buffer on the inspected host, not a payload-copy or
resident-memory measurement.

## Correction

ByteBuffer now owns its admitted `create` operation. The complete storage charge
is computed next to the private storage representation; callers supply only their
existing allocation admission callback and execution control. Both production VM
and source-reference dispatch use it. The unadmitted convenience constructor is
compiled only for tests. Reference source dispatch, type/ownership admission and
expected values remain independent; sharing sealed storage mechanics does not
make the production dispatcher its oracle.

The small capacity-growth helper checks cancellation before the reservation and
immediately after it, before `try_reserve_exact`. Its previous ordering could
allocate after the callback had cancelled, then notice cancellation before push.
This second finding came from source inspection, not a predecessor execution
claim. The new test checks unchanged capacity, pointer and contents for both empty
and populated vectors after refused or cancelled reservation, as well as admitted
growth. Constructor tests require zero constructions on pre-cancellation, quota
refusal and cancellation during reservation, and one construction on success.
No allocation or execution limit is raised. Cancellation is cooperative: these
are tested checkpoints, not a promise of atomic exclusion of concurrent cancellation.

## Focused verification and source freeze

`cargo test --locked --all-features --lib byte_buffer_ -- --nocapture`
completed with 29 passes, zero failures/ignored cases and 984 filtered tests in
2.33 seconds after compilation. `focused-green1.log` retains the original output;
all three new regressions executed. Existing exact bytes, scoped loans, source/VM
results, raw boundary refusals, forged artifact rejection and failure cleanup also
passed in that selection. Formatting and `git diff --check` passed. These focused
results are not a substitute for the new frozen-source full profile.

## Accepted source and optimized product

Implementation `ac218900c15c886a52418f9309af1a7fb10b7f2d`, tree
`fc6cbcf277a2f7feae7bd4d8a22cf01c6a367867`, completed the full profile with all
26 gates passed freshly, zero reused results, stable inputs and no unrun gates.
The original receipt is
`.artifacts/lkjscript-dev/check/1790794964348416386-3645743-0/receipt.json`;
`full1.log` and `full1.exit` retain the launcher terminal (exit 0). Elapsed time
was 1,156.854956808 seconds with two checker workers. No source file was changed
between that source freeze and observation of the complete terminal receipt.

Workspace execution passed 1,438 top-level tests with zero failures and 29 existing
ignored cases. Root-library execution passed 1,005 cases with eight ignored;
public CLI execution passed 181 with one ignored. Two nested one-test child controls
are not counted twice in the top-level total. All 26 gates, including offline
packages, generated discovery, source-free command/HTTP application lifecycles,
standard/application artifacts and their comparisons, completed successfully.

The optimized product and exact public test executable were separately copied from
the frozen full run into this campaign's evidence directory. Product
`native-lkjscript` reports v0.1.61 and has SHA-256
`89b90289b66cc2e1ceef2068d6167462af20dd346175978aa7412eab49ae56d5`;
`native-public-cli-tests` has SHA-256
`8d2e636254269e8ae46baa3fa987396e107a2ff7ac91b402764cff7c4e318ee1`.
The explicit `LKJSCRIPT_RELEASE_CANDIDATE` selector binds that copied executable.
From `/tmp`, with the environment cleared and PATH empty, all 11 selected
`native_owned_` and `native_byte_buffer_` cases passed in 15.62 seconds, zero
failures/ignored and 171 filtered cases, with two test threads. `native-optimized.log`
and `.exit` retain this complete result. It covers three-package ownership flow,
source removal, exact witness selection, unchanged drafts, semantic rejection and
implementation edits. It is supplementary development-product proof, not a new
final-distribution candidate or a public v0.1.61 release.

## Separate completed public delivery

The already accepted v0.1.60 candidate was promoted unchanged during this continuation.
Producer `36617982924/1` remains bound to source `2962c43f`, not this implementation.
Promotion `36763094941/1`, from pre-existing integrated controller `9afa7997`,
completed with `immutable_published_and_public_verified`; immutable release
`400357606` is now public/latest. The [delivery owner](20260930-descendant-inventory.md#completed-v0160-publication)
records the independently checked original/published inventory and successful
anonymous installed lifecycle. The tag, scoped selector, immutability setting,
credentials, protections and original candidate were not changed by this promotion.
No running application, accepted supplier or operational data was upgraded.

The status page now separates actual public availability, integrated development,
authority, limitations and proof. Its former detailed chronology remains linked at
immutable predecessor `9afa7997`; campaign records are retained rather than erased.
Only README's existing current-limit anchor points into status in the inspected tree,
and that heading is preserved. The final reporting edits do not change this tested
language implementation or restamp its receipt. Post-report checks passed:
formatting, diff whitespace, all 25 tracked relative status-link destinations,
and `cargo test --locked -p lkjscript-site` (eight library tests and one binary
test; zero failures or ignored cases). `status-site-check.log` and `.exit` retain
that targeted presentation result. Status is now 180 lines, with its historical
snapshot and campaign owners still linked. The status bytes checked here have
SHA-256 `df15a079fc899d737839988cdca598073a7fdf8cc95c9a6a4c6092a0a68c7c7e`.

## Next selected language boundary

Prefer owned-data composition before expanding dispatch or supervisor machinery.
A useful first experiment is an owned aggregate that keeps a byte owner together
with ordinary metadata, so a packet or parser result can move as one value across
an ordinary library boundary. Establish construction, whole-value transfer,
consuming decomposition and failure cleanup before permitting partial moves or
asynchronous handoff. Preserve explicit ownership, exact package identity and an
independent source evaluator; do not hide copies behind generic convenience.
This is a selected next experiment, not a claim that aggregates, task-memory
signatures, mutable references or region placement are already implemented.
