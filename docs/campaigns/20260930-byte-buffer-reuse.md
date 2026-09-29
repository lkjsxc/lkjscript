# Unique byte buffer reuse

Date: 2026-09-30 (Asia/Tokyo).

## Request and entry state

The immediate user request was: `Continue`.
It continues delegated implementation on `https://github.com/lkjsxc/lkjscript`, not
`lkjstr`. The previous affine-proof correction is complete on entry. Local and remote
main both selected `301a6586a34a42d3616c237b19542fa44fdc6547`; the shared checkout is
`/home/coder/workspace/lkjscript` in `lkjsxc/tomato-ocelot-73`.
The pinned compiler is Rust 1.98.0. Existing unrelated untracked campaign/CLI files,
the prior stash, other worktrees and services remain outside this change.

## Selected increment and exclusions

The initial exploration considered a new owned-memory type with scoped reads. That
would require a coherent extension of non-capability ownership, returns, captures,
interfaces and both evaluators. It is not implemented by relabeling the existing
requirement-bound queue handles. This campaign instead selects a concrete derived
storage improvement that works with the already-admitted ordinary value semantics
and terminal-local transfers: reusable storage for byte concatenation.

`Bytes` remains an unrestricted immutable value. `core.bytes.concat(Bytes, Bytes)`
retains its exact pure signature, left-to-right argument evaluation, byte ordering
and value result. No new type, borrow syntax, resource grant, region collector,
associated-type witness or language-level unique-ownership guarantee is introduced.
This increment is not the coordinated owned-region/trait slice in the roadmap.

## Representation and execution

One private carrier represents either the existing shared `Arc<[u8]>` payload or a
concatenation-owned `Arc<Vec<u8>>`. Ordinary raw constructors retain their previous
shared-slice path. Only the normal checked VM concatenation path produces reusable
vector storage. The canonical reference and explicit raw intrinsic helpers retain
independent copying implementations; they do not call the reuse implementation.

An empty operand returns the other carrier without allocating a byte payload. A
nonempty concatenation may append in place only when the left vector has enough
capacity and safe `Arc::get_mut` proves no other strong or weak observer exists.
A unique full vector grows geometrically, bounded by the existing maximum result
size. Growth allocates a new vector and explicitly copies both slices before replacing
the exclusively owned old vector. A shared left operand always gets a fresh buffer;
a retained map key, capture or old value never changes contents or ordering.

The fresh path admits the vector descriptor and requested payload capacity separately,
both before allocation/copy. Growth admits its replacement capacity before allocation;
in-capacity and empty reuse do not manufacture another payload allocation charge.
Fallible payload-reservation failure remains a resource error. Process-level allocator
abort is not made recoverable by this change. Copy loops check cancellation at 64 KiB
chunks; an interrupted exclusive value is discarded, not published or rolled back into
a second owner. No external operation is replayed for differential checking.

Equality, ordering, debugging and encoders observe only the visible byte slice, not
its storage variant, capacity or address. Conversion into/out of a map key clones the
carrier without copying payloads. WebSocket output carries the same immutable carrier
rather than converting it back into another buffer while queued. Raw type/origin and
logical-occurrence admissions remain unchanged; spare capacity is not serialized.

The Rust standard library's [get_mut contract](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.get_mut)
is the ownership primitive, not an unchecked strong-count observation. The
[reservation contract](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.try_reserve_exact)
permits allocator rounding, so requested capacity is not exact allocator consumption.
No dependency or pinned-toolchain update is needed.

## Costs and observation limits

The representation adds a vector descriptor for reusable concatenation results and
may retain unused capacity. This trades extra metadata/spare storage for less repeated
prefix copying; it is not a claim that all programs use less memory. The existing Arc
counters and allocator overhead remain outside scalar allocation accounting. Raw
admission still charges each logical byte occurrence, including aliases, rather than
claiming a physical-memory measurement. Optional cumulative-allocation quotas may now
stop at a different boundary, because actual selected storage charges have changed;
value agreement under adequate budgets is not identical quota-exhaustion behavior.

`value_work.bytes` records production checked-concatenation work: calls, empty reuse,
in-place appends, growths, fresh buffers, explicit payload bytes copied and requested
capacity. The counters are observations, not execution fuel, permission or proof by
themselves. The independent reference does not instrument this optimization and its
zero counters must not be interpreted as zero copying. There is no timing/RSS speedup
claim, global zero-copy claim or promise about allocator-internal work.

## Predecessor and focused observations

