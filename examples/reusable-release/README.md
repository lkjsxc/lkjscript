# Exact reusable releases

`run.sh` exercises the retained reusable-release architecture through production binaries. It
authors `shared-codec` twice from unrelated workspace histories and proves canonical byte equality,
then publishes a distinct R2 under the same human coordinate. Independent
`consumer-normalizer` and `consumer-inspector` workspaces bind R1 through different exports.

The same workflow builds a typed R1/R2 coexistence application and a diamond in which both
consumers share one exact R1. It rejects private access and R2-for-R1 nominal substitution, removes
the complete mutable state directory, and then rebuilds, validates, inspects, tests, and runs all
four applications from immutable release files with no network or ambient resolver state.

Run it from the repository root after a release build:

```sh
examples/reusable-release/run.sh
```

The final stdout line is one canonical JSON evidence summary. Temporary workspaces and artifacts
are created outside the repository and removed automatically.
