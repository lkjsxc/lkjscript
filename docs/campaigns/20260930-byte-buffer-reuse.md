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
service replacement is selected by this campaign.
