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

Author a native proposal, inspect it with `change plan --input-file ...`, apply
the same request with its exact token, then run `check`. Rejected proposals cannot
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

Build the verifier in a new owned bootstrap-only target, retain its exact source
snapshots and original Cargo output, select the executable from that output, and
copy it immutably before running checks. For example, from the selected checkout:

```bash
set -euo pipefail
capture_bootstrap_source() {
  source_head=$(git rev-parse HEAD) || return "$?"
  source_tree=$(git rev-parse 'HEAD^{tree}') || return "$?"
  source_status=$(git status --porcelain=v1 --untracked-files=all) || return "$?"
  jq -n --arg source "$source_head" --arg tree "$source_tree" --arg status "$source_status" \
    '{source:$source,tree:$tree,status:$status}' > "$1"
}
bootstrap_value_hash() {
  local line
  line=$(printf '%s' "$1" | sha256sum) || return "$?"
  printf '%s' "${line%% *}"
}
bootstrap_file_proof() {
  local path=$1 maximum=$2 parent next before after mode bytes hash rest
  if [[ ! -e "$path" && ! -L "$path" ]]; then
    parent=$(dirname -- "$path") || return "$?"
    while [[ ! -e "$parent" && ! -L "$parent" ]]; do
      next=$(dirname -- "$parent") || return "$?"
      [[ "$next" != "$parent" ]] || return 1
      parent=$next
    done
    [[ -d "$parent" && -x "$parent" ]] || return 1
    jq -cn --arg path "$path" '{path:$path,kind:"absent"}'
    return "$?"
  fi
  if [[ ! -f "$path" || -L "$path" ]]; then
    jq -cn --arg path "$path" '{path:$path,kind:"unsupported"}'
    return "$?"
  fi
  before=$(stat -c '%a|%s|%d|%i|%y|%z' -- "$path") || return "$?"
  IFS='|' read -r mode bytes rest <<< "$before"
  [[ "$mode" =~ ^[0-7]+$ && "$bytes" =~ ^[0-9]+$ ]] || return 1
  printf -v mode '%04o' "$((8#$mode))"
  if (( bytes > maximum )); then
    jq -cn --arg path "$path" --arg mode "$mode" --argjson bytes "$bytes" \
      '{path:$path,mode:$mode,kind:"oversized",bytes:$bytes}'
    return "$?"
  fi
  hash=$(head -c "$((maximum + 1))" -- "$path" | sha256sum) || return "$?"
  after=$(stat -c '%a|%s|%d|%i|%y|%z' -- "$path") || return "$?"
  [[ "$before" == "$after" ]] || return 1
  jq -cn --arg path "$path" --arg mode "$mode" --argjson bytes "$bytes" \
    --arg sha256 "${hash%% *}" '{path:$path,mode:$mode,kind:"file",bytes:$bytes,sha256:$sha256}'
}
bootstrap_context() {
  local output=$1 repo name value digest item kind current next paths sorted exported bytes
  local parent_environment='{}' override_states='{}' configs='[]' errors='[]'
  local cargo_proof rustc_proof child_environment count=0 total=0
  local -A candidates=() overrides=()
  repo=$(readlink -e "$PWD") || return "$?"
  if [[ "$repo" != "$PWD" ]]; then errors=$(jq -c '.+=["checkout is not canonical"]' <<< "$errors") || return "$?"; fi
  for name in HOME bootstrap_cargo_home bootstrap_rustup_home bootstrap_target; do
    value=${!name}
    current=$(readlink -m -- "$value") || return "$?"
    if [[ "$value" != /* || "$current" != "$value" ]]; then
      errors=$(jq -c --arg name "$name" '.+=["path absent or not canonical: "+$name]' <<< "$errors") || return "$?"
    fi
  done
  [[ -n "$PATH" ]] || return 1
  for name in HOME PATH CARGO_HOME RUSTUP_HOME LANG LC_ALL TMPDIR TZ CARGO_TERM_COLOR CARGO_NET_OFFLINE \
    CARGO_BUILD_JOBS CARGO_ABOUT_VERSION CARGO_ABOUT_ARCHIVE_SHA256; do
    if [[ -v "$name" ]]; then
      value=${!name}
      digest=$(bootstrap_value_hash "$value") || return "$?"
      parent_environment=$(jq -c --arg name "$name" --arg sha256 "$digest" \
        '.[$name]={state:"set",value_sha256:$sha256}' <<< "$parent_environment") || return "$?"
      if [[ ( "$name" == CARGO_HOME || "$name" == RUSTUP_HOME ) && -z "$value" ]]; then
        errors=$(jq -c --arg name "$name" '.+=["empty home override: "+$name]' <<< "$errors") || return "$?"
      fi
    else
      parent_environment=$(jq -c --arg name "$name" '.[$name]={state:"unset"}' <<< "$parent_environment") || return "$?"
    fi
  done
  for name in AR CC CFLAGS CXX CXXFLAGS PKG_CONFIG_PATH RANLIB RUSTC RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER \
    RUSTFLAGS RUSTDOC RUSTDOCFLAGS RUSTC_BOOTSTRAP RUSTUP_TOOLCHAIN SOURCE_DATE_EPOCH CARGO_BUILD_TARGET \
    CARGO_ENCODED_RUSTFLAGS CARGO_ENCODED_RUSTDOCFLAGS CARGO_TARGET_DIR; do overrides[$name]=1; done
  exported=$(compgen -e) || return "$?"
  while IFS= read -r name; do
    case "$name" in
      CARGO_HOME|CARGO_NET_OFFLINE|CARGO_TERM_COLOR|CARGO_BUILD_JOBS|CARGO_ABOUT_VERSION|CARGO_ABOUT_ARCHIVE_SHA256) ;;
      CARGO_*) overrides[$name]=1 ;;
    esac
  done <<< "$exported"
  sorted=$(printf '%s\n' "${!overrides[@]}" | LC_ALL=C sort) || return "$?"
  while IFS= read -r name; do
    if [[ -v "$name" ]]; then
      digest=$(bootstrap_value_hash "${!name}") || return "$?"
      override_states=$(jq -c --arg name "$name" --arg sha256 "$digest" \
        '.[$name]={state:"set",value_sha256:$sha256}' <<< "$override_states") || return "$?"
      errors=$(jq -c --arg name "$name" '.+=["unsupported build override: "+$name]' <<< "$errors") || return "$?"
    else
      override_states=$(jq -c --arg name "$name" '.[$name]={state:"unset"}' <<< "$override_states") || return "$?"
    fi
  done <<< "$sorted"
  current=$repo
  while :; do
    (( count += 1 )); (( count <= 64 )) || return 1
    candidates["${current%/}/.cargo/config"]=1; candidates["${current%/}/.cargo/config.toml"]=1
    [[ "$current" != / ]] || break
    next=$(dirname -- "$current") || return "$?"
    current=$next
  done
  candidates["${bootstrap_cargo_home%/}/config"]=1; candidates["${bootstrap_cargo_home%/}/config.toml"]=1
  (( ${#candidates[@]} <= 130 )) || return 1
  paths=$(printf '%s\n' "${!candidates[@]}" | LC_ALL=C sort) || return "$?"
  while IFS= read -r current; do
    item=$(bootstrap_file_proof "$current" 1048576) || return "$?"
    kind=$(jq -er '.kind' <<< "$item") || return "$?"
    configs=$(jq -c --argjson item "$item" '.+=[$item]' <<< "$configs") || return "$?"
    bytes=$(jq -er '.bytes // 0' <<< "$item") || return "$?"
    total=$((total + bytes)); (( total <= 8388608 )) || return 1
    if [[ "$kind" != absent ]]; then
      errors=$(jq -c '.+=["bootstrap requires absent effective Cargo configs"]' <<< "$errors") || return "$?"
    fi
  done <<< "$paths"
  cargo_proof=$(bootstrap_file_proof "$bootstrap_cargo" 536870912) || return "$?"
  rustc_proof=$(bootstrap_file_proof "$bootstrap_rustc" 536870912) || return "$?"
  if ! jq -e --argjson cargo "$cargo_proof" --argjson rustc "$rustc_proof" \
    '$cargo.kind=="file" and $rustc.kind=="file" and ($cargo.path|startswith("/")) and ($rustc.path|startswith("/"))' \
    <<< null > /dev/null; then errors=$(jq -c '.+=["compiler/Cargo is not a regular absolute file"]' <<< "$errors") || return "$?"; fi
  child_environment=$(jq -cn --args '$ARGS.positional|map(split("=")|{key:.[0],value:(.[1:]|join("="))})|from_entries' \
    "${bootstrap_env[@]}") || return "$?"
  if [[ -v CARGO_BUILD_JOBS && ! "$CARGO_BUILD_JOBS" =~ ^[1-9][0-9]*$ ]]; then
    errors=$(jq -c '.+=["CARGO_BUILD_JOBS must be a positive integer"]' <<< "$errors") || return "$?"
  fi
  jq -nS --arg repo "$repo" --arg target "$bootstrap_target" --argjson errors "$errors" \
    --argjson parent "$parent_environment" --argjson overrides "$override_states" --argjson configs "$configs" \
    --argjson cargo "$cargo_proof" --argjson rustc "$rustc_proof" --argjson child "$child_environment" \
    '{schema:"lkjscript.bootstrap_context.no_config_no_override.v1",repository:$repo,target:$target,
      supported:($errors|length==0),errors:$errors,parent_environment:$parent,override_states:$overrides,
      configs:$configs,executables:{cargo:$cargo,rustc:$rustc},child_environment:$child}' > "$output"
}
mkdir -p .artifacts
bootstrap_root=$(mktemp -d "$PWD/.artifacts/verifier-bootstrap.XXXXXXXX")
bootstrap_target="$bootstrap_root/target"
test -d "$bootstrap_root" && test ! -L "$bootstrap_root"
test ! -e "$bootstrap_target" && test ! -L "$bootstrap_target"
bootstrap_cargo=$(rustup which --toolchain 1.98.0 cargo)
bootstrap_cargo=$(readlink -e "$bootstrap_cargo")
bootstrap_rustc=$(rustup which --toolchain 1.98.0 rustc)
bootstrap_rustc=$(readlink -e "$bootstrap_rustc")
bootstrap_sysroot=$("$bootstrap_rustc" --print sysroot)
bootstrap_sysroot_rustc=$(readlink -e "$bootstrap_sysroot/bin/rustc")
test "$bootstrap_sysroot_rustc" = "$bootstrap_rustc"
bootstrap_cargo_home=${CARGO_HOME-$HOME/.cargo}
bootstrap_rustup_home=${RUSTUP_HOME-$HOME/.rustup}
bootstrap_env=(
  "HOME=$HOME" "PATH=$PATH" "CARGO_HOME=$bootstrap_cargo_home" "RUSTUP_HOME=$bootstrap_rustup_home"
  "CARGO_TARGET_DIR=$bootstrap_target" "RUSTC=$bootstrap_rustc" "CARGO_NET_OFFLINE=true"
  "CARGO_TERM_COLOR=never" "LANG=C"
)
if [[ -v CARGO_BUILD_JOBS ]]; then bootstrap_env+=("CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS"); fi
capture_bootstrap_source "$bootstrap_root/source-before.json"
bootstrap_context "$bootstrap_root/context-before.json"
jq -e '.supported == true' "$bootstrap_root/context-before.json" > /dev/null
# This example pins a clean committed source. Dirty inputs need complete source snapshots.
jq -e '.status == ""' "$bootstrap_root/source-before.json" > /dev/null
{ "$bootstrap_cargo" --version; "$bootstrap_rustc" --version --verbose; } > "$bootstrap_root/toolchain.txt"
jq -n --arg cwd "$PWD" --arg target "$bootstrap_target" --arg cargo "$bootstrap_cargo" \
  --slurpfile context "$bootstrap_root/context-before.json" \
  --arg started_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  '{cwd:$cwd,started_at:$started_at,launcher:["env","-i"],environment:$context[0].child_environment,argv:[$cargo,"build","--release","--locked",
    "-p","lkjscript-dev","--bin","lkjscript-dev","--target-dir",$target,"--message-format=json"]}' \
  > "$bootstrap_root/command.json"
set +e
env -i "${bootstrap_env[@]}" "$bootstrap_cargo" build --release --locked -p lkjscript-dev --bin lkjscript-dev \
  --target-dir "$bootstrap_target" --message-format=json \
  > "$bootstrap_root/cargo.jsonl" 2> "$bootstrap_root/cargo.stderr"
bootstrap_status=$?
set -e
jq -n --argjson exit_code "$bootstrap_status" \
  --arg completed_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  '{completed_at:$completed_at,exit_code:$exit_code}' > "$bootstrap_root/process.json"
capture_bootstrap_source "$bootstrap_root/source-after.json"
bootstrap_context "$bootstrap_root/context-after.json"
test "$bootstrap_status" -eq 0
cmp "$bootstrap_root/source-before.json" "$bootstrap_root/source-after.json"
cmp "$bootstrap_root/context-before.json" "$bootstrap_root/context-after.json"
jq -e '.supported == true' "$bootstrap_root/context-after.json" > /dev/null
jq -e -s --arg manifest "$PWD/tools/lkjscript-dev/Cargo.toml" \
  --arg entry "$PWD/tools/lkjscript-dev/src/main.rs" --arg workspace "$PWD/" \
  --arg executable "$bootstrap_target/release/lkjscript-dev" \
  --slurpfile source "$bootstrap_root/source-before.json" '
  [.[] | select(.reason == "compiler-artifact"
    and (.manifest_path | startswith($workspace)))] as $workspace_artifacts
  | if ($workspace_artifacts | length) == 0 or any($workspace_artifacts[]; .fresh != false)
    then error("bootstrap workspace artifact was not freshly built") else . end
  | [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
    and .target.name == "lkjscript-dev" and .target.kind == ["bin"]
    and .target.src_path == $entry and .profile.test == false and .profile.opt_level == "3"
    and .fresh == false and .executable == $executable)] as $tools
  | [.[] | select(.reason == "build-finished")] as $finished
  | if ($tools | length) == 1 and ($finished | length) == 1 and $finished[0].success == true
    then {source:$source[0],checker:$tools[0],build_finished:$finished[0],workspace_artifacts:$workspace_artifacts}
    else error("bootstrap artifact/completion mismatch") end
  ' "$bootstrap_root/cargo.jsonl" > "$bootstrap_root/selection.json"
checker=$(jq -er '.checker.executable' "$bootstrap_root/selection.json")
test -f "$checker" && test ! -L "$checker"
test ! -e "$bootstrap_root/lkjscript-dev" && test ! -L "$bootstrap_root/lkjscript-dev"
cp "$checker" "$bootstrap_root/lkjscript-dev"
chmod 0555 "$bootstrap_root/lkjscript-dev"
cmp "$checker" "$bootstrap_root/lkjscript-dev"
sha256sum "$checker" "$bootstrap_root/lkjscript-dev" > "$bootstrap_root/checker.sha256"
CARGO_TARGET_DIR="$PWD/target" "$bootstrap_root/lkjscript-dev" check changed --jobs 1 --machine
CARGO_TARGET_DIR="$PWD/target" "$bootstrap_root/lkjscript-dev" check full --fresh --jobs 1 --machine
```

