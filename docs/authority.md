# Documentation authority

This document defines which artifact owns each kind of claim. It does not describe current feature
status or planned architecture.

- Current task instructions and the root [`AGENTS.md`](../AGENTS.md) own engineering procedure.
- Accepted normative specifications own intended externally visible language and semantic-workspace
  behavior. Until those specifications exist, the current task and `AGENTS.md` define the target.
- Executable code, tests, CLI definitions, schemas, and manifests own behavior in this checkout.
  [`current.md`](current.md) summarizes that evidence and must report gaps rather than redefine the
  target.
- Cargo metadata owns workspace membership and dependency edges. [`architecture.md`](architecture.md)
  explains responsibilities and trust boundaries but cannot override the executable graph.
- Reproducible benchmark harnesses, workload definitions, and recorded methodology own performance
  evidence.
- Accepted decisions own durable rationale and reversal conditions, not current status or language
  semantics.
- [`roadmap.md`](roadmap.md) owns ordering and intent only; it owns no implemented fact.
- Git history owns historical claims.

When claims conflict, identify their dimension, inspect the owning artifact and executable evidence,
and update or delete stale material in the same change. Do not create a digest, revision, registry,
or global ordering to make unrelated claims appear authoritative.
