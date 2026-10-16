# lkjscript

A language and application platform for agents. Build, inspect, change, test and
run typed programs through one executable.

The long-term aim is to become stronger by building and owning as much of the
platform as feasible: libraries, development tools, compiler, runtime and
persistence, eventually in lkjscript itself. This is a direction, not a claim of
completed self-hosting. Current Rust and operating-system boundaries are bootstrap
mechanisms; native replacements must own useful behavior and its failures.

The accepted typed meaning graph is the program's editable authority. Native text
proposes reviewed changes; stable identities preserve declarations through edits.
Types, ownership, effects and exact dependencies make execution authority explicit.

Ordinary application and library development uses the installed product.
[Current status](docs/status.md) owns availability; [direction](docs/direction.md)
and [roadmap](docs/roadmap.md) explain the language's design and next priorities.

## Get started

The public binary supports Linux x86-64 with a statically linked musl executable.
Download the selected public installer into a fresh temporary directory and read
it before execution:

```sh
lkjscript_install_dir=$(mktemp -d) &&
curl -q --fail --location --proto '=https' --proto-redir '=https' \
  --connect-timeout 15 --max-time 180 --max-filesize 16384 \
  --output "$lkjscript_install_dir/install.sh" \
  https://github.com/lkjsxc/lkjscript/releases/latest/download/install.sh &&
cat "$lkjscript_install_dir/install.sh"
```

After a successful download and inspection, install in the same shell:

```sh
sh "$lkjscript_install_dir/install.sh" --prefix "$HOME/.local"
export PATH="$HOME/.local/bin:$PATH"
lkjscript runtime list
```

