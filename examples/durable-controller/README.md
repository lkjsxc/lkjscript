# Durable local release controller

This production-path example authors one typed state machine, publishes it as a reusable release,
builds an application-format-4 controller and a separate exact payload application, deletes the
source workspace and release, and operates two durable application instances through the public
`lkjscript instance` commands.

The controller is a pure transition kernel importing the exact application-activation host
interface. Its application-owned nominal command sum can request validation, atomic activation in
one granted slot, or reconciliation after unknown visibility. The trusted local adapter validates
the command route and exact immutable instance-bound grant. Only a later `resume` transition
consumes the application-owned nominal outcome. No program receives ambient filesystem authority.

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
