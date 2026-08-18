# Durable immutable-blob publisher

This example authors a typed publisher state machine, builds an application world importing the
bounded immutable-blob interface, deletes its source workspace and standalone release, and operates
two durable instances through public commands.

The lkjscript application—not the Python driver—decides when to put content, reconcile unknown
visibility by exact digest, retry known absence or failure, complete, and cancel. The production
adapter can only publish immutable content in its instance-bound private namespace. A disjoint fake
adapter exercises unknown visibility and reconciliation without filesystem work.

Run from the repository root:

```sh
./examples/durable-blob-publisher/run.sh
```

The bootstrap assumes one trusted Linux operator and a trusted local POSIX-like filesystem. A
process boundary is not a sandbox, and a content digest is neither provenance nor authority.
