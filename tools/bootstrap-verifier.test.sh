#!/usr/bin/env bash
# Exercise the real bootstrap owner with an owned Git checkout and mock toolchain.
set -euo pipefail
bootstrap_script=$(readlink -e "${1-$(dirname "$0")/bootstrap-verifier.sh}")
fixture=$(mktemp -d)
trap 'status=$?; if (( status == 0 )); then rm -r -- "$fixture"; else printf "Retained failed bootstrap fixture: %s\n" "$fixture" >&2; fi' EXIT
cd "$fixture"
mkdir -p tools .artifacts/home .artifacts/cargo .artifacts/rustup .artifacts/toolchain/bin .artifacts/path
cp "$bootstrap_script" tools/bootstrap-verifier.sh
printf '/.artifacts/\n/.cargo/\n/target/\n' > .gitignore
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
test "$target" = "$PWD/target" && test -d "$target" && test ! -L "$target"
mkdir -p "$target/bootstrap"
env | LC_ALL=C sort > .artifacts/child-environment
printf 'original stderr\n' >&2
if [[ "$mode" == fail-cargo ]]; then printf '{"reason":"compiler-diagnostic"}\n'; exit 7; fi
if [[ "$mode" == source-change ]]; then printf 'changed\n' >> source.txt; fi
if [[ "$mode" == context-change ]]; then printf '# changed\n' >> .artifacts/toolchain/bin/rustc; fi
if [[ "$mode" == mold-config-change ]]; then printf '# changed\n' >> .cargo/config.toml; fi
if [[ "$mode" == mold-bytes-change ]]; then printf '# changed\n' >> .artifacts/toolchain/bin/mold; fi
if [[ "$mode" == mold-resolution-change ]]; then
  ln -sf "$PWD/.artifacts/toolchain/bin/mold-second" .artifacts/path/mold
fi
if [[ "$mode" == ld-mold-resolution-change || "$mode" == mold-both-resolution-change ]]; then
  ln -sf "$PWD/.artifacts/toolchain/bin/mold-second" .artifacts/path/ld.mold
fi
if [[ "$mode" == mold-both-resolution-change ]]; then
  ln -sf "$PWD/.artifacts/toolchain/bin/mold-second" .artifacts/path/mold