The first new test build had an incorrect test-helper declaration type. Its compilation
failure is preserved at `/tmp/lkjscript-byte-reuse-before.log`; it is not behavioral
failure-sensitivity evidence. After correcting only the helper, the unchanged product
was exercised by three native-authored VM tests. Two pointer-identity tests failed:
empty concatenation copied its nonempty operand, and repeated concatenation did not
reuse the selected unique payload. The retained-prefix immutability case passed.
The complete result was one passed, two failed, no ignored cases, in 0.26 seconds:
`/tmp/lkjscript-byte-reuse-predecessor.log`, `TEST_EXIT=101`.

After integration, `cargo test --locked --lib byte -- --nocapture` passed 51 selected
tests with zero failures/ignored cases in 4.49 seconds. This includes the three VM
witnesses, six new storage cases and existing byte, map/codec, resource and source-byte
checks. Log: `/tmp/lkjscript-byte-reuse-focused.log`, `TEST_EXIT=0`.
The storage cases separately test exact/one-short admissions, no-copy failure, in-place
pointer retention, geometric copy bounds, a weak observer promoted during reservation,
map-key equality across representations and cancellation with retained aliases.
These observations precede final-source acceptance and do not validate later edits.

A later VM budget test initially assumed the final allocation was the concatenation
payload. Its one-byte-short failure actually occurred at the existing checked operand
placement after the three-byte result had been copied. The original failed assertion is
retained in `/tmp/lkjscript-byte-reuse-budget.log`; it is not a product regression or
source acceptance. The test now preserves that distinct late-failure case and adds a
bounded sweep of every smaller request budget, requiring both a no-copy payload admission
failure and a copied-but-unpublished operand failure. Both must clean all VM-owned slots.
The exact storage reservation tests remain independently specified rather than deriving
expected byte sizes from the changed producer. Both corrected VM cases passed in 0.20
seconds (two passed, no failures/ignored cases):
`/tmp/lkjscript-byte-reuse-budget-corrected.log`, `TEST_EXIT=0`.

The first all-target/all-feature Clippy pass rejected a new test's `get(...).is_none()`
style, not product behavior. The assertion was expressed as `!contains_key(...)` without
changing its expected result. Growth fixtures now observe the allocator's actual capacity
rather than assuming a minimal reservation. Corrected Clippy passed in 16.30 seconds:
`/tmp/lkjscript-byte-reuse-clippy-corrected.log`, `CLIPPY_EXIT=0`; the original failed log
remains at `/tmp/lkjscript-byte-reuse-clippy.log`.

## Native consumer and acceptance

The literal `tests/fixtures/reusable-byte-buffer.lkjc` composes standard byte operations,
ordinary generics, tail recursion, a retained closure, persistent map keys and typed
data round-tripping. The public test authors/checks/builds it with a copied executable,
then runs the artifact after removing the authoring path and literal request. Expected
bytes and records are constructed independently by the test, not from VM counters.

The development public test passed all eight project/detached routes in 5.52 seconds
(one test, zero failed/ignored, 167 filtered). Log:
`/tmp/lkjscript-byte-reuse-native.log`, `TEST_EXIT=0`. For 1,024 four-byte concatenations,
the detached production observed one empty reuse, one fresh buffer, nine growths,
1,013 in-place appends and 8,184 explicit copied bytes / requested capacity bytes.
The retained-map/closure case deliberately used two fresh buffers and zero in-place
appends, while preserving all independent expected fields. These are structural
work observations, not a timing or RSS comparison against the predecessor.

Exact-source full acceptance, copied final-producer observations and mainline delivery
are recorded only after their respective results. No binary publication or running
service replacement is selected at this initial checkpoint.

## Resumption: accepted source and mainline delivery

The owner's next `Continue` resumed the existing local implementation
`0f0f4d920057bbd5c7536ea3469b09209a48cbe5`, tree
`eaa3977df4d01ea8dc35cbb174230d8e03f6aeb6`, one commit ahead of remote main
`301a6586`. No running build owned this checkout. The unrelated untracked files,
stash, other worktrees and services were preserved. The older conversation's
shared-runtime publication handoff had already completed; public/latest was
independently observed as v0.1.55, not the previously reported v0.1.52.

The inherited implementation was reviewed without changing its production code.
One fresh full-profile run against that frozen source passed all 26 selected gates,
26 fresh, zero reused, zero unrun, with `input_stable=true`, no failure, and elapsed
512.455208703 seconds. Both input observations are
`verification_7481b95b9bf7fd83b3c536e3100b88c64a0bb88c5a138f6663649ffd8ec02178`.
The authoritative receipt is
`.artifacts/lkjscript-dev/check/1790702351122236674-2913089-0/receipt.json`.
The outer logs and exit record are retained under
`.artifacts/byte-buffer-reuse-20260930/full-final.*`; the verifier was copied and
kept immutable. Checker concurrency was two gates and Cargo used four build jobs.

