# Actual compiler-21 artifact

`predecessor.lkja` is unchanged output from the copied development executable used
for the accepted v0.1.70 work. Its SHA-256 is
`ba7f5b786ed98b15095eff6861f51fd035b00d056ef16f4cc94687a9bd71ad73`.
The original retained evidence is indexed by `provenance.json`; that executable is
still present at the recorded worktree's `target/release/lkjscript` with the same digest.

`accepted.request` and `commands.json` retain the literal public authoring/build
inputs. `producer-build.stdout` records the original successful build, exact
accepted revision and artifact identities. `original-observation.json` records
eight complete executions, four after source removal, with joined cleanup. These
are historical observations, not fresh successor execution claims.

The current compiler test requires this actual Graph-21/compiler-21/bytecode-17/
artifact-28 bundle to reject with `source/compiler_unit_contract`. A separate test
reconstructs the frozen compiler-21 wire layout and uses a deliberately malformed
payload to prove rejection before payload decoding. Reconstructed bytes are not
described as output from the predecessor executable.
