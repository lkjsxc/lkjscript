#!/usr/bin/env bash
# Cold-build and retain source-bound verification tools. Stdout is the new root.
set -euo pipefail

usage() {
  printf '%s\n' 'usage: bash tools/bootstrap-verifier.sh [--output ABSENT_ABSOLUTE_ROOT] [--expected-source COMMIT] [--with-controller]' >&2
}
bootstrap_root=
expected_source=
with_controller=false
while (( $# )); do
  case "$1" in
    --output|--expected-source)
      (( $# >= 2 )) || { usage; exit 2; }
      case "$1" in
        --output) bootstrap_root=$2 ;;
        --expected-source) expected_source=$2 ;;
      esac
      shift 2 ;;
    --with-controller) with_controller=true; shift ;;
    --help) usage; exit 0 ;;
    *) usage; exit 2 ;;
  esac
done
[[ -z "$expected_source" || "$expected_source" =~ ^[0-9a-f]{40}$ ]] || { usage; exit 2; }
[[ $(readlink -e "$PWD") == "$PWD" ]]
[[ $(git rev-parse --show-toplevel) == "$PWD" ]]
if [[ -z "$bootstrap_root" ]]; then
  mkdir -p .artifacts
  bootstrap_root=$(mktemp -d "$PWD/.artifacts/verifier-bootstrap.XXXXXXXX")
