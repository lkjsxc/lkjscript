#!/usr/bin/env bash
# Build source-bound verification tools in the one shared target. Stdout is its bin root.
set -euo pipefail

usage() {
  printf '%s\n' 'usage: bash tools/bootstrap-verifier.sh [--output ABSENT_ABSOLUTE_RECEIPT_ROOT] [--expected-source COMMIT] [--with-controller] [--retain-archive]' >&2
}
bootstrap_root=
expected_source=
with_controller=false
retain_archive=false
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
    --retain-archive) retain_archive=true; shift ;;
    --help) usage; exit 0 ;;
    *) usage; exit 2 ;;
  esac
done
[[ -z "$expected_source" || "$expected_source" =~ ^[0-9a-f]{40}$ ]] || { usage; exit 2; }
[[ $(readlink -e "$PWD") == "$PWD" ]]
[[ $(git rev-parse --show-toplevel) == "$PWD" ]]
bootstrap_target="$PWD/target"
[[ ! -L "$bootstrap_target" ]]
mkdir -p -- "$bootstrap_target"
# Cargo replaces executable paths. Share this lease with check before observing bytes.
exec {bootstrap_lease}> "$bootstrap_target/.lkjscript-check.lock"
flock --exclusive "$bootstrap_lease"
if [[ -z "$bootstrap_root" ]]; then
  if "$retain_archive"; then
    bootstrap_root=$(mktemp -d "$bootstrap_target/bootstrap-archive.XXXXXXXX")
  else
    bootstrap_root="$bootstrap_target/bootstrap-receipt"
    [[ ! -L "$bootstrap_root" ]]
    mkdir -p -- "$bootstrap_root"
  fi
