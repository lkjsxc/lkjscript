# lkjscript

`lkjscript` is a meaning-oriented programming language and application platform. One accepted
revision of a typed Graph 5 repository is the sole editable authority for a program. Names are
mutable locators; stable typed identities preserve continuity. Source text, compact requests,
indexes, compiler caches, artifacts, deployment descriptors, and runtime handles are projections
or consumers rather than alternate program truths.

The released executable supports an offline command-application lifecycle through one copied
binary. It creates Graph 5 projects, inspects and changes accepted meaning, runs graph-owned tests,
builds deterministic artifact-10 bundles, and executes pure command targets through both the
production VM and an implementation-disjoint semantic reference interpreter.

The public binary target is exactly `x86_64-unknown-linux-gnu`. The current candidate requires
the ELF interpreter `/lib64/ld-linux-x86-64.so.2`, `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and
GLIBC 2.38 or newer. No broader Linux portability is claimed.

## Download

Download the latest supported archive and its checksum without running a remote installer:

```sh
mkdir -p /tmp/lkjscript-download
cd /tmp/lkjscript-download
curl --fail --location --remote-name \
  https://github.com/lkjsxc/lkjscript/releases/latest/download/lkjscript-x86_64-unknown-linux-gnu.tar.gz
curl --fail --location --remote-name \
  https://github.com/lkjsxc/lkjscript/releases/latest/download/SHA256SUMS
sha256sum --check SHA256SUMS
tar -xzf lkjscript-x86_64-unknown-linux-gnu.tar.gz
./lkjscript/lkjscript capabilities
```

The archive also contains the Apache-2.0 project license, exact third-party notices, and canonical
release metadata. Its stable filename makes the latest URL durable; the
[`v0.1.1` release page](https://github.com/lkjsxc/lkjscript/releases/tag/v0.1.1) owns the immutable
version-specific bytes. See the [maintainer release procedure](docs/release.md) for identity,
packaging, verification, and recovery details.

Installation is optional. Select a directory you own rather than piping a download into a shell:

```sh
install_dir="$PWD/bin"
mkdir -p "$install_dir"
install -Dm755 ./lkjscript/lkjscript "$install_dir/lkjscript"
"$install_dir/lkjscript" capabilities
```

## Start from one binary

Use the extracted or installed executable outside the checkout to create a useful command project:

```sh
mkdir -p /tmp/lkjscript-demo
cp /tmp/lkjscript-download/lkjscript/lkjscript /tmp/lkjscript-demo/lkjscript
cd /tmp/lkjscript-demo
./lkjscript capabilities
./lkjscript new ./hello --template command --name hello
./lkjscript --project ./hello status
./lkjscript --project ./hello check
./lkjscript --project ./hello build --output ./hello.lkja
./lkjscript --project ./hello run main
```

The final command returns the typed text value `"hello"`. The command recipe owns one application
module, a private pure implementation, a component and port, target `main`, one graph-owned test,
and an exact dependency on the built-in standard package. It contains Graph 5 authority only and
does not read the checkout, Cargo, the network, or an external template. Use `--template minimal`
for an empty dependency-free package.

Project creation accepts an absent or empty ordinary destination. It rejects invalid names,
nonempty destinations, non-directory parents, and symlink path components before visibility. The
repository is built and synchronized in a private sibling stage, then made visible by one rename.

`check`, `build`, and `run` share exact project discovery, dependency resolution, compilation,
artifact linking/loading, and dense runtime preparation. `check` runs every graph-owned test
through both execution tiers. `build` requires an explicit absent output path and never replaces a
file, directory, or symlink. `run` accepts a pure command target and the strict bounded JSON-array
argument adapter:

```sh
./lkjscript --project ./hello run main --arguments '[]'
```

All three operations identify the exact observed revision and leave accepted `HEAD` unchanged.

## Inspect and change meaning

Global `--project PATH` selects a repository. From within a repository, discovery also walks
ordinary parent directories without following symbolic links:

```sh
./lkjscript --project ./hello query find module application
./lkjscript --project ./hello query owners --limit 20
./lkjscript --project ./hello inspect owner module mod_...
```

Queries read canonical owner bindings and committed namespace/relation witnesses at one revision.
Growing results use bounded pages and revision-bound `qcont_` continuations; query never writes a
cursor, index, or semantic revision.

Changes are typed semantic intent. For a direct rename, use the exact revision and owner returned
by `status` and `query`:

```sh
./lkjscript --project ./hello change plan rename.owner \
  --base rev_... --owner mod_... --name application-renamed
./lkjscript --project ./hello change apply rename.owner \
  --base rev_... --owner mod_... --name application-renamed --plan plan_...