The installer requires standard shell tools, curl, sha256sum and tar. Its embedded
hashes bind the downloaded archive and executable to the inspected script. It
retains immutable runtime slots and leaves shell profiles and application data
alone. See [installation and recovery](docs/spec/product-surface.md#local-runtime-installation)
and [the release procedure](docs/release.md) for exact selection and offline use.

In a fresh working directory, create a command application:

```sh
lkjscript new hello --template command --name hello
lkjscript --project hello status
lkjscript --project hello check
lkjscript --project hello run main
lkjscript --project hello build --output hello/generated/application.lkja
lkjscript run --deployment hello/command.deployment.json
```

Both runs return `"hello"`. The deployment runs the accepted bundle independently
of the authoring graph. Keep the executable, bundle and deployment descriptor
for standalone use; the bundle depends on its runtime. Follow
[your first native command](docs/guides/native-command.md) to author a function,
add an expected-value test and publish a reviewed change.

Run `lkjscript gc --project hello` to compact reachable repository objects into one
compressed pack while preserving accepted history and selected dependencies. See
[storage compaction](docs/spec/semantic-storage.md#reachability-compaction) for reader
leases, capacity and interruption behavior.

## Build with the language

| Capability | Explore |
| --- | --- |
| Ordinary generic libraries and exact offline dependencies | [Native libraries](docs/guides/native-library.md), [standard supplier](packages/standard/README.md) |
| Affine working storage for ordinary data | [Data sequences](docs/guides/native-data-sequences.md), [mixed storage contracts](examples/indexed-data-storage/README.md), [generic consumer](examples/data-sequences/README.md), [native dependency planner](examples/dependency-plan/README.md) |
| Owned memory, collections and exact implementation witnesses | [Owned generics](docs/guides/native-owned-generics.md), [products](docs/guides/native-owned-products.md), [recoverable outcomes](docs/guides/native-owned-choices.md), [scoped borrowing](docs/guides/native-owned-borrows.md), [dynamic sequences](docs/guides/native-owned-sequences.md), [parameterized worklists](docs/guides/native-owned-worklists.md), [borrowed results](docs/guides/native-owned-read-results.md), [effectful composition](examples/owned-effects/README.md) |
| Generic owned implementations with explicit prerequisites | [Implementation schemes and adapters](docs/guides/native-generic-owned-implementations.md), [independent item witness](examples/generic-owned-implementations/README.md), [nested prerequisite composition](examples/composable-owned-implementations/README.md) |
| Transferable generic parallel groups and joined reusable workers | [Parallel guide](docs/guides/native-transferable-parallel.md), [scoped parallel reads](docs/guides/native-scoped-parallel-reads.md), [transformation and reduction](examples/parallel-work/README.md) |
| Typed web applications and form handling | [Web starter](docs/guides/native-web.md), [HTTP](docs/guides/native-http.md), [forms](docs/guides/native-forms.md) |
| Stateful applications, subscriptions and durable work | [Native editor](docs/guides/native-editor.md), [stateful HTTP](docs/generated/stateful-http-authoring.md), [lkjournal](applications/lkjournal/README.md) |
| Multiple services in one process with private instance authority | [Shared runtime](docs/spec/shared-runtime.md), [resident execution policy](docs/guides/resident-policy.md) |

Use `lkjscript capabilities` to discover the selected executable's operations,
grammar, authority boundaries and limits. [Guides](docs/guides/) explain authoring;
[specifications](docs/spec/) define language semantics.
[Security](docs/security.md) describes execution grants and trust boundaries.

[Nominal owned records](docs/spec/owned-records.md) let a library expose an affine
type while its defining module owns construction and field access. The standard
[FIFO](docs/guides/native-owned-fifo.md) is the maintained consumer; current source
acceptance and public availability are recorded separately in status.

## Inspect and change programs

```sh
lkjscript capabilities change
lkjscript capabilities query
lkjscript --project hello query find module application
lkjscript --project hello query owners --limit 20
```

Use `lkjscript --project hello tx --input-file delta.lkjx` for
[symbolic upserts](docs/spec/concurrent-changes.md#symbolic-transactions): names select
existing identities, both evaluators check affected tests, and one atomic publication
returns a JSON summary. `capabilities tx` describes the bounded grammar.

For a separately reviewed proposal, use `change plan --input-file ...`, apply the
same request with its exact token, then run `check`. Rejected proposals cannot
partially publish meaning.

When another candidate publishes first, [refresh your reviewed proposal](docs/spec/concurrent-changes.md)
at the revision from current `status`, retaining the original input and plan:

```sh
lkjscript --project hello change refresh --input-file proposal.lkjc --plan ORIGINAL_TOKEN --onto CURRENT_REVISION
```

Review the renewed token before applying the original input. Conflicting meaning
rejects refresh; disjoint edits compose with preserved identities.

`change draft` reconstructs declarations from accepted owners. Follow the
[command guide](docs/guides/native-command.md), [grammar](docs/generated/change-grammar.md)
and [function inspection](docs/generated/function-definition.md).

## Develop the language

Use the pinned Rust toolchain and locked dependencies. Read
[repository guidance](AGENTS.md), [architecture](docs/architecture.md) and
[verification](docs/spec/verification.md) before changing the language.

From the canonical checkout root, bootstrap a clean committed source with the
[shared bootstrap command](tools/bootstrap-verifier.sh):

```bash
verifier_root=$(bash tools/bootstrap-verifier.sh)
CARGO_TARGET_DIR="$PWD/target" "$verifier_root/lkjscript-dev" check changed --jobs 1 --machine
CARGO_TARGET_DIR="$PWD/target" "$verifier_root/lkjscript-dev" check full --fresh --jobs 1 --machine
```

The command creates an owned cold target, builds the pinned verifier offline with
locked dependencies in the optimized `bootstrap` profile, admits its exact fresh Cargo artifact and retains a byte-identical
read-only executable. Original source, compiler/configuration context, command, process
and Cargo records remain in `"$verifier_root/evidence"`. Fetch locked dependencies
first if they are unavailable locally. `--output ABSENT_ABSOLUTE_ROOT` selects an
explicit root; `--expected-source COMMIT` binds an expected commit;
`--with-controller` also retains the release controller.

Bootstrap refuses effective Cargo config files and inherited compiler, flags,
profile or target overrides. Inherited settings are recorded as set/unset states
and value hashes. Positive `CARGO_BUILD_JOBS` reaches the build child;
`CARGO_ABOUT_VERSION` and `CARGO_ABOUT_ARCHIVE_SHA256` are recorded separately.
Set build concurrency and checker `--jobs` independently.

The verifier stays bound to its compile-time checkout. The normal check target
independently needs a cold first CHECK8 run or authenticated matching unit origins;
bootstrap neither clears nor certifies existing outputs. `changed` selects current
Git-status paths, so a clean-tree run does not prove a committed change. See
[verification](docs/spec/verification.md) for admission and
[release acceptance](docs/release.md#coverage-and-admission) for source and final bytes.

lkjscript is licensed under [Apache-2.0](LICENSE).
