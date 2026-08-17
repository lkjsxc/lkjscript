# Agent maintenance corpus

This retained example evolves a release/deployment policy through eight immutable revisions. It
combines resource limits, target and rollout variants, trust checks, nested nominal values, checked
arithmetic, a counted loop, and exact decisions.

The production driver uses context packets and editable semantic documents to create an incomplete
application, reject and correct a typed repair, extend behavior, replace a function body without
durable anonymous-term churn, rename a durable entity, diagnose and repair an exact arithmetic trap,
replace `Limits` with `DeploymentLimits`, migrate every use, and delete displaced entities only after
references are gone. It validates before commit, reopens state on every direct command, runs old and
current revisions, and reviews exact change.

Current measurements include 81 CLI processes and no socket connections, a 15,663-byte initial
document, a 9,100-byte declaration migration document, artifacts from 8,354 to 9,457 bytes, and an
exact repeated-context reduction from 38,485 to 107 bytes. Provider telemetry is unavailable.

Run from the repository root:

```sh
./examples/agent-maintenance/run.sh
```

The driver reuses the job-policy payload builder but invokes only production binaries and public CLI
boundaries. The sealed intent and independent oracles are in [`tasks/maintenance.md`](tasks/maintenance.md).