else
  [[ "$bootstrap_root" == /* && $(readlink -m -- "$bootstrap_root") == "$bootstrap_root" ]]
  # mkdir refuses both an existing directory and a dangling symlink.
  mkdir -m 0700 -- "$bootstrap_root"
fi
trap 'printf "Bootstrap failed; retain originals at %s\n" "$bootstrap_root" >&2' ERR
bootstrap_target="$bootstrap_root/target"
evidence="$bootstrap_root/evidence"
mkdir -m 0700 "$evidence"
test ! -e "$bootstrap_target" && test ! -L "$bootstrap_target"

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

# Resolve the repository's exact pin, bypassing rustup shims in the build child.
bootstrap_toolchain=$(sed -n 's/^channel = "\([^"]*\)"$/\1/p' rust-toolchain.toml)
[[ "$bootstrap_toolchain" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
bootstrap_cargo=$(rustup which --toolchain "$bootstrap_toolchain" cargo)
bootstrap_cargo=$(readlink -e "$bootstrap_cargo")
bootstrap_rustc=$(rustup which --toolchain "$bootstrap_toolchain" rustc)
bootstrap_rustc=$(readlink -e "$bootstrap_rustc")
bootstrap_sysroot=$("$bootstrap_rustc" --print sysroot)
test "$(readlink -e "$bootstrap_sysroot/bin/rustc")" = "$bootstrap_rustc"
bootstrap_cargo_home=${CARGO_HOME-$HOME/.cargo}
bootstrap_rustup_home=${RUSTUP_HOME-$HOME/.rustup}
bootstrap_env=(
  "HOME=$HOME" "PATH=$PATH" "CARGO_HOME=$bootstrap_cargo_home" "RUSTUP_HOME=$bootstrap_rustup_home"
  "CARGO_TARGET_DIR=$bootstrap_target" "RUSTC=$bootstrap_rustc" "CARGO_NET_OFFLINE=true"
  "CARGO_TERM_COLOR=never" "LANG=C"
)
if [[ -v CARGO_BUILD_JOBS ]]; then bootstrap_env+=("CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS"); fi
capture_bootstrap_source "$evidence/bootstrap-source-before.json"
bootstrap_context "$evidence/bootstrap-context-before.json"
if ! jq -e '.supported == true' "$evidence/bootstrap-context-before.json" > /dev/null; then
  jq -r '.errors[]' "$evidence/bootstrap-context-before.json" >&2
  exit 1
fi
# A clean tree binds all compiler inputs to the retained commit/tree identity.
jq -e --arg expected "$expected_source" '.status == "" and ($expected == "" or .source == $expected)' \
  "$evidence/bootstrap-source-before.json" > /dev/null
{ "$bootstrap_cargo" --version; "$bootstrap_rustc" --version --verbose; } > "$evidence/bootstrap-toolchain.txt"
bootstrap_command=("$bootstrap_cargo" build --profile bootstrap --locked -p lkjscript-dev)
if "$with_controller"; then bootstrap_command+=(--bins); else bootstrap_command+=(--bin lkjscript-dev); fi
bootstrap_command+=(--target-dir "$bootstrap_target" --message-format=json)
jq -n --arg cwd "$PWD" --slurpfile context "$evidence/bootstrap-context-before.json" \
  --arg started_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --args \
  '{cwd:$cwd,started_at:$started_at,launcher:["env","-i"],environment:$context[0].child_environment,argv:$ARGS.positional}' \
  -- "${bootstrap_command[@]}" > "$evidence/bootstrap-command.json"
set +e
env -i "${bootstrap_env[@]}" "${bootstrap_command[@]}" \
  > "$evidence/bootstrap-cargo.jsonl" 2> "$evidence/bootstrap-cargo.stderr"
bootstrap_status=$?
set -e
jq -n --argjson exit_code "$bootstrap_status" --arg completed_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  '{completed_at:$completed_at,exit_code:$exit_code}' > "$evidence/bootstrap-process.json"
capture_bootstrap_source "$evidence/bootstrap-source-after.json"
bootstrap_context "$evidence/bootstrap-context-after.json"
test "$bootstrap_status" -eq 0
cmp "$evidence/bootstrap-source-before.json" "$evidence/bootstrap-source-after.json"
cmp "$evidence/bootstrap-context-before.json" "$evidence/bootstrap-context-after.json"
jq -e '.supported == true' "$evidence/bootstrap-context-after.json" > /dev/null
jq -e -s --arg manifest "$PWD/tools/lkjscript-dev/Cargo.toml" \
  --arg source_root "$PWD/tools/lkjscript-dev/src" --arg workspace "$PWD/" \
  --arg target "$bootstrap_target/bootstrap" --argjson controller "$with_controller" \
  --slurpfile source "$evidence/bootstrap-source-before.json" '
  def tool($name; $entry):
    [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
      and .target.name == $name and .target.kind == ["bin"]
      and .target.src_path == ($source_root + "/" + $entry)
      and .profile.test == false and .profile.opt_level == "2"
      and .profile.debuginfo == 0 and .profile.debug_assertions == true and .profile.overflow_checks == true
      and .fresh == false and .executable == ($target + "/" + $name))]
    | if length == 1 then .[0] else error("missing or ambiguous fresh bootstrap artifact") end;
  [.[] | select(.reason == "compiler-artifact" and (.manifest_path | startswith($workspace)))] as $workspace_artifacts
  | if ($workspace_artifacts | length) == 0 or any($workspace_artifacts[]; .fresh != false)
    then error("bootstrap workspace artifact was not freshly built") else . end
  | [.[] | select(.reason == "build-finished")] as $finished
  | if ($finished | length) != 1 or $finished[0].success != true
    then error("bootstrap did not finish successfully") else . end
  | tool("lkjscript-dev"; "main.rs") as $checker
  | (if $controller then {controller:tool("lkjscript-release-controller"; "release/controller_main.rs")} else {} end) as $extra
  | {source:$source[0],checker:$checker,build_finished:$finished[0],workspace_artifacts:$workspace_artifacts} + $extra
  ' "$evidence/bootstrap-cargo.jsonl" > "$evidence/bootstrap-selection.json"
bootstrap_tools=(lkjscript-dev)
if "$with_controller"; then bootstrap_tools+=(lkjscript-release-controller); fi
for name in "${bootstrap_tools[@]}"; do
  if [[ "$name" == lkjscript-dev ]]; then key=checker; else key=controller; fi
  selected=$(jq -er --arg key "$key" '.[$key].executable' "$evidence/bootstrap-selection.json")
  test -f "$selected" && test ! -L "$selected"
  test ! -e "$bootstrap_root/$name" && test ! -L "$bootstrap_root/$name"
  cp "$selected" "$bootstrap_root/$name"
  chmod 0555 "$bootstrap_root/$name"
  cmp "$selected" "$bootstrap_root/$name"
  sha256sum "$selected" "$bootstrap_root/$name" >> "$evidence/tools.sha256"
done
sha256sum "$evidence"/bootstrap-* > "$evidence/bootstrap-evidence.sha256"
printf '%s\n' "$bootstrap_root"
