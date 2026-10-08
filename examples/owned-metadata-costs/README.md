# Immutable metadata across owned boundaries

This literal native workload isolates repeated admission of immutable metadata
inside owned products. It is a measurement and regression witness, not a runtime
optimization, new intrinsic or production compiler pass. [Status](../../docs/status.md)
owns acceptance and retained original evidence.

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

## What the measurements establish

The public matrix contains 5 modes, item counts 0, 8, 64, 256 and 1,024, and
repetition counts 0, 1, 32 and 128: 100 complete result comparisons. Its independent
host oracle uses only integer multiplication. Seven native fixed-result tests run
through both evaluators; the detached 100-point matrix uses production execution.
Do not describe that matrix as 100 differential executions.

With 1,024 items and 128 repetitions, all modes admit 1,027 input nodes. The
accepted predecessor executable observes the following raw-result admission work:

| Mode | Raw-result nodes |
| --- | ---: |
| Ordinary list selection | 0 |
| Ordinary map selection | 0 |
| Pack and unpack | 262,656 |
| Pack and drop | 131,328 |
| Borrowed metadata | 132,354 |

On this workload the observations match `2*K*(N+2)`, `K*(N+2)` and `(K+1)*(N+2)`
for the last three modes. The zero-repetition borrowed case still packs once.
These counts isolate the relevant boundaries; they are neither bytes copied nor
RSS, and do not assign a causal fraction of total wall time to admission.

The regression enforces upper bounds, not mandatory rescanning: future complete
proof propagation may reduce raw-result work. It retains exact complete input
admission, zero internal descendant guards and zero capture admission for this
workload. Malformed final elements reject even with zero repetitions, without a
result file; each mode then accepts a fresh valid invocation. Explicit instruction
refusal remains a resource diagnostic, never a successful partial result.
Executable/artifact bytes remain unchanged, and successful invocations leave no
local handles, frames, transactions, tasks or workers behind.

## Next boundary

Propagating already checked immutable metadata through these envelopes is a
candidate optimization, not implemented by this witness. It requires exact
program/type/origin binding, complete raw ingress, invalidation after raw mutation,
borrow and transfer provenance, reserve-before-growth accounting and joined
cleanup. A declared result type, allocation address or matching shape alone is
not a reusable certificate. Preserve the current observations and failed
experiments when comparing a future implementation.
