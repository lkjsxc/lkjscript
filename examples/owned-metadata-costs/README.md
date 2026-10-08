# Immutable metadata across owned boundaries

This literal native workload isolates admission of immutable metadata inside owned
products. It also enforces the production borrowed-field projection's retained
allocation proof. This is not a new intrinsic or production compiler pass.
[The decision](../../docs/decisions/borrowed-immutable-metadata.md) owns the proof
boundary; [status](../../docs/status.md) owns source acceptance and distribution.

## Author and run through the product

Create a fresh minimal project with a copied executable outside the checkout.
Export that executable's builtin standard transport, stage it, and author
`workload.lkjc` as an ordinary request with the current `request base=REVISION`
and exact `add.dependency package=PACKAGE semantic-revision=REVISION
package-revision=PACKAGE_REVISION` binding from the export. Use `change plan`,
then `change apply` with its returned plan token. The
[dependency component guide](../dependency-components/README.md) describes the
same public authoring and exact supplier-binding protocol.

Run `check`, then build `collection-cost.lkja` beside `command.deployment.json`.
The workload remains executable after deleting its disposable authoring project
and standard transport. Inputs and results are ordinary runner data; no external
semantic generator or host-computed program result participates.

```sh
./lkjscript run --deployment command.deployment.json \
  --arguments-file arguments.json --result-file result.json
```

Use an absent result path. Arguments have the shape `[MODE, ITEMS, REPETITIONS]`.
For example, `[2,[5,9],3]` returns `6`. The measured domain is modes 0 through 4,
nonnegative repetition counts and a list of I64 items. The workload returns the
item count times the repetition count; scalar overflow retains ordinary arithmetic
semantics. This is not a general-purpose timing or hostile-input sandbox API.

| Mode | Operation repeated on the same immutable payload |
| --- | --- |
| 0 | Select a record from an ordinary list. |
| 1 | Select the same-shaped record from an ordinary map. |
| 2 | Pack metadata into an owned product, then consume and unpack it. |
| 3 | Pack the metadata and drop the owned product; retain the original immutable alias. |
| 4 | Pack once, then repeatedly read metadata through an explicitly borrowed product. |

The envelope also holds an owned sequence, so it is genuinely owned rather than
an ordinary record with a different spelling. Each operation uses the same record
containing the same list. Empty sequence owners are cleaned up, not silently
exported. Borrowed reads do not transfer the owner's custody.

## Complete admission and observed work

The public matrix contains 5 modes, item counts 0, 8, 64, 256 and 1,024, and
repetition counts 0, 1, 32 and 128: 100 complete result comparisons. Its independent
host oracle uses only integer multiplication. Seven native fixed-result tests run
through both evaluators; the detached 100-point matrix uses production execution.
Do not describe that matrix as 100 differential executions. The reference evaluator
retains its independent recursive admission at borrowed metadata boundaries.

With 1,024 items and 128 repetitions, all modes still admit 1,027 input nodes.
The predecessor is source `2af992d73f8baea796983e6197f4e6e1175659a0`.
Focused production observations are:

| Mode | Predecessor raw-result nodes | Borrowed-proof implementation |
| --- | ---: | ---: |
| Ordinary list selection | 0 | 0 |
| Ordinary map selection | 0 | 0 |
| Pack and unpack | 262,656 | 262,656 |
| Pack and drop | 131,328 | 131,328 |
| Borrowed metadata | 132,354 | 1,026 |

Pack/unpack and pack/drop still observe `2*K*(N+2)` and `K*(N+2)`. Borrowed
metadata changes from `(K+1)*(N+2)` to `N+2`: packing admits it once, including
when K is zero. These count work, not copied bytes or RSS. The strengthened public
regression rejects the predecessor at mode 4, N=0, K=1, then passes the candidate.
This expected negative control is retained separately from successful acceptance.

Malformed final elements reject even with zero repetitions, without a result file;
each mode then accepts a fresh valid invocation. Explicit instruction refusal stays
a resource diagnostic, never a successful partial result. Complete input admission,
zero internal descendant guards and zero capture admission remain checked.
Executable/artifact bytes remain unchanged, and successful invocations leave no
local handles, frames, transactions, tasks or workers behind.

## Matched invocation measurements

The same detached predecessor artifact, inputs and release-build profiles are used
for both executables. Each workload retains one warmup pair and seven measured
pairs in alternating AB/BA order. The complete comparison has 18 workloads and
288 invocations, including controls and all warmups; no owned build overlaps it.
Existing unrelated services are not stopped. Original samples, environment, hashes
and summaries remain in `.artifacts/20261008-borrowed-metadata/matched-metadata/`.

For mode 4, K=128, invocation medians change from 7.066 ms to 0.465 ms at N=1,024,
and from 103.848 ms to 2.458 ms at N=16,384. The corresponding raw-result counts
change from 132,354 to 1,026 and from 2,113,794 to 16,386. Complete input counts stay
1,027 and 16,387. These are invocation-stage measurements on this host, not general
language speedups: preparation medians remain approximately 74 ms and 77 ms at
those sizes, and decoding, startup and result encoding are separate costs.
The unchanged pack/unpack and pack/drop controls do not acquire this mechanism.

## Implemented boundary and remaining work

Only production structural field reads from a live, admitted borrowed owned
product retain their proof. Exact prepared program, current memory domain, closed
product type, selected field, ordinary eligibility and live allocation admission
are checked. Raw mutation, interrupted adoption and revocation cannot restore a
certificate through shape equality. The selected immutable value does not retain
the parent loan or owner. Inline sum/option clone spines still reserve before growth.

Packing, consuming unpack, raw ingress, capture and ownership-transfer admission
remain separate costs. Extending reuse to those boundaries requires their own
checked construction/extraction contract, not a caller-supplied result type or
an address cache. No type/meaning/interface/artifact encoding changes are required
by this derived runtime mechanism; finalized distribution remains independently
accepted and published.
