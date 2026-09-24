# Choose resident cumulative quotas explicitly

This guide describes development deployment contract 5, not immutable public v0.1.44.
It changes deployment policy, not the authored program, artifact or capability grants.
Use an executable that advertises nullable fields through:

```sh
lkjscript capabilities --section deployment
```

## No cumulative work budget

A newly created HTTP project keeps its generated runtime, HTTP, stream and grant
objects. Its execution object explicitly chooses:

```json
{
  "instruction_fuel": null,
  "maximum_call_depth": 4096,
  "maximum_value_stack": 1000000,
  "maximum_allocated_bytes": null,
  "maximum_collection_items": null,
  "maximum_capability_calls": null
}
```

These four nulls remove cumulative instruction, allocation, collection-item and
invocation capability-call budgets. They do not disable accounting, type checks,
operational deadlines, cancellation, stack limits or exact grants. The generated
runtime still selects 16 active tasks, 64 queued tasks, a 30-second request deadline,
30-second shutdown drain and 5-second cancellation grace. Change these operational
choices explicitly for a real deployment rather than treating a sample as its policy.

The entire execution and runtime objects are still required for resident routes.
Deleting either object, replacing either with null, omitting instruction_fuel, or
omitting a structural limit is not an unmetered mode and is rejected.

## Bound only the work dimension you intend

To keep the same program and live limits while capping instructions, copy its
deployment descriptor and replace just instruction_fuel with a positive integer.
Keep the other three nulls to avoid silently installing different work quotas.
Likewise, maximum_capability_calls can independently cap invocation calls even when
instructions are unbounded. This never grants an operation or changes a grant limit.

A quota-exhausted HTTP invocation reports a bounded resource failure with HTTP 500
and x-lkjscript-failure-class: resource. Its exact code names the exhausted owner.
An operational deadline instead reports HTTP 503 and the cancelled class. Neither
response establishes rollback of earlier application effects. Health/recovery work
uses a new invocation; retrying a failed effectful request remains application policy.

## Existing descriptors and recovery

An older three-field execution object with numeric instruction_fuel retains its
previous behavior: absent new fields still mean 268,435,456 cumulative allocated
bytes, 1,000,000 cumulative collection items and 100,000 invocation capability calls.
Explicit null removes a selected quota; omission preserves that legacy default.
Numeric zero is invalid, not another spelling of unlimited. Serialization writes
all six actual selections, including materialized legacy defaults.

Older executables reject new fields and null fuel rather than silently ignoring them.
Keep the original numeric descriptor and executable for rollback; changing the
runtime selection does not rewrite application data or grants. An artifact remains
a runtime-dependent bundle, not a standalone machine executable.

## Operational boundary

These work counters are per invocation. Unbounded observations saturate at u64::MAX
and then express a lower bound. Allocated-byte accounting measures cumulative
allocation work, not process RSS or maximum live heap. A deadline also is not a
hard operating-system CPU or memory sandbox. Use an appropriate external isolation
boundary for untrusted programs or public multi-tenant service operation.

The maintained public test authors [a native counting and HTTP workload](../../tests/fixtures/resident-policy.lkjc),
then runs it without its authoring project. It requires a case exceeding the old
10,000,000-instruction budget, separate quota exhaustion, deadline cancellation,
post-failure health and joined server shutdown. The
[campaign](../campaigns/202609250154.md) distinguishes planned and actually executed
results, exact source acceptance and public-release status.
