# Joined parallel computation

Development 0.1.68 adds two child tasks with owned inputs and a joined ordinary
result. Check the installed executable's `capabilities --section change`:
it must advertise `parallel`. Public v0.1.64 does not supply this expression.

For two already declared empty-effect graph tasks, the native form is:

```text
(parallel
  (call workers::buffer-job (i64 32) (local buffer))
  (call workers::cell-job (local count) (local cell)))
```

The enclosing function declares `(effect (task))`. Each child is a direct named,
monomorphic task with an empty effect row. Ordinary arguments come first; owned
arguments are exact live locals passed to `consume` parameters. Results must be
closed ordinary data, producing `(record (left I64) (right I64))` in this example.
All argument expressions run once in the parent, left call first. Child bodies
may overlap; the parent receives the pair only after both finish.

The [literal worker library](../../tests/fixtures/parallel-library.lkjc) supplies
buffer and scalar-cell reductions, a generic Owned reduction contract, exact static
implementations, and concrete task wrappers. Generic composition occurs inside the
wrapper; the child identity itself remains exact and monomorphic. The
[literal consumer](examples/parallel-consumer.lkjc) uses two groups: independent
buffer/cell inputs, followed by an owned product and owned choice. Constructors first
bind each owned input to a typed local, preserving the existing transfer rules.

Use the [native library workflow](native-library.md) to create a command-template
worker package, review/apply the worker request at its observed base, and export its
exact transport. Create a minimal consumer, stage that transport, select its observed
package/revision with `dependency.add` and `use`, then review/apply the consumer input.
Run `check`, inspect/draft the accepted module, and build an ordinary detached artifact.
Execute this task with `run --deployment PATH`; its deployment selects the artifact
and target with empty grants. Plain `run TARGET` is the pure differential runner
and rejects tasks even when their effect rows are empty.
The [maintained public test](../../tests/public_cli/native_parallel.rs) preserves the
literal inputs and complete executable sequence, including a third entry package,
identity-preserving edits and execution after source removal.

For the original example and input `[16, -99, false]`, the independent arithmetic is
`T = 16 * 17 / 2 = 136`. The expected result is:

```json
{"simple":{"left":499,"right":37},"aggregate":{"left":187,"right":141}}
```

Editing the first child's argument from 31 to 32 changes only `simple.left` to
531. The test checks this edit, four input cases and both choice cases using
closed-form arithmetic. Its detached runtime admits only one ordinary resident task;
child execution does not need another resident slot held by its parent.

`run` reports `parallel_scopes` and `parallel_workers_spawned` inside its production
observation. They count groups and created workers, not simultaneous activity or
speedup. Worker exhaustion falls back to the calling thread. Optional cumulative
quotas apply to the entire invocation, including both children and later parent work.
On a trap or cancellation, started children are cancelled cooperatively and joined,
and every transferred owner is reclaimed before the invocation returns.

No capability grants or external adapters enter these children. Use ordinary parent
task code for external effects before or after the group. Channels, detached tasks,
borrowed child inputs and owned child results are separate future boundaries. See
the [normative contract](../spec/structured-parallel.md) and
[current acceptance status](../status.md) for precise limits and evidence.
