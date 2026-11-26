#!/usr/bin/env bash
# Unit source presentation is selected before ordinary and imported generic lowering.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
work=$(mktemp -d "$PWD/target/unit-file-path.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'unit-file-path: %s\n' "$*" >&2; exit 1; }
mkdir -p "$work/source/definitions" "$work/store"
head -c 32 /dev/urandom > "$work/key"
cat > "$work/source/units.lkj" <<'LKJ'
const UNITS: &[&str] = &["base=definitions/base.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["app=base"];
const ENTRY: &str = "app::main";
LKJ
cat > "$work/source/definitions/base.lkj" <<'LKJ'
pub mod nested;
const LOCATION: &str = file!();
pub fn own() -> &'static str { LOCATION }
pub fn generic<T: Copy>(value: T) -> &'static str { let _copy = value; file!() }
LKJ
cat > "$work/source/definitions/nested.lkj" <<'LKJ'
pub fn generic<T: Copy>(value: T) -> &'static str { let _copy = value; file!() }
LKJ
cat > "$work/source/app.lkj" <<'LKJ'
pub fn main(mut out: Stdout) {
    out.write(file!().as_bytes()); out.write(b"\n");
    out.write(crate::base::own().as_bytes()); out.write(b"\n");
    out.write(crate::base::generic(3i64).as_bytes()); out.write(b"\n");
    out.write(crate::base::nested::generic(5i64).as_bytes()); out.write(b"\n");
}
LKJ
printf '%s\n' "$work/source/app.lkj" "$work/source/definitions/base.lkj" \
    "$work/source/definitions/base.lkj" "$work/source/definitions/nested.lkj" > "$work/expected"
for pass in cold warm; do
    "$compiler" build-units units.lkj --source-dir "$work/source" --library-dir "$PWD/library" \
        --store "$work/store" --store-key "$work/key" --grant out -o "$work/image" \
        > "$work/build-$pass.out" 2> "$work/build-$pass.err" || { cat "$work/build-$pass.err" >&2; fail "unit build $pass"; }
    "$work/image" > "$work/actual"
    if ! cmp -s "$work/expected" "$work/actual"; then
        printf 'unit-file-path %s expected:\n' "$pass" >&2
        cat "$work/expected" >&2
        printf 'unit-file-path %s actual:\n' "$pass" >&2
        cat "$work/actual" >&2
        fail "file! escaped its selected physical source mapping"
    fi
done
# Unit manifests deliberately remain unconditional literal data in both settings.
{ printf '#[cfg(debug_assertions)]\n'; cat "$work/source/units.lkj"; } > "$work/source/conditional.lkj"
for setting in on off; do
    if "$compiler" check-units conditional.lkj --source-dir "$work/source" --debug-assertions "$setting" \
        --store "$work/store" --store-key "$work/key" > "$work/manifest.out" 2> "$work/manifest.err"; then
        fail "conditional unit manifest accepted with $setting"
    fi
    grep -q "E_UNIT_SOURCE.*unconditional literal" "$work/manifest.err" || { cat "$work/manifest.err" >&2; fail "manifest schema refusal"; }
done
printf 'ok unit file!: caller, constant/nongeneric definition, imported generic root and external-module paths; cold/warm builds; unconditional manifest refusal\n'
