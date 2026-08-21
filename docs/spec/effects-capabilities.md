# Effect, capability, and failure contract 1

This specification owns static effect discovery, requirement/grant binding, operation accounting,
task-scoped capability calls, and closed execution failures. Interfaces and application tasks own
semantic operation policy; adapters own only generic mechanics.

## Interface and effect declarations

Each interface operation defines ordered typed parameters/result, idempotency
(`idempotent`, `idempotent-with-key`, or `non-idempotent`), and visibility (`no-visibility` or
`possible-visibility`). Possible visibility requires an idempotency-key contract; unsafe
combinations reject semantically. These annotations constrain adapter failure classification and do
not by themselves make an application retry.

A pure function cannot contain or transitively reach `perform` or `transaction`. A task declares
capability aliases; semantic closure records every operation used. A component requirement must
name the exact interface, include every used operation, and set maximum limits. Artifacts expose
that closed requirement before deployment.

## Grants and accounting

A grant descriptor contains contract 1, interface owner, canonical adapter kind, sharing domain,
64-hex authority revision and descriptor digest, operation set, and concrete limits. Binding
requires exactly one grant per requirement. Foreign interfaces/adapters, missing or extra
requirements, missing operations/limits, and a granted limit greater than the application-requested
maximum reject before work.

Each admitted task gets fresh call/input/output counters while sharing only the declared adapter
domain. `maximum_calls`, `maximum_input_bytes`, and `maximum_output_bytes` are enforced with checked
arithmetic before/after calls. Adapter-specific row, random, stream, queue, object, and pool bounds
also apply. Grants and counters are operational values and cannot be serialized.

## Ordering, cancellation, and retry

Capability calls occur in language evaluation order. Adapters receive runtime-owned cancellation
and deadline control and must check it before blocking or publishing where the external API allows.
Cancellation is not an instruction-fuel substitute. It does not roll back an already completed
external publication.

The closed execution failure classes are:

- `trap`: violated pure/language operation contract;
- `capability`: known external failure or denial;
- `possible_visibility`: success/failure cannot be established after possible publication;
- `resource`: a declared bound or admission resource was exhausted;
- `cancelled`: owner cancellation or operational deadline; and
- `infrastructure`: runtime/adapter invariant or host mechanism failed.

Each error also has a stable code, bounded safe message, retryable flag, and possibly-visible flag.
Only a known safe class may set retryable. Possible visibility is always explicit and blocks blind
retry; application policy must reconcile using a read/query operation. Expected domain failures
remain normal typed values.

## Transactions and live resources

`transaction capability binding body` opens one lexical adapter transaction, evaluates the body
with a temporary alias, commits only after body success, and rolls back after body failure. Commit
failure may be possibly visible. The transaction object is owned by the task, cannot nest in the
current PostgreSQL contract, cannot escape as `Value`, and cannot enter source, artifacts, durable
state, streams, another task, or another process. Rollback is idempotent and Drop attempts
best-effort rollback; cleanup failure is retained as adapter/runtime evidence.

Streams and secrets are likewise opaque task/deployment resources. No finalization depends on
language garbage collection. Scope close, cancellation, peer failure, shutdown, and adapter
shutdown each reach an explicit cleanup route. Restart discards live handles and reconstructs only
application/database/object/queue authority.

`ScriptedAdapter` is the deterministic reference fake. It has independent scripted results and
transactions, rejects unexpected call order or exhaustion, and cannot accidentally access
production PostgreSQL, object, clock, random, or secret state.
