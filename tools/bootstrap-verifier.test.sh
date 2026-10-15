#!/usr/bin/env bash
# Exercise the real bootstrap owner with an owned Git checkout and mock toolchain.
set -euo pipefail
bootstrap_script=$(readlink -e "${1-$(dirname "$0")/bootstrap-verifier.sh}")
fixture=$(mktemp -d)
trap 'status=$?; if (( status == 0 )); then rm -rf -- "$fixture"; else printf "Retained failed bootstrap fixture: %s\n" "$fixture" >&2; fi' EXIT
cd "$fixture"
mkdir -p tools .artifacts/home .artifacts/cargo .artifacts/rustup .artifacts/toolchain/bin .artifacts/path
cp "$bootstrap_script" tools/bootstrap-verifier.sh
printf '/.artifacts/\n/.cargo/\n' > .gitignore
printf '[toolchain]\nchannel = "1.98.0"\n' > rust-toolchain.toml
printf 'fixture source\n' > source.txt
git -c init.defaultBranch=fixture init --quiet --template=
git add .
git -c user.name=Fixture -c user.email=fixture@example.invalid -c commit.gpgsign=false \
  commit --quiet --no-verify -m 'Owned bootstrap fixture'
expected_source=$(git rev-parse HEAD)
cat > .artifacts/path/rustup <<'SH'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == which && "$2" == --toolchain && "$3" == 1.98.0 ]]
printf '%s/.artifacts/toolchain/bin/%s\n' "$PWD" "$4"
SH
cat > .artifacts/toolchain/bin/rustc <<'SH'
#!/usr/bin/env bash
if [[ "$1" == --print && "$2" == sysroot ]]; then
  printf '%s/.artifacts/toolchain\n' "$PWD"
else printf 'rustc fixture\n'; fi
SH
cat > .artifacts/toolchain/bin/cargo <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == --version ]]; then printf 'cargo fixture\n'; exit 0; fi
mode=$(cat .artifacts/mode)
target=
controller=false
while (( $# )); do
  case "$1" in
    --target-dir) target=$2; shift ;;
    --profile) [[ "$2" == bootstrap ]]; shift ;;
    --bins) controller=true ;;
  esac
  shift
done
test ! -e "$target" && test ! -L "$target"
mkdir -p "$target/bootstrap"
env | LC_ALL=C sort > .artifacts/child-environment
printf 'original stderr\n' >&2
if [[ "$mode" == fail-cargo ]]; then printf '{"reason":"compiler-diagnostic"}\n'; exit 7; fi
if [[ "$mode" == source-change ]]; then printf 'changed\n' >> source.txt; fi
if [[ "$mode" == context-change ]]; then printf '# changed\n' >> .artifacts/toolchain/bin/rustc; fi
artifact() {
  local name=$1 entry=$2 executable="$target/bootstrap/$1" fresh=false level=2
  printf '#!/bin/sh\nexit 0\n' > "$executable"
  case "$mode" in
    fresh) fresh=true ;;
    wrong-entry) entry=unrelated.rs ;;
    wrong-profile) level=3 ;;
    wrong-executable) executable="$target/bootstrap/unrelated" ;;
    symlink) rm "$executable"; ln -s /bin/true "$executable" ;;
  esac
  jq -cn --arg manifest "$PWD/tools/lkjscript-dev/Cargo.toml" --arg name "$name" \
    --arg entry "$PWD/tools/lkjscript-dev/src/$entry" --arg executable "$executable" \
    --argjson fresh "$fresh" --arg level "$level" \
    '{reason:"compiler-artifact",manifest_path:$manifest,target:{name:$name,kind:["bin"],src_path:$entry},
      profile:{test:false,opt_level:$level,debuginfo:0,debug_assertions:true,overflow_checks:true},fresh:$fresh,executable:$executable}'
}
artifact lkjscript-dev main.rs
if [[ "$mode" == duplicate ]]; then artifact lkjscript-dev main.rs; fi
if [[ "$mode" == warm-library ]]; then
  jq -cn --arg manifest "$PWD/Cargo.toml" '{reason:"compiler-artifact",manifest_path:$manifest,fresh:true}'