else
  [[ "$bootstrap_root" == /* && $(readlink -m -- "$bootstrap_root") == "$bootstrap_root" ]]
  # mkdir refuses both an existing directory and a dangling symlink.
  mkdir -m 0700 -- "$bootstrap_root"
fi
evidence="$bootstrap_root/evidence"
bootstrap_phase=preparation
finish_bootstrap() {
  local result=$?
  trap - EXIT
  if (( result != 0 )); then
    set +e
    # A failed retry must replace a previous success, even for explicit exit paths.
    if [[ ! -L "$bootstrap_root/receipt.pending" && ! -L "$bootstrap_root/receipt.json" ]] &&
      jq -cn --arg target "$bootstrap_target" --arg phase "$bootstrap_phase" \
        --argjson code "$result" --argjson archived "$retain_archive" \
        '{schema:"lkjscript.bootstrap.v1",status:"failed",target:$target,
          phase:$phase,exit_code:$code,originals_retained:$archived}' \
        > "$bootstrap_root/receipt.pending"; then
      mv -- "$bootstrap_root/receipt.pending" "$bootstrap_root/receipt.json"
    elif [[ -f "$bootstrap_root/receipt.json" || -L "$bootstrap_root/receipt.json" ]]; then
      rm -- "$bootstrap_root/receipt.json"
    fi
    if ! "$retain_archive" && [[ -d "$evidence" && ! -L "$evidence" ]]; then
      rm -r -- "$evidence"
    fi
    printf 'Bootstrap failed during %s; receipt: %s/receipt.json\n' "$bootstrap_phase" "$bootstrap_root" >&2
  fi
  exit "$result"
}
trap finish_bootstrap EXIT
[[ ! -L "$evidence" ]]
if [[ -d "$evidence" ]]; then rm -r -- "$evidence"; fi
mkdir -p -m 0700 "$evidence"

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
bootstrap_native_mold_config() {
  # This is a closed TOML shape, not a general Cargo config parser. Preserve all
  # whitespace inside quoted flags and reject extra tables, keys and values.
  jq -eRs '
    (explode | all((. >= 32 and . != 127) or . == 9 or . == 10 or . == 13))
    and (split("\n") | map(select(test("^[ \\t]*(#.*)?\\r?$") | not))
      | length == 2
        and (.[0] | test("^[ \\t]*\\[target\\.x86_64-unknown-linux-gnu\\][ \\t]*(#.*)?\\r?$"))
        and (.[1] | test("^[ \\t]*rustflags[ \\t]*=[ \\t]*\\[[ \\t]*\"-C\"[ \\t]*,[ \\t]*\"link-arg=-fuse-ld=mold\"[ \\t]*\\][ \\t]*(#.*)?\\r?$")))
  ' "$1" > /dev/null
}
bootstrap_mold_proof() {
  local selected resolved proof
  selected=$(type -P "$1") || return "$?"
  [[ "$selected" == /* && -x "$selected" ]] || return 1
  resolved=$(readlink -e -- "$selected") || return "$?"
  [[ -x "$resolved" ]] || return 1
  proof=$(bootstrap_file_proof "$resolved" 536870912) || return "$?"
  jq -e '.kind == "file"' <<< "$proof" > /dev/null || return "$?"
  jq -cn --arg selected "$selected" --argjson resolved "$proof" \
    '{selected_path:$selected,resolved:$resolved}'
}
bootstrap_context() {
  local output=$1 repo name value digest item kind current next paths sorted exported bytes
  local parent_environment='{}' override_states='{}' configs='[]' errors='[]'
  local cargo_proof rustc_proof mold_proof=null ld_mold_proof=null needs_mold=false child_environment count=0 total=0
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
    CARGO_ENCODED_RUSTFLAGS CARGO_ENCODED_RUSTDOCFLAGS; do overrides[$name]=1; done
  exported=$(compgen -e) || return "$?"
  while IFS= read -r name; do
    case "$name" in
      CARGO_HOME|CARGO_TARGET_DIR|CARGO_NET_OFFLINE|CARGO_TERM_COLOR|CARGO_BUILD_JOBS|CARGO_ABOUT_VERSION|CARGO_ABOUT_ARCHIVE_SHA256) ;;
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
  if [[ -v CARGO_TARGET_DIR && "$CARGO_TARGET_DIR" != "$bootstrap_target" ]]; then
    errors=$(jq -c '.+=["CARGO_TARGET_DIR must name the shared checkout target"]' <<< "$errors") || return "$?"
  fi
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
    if [[ "$kind" == file ]] && bootstrap_native_mold_config "$current"; then
      [[ "$item" == "$(bootstrap_file_proof "$current" 1048576)" ]] || return 1
      needs_mold=true
    elif [[ "$kind" != absent ]]; then
      errors=$(jq -c '.+=["bootstrap supports only the exact native GNU target mold config"]' <<< "$errors") || return "$?"
    fi
  done <<< "$paths"
  if "$needs_mold"; then
    # Native compiler drivers select ld.mold for -fuse-ld=mold. Bind both
    # spellings and require one canonical executable, not merely matching names.
    if ! mold_proof=$(bootstrap_mold_proof mold) || ! ld_mold_proof=$(bootstrap_mold_proof ld.mold); then
      mold_proof=null
      ld_mold_proof=null
      errors=$(jq -c '.+=["configured mold and ld.mold must be executable regular files"]' <<< "$errors") || return "$?"
    elif ! jq -e --argjson mold "$mold_proof" --argjson ld "$ld_mold_proof" \
      '$mold.resolved == $ld.resolved' <<< null > /dev/null; then
      errors=$(jq -c '.+=["mold and ld.mold resolve to different executables"]' <<< "$errors") || return "$?"
    fi
  fi
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
    --argjson cargo "$cargo_proof" --argjson rustc "$rustc_proof" --argjson mold "$mold_proof" \
    --argjson ld_mold "$ld_mold_proof" \
    --argjson child "$child_environment" \
    '{schema:"lkjscript.bootstrap_context.shared_target.v1",repository:$repo,target:$target,
      supported:($errors|length==0),errors:$errors,parent_environment:$parent,override_states:$overrides,
      configs:$configs,executables:{cargo:$cargo,rustc:$rustc,mold:$mold,ld_mold:$ld_mold},child_environment:$child}' > "$output"
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
bootstrap_phase=cargo
env -i "${bootstrap_env[@]}" "${bootstrap_command[@]}" \
  > "$evidence/bootstrap-cargo.jsonl" 2> "$evidence/bootstrap-cargo.stderr"
bootstrap_status=$?
set -e
bootstrap_phase=admission
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
      and (.fresh | type) == "boolean" and .executable == ($target + "/" + $name))]
    | if length == 1 then .[0] else error("missing or ambiguous bootstrap artifact") end;
  [.[] | select(.reason == "compiler-artifact" and (.manifest_path | startswith($workspace)))] as $workspace_artifacts
  | if ($workspace_artifacts | length) == 0
    then error("bootstrap workspace artifact inventory is empty") else . end
  | [.[] | select(.reason == "build-finished")] as $finished
  | if ($finished | length) != 1 or $finished[0].success != true
    then error("bootstrap did not finish successfully") else . end
  | tool("lkjscript-dev"; "main.rs") as $checker
  | (if $controller then {controller:tool("lkjscript-release-controller"; "release/controller_main.rs")} else {} end) as $extra
  | {source:$source[0],checker:$checker,build_finished:$finished[0],workspace_artifacts:$workspace_artifacts} + $extra
  ' "$evidence/bootstrap-cargo.jsonl" > "$evidence/bootstrap-selection.json"
bootstrap_tools=(lkjscript-dev)
if "$with_controller"; then bootstrap_tools+=(lkjscript-release-controller); fi
bootstrap_binaries='[]'
for name in "${bootstrap_tools[@]}"; do
  if [[ "$name" == lkjscript-dev ]]; then key=checker; else key=controller; fi
  selected=$(jq -er --arg key "$key" '.[$key].executable' "$evidence/bootstrap-selection.json")
  test -f "$selected" && test ! -L "$selected"
  proof=$(bootstrap_file_proof "$selected" 536870912)
  jq -e '.kind == "file"' <<< "$proof" > /dev/null
  bootstrap_binaries=$(jq -c --arg name "$name" --argjson proof "$proof" \
    '.+=[{name:$name,output:$proof}]' <<< "$bootstrap_binaries")
done
context_digest=$(sha256sum "$evidence/bootstrap-context-after.json")
bootstrap_phase=receipt
jq -cn --slurpfile source "$evidence/bootstrap-source-after.json" \
  --slurpfile selection "$evidence/bootstrap-selection.json" \
  --arg target "$bootstrap_target" --arg context "${context_digest%% *}" \
  --argjson binaries "$bootstrap_binaries" --argjson archived "$retain_archive" \
  '{schema:"lkjscript.bootstrap.v1",status:"passed",source:$source[0],target:$target,
    context_sha256:$context,binaries:$binaries,originals_retained:$archived,
    cargo_reused:([$selection[0].workspace_artifacts[]|select(.fresh)]|length)}' \
  > "$bootstrap_root/receipt.pending"
test "$(wc -c < "$bootstrap_root/receipt.pending")" -lt 65536
mv -- "$bootstrap_root/receipt.pending" "$bootstrap_root/receipt.json"
if ! "$retain_archive"; then rm -r -- "$evidence"; fi
printf '%s\n' "$bootstrap_target/bootstrap"
