#!/usr/bin/env bash
# Real source macro scope, exact nom producers, and cold/warm source-unit admission.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
shift $(( $# > 0 ? 1 : 0 ))
witness=
if (($#)); then
    [[ $1 == --witness && $# == 2 ]] || { printf 'usage: %s [COMPILER] [--witness RUSTC]\n' "$0" >&2; exit 2; }
    witness=$2
fi
root=$PWD
mkdir -p target
work=$(mktemp -d "$root/target/declarative-macros.XXXXXX")
cleanup() {
    code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then
        for log in "$work/error" "$work/rust-error"; do [[ ! -f $log ]] || cat "$log" >&2; done
    fi
    if [[ ${LKJSCRIPT_MACRO_KEEP:-0} == 1 ]]; then printf 'retained_fixture=%s\n' "$work" >&2
    else rm -rf -- "$work"; fi
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
while read -r expected name source; do
    awk -v name="$name" '
        $0 == "// nom-source: " name " begin" { copying=1; next }
        $0 == "// nom-source: " name " end" { copying=0; next }
        copying { print }
    ' "$source" > "$work/source-$name"
    [[ $(sha256sum "$work/source-$name" | cut -d' ' -f1) == "$expected" ]]
done < tests/fixtures/nom_macro_sources.sha256
printf 'ok declarative macros: unchanged nom 8.0.0 succ, tuple chain and error producers\n'
cp -R tests/fixtures/macros "$work/project"
for engine in native interpreter; do
    "$compiler" run main.lkj --source-dir "$work/project" --library-dir "$root/library" --no-store --engine "$engine" > "$work/output" 2> "$work/error"
    [[ $(cat "$work/output") == 'macro scopes ok' && ! -s $work/error ]]
    printf 'ok declarative macros %s: lexical scope, shadowing, exports, defining crate and include origin\n' "$engine"
done
mkdir "$work/units" "$work/store"
head -c 32 /dev/urandom > "$work/key"
chmod 600 "$work/key"
cat > "$work/units/units.lkj" <<'LKJ'
const UNITS: &[&str] = &["math=math.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["app=math"];
const ENTRY: &str = "app::main";
LKJ
cat > "$work/units/math.lkj" <<'LKJ'
macro_rules! next { ($name:ident, $t:ty, $value:expr) => { let $name: $t = $value + 1usize; }; }
mod selected { pub type Number = usize; }
pub fn increment(value: usize) -> usize {
    use self::selected::Number;
    next!(result, Number, value);
    result
}
LKJ
cat > "$work/units/app.lkj" <<'LKJ'
pub fn main(mut world: World) {
    let text = format!("{}\n", crate::math::increment(2usize));
    world.stdout().write(text.as_bytes());
}
LKJ
units=(units.lkj --source-dir "$work/units" --library-dir "$root/library" --store "$work/store" --store-key "$work/key")
"$compiler" build-units "${units[@]}" -o "$work/cold" > "$work/output" 2> "$work/error"
[[ $("$work/cold") == 3 ]]
"$compiler" build-units "${units[@]}" -o "$work/warm" > "$work/output" 2> "$work/error"
cmp "$work/cold" "$work/warm"
[[ $("$work/warm") == 3 ]]
sed -i 's/+ 1usize/+ 2usize/' "$work/units/math.lkj"
"$compiler" build-units "${units[@]}" -o "$work/changed" > "$work/output" 2> "$work/error"
[[ $("$work/changed") == 4 ]]
cp "$work/units/math.lkj" "$work/valid-math"
sed -i 's/+ 2usize/+ true/' "$work/units/math.lkj"
cp "$work/units/math.lkj" "$work/invalid-math"
if "$compiler" build-units "${units[@]}" -o "$work/invalid" > "$work/output" 2> "$work/error"; then
    printf 'invalid statement template accepted\n' >&2; exit 1
fi
[[ ! -e $work/invalid ]]
cmp "$work/invalid-math" "$work/units/math.lkj"
cp "$work/valid-math" "$work/units/math.lkj"
"$compiler" build-units "${units[@]}" -o "$work/restored" > "$work/output" 2> "$work/error"
cmp "$work/changed" "$work/restored"
printf 'ok declarative macros: cold/warm statement captures and block imports match; template edit rechecks dependents and failed output remains unpublished\n'
if [[ -n $witness ]]; then
    cp "$root/witness/bridge/output.rs" "$work/host.rs"
    for case in declarative_macros nom_tuple_macros; do
        {
            printf '#![allow(dead_code,unused_imports,unused_variables,deprecated)]\n#[path="%s/host.rs"] mod host;\nuse host::World;\n' "$work"
            sed 's/^pub fn main(mut world: World) {/pub fn specimen(mut world: World) {/' "tests/conformance/output/$case.lkj"
            printf '\nfn main() { host::enter(specimen); }\n'
        } > "$work/$case.rs"
        "$witness" --edition=2024 --crate-name "$case" -A warnings "$work/$case.rs" -o "$work/$case" > "$work/output" 2> "$work/rust-error"
        "$work/$case" > "$work/output" 2> "$work/rust-error"
        expected='macros ok'
        [[ $case != nom_tuple_macros ]] || expected='nom macros ok'
        [[ $(cat "$work/output") == "$expected" && ! -s $work/rust-error ]]
        printf 'ok declarative macros Rust root-crate witness: %s\n' "$case"
    done
    for case in macro_stmt_use_after macro_stmt_use_before macro_stmt_use_stmt; do
        {
            cat "tests/conformance/pass/$case.lkj"
            printf '\nfn main() { assert_eq!(probe(), 42); }\n'
        } > "$work/$case.rs"
        for level in 0 3; do
            "$witness" --edition=2024 --crate-name "$case" -A warnings -C "opt-level=$level" "$work/$case.rs" -o "$work/$case" > "$work/output" 2> "$work/rust-error"
            "$work/$case" > "$work/output" 2> "$work/rust-error"
            [[ ! -s $work/output && ! -s $work/rust-error ]]
        done
        printf 'ok statement macro Rust root-crate witness: %s\n' "$case"
    done
    mkdir "$work/rust-scopes"
    cp -R "$work/project/definitions" "$work/rust-scopes/definitions"
    mv "$work/rust-scopes/definitions/mod.lkj" "$work/rust-scopes/definitions/mod.rs"
    cp "$work/project/caller.lkj" "$work/rust-scopes/caller.rs"
    cp "$work/project/payload.txt" "$work/rust-scopes/payload.txt"
    {
        printf '#![allow(dead_code,unused_imports,unused_variables)]\n#[path="%s/host.rs"] mod host;\nuse host::World;\n' "$work"
        sed 's/^pub fn main(mut world: World) {/pub fn specimen(mut world: World) {/' "$work/project/main.lkj"
        printf '\nfn main() { host::enter(specimen); }\n'
    } > "$work/rust-scopes/main.rs"
    "$witness" --edition=2024 --crate-name macro_scopes -A warnings "$work/rust-scopes/main.rs" -o "$work/rust-scopes/image" > "$work/output" 2> "$work/rust-error"
    "$work/rust-scopes/image" > "$work/output" 2> "$work/rust-error"
    [[ $(cat "$work/output") == 'macro scopes ok' && ! -s $work/rust-error ]]
    printf 'ok declarative macros Rust witness: source module scope and include origin\n'
fi