This bootstrap example supports only absent effective Cargo config files and no
inherited compiler/flags/profile/target overrides. A declared positive
`CARGO_BUILD_JOBS` is recorded and passed to the same child; `CARGO_ABOUT_VERSION` and `CARGO_ABOUT_ARCHIVE_SHA256` are recorded and
excluded from child build controls. It records every ancestor/Cargo-home config candidate,
actual Cargo/compiler byte identities and approved environment/unset states before
and after building; existing config files are hashed without retaining their contents
and refused. Cargo receives only the declared closed environment. Keep these context
records with the complete source, command and process originals; the Cargo excerpt
and a matching checksum do not replace those bindings. The verifier remains bound to its compile-time checkout. The normal check
target must independently be cold at its first CHECK8 run or have authenticated
matching unit origins. This recipe does not clear it or certify pre-existing,
restored or relocated workspace outputs. Later warm checks retain the same origin
requirements. Use a real canonical checkout path for the example's exact selectors.
`cargo run --release` before checking would seed the same release target again;
ordinary debug-only development runs are not release-producer evidence.

`changed` selects current Git-status paths; a clean-tree run does not prove a
committed change. Set `CARGO_BUILD_JOBS` and checker `--jobs` independently.
[Release acceptance](docs/release.md#coverage-and-admission) checks source and final bytes
separately. Retain evidence in `.artifacts/`.

lkjscript is licensed under [Apache-2.0](LICENSE).