The actual Cargo-reported release public-test executable and its product were copied
to `.artifacts/byte-buffer-reuse-20260930/retained-final/` before later builds could
replace either path. With a cleared environment and unrelated `/tmp` working directory,
`native_byte_reuse` and `native_map_keys` passed: two tests, zero failed or ignored,
166 filtered, 4.77 seconds. The byte test covers eight project/detached routes; the
map test covers Text/Bytes through both routes. `byte-map.log` and `byte-map.exit`
retain the original result. After full verification, the copied product was again
byte-compared with the final release producer and remained identical.

The 1,024 four-byte concatenations observed one empty reuse, one fresh buffer, nine
growths, 1,013 in-place appends and 8,184 explicit copied/requested-capacity bytes.
The retained-prefix/closure case observed two fresh buffers and zero in-place appends.
Expected content, map lookup and round-trip values were independently asserted. These
are execution-work observations, not elapsed-time comparisons or RSS measurements.
The authoring path and literal request were absent for detached runs; the renamed
project was retained and its original HEAD remained unchanged.

After refreshing main, divergence and branch protections, the implementation was
pushed normally as lkjsxc. Independent GitHub ref reading confirmed main at exact
`0f0f4d92`. All local verification owned by this resumption completed. The following
release notes and reporting edits do not replace this tested-source identity.

## Revised publication selection: consolidate v0.1.60

This resumption selects a v0.1.60 successor to public v0.1.55. It deliberately
supersedes the initial no-publication selection above and the separate v0.1.57
promotion plan; it does not modify any published release or running service.
The [notes](../releases/v0.1.60.md) collect the intervening recursive/package resource
contracts, exact deployment grants, resource suffixes, effectful callbacks, local
transfers, immutable key sharing, bounded affine proof work and byte storage reuse.

The v0.1.57 producer `36461408308/1` at source `3438e2ed` completed successfully;
its original terminal records `candidate_accepted` at 2026-09-28T18:52:11Z. All
five original artifact records were unexpired when read, with the earliest expiry
2026-10-12T18:51:40Z. That successful evidence is retained, not relabeled as failure.
Its publication is superseded because it lacks the subsequently demonstrated
[affine proof-work correction](20260929-affine-validation-work.md), as well as the
new composition and storage work. No tag or promotion for v0.1.57 is created.
Its planned additional 16 final-archive cases are not claimed as executed.

A new v0.1.60 candidate must run its own source and finalized-artifact gates. Before
promotion, use the authenticated final archive executable with the retained public
test harness and filters `native_generic_resources`, `native_byte_reuse`,
`native_map_keys`, `native_terminal_values`, and `shared_runtime`. Use a cleared
environment, retain exact producer/run/attempt and original results, and require
all selected cases to pass. The locally copied optimized executable is not that
final archive. Only then use the existing annotated-tag, scoped-selection and
unchanged-asset promotion procedure. Do not rebuild accepted assets or infer public
availability from either source acceptance or a successful candidate alone.

## Selected successor candidate

After notes/reporting-only commit `89241cd33c88d27ef1b6f774e24ab9b0726ff093`, tree
`53bb7a56b0dd072b07b253ac7af20c36ccddbfaf`, reached main normally, existing release
runs, v0.1.60 tag occupancy and public releases were refreshed. No duplicate v0.1.60
producer, tag or release was present. One non-publishing candidate was dispatched:
[36605595167/1](https://github.com/lkjsxc/lkjscript/actions/runs/36605595167), created
2026-09-29T17:31:54Z (2026-09-30 02:31:54 JST). Independent GitHub reading confirms
`workflow_dispatch`, branch main, original attempt 1, and exact product/controller
source `89241cd3`. Its observed state advanced from queued to in progress, with no
terminal acceptance yet. No tag, scoped-selector update or promotion was created.

The local full-profile receipt remains bound to implementation source `0f0f4d92`;
the five-file `89241cd3` descendant changes notes and reporting only. Documentation
validation passed generated-reference comparison, no-Python and product-surface
checks, with zero policy violations. This does not relabel the local full run with
a later source SHA. The hosted candidate must establish its own acceptance.

Resume this original producer at its actual failed or incomplete boundary, or use
its accepted original assets for the final-archive cases above. Do not dispatch a
replacement merely because this handoff was recorded while it was running. All local
jobs owned here have finished; external CI remains the pending publication boundary.
The final reporting descendant records that state without changing candidate inputs.
