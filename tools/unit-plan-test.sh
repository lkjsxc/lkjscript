#!/usr/bin/env bash
# Streaming manifest contracts and the actual unit-command integration.
set -euo pipefail
cd "$(dirname "$0")/.."
plan_compiler=$(realpath "${LKJSCRIPT_BINARY:-target/lkjscript}")
mkdir -p target
plan_fixture=$(mktemp -d target/unit-plan.XXXXXX)
plan_key_root=$(mktemp -d "$(pwd -P)/target/unit-plan-key.XXXXXX")
plan_cleanup() {
    local result=$?
    if (( result != 0 )); then
        for diagnostic in "$plan_fixture"/*.err; do
            [[ -f "$diagnostic" ]] || continue
            printf '%s\n' "$diagnostic" >&2
            cat "$diagnostic" >&2
        done
    fi
    rm -rf "$plan_fixture" "$plan_key_root"
}
trap plan_cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$plan_fixture/store"
plan_key=$(XDG_STATE_HOME="$plan_key_root" sh tools/store-key.sh)
plan_root=$(realpath "$plan_fixture")

"$plan_compiler" build src/main.lkj --library-dir library --entry units::plan_disk::contracts_main --no-store \
    -o "$plan_root/contracts" > "$plan_root/contracts-build.out" 2> "$plan_root/contracts-build.err"
"$plan_root/contracts" > "$plan_root/contracts-native.out" 2> "$plan_root/contracts-native.err"
grep -Fxq -- 'ok streaming manifest 98 controls' "$plan_root/contracts-native.out"
cat "$plan_root/contracts-native.out"
"$plan_compiler" run src/main.lkj --library-dir library --entry units::plan_disk::smoke_main \
    --engine interpreter --no-store \
    > "$plan_root/contracts-interpreter.out" 2> "$plan_root/contracts-interpreter.err"
grep -Fxq -- 'ok streaming manifest 2 controls' "$plan_root/contracts-interpreter.out"
cat "$plan_root/contracts-interpreter.out"

cat > "$plan_root/units.lkj" <<'SOURCE'
/* The dependency declaration is deliberately first. /* Nested comment. */ */
pub(crate) const r#DEPENDS: (&'static [(&'static r#str)]) = (&[("app=base"),]);
const r#ENTRY: &r#str = r#"app::main"#;
const r#UNITS: &[&r#str] = &[r##"app=app.lkj"##, "base=\u{57fa}.lkj"];
SOURCE
cat > "$plan_root/基.lkj" <<'SOURCE'
pub fn r#answer<T>() -> i64 { r#include_str!("answer.txt").len() as i64 + 40i64 }
SOURCE
printf ok > "$plan_root/answer.txt"
cat > "$plan_root/app.lkj" <<'SOURCE'
use crate::r#base::r#answer;
pub fn main(mut out: Stdout) {
    out.write(format!("{}\n", r#answer::<i64>()).as_bytes());
}
SOURCE
plan_options=(units.lkj --source-dir "$plan_root" --store "$plan_root/store" --store-key "$plan_key")
"$plan_compiler" check-units "${plan_options[@]}" > "$plan_root/check.out" 2> "$plan_root/check.err"
grep -Fq -- 'checked=2 reused=0 emitted=0 total=2' "$plan_root/check.out"
"$plan_compiler" build-units "${plan_options[@]}" --grant out -o "$plan_root/program" \
    > "$plan_root/build.out" 2> "$plan_root/build.err"
[[ $("$plan_root/program") == 42 ]]
"$plan_compiler" build-units "${plan_options[@]}" --grant out -o "$plan_root/warm" \
    > "$plan_root/warm.out" 2> "$plan_root/warm.err"
grep -Fq -- 'checked=0 reused=2 emitted=0 total=2' "$plan_root/warm.out"
cmp "$plan_root/program" "$plan_root/warm"
cp "$plan_root/units.lkj" "$plan_root/units.saved"

cat > "$plan_root/units.lkj" <<'SOURCE'
#[cfg(test)]
const UNITS: &[&str] = &["app=app.lkj"];
const DEPENDS: &[&str] = &[];
const ENTRY: &str = "app::main";
SOURCE
if "$plan_compiler" build-units "${plan_options[@]}" --grant out -o "$plan_root/program" \
    > "$plan_root/cfg.out" 2> "$plan_root/cfg.err"; then
    echo 'conditional manifest was silently accepted' >&2; exit 1
fi
grep -Fq -- 'unconditional literal constants' "$plan_root/cfg.err"
grep -Fq -- 'select a separate manifest' "$plan_root/cfg.err"
cmp "$plan_root/program" "$plan_root/warm"

# Complete topology must refuse before invalid ENTRY or absent source reads.
cat > "$plan_root/units.lkj" <<'SOURCE'
const ENTRY: &str = "invalid";
const DEPENDS: &[&str] = &["a=b", "b=a"];
const UNITS: &[&str] = &["a=absent-a.lkj", "b=absent-b.lkj"];
SOURCE
if "$plan_compiler" build-units "${plan_options[@]}" --grant out -o "$plan_root/program" \
    > "$plan_root/cycle.out" 2> "$plan_root/cycle.err"; then
    echo 'cyclic manifest was accepted' >&2; exit 1
fi
grep -Fq -- 'unit dependency cycle' "$plan_root/cycle.err"
cmp "$plan_root/program" "$plan_root/warm"
mv "$plan_root/units.saved" "$plan_root/units.lkj"

# The manifest read itself must protect the private key before lexing bytes.
ln "$plan_key" "$plan_root/private.lkj"
if "$plan_compiler" check-units private.lkj --source-dir "$plan_root" \
    --store "$plan_root/store" --store-key "$plan_key" \
    > "$plan_root/key.out" 2> "$plan_root/key.err"; then
    echo 'private key was accepted as a manifest' >&2; exit 1
fi
grep -Fq -- 'E_CHILD_IMAGE source or include attempted to read the private admission key' "$plan_root/key.err"
if compgen -G "$plan_root/store/.lkjscript-unit-*.scratch" > /dev/null; then
    echo 'manifest command left a named scratch file' >&2; exit 1
fi
if compgen -G "$plan_root/.lkjscript-unit-*.tmp" > /dev/null; then
    echo 'manifest refusal left an unpublished output' >&2; exit 1
fi
printf 'ok streaming manifest commands: literal grammar, warm reuse, early cycle refusal, key exclusion and atomic publication\n'