fi
if "$controller"; then artifact lkjscript-release-controller release/controller_main.rs; fi
printf '{"reason":"build-finished","success":true}\n'
SH
chmod +x .artifacts/path/rustup .artifacts/toolchain/bin/{cargo,rustc}
fixture_env=("HOME=$fixture/.artifacts/home" "PATH=$fixture/.artifacts/path:/usr/bin:/bin"
  "CARGO_HOME=$fixture/.artifacts/cargo" "RUSTUP_HOME=$fixture/.artifacts/rustup"
  'CARGO_BUILD_JOBS=3' 'CARGO_ABOUT_VERSION=private-release-version' 'FORBIDDEN_SECRET=private')
run() {
  env -i "${fixture_env[@]}" bash tools/bootstrap-verifier.sh \
    --output "$fixture/.artifacts/$case_name" --expected-source "$expected_source" "$@" \
    > ".artifacts/$case_name.stdout" 2> ".artifacts/$case_name.stderr"
}
refuse() {
  if run "$@"; then printf 'Unexpected bootstrap admission: %s\n' "$case_name" >&2; exit 1; fi
}
printf 'valid\n' > .artifacts/mode
case_name=valid
run
root="$fixture/.artifacts/$case_name"
test "$(cat ".artifacts/$case_name.stdout")" = "$root"
test "$(stat -c %a "$root/lkjscript-dev")" = 555
cmp "$root/target/bootstrap/lkjscript-dev" "$root/lkjscript-dev"
jq -e '.exit_code == 0' "$root/evidence/bootstrap-process.json" > /dev/null
jq -e '.environment.CARGO_BUILD_JOBS == "3" and .environment.CARGO_NET_OFFLINE == "true"
  and (.environment | has("CARGO_ABOUT_VERSION") | not)
  and .argv[-1] == "--message-format=json"' "$root/evidence/bootstrap-command.json" > /dev/null
jq -e '.parent_environment.CARGO_ABOUT_VERSION.state == "set"
  and .override_states.RUSTFLAGS.state == "unset" and all(.configs[]; .kind == "absent")' \
  "$root/evidence/bootstrap-context-before.json" > /dev/null
! rg -q 'FORBIDDEN_SECRET|CARGO_ABOUT|private' .artifacts/child-environment
(cd "$root/evidence" && sha256sum --check --strict bootstrap-evidence.sha256 > /dev/null)
case_name=both
run --with-controller
test -f ".artifacts/both/lkjscript-release-controller"
jq -e '.controller.target.name == "lkjscript-release-controller"' \
  .artifacts/both/evidence/bootstrap-selection.json > /dev/null
for case_name in fresh duplicate warm-library wrong-entry wrong-profile wrong-executable symlink source-change context-change fail-cargo; do
  printf '%s\n' "$case_name" > .artifacts/mode
  refuse
  test ! -e ".artifacts/$case_name/lkjscript-dev"
  git restore source.txt
done
jq -e '.exit_code == 7' .artifacts/fail-cargo/evidence/bootstrap-process.json > /dev/null
test "$(cat .artifacts/fail-cargo/evidence/bootstrap-cargo.stderr)" = 'original stderr'
printf 'valid\n' > .artifacts/mode
case_name=override
fixture_env+=('RUSTFLAGS=private-override')
refuse
test ! -e ".artifacts/$case_name/target"
unset 'fixture_env[-1]'
case_name=invalid-jobs
fixture_env+=('CARGO_BUILD_JOBS=0')
refuse
unset 'fixture_env[-1]'
case_name=config
mkdir .cargo
printf '[build]\njobs=1\n' > .cargo/config.toml
refuse
test ! -e ".artifacts/$case_name/target"
rm -r .cargo
case_name=dirty
printf 'dirty\n' >> source.txt
refuse
git restore source.txt
case_name=mismatched-source
refuse --expected-source 0000000000000000000000000000000000000000
case_name=existing
mkdir ".artifacts/$case_name"
printf 'retained\n' > ".artifacts/$case_name/marker"
refuse
test "$(cat ".artifacts/$case_name/marker")" = retained
case_name=dangling-output
ln -s absent ".artifacts/$case_name"
refuse
test -L ".artifacts/$case_name"
printf 'Bootstrap admission, refusal and original-evidence fixtures passed.\n'
