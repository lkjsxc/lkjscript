# Durable local release controller

This production-path example authors one typed state machine, publishes it as a reusable release,
builds an application-format-3 controller and a separate exact payload application, deletes the
source workspace and release, and operates two durable application instances through the public
`lkjscript instance` commands.

The controller is a pure transition kernel. It requests one command at a time as typed data:
validate an exact application, atomically activate it in one granted slot, or reconcile an
activation whose visibility is unknown. The trusted local host executor validates the command and
the exact instance-bound grant. Only a later `resume` transition consumes the recorded typed host
outcome. No program receives ambient filesystem authority.

The workflow proves validate-only parity, duplicate event replay, stale-base rejection, grant
denial, restart reconstruction, replacement, known pre-visibility failure, bounded history,
cross-instance grant rejection, corruption rejection, source deletion, and tombstoned identity.
Focused Rust fault tests cover the visibility-unknown interval and reconciliation without silently
repeating activation.

Run from the repository root:

```sh
./examples/durable-controller/run.sh
```

The bootstrap trust boundary is one trusted Linux operator and a trusted local POSIX-like
filesystem. The process boundary is not a sandbox.
