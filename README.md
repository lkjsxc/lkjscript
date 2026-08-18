# lkjscript

`lkjscript` is a local typed semantic programming system designed for coding agents. An agent can
author immutable workspace revisions, publish one workspace-independent reusable semantic release,
bind exact releases from other workspaces, and build a single-file application that remains
valid, testable, and runnable after every source workspace is removed.

Accepted program meaning has one typed representation. Documents and JSON are proposals; context
and review are bounded views; canonical release/application bytes are immutable distribution
authorities; Core IR, ownership plans, compiler IDs, and runtime handles are derived. Deterministic
Rust validators decide acceptance.

## Reusable-release proof

The retained production workflow authors `shared-codec` with four exports, a nominal `Frame`, a
private helper, and release cases. It builds canonical-equal R1 bytes from unrelated workspace and
allocator histories, then builds a distinct R2 under the same human coordinate. Independent
`consumer-normalizer` and `consumer-inspector` releases consume different R1 exports.

The same example proves both versions coexist without nominal unification and constructs this exact
diamond:

```text
release-diamond
  -> consumer-normalizer -> shared-codec R1
  -> consumer-inspector  -> shared-codec R1
```

R1 occurs once in the validated application graph. The driver rejects private access, corrupted,
missing, and extra dependencies, and R2-for-R1 nominal substitution. It removes the complete state
directory and then byte-identically rebuilds, validates, inspects, tests, typed-runs, and
stream-runs four applications using immutable files only.

```sh
cargo build --release --locked
examples/reusable-release/run.sh
```

`examples/binary-canonicalizer/run.sh` remains the larger repair/history/runtime workload and now
publishes its semantic program as a reusable release before building application format 2.

## Exact release and application commands

Release construction names an exact workspace and revision in strict contract-version-1 JSON:

```text
lkjscript release build --state DIR [--dependency FILE ...] --validate-only
lkjscript release build --state DIR [--dependency FILE ...] --output /absolute/release.lkjr
lkjscript release validate --artifact /absolute/release.lkjr
lkjscript release inspect --artifact /absolute/release.lkjr
lkjscript release test --artifact /absolute/release.lkjr [--dependency FILE ...]
```

The canonical release format is `LKJREL\0\x01`, format 1, schema `lkjscript-tsm006`. Its exact
`ReleaseId` is a domain-separated digest of the complete canonical payload. Coordinate and user
version are immutable human metadata, not dependency or nominal identity. Exports and dependency
slots are explicit; consumers can target only exports. Provenance and signatures are explicitly
absent.

Application build consumes a complete explicit exact-release graph in strict contract-version-2
JSON. It never opens a workspace or resolves a name, version, HEAD, store, or network result:

```text
lkjscript app build --release FILE [--release FILE ...] --validate-only
lkjscript app build --release FILE [--release FILE ...] --output /absolute/application.lkja
lkjscript app validate --artifact /absolute/application.lkja
lkjscript app inspect --artifact /absolute/application.lkja
lkjscript app test --artifact /absolute/application.lkja
lkjscript app run --artifact /absolute/application.lkja
lkjscript app stream --artifact /absolute/application.lkja
```

Application format 2 (`LKJAPP\0\x02`) embeds every reachable release once plus one entry,
invocation profile, policy, and application cases. Every build runs all release and application
cases. Every load independently validates and canonically re-encodes the complete graph before
compile or execution. Format 1 rejects; there is no compatibility reader.

`typed` values carry exact release/item identity for nominal types and members. `bytes_stream`
accepts exactly one `bytes -> bytes` export, reads at most 65,536 uninterpreted standard-input
bytes, and writes exactly the semantic result. Neither profile grants filesystem, environment,
network, clock, randomness, or process authority.

## Agent authoring

Normal development uses one `lkjscript` process under an exclusive local state lock; no background
service is required.

```sh
STATE=$(mktemp -d)
chmod 700 "$STATE"

target/release/lkjscript agent orient
target/release/lkjscript agent create --state "$STATE"
target/release/lkjscript agent context \
  --state "$STATE" --workspace WORKSPACE --revision 0 --purpose orient
```

`agent view` and `agent diff` provide bounded deterministic review. `agent document`, `validate`,
and `apply` use one exact-base, schema-bound editable document. `agent run` selects an exact
revision and function. Normal authoring needs no global schema dump; exact context and schema
digests support compact unchanged results.

`lkjscript --state DIR session` amortizes Engine startup for line-delimited protocol-v10 requests.
The optional `lkjscriptd` socket adapter remains only for its framed client, correlation, timeout,
disconnect, shutdown, and authority-lock diagnostics; it calls the same Engine.

## Semantic and runtime model

Workspaces use durable IDs for continuity-bearing declarations, members, functions, parameters,
and repairable holes, plus revision-bound function-local IDs for body structure. Release projection
erases both domains and assigns canonical local IDs. Public release nominal identity is exactly
`(ReleaseId, ReleaseItemId)`; compiler and runtime tags remain private derivatives.

The language supports `unit`, `bool`, checked `i64`, immutable `bytes`, immutable nominal records,
fixed variants, calls, conditions, counted loops, exhaustive lazy matching, exact
construction/projection, and typed holes. Only a complete selected closure enters independently
verified Core IR. One explicit-frame interpreter is the correctness oracle.

Managed immutable bytes retain a verified optimization and an allocate-new differential oracle.
Full workspace snapshots and deterministic query scans remain because current reusable-release
workloads have not crossed their replacement gates. There is no registry, resolver, lockfile,
mutable semantic store, signature system, host effect, sandbox, bytecode, JIT, AOT, or native-code
artifact.

## Verification

The supported bootstrap is stable Rust edition 2024 on Linux x86-64. The crate forbids local unsafe
Rust.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run retained public workflows with:

```sh
for example in \
  job-policy named-data release-channel release-manifest \
  binary-canonicalizer reusable-release agent-maintenance
do
  "examples/$example/run.sh"
done
```

This is not a formal proof, hostile-host sandbox, cross-platform claim, registry, or production
deployment system. The trusted computing base includes Rust, Cargo, resolved dependencies, the
operating system, filesystem, and CPU.

## Current documentation

- [typed semantic program and identity model](docs/spec/semantic-model.md)
- [language](docs/spec/language.md)
- [reusable semantic releases](docs/spec/reusable-release.md)
- [application artifact and invocation](docs/spec/application.md)
- [protocol and editable documents](docs/spec/protocol.md)
- [architecture](docs/architecture.md)
- [implemented status](docs/status.md)
- [measurements and decisions](docs/performance.md)
- [future evidence gates](docs/roadmap.md)
