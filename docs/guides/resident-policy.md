# Choose cumulative execution quotas explicitly

Ordinary source execution, source differential batches, Rust policy defaults and
trusted foreground execution impose no cumulative work quotas. This does not remove
live call/stack controls, value admission, operational deadlines or capability grants.
Discover the current executable's exact policy contract with:

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
cancellation or exact grants. The generated runtime still selects 16 active tasks,
64 queued tasks, a 30-second request deadline, 30-second shutdown drain and
5-second cancellation grace. Choose these operational policies explicitly for a
deployment; they are not a hostile-code sandbox.

The execution and runtime objects remain required for resident routes. Deleting
either object, replacing either with null, omitting instruction_fuel or omitting
a structural limit rejects. A Command deployment may omit both policy objects.

## Bound only the work dimension you intend

To cap instructions, set instruction_fuel to a positive integer. Each of the other
three cumulative fields independently accepts a positive integer or null.
Omitting any of those optional fields also means no quota for that dimension.
For example, a three-field object with instruction_fuel, maximum_call_depth and
maximum_value_stack caps only its explicitly selected instructions and live state.

No work policy grants an operation or changes an exact per-grant maximum.
A quota-exhausted HTTP invocation reports HTTP 500 with
x-lkjscript-failure-class: resource and the exhausted owner's code. A deadline
reports HTTP 503 and the cancelled class. Neither proves rollback of prior effects.
Health/recovery work uses a new invocation; effectful retries remain application policy.

## Current-format behavior

Optional-field omission no longer installs the historical 256 MiB allocation,
1,000,000 collection-item or 100,000 invocation-call quotas. A caller that wants
those limits must state their numbers. Numeric zero is invalid, not unlimited.
Serialization materializes all six selections, including null for absent quotas;
round trips preserve explicit numbers and absence without hidden defaults.

This is an intentional policy compatibility cut. No authored program, artifact,
grant, accepted graph or application data is rewritten. An immutable older
executable retains its old interpretation; use matching executables and descriptors
when reproducing historical behavior. A .lkja bundle still requires its runtime.

Rust embedders use Option<u64> for each cumulative dimension. RunPolicy::default()
and RunPolicy::unmetered() both select four None values. NormalizedRunPolicy::default()
and its named foreground profile select the same cumulative policy. Explicit Some
values retain deterministic precharge/refusal semantics. Call-depth and operand-stack
defaults remain separate implementation limits pending iterative evaluator work.

## Observations and cancellation

Work counters are per invocation. Unmetered observations saturate at u64::MAX;
saturation expresses a lower bound and grants no capacity. Allocated-byte accounting
measures cumulative modeled allocation, not process RSS or maximum live heap.
Checked storage arithmetic, cancellation and exact grant accounting remain enforced.

Without an explicit instruction quota or cancellation/deadline, a divergent program
may keep running. Removing default fuel is not a termination proof. Finite validation
and compiler passes need independent progress arguments.

The maintained [counting and HTTP workload](../../tests/fixtures/resident-policy.lkjc)
runs through ordinary source differential execution and detached execution beyond
the former instruction/allocation defaults. It also checks each explicit quota,
deadline cancellation, post-failure health and joined server shutdown.
