# Release-manifest classifier

This public-path example constructs a pure classifier for an exact 32-byte binary release manifest.
It publishes a reachable typed hole, rejects a byte-for-boolean repair without publication or durable
identity consumption, commits an identity-preserving repair, checks every decision and a bounds trap,
renames presentation metadata, reopens the workspace, and executes historical revisions.

Run from the repository root:

```sh
./examples/release-manifest/run.sh
```

The script uses only production release binaries and deletes only its private temporary state.
