# Runtime JIT Instead of Offline PGO: Rejected

[Authority](../runtime-jit-instead-of-offline-pgo.md)

## Status

**Mixed.** Current, Accepted Target, Deferred, Rejected, and historical evidence status follows the
explicit labels in this capsule and its authority; this capsule cannot promote a capability.

## Rejected

- Offline or ahead-of-time PGO as an accepted target.
- Persistent cross-run JIT profiles or native-code caches in this plan.
- Calling the observation hook a JIT.
- Calling emitted but unexecuted machine code a baseline JIT.
- Calling next-invocation compilation OSR.
- Calling abort or whole-program restart deoptimization.
- Hiding compilation cost behind steady-state-only claims.
- Carrying universal tagged VM values through typed native hot paths.
- A backend that independently interprets untyped syntax.
- Writable-and-executable pages.
- Background compilation before process-safe ownership and cancellation.
- Unchecked assumptions or undefined behavior as optimization mechanisms.
