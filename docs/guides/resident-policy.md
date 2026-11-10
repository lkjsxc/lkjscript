# Execution policy

Ordinary source execution and source batches run each invocation once on the VM.
Call frames and operands grow with available storage. Compiler and evaluator
threads reserve virtual stack space from physical memory; native executables raise
their soft stack limit toward the same size while respecting the caller's hard OS
limit. Cancellation, semantic admission and capability grants remain enforced.

Discover the selected executable's policy with:

```sh
lkjscript capabilities --section deployment
```

## Explicit cumulative quotas

The current execution descriptor still supports these caller-supplied dimensions:

```json
{
  "instruction_fuel": null,
  "maximum_allocated_bytes": null,
  "maximum_collection_items": null,
  "maximum_capability_calls": null
}
```

Null means no quota in that dimension. Numeric values, including zero, are exact
allowances. `instruction_fuel` is required inside an explicit execution object;
the other three fields may be omitted. Resident routes require execution and
runtime objects. Command deployments may omit both. Call-depth and value-stack
policy fields have been removed from the current format.

Rust `RunPolicy::default()` and `RunPolicy::unmetered()` select four `None` values;
`NormalizedRunPolicy::foreground()` does likewise. Runtime deadlines and grants
are independent caller controls. A resource failure, cancellation or lost response
can follow externally visible effects; inspect the relevant state before retrying.

## Observations

Work counters describe one invocation and carry saturation flags. Modeled allocated
bytes are cumulative charges; process RSS is measured separately by the test runner.
The remaining modeled accounting is a future simplification target in `direction.md`.

The [resident-policy fixture](../../tests/fixtures/resident-policy.lkjc) exercises
explicit quotas, deadline cancellation, recovery and joined shutdown.
