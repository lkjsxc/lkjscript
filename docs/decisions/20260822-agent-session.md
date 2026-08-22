# Optional stdio agent session

Date: 2026-08-22 UTC.

## Status

Accepted decision: defer a resident agent session. CLI v4 remains stateless-correct and no
`session` command or protocol is implemented. This is an evidence-based deferral, not a claim that
a session can never be useful.

## Historical measured evidence

Seven warm samples were taken before the graph-3 cutover on Linux `7.0.0-29-generic` x86-64 with an
intermediate graph-2 CLI v3 release binary
SHA-256 `1a0cfe9610da56bf1f1b1395dd3601a3d0aa481d35cae0dc797503ab761e1314`.
The Rust and Cargo toolchain was 1.96.0. Shell timing had 1 ms resolution and included process
startup. The measured schema digest was
`831e1258c592e51557affb9713f4268e49248c84dec2ae397a7f9a4a5b46e065`.
Repository-bound samples used `lkjournal` revision
`rev_441156fb5408ad27352d91a8e8ac42f60cd6e2704ee9b42f58a01cc64a6ef218`.

| Standalone operation | Warm p50 | stdout |
|---|---:|---:|
| full `capabilities` | 2 ms | 1,394 B |
| cached `capabilities --known-schema` | 2 ms | 188 B |
| exact `query find Web --exact` | 2 ms | 632 B |
| five-process discovery/orientation/find/body/context workflow | 14 ms | 13,253 B total |

The five-process workflow used a cached capability handshake, bounded project inspection, exact
name lookup, selected body inspection, and bounded context for component
`decl_1cd2427798155aab924e8a7902505f02`. Provider requests, input/cached/output tokens, retries,
correction depth, and monetary cost were unavailable; byte counts are not token or cost estimates.

There is no implementation-equivalent stdio prototype, so no equal completed-task comparison of
wall time, RSS, requests, correction rounds, cancellation, or stale-revision behavior exists. The
small observed standalone startup cost alone does not justify adding protocol state and another
long-lived resource owner.

The current graph-4 schema digest is
`1980273fe10405fbf7aa7940c607af819c1261bd8b89019243326da31841df6c`. No equal current graph-4
standalone/session workflow has been retained, so the table above remains historical evidence, not
a performance claim for the current executable.

## Decision

Keep `capabilities --known-schema`, revision-bound continuations, bounded context, and stateless
commands as the current economy mechanisms. Do not add a resident agent session in this cutover.
`serve` and `worker` are application runners; they are not agent sessions and must not be reused as
an authoring control channel.

A future experiment may be retained only if it compares the same complete create, inspect,
change, stale-base recovery, check, build, and refactor tasks against separate processes and shows
a material benefit after including handshake, memory, shutdown, failures, and correction rounds.
No benefit may be inferred from process count or response bytes alone.

## Conditional future contract

If evidence crosses that gate, the candidate remains the same executable with an explicitly named
stdio-only bounded framed or JSONL mode. It must:

- handshake binary, CLI, graph, schema, and budget identities once;
- open one exact repository and pin one exact revision;
- report stale HEAD rather than silently following it during a change;
- cache only disposable schemas, indexes, prepared compiler data, and revision-bound context
  handles;
- publish exclusively through the normal change/transaction/repository path;
- bound frame size, output, memory, concurrent requests, idle lifetime, cancellation, and shutdown;
- survive malformed, truncated, oversized, reordered, and unknown frames without authority change;
- leave no durable session file, implicit draft, hidden current base, or unpublished writer state;
- use no network listener and introduce no TLS or certificate surface.

These are selection constraints, not implemented behavior or a reserved compatibility protocol.

## Rejected alternatives and reversal gate

Rejected now are a mandatory daemon, automatic background repository following, a session-owned
graph, implicit mutable base, network authoring service, unbounded body cache, and publication that
bypasses the repository lock. Killing a future session must be equivalent to killing a stateless
command before or after the existing atomic visibility point.

Reconsider only after retaining raw cold and warm samples for both modes on equal tasks, including
RSS, bytes, request count, failures, stale HEAD, cancellation, and provider telemetry when
available. Delete the prototype if the improvement is not material across complete workflows or if
standalone correctness, deterministic output, bounded resources, or secret safety regresses.
