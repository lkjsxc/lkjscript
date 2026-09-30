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
