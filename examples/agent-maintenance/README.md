# Agent maintenance corpus

This retained example evolves a release and deployment policy through eight immutable revisions. The policy combines resource limits, target and rollout variants, trust checks, nested named values, checked arithmetic, a counted loop, and exact typed decisions. It is a maintenance corpus rather than a source project: the authoritative program exists only in the local service's typed, versioned program model.

The production driver uses the preferred semantic workbench to create an incomplete application, inspect a repair packet, reject and correct a typed repair, extend behavior, refactor without changing results, rename presentation metadata, diagnose and repair an exact arithmetic trap, replace an immutable input declaration, migrate its construction, projection, function input, output flow, and calls, and delete the displaced declarations only after their references are removed. It validates large changes before commit, restarts the daemon, and reruns old and current revisions.

The final application accepts eligible deployments with a deterministic score and rejects CPU, memory, target, trust, or disabled-rollout cases with an exact named reason. It performs no host effect and receives no ambient authority.

Run it from the repository root:

```sh
./examples/agent-maintenance/run.sh
```

The driver imports the retained job-policy public-payload builder to avoid a second copy of the application definition. It still invokes only production binaries and public CLI boundaries. Reading the driver is not required to understand the application contract; sealed maintenance tasks and their allowed reading set are in [`tasks/maintenance.md`](tasks/maintenance.md).
