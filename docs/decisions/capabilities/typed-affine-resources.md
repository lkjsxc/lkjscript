# Typed Affine Resources

## Status

**Accepted contract with implemented foundation.** Universal source `handle` is
removed. Exact kinds now cross source typing, HIR, verified SSA, bytecode
validation, and VM resource-kind checks. Core, VM, and evaluator lifecycle
foundations now enforce provider/state facts and reusable generations. The
complete capability remains non-Current until exactly-once cleanup, evaluator
resource-operation dispatch, forced native host execution, malformed-input,
and acceptance coverage are complete. An opaque runtime token alone does not
qualify.

## Closed initial kinds

The destination vocabulary is:

```text
input-stream
output-stream
file-reader
file-writer
file-appender
directory
tcp-listener
tcp-stream
sqlite-connection
sqlite-statement
terminal-session
```

A cycle may promote only a smaller complete set when retained evidence proves
that another kind cannot cross every required layer safely. Universal `handle`
does not remain for a promoted domain.

## Static contract

Each resource type owns an affine value directly. It is not wrapped in
`owned`, copied, cloned implicitly, compared, serialized, built from an integer,
or stored in an unsupported traced aggregate. Move transfers one cleanup
obligation. Borrow does not. Use after move, close, or drop and double cleanup
are verification failures.

Every value carries exact provider/capability origin, process or session scope,
resource kind, state, ownership, and declared `send` and `sync` facts. Parent
relationships such as statement-to-connection remain explicit. Operations
state exact input/output typestate and recoverable lowercase error types.

## Runtime slots

The runtime table validates an opaque tuple:

```text
resource-kind
slot-index
generation
state
provider
ownership
```

A stale generation and wrong kind fail before host access. Reuse increments the
generation. Malformed bytecode cannot bypass kind, state, provider, or ownership
validation. Raw host descriptors never become source values.

## Deterministic cleanup

The compiler elaborates exactly one cleanup for each initialized obligation on
normal scope exit, early return, typed break, structured trap, structured exit,
failed branch initialization, function failure, and future cancellation edges.
A move transfers the obligation. Explicit `drop` or a typed consuming `close`
removes it. Cleanup outcomes are typed and bounded.

GC traces memory only. It does not finalize resources. Arbitrary user finalizers
are rejected. Runtime process teardown is a safety net and never substitutes
for verified lexical cleanup.

## Verified representation

HIR and SSA record resource kind/state/origin, move, borrow, operation,
transition, explicit and compiler cleanup, cleanup edge, host effect, typed
error, safepoint, and frame state. Verification proves initialization,
kind-correct access, no use after consumption, no double cleanup, and one
cleanup on every structured outcome.

The implemented bytecode foundation publishes exact resource parameter and
resource-result metadata plus explicit global-to-prototype links. Validation
rejects untyped `any` values at host-resource instructions, checks exact kinds
at statically known calls and returns, and rejects resource flow through calls
without metadata. This does not supply the pending cleanup or provider/state
proofs.

VM and forced native execution preflight complete reachable support, preserve
ownership and exact roots, execute cleanup, produce real native entries, and
never silently fall back. An unsupported tier rejects before effects.

## Public operations

Public names describe typed actions, including `open-file-reader`,
`open-file-writer`, `open-file-appender`, `open-directory`, `read-into`,
`write-from`, `sync-file`, `truncate-file`, `rename-path`, `listen-tcp`,
`accept-tcp`, `receive-into`, `send-from`, `prepare-sqlite`, `step-sqlite`, and
`reset-sqlite-statement`. Every acquisition accepts the exact capability.
Internal provider calls may use stable IDs and a documented `host-` prefix.