```

Larger changes use strict compact records:

```sh
cat >change.lkjc <<'EOF'
request base=rev_...
create.module as=$notes name=notes
create.record as=$note module=$notes name=Note visibility=public
add.field as=$text record=$note name=text type=text
EOF
./lkjscript --project ./hello change plan --input-file change.lkjc \
  --output ./change.logical-plan
./lkjscript --project ./hello change apply --input-file change.lkjc --plan plan_...
```

Plan and apply share parsing, normalization, allocation, impact analysis, validation, selected
tests, and logical-result construction. The reviewed token binds both the request and its complete
logical semantic effects. The optional plan file is external evidence and is never imported as
authority. Apply reprepares against the exact base under the publication protocol.

After an accepted change, the executable may update an exact base compiler cache while the
prepared publication remains in memory. Cache status is reported separately. Cache failure cannot
roll back or misreport an accepted semantic revision; the next lifecycle command clean-builds and
replaces invalid derived state.

Run focused discovery for exhaustive current grammar, limits, and response fields:

```sh
./lkjscript capabilities change
./lkjscript capabilities query
./lkjscript capabilities check
./lkjscript capabilities build
./lkjscript capabilities run
```

## Built-in standard package

The executable embeds one exact package transport and one exact artifact-10 bundle generated from
`packages/standard`:

```sh
./lkjscript package builtin inspect
./lkjscript package builtin export --kind transport --output ./standard.lkjp
./lkjscript package builtin export --kind artifact --output ./standard.lkja
```

Both assets are strictly decoded and cross-checked at initialization. Product verification
regenerates their maintained owners and compares the bytes exactly. The built-in is not a general
package registry and never performs ambient path or network resolution.

## Maintained consumers

The standard package and `lkjournal` are Graph 5 repositories and use the same lifecycle:

```sh
./target/release/lkjscript --project packages/standard check
./target/release/lkjscript --project packages/standard build \
  --output /tmp/standard-current.lkja
./target/release/lkjscript --project applications/lkjournal check
./target/release/lkjscript --project applications/lkjournal build \
  --output /tmp/lkjournal-current.lkja
```

Their checked-in files under `generated/` are deterministic current outputs. The standard artifact
and transport also own the executable's built-in bytes.

`serve` and `worker` are deliberately separate. They currently load the explicitly named frozen
artifact-4 file under `applications/lkjournal/frozen-service/` plus deployment descriptors and
host adapters. They do not open an editable project repository, and the frozen artifact is not a
current build output or evidence of normalized service completion:

```sh
export LKJOURNAL_DATABASE_URL='postgresql://operator:password@127.0.0.1/lkjournal'
export LKJOURNAL_BOOTSTRAP_TOKEN='replace-with-a-random-bootstrap-token'
cd applications/lkjournal
../../target/release/lkjscript serve --deployment service.deployment.json
../../target/release/lkjscript worker --deployment worker.deployment.json
```

The HTTP listener is plaintext and PostgreSQL uses `NoTls`; encrypted transport requires an
external trusted boundary.

## Public surface and compatibility

The executable registry is the exhaustive owner of current operations, request/response models,
contracts, limits, diagnostics, and security nonclaims. See the generated
[contract table](docs/generated/contracts.md) and [operation table](docs/generated/operations.md).
Finite outcomes use deterministic bounded compact records and keep stderr empty.

Predecessor Graph 4 repositories are rejected before mutation or cache work. Removed operations
such as `draft`, `history`, general package staging, `review`, `backup`, `restore`, and `doctor` are
absent from discovery and receive the ordinary `cli_usage` failure. There is no compatibility
flag, legacy mode, migration command, graph edition, fallback reader, or dual write.

## Build and verify the repository

Application users need only the executable. Contributors use the repository-owned verification
profiles:

```sh
cargo build --workspace --release --locked
cargo run --locked -p lkjscript-dev -- check focused
cargo run --locked -p lkjscript-dev -- check changed
cargo run --locked -p lkjscript-dev -- check product
cargo run --locked -p lkjscript-dev -- check service
cargo run --locked -p lkjscript-dev -- check full
```

The harness records exact fingerprints, classifications, receipts, and bounded child logs under
`.artifacts/lkjscript-dev/check/`. The authoritative `full` profile requires fresh gates.

Normative contracts live under [docs/spec](docs/spec), current facts and limitations in
[docs/status.md](docs/status.md), the dependency map in
[docs/architecture.md](docs/architecture.md), and measurements in
[docs/performance.md](docs/performance.md).

The platform does not claim hostile-code sandboxing, multi-tenant isolation, distributed
consensus, encrypted graph storage, artifact signatures, or portability beyond its verified
environment.
