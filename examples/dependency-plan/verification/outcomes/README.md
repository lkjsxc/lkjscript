# Check complete dependency outcomes

`verify-dependency-outcome` checks a claimed `valid`, `invalid` or `capacity`
result against the complete original proposal. The independently exported checker
owns `ClaimedDependencyOutcome` and the two-case `OutcomeVerification` type.
Its producer is not needed to author, export, build or execute these commands.
Follow the [parent guide](../README.md) to build `dependency-plan-verifier.lkja`.

## Standalone input and result

Place `single.deployment.json` beside the standalone artifact, and run it with the
copied executable outside the source checkout:

```sh
./lkjscript run --deployment single.deployment.json \
  --arguments-file claim.json --result-file report.json
```

Use an absent result path. The input is `[PROPOSAL, CLAIM]`. For example:

```json
[
  {"roots":[90,80],"nodes":[]},
  {"case":"invalid","value":{"code":"missing-root","owner":-1,"target":90}}
]
```

This returns `{"case":"verified","value":null}`: the negative source outcome
is the exact required one. It does not mean the graph is valid. Claiming target
80 instead rejects with `source-diagnostic`, even though 80 is also missing:
authored diagnostic precedence is part of the result. Changing the diagnostic
code or owner also rejects. All signed I64 identities remain valid data.

A valid graph accompanied by `{"case":"capacity","value":"nodes"}` instead
rejects with `source-outcome`. A source exceeding several capacities must report
the first required dimension: nodes, then roots, then successor entries. A wrong
dimension or a text value differing by case, trailing whitespace or a null byte
rejects with `source-capacity`; comparison is exact, not a substring test.

A `valid` claim contains the unchanged complete planner payload. It receives the
existing independent partition, dependency, connectivity and earliest-stage checks.
The old `[PROPOSAL, PLAN]` interface and its `Verification` result remain unchanged;
it still reports source invalidity/capacity before considering a successful plan.
The new interface instead verifies the *claimed source outcome* and returns only
`verified(Unit)` or `rejected(Text)` on a successful invocation.

`batch.deployment.json` takes `[[{"proposal":PROPOSAL,"claim":CLAIM}, ...]]`
and returns one `OutcomeVerification` per sample, in order. Empty batches return
`[]`. Both descriptors select the standalone checker artifact, not the producer.

## Checked consumer

Checked flat and chunked planner entrypoints submit every complete valid, invalid
and capacity outcome to the exact imported checker. They forward only the immutable
claim actually checked, converting its nominal wrapper back to the planner outcome.
There is no separately supplied result that can differ from the verified claim.
A rejected claim becomes the existing planner `inconsistent` outcome.

`inconsistent` is not a supported claim case. The consumer preserves this internal
failure without treating it as an independently verified source diagnosis. Runtime
cancellation, allocation failure and selected execution-budget refusal likewise
remain runtime failures, not native capacity reports or proof of safe retry.

Complete raw argument admission precedes native checking, even for malformed claim
fields on an already invalid or over-capacity source. Batch state is per sample;
a false claim does not contaminate the next report. Batches fit both the 1 MiB argument-byte and 100,000 aggregate JSON-entry limits.
No limits, grants, configuration,
secrets, runtime intrinsics, encodings or product identifiers are changed.

## Evidence and remaining trust

The extended public suite retains all original correct-plan and malformed-plan
coverage, then checks 8,760 complete-outcome claims with sources present and deleted.
These include all 4,635 existing canonical outcomes, 3,072 false negative claims on
all 512 directed three-vertex graphs, signed diagnostic matrices, simultaneous
capacity violations and the existing 41 claim mutations/recovery cases. In each
phase, 29 verified claims are true negative outcomes, not valid graphs.

The standalone checker executes correct and false negative claims before the
producer exists. Copied-executable tests also cover exact raw-type refusal, refusal
of an `inconsistent` claim, explicit instruction exhaustion, absent failure outputs,
subsequent recovery, joined cleanup and unchanged artifact/executable bytes.
Nine new fixed native tests cover the consumer and empty outcome batch through
both evaluators. The test-first predecessor rejects the strengthened suite; its
failure remains separate from candidate acceptance. A development batch also
exceeded the unchanged JSON-entry limit; the corrected test fixture partitions
by entries as well as bytes, retaining every case. Exact successful source and
receipts remain [status-owned](../../../../docs/status.md).

This closes an unchecked-negative-report path in the native consumer, not a
production compiler vulnerability. Source admission/reachability is shared with
the producer and remains trusted; the checker itself is not formally verified.
A `verified` report is ordinary data tied to this input, claim and selected checker,
never an execution grant, publication right or reusable certificate for another
source. Additional negative-outcome checking is real work, not a speedup claim.