fi
artifact() {
  local name=$1 entry=$2 executable="$target/bootstrap/$1" fresh=false level=2
  printf '#!/bin/sh\nexit 0\n' > "$executable"
  chmod +x "$executable"
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
printf '#!/bin/sh\nexit 0\n' > .artifacts/toolchain/bin/mold
chmod +x .artifacts/toolchain/bin/mold
ln -s "$fixture/.artifacts/toolchain/bin/mold" .artifacts/path/mold
ln -s "$fixture/.artifacts/toolchain/bin/mold" .artifacts/path/ld.mold
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
test "$(cat ".artifacts/$case_name.stdout")" = "$fixture/target/bootstrap"
test -x target/bootstrap/lkjscript-dev
test ! -e "$root/lkjscript-dev"
test ! -e "$root/target"
test ! -e "$root/evidence"
jq -e --arg target "$fixture/target" '.status == "passed" and .target == $target
  and .originals_retained == false and (.binaries | length) == 1
  and .binaries[0].output.path == ($target + "/bootstrap/lkjscript-dev")
  and (.binaries[0].output.sha256 | length) == 64' "$root/receipt.json" > /dev/null
test "$(wc -c < "$root/receipt.json")" -lt 65536
case_name=archive
run --retain-archive
root="$fixture/.artifacts/$case_name"
jq -e '.exit_code == 0' "$root/evidence/bootstrap-process.json" > /dev/null
jq -e '.environment.CARGO_BUILD_JOBS == "3" and .environment.CARGO_NET_OFFLINE == "true"
  and (.environment | has("CARGO_ABOUT_VERSION") | not)
  and .argv[-1] == "--message-format=json"' "$root/evidence/bootstrap-command.json" > /dev/null
jq -e '.parent_environment.CARGO_ABOUT_VERSION.state == "set"
  and .override_states.RUSTFLAGS.state == "unset" and all(.configs[]; .kind == "absent")' \
  "$root/evidence/bootstrap-context-before.json" > /dev/null
! rg -q 'FORBIDDEN_SECRET|CARGO_ABOUT|private' .artifacts/child-environment
jq -e '.originals_retained == true' "$root/receipt.json" > /dev/null
case_name=both
run --with-controller
test -f target/bootstrap/lkjscript-release-controller
jq -e '.binaries[1].name == "lkjscript-release-controller"' \
  .artifacts/both/receipt.json > /dev/null
for case_name in fresh warm-library; do
  printf '%s\n' "$case_name" > .artifacts/mode
  run
  jq -e '.status == "passed" and .cargo_reused > 0' ".artifacts/$case_name/receipt.json" > /dev/null
done
for case_name in duplicate wrong-entry wrong-profile wrong-executable symlink source-change context-change fail-cargo; do
  printf '%s\n' "$case_name" > .artifacts/mode
  refuse
  jq -e '.status == "failed" and .originals_retained == false' ".artifacts/$case_name/receipt.json" > /dev/null
  test ! -e ".artifacts/$case_name/evidence"
  # The shared target is allowed to contain failed outputs; no receipt admits them.
  if test -L target/bootstrap/lkjscript-dev; then rm target/bootstrap/lkjscript-dev; fi
  git restore source.txt
done
case_name=fail-cargo-archive
refuse --retain-archive
jq -e '.status == "failed" and .originals_retained == true' .artifacts/fail-cargo-archive/receipt.json > /dev/null
jq -e '.exit_code == 7' .artifacts/fail-cargo-archive/evidence/bootstrap-process.json > /dev/null
test "$(cat .artifacts/fail-cargo-archive/evidence/bootstrap-cargo.stderr)" = 'original stderr'
printf 'valid\n' > .artifacts/mode
env -i "${fixture_env[@]}" bash tools/bootstrap-verifier.sh > .artifacts/default.stdout 2> .artifacts/default.stderr
jq -e '.status == "passed"' target/bootstrap-receipt/receipt.json > /dev/null
printf 'fail-cargo\n' > .artifacts/mode
if env -i "${fixture_env[@]}" bash tools/bootstrap-verifier.sh > .artifacts/default.stdout 2> .artifacts/default.stderr; then
  printf 'Unexpected default retry admission\n' >&2; exit 1
fi
jq -e '.status == "failed" and .originals_retained == false' target/bootstrap-receipt/receipt.json > /dev/null
test ! -e target/bootstrap-receipt/evidence
printf 'valid\n' > .artifacts/mode
case_name=override
fixture_env+=('RUSTFLAGS=private-override')
refuse
test ! -e ".artifacts/$case_name/target"
unset 'fixture_env[-1]'
case_name=shared-target
fixture_env+=("CARGO_TARGET_DIR=$fixture/target")
run
unset 'fixture_env[-1]'
case_name=second-target
fixture_env+=("CARGO_TARGET_DIR=$fixture/another-target")
refuse
test ! -e another-target
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
mkdir .cargo
native_mold_config() {
  printf '# Native GNU only; other targets keep their linker.\n[target.x86_64-unknown-linux-gnu] # target\n  rustflags = [ "-C", "link-arg=-fuse-ld=mold" ] # flags\n' > .cargo/config.toml
}
native_mold_config
case_name=mold-config
run --retain-archive
root="$fixture/.artifacts/$case_name"
jq -e --arg config "$fixture/.cargo/config.toml" --arg selected "$fixture/.artifacts/path/mold" \
  --arg driver "$fixture/.artifacts/path/ld.mold" \
  --arg resolved "$fixture/.artifacts/toolchain/bin/mold" '
    .supported and ([.configs[] | select(.path == $config and .kind == "file" and (.sha256 | length) == 64)] | length) == 1
    and .executables.mold.selected_path == $selected
    and .executables.mold.resolved.path == $resolved
    and .executables.mold.resolved.kind == "file"
    and .executables.ld_mold.selected_path == $driver
    and .executables.ld_mold.resolved == .executables.mold.resolved
    and (.executables.mold.resolved.sha256 | length) == 64' \
  "$root/evidence/bootstrap-context-before.json" > /dev/null
cmp "$root/evidence/bootstrap-context-before.json" "$root/evidence/bootstrap-context-after.json"
case_name=mold-compact
run
jq -e '.status == "passed" and .originals_retained == false' \
  ".artifacts/$case_name/receipt.json" > /dev/null
test ! -e ".artifacts/$case_name/evidence"
for case_name in mold-config-change mold-bytes-change mold-resolution-change ld-mold-resolution-change mold-both-resolution-change; do
  native_mold_config
  ln -sf "$fixture/.artifacts/toolchain/bin/mold" .artifacts/path/mold
  ln -sf "$fixture/.artifacts/toolchain/bin/mold" .artifacts/path/ld.mold
  cp .artifacts/toolchain/bin/mold .artifacts/toolchain/bin/mold-second
  printf '%s\n' "$case_name" > .artifacts/mode
  refuse --retain-archive
  jq -e '.status == "failed" and .phase == "admission"' ".artifacts/$case_name/receipt.json" > /dev/null
done
printf 'valid\n' > .artifacts/mode
ln -sf "$fixture/.artifacts/toolchain/bin/mold" .artifacts/path/mold
case_name=mold-driver-mismatch
refuse
jq -e '.status == "failed" and .phase == "preparation"' ".artifacts/$case_name/receipt.json" > /dev/null
ln -sf "$fixture/.artifacts/toolchain/bin/mold" .artifacts/path/ld.mold
invalid_mold_configs=(
  '[target.x86_64-unknown-linux-gnu]\nrustflags=["-C","link-arg=-fuse-ld=mold"]\nlinker="arbitrary"'
  '[target.x86_64-unknown-linux-musl]\nrustflags=["-C","link-arg=-fuse-ld=mold"]'
  '[target.x86_64-unknown-linux-gnu]\nrustflags=["- C","link-arg=-fuse-ld=mold"]'
  '[target.x86_64-unknown-linux-gnu]\nrustflags=["link-arg=-fuse-ld=mold","-C"]'
  '[target.x86_64-unknown-linux-gnu]\nrustflags=["-C","link-arg=-fuse-ld=mold"]\nrustflags=[]'
  '[target.x86_64-unknown-linux-gnu]\nrustflags=["-C","link-arg=-fuse-ld=mold"]\n[build]\njobs=1'
  '[target.x86_64-unknown-linux-gnu]\nrustflags=["-C","link-arg=-fuse-ld=other"]'
  '[target.x86_64-unknown-linux-gnu]\nrustflags=["-C","link-arg=-fuse-ld=mold"]\n#\x00'
)
for index in "${!invalid_mold_configs[@]}"; do
  case_name="mold-malformed-$index"
  printf '%b\n' "${invalid_mold_configs[$index]}" > .cargo/config.toml
  rm -f .artifacts/child-environment
  refuse
  jq -e '.status == "failed" and .phase == "preparation"' ".artifacts/$case_name/receipt.json" > /dev/null
  test ! -e .artifacts/child-environment
done
# A closed PATH makes absent/non-executable mold observable without touching host tools.
mkdir .artifacts/no-mold-path
for command in bash cat chmod cmp date dirname env flock git head jq mkdir mktemp mv readlink rm sed sha256sum sort stat wc; do
  ln -s "$(command -v "$command")" ".artifacts/no-mold-path/$command"
done
ln -s "$fixture/.artifacts/path/rustup" .artifacts/no-mold-path/rustup
fixture_env+=("PATH=$fixture/.artifacts/no-mold-path")
native_mold_config
case_name=mold-missing
refuse
test ! -e .artifacts/child-environment
ln -s "$fixture/.artifacts/toolchain/bin/mold" .artifacts/no-mold-path/mold
case_name=ld-mold-missing
refuse
test ! -e .artifacts/child-environment
rm .artifacts/no-mold-path/mold
printf '#!/bin/sh\nexit 0\n' > .artifacts/no-mold-path/mold
chmod 0600 .artifacts/no-mold-path/mold
case_name=mold-not-executable
refuse
test ! -e .artifacts/child-environment
rm .artifacts/no-mold-path/mold
ln -s "$fixture/.artifacts/toolchain/bin/mold" .artifacts/no-mold-path/mold
printf '#!/bin/sh\nexit 0\n' > .artifacts/no-mold-path/ld.mold
chmod 0600 .artifacts/no-mold-path/ld.mold
case_name=ld-mold-not-executable
refuse
test ! -e .artifacts/child-environment
unset 'fixture_env[-1]'
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
printf 'Shared-target bootstrap admission, reuse and refusal fixtures passed.\n'
