#!/usr/bin/env bash
# Raw descriptors are checked, independently interpreted, and exercised by native ABI/linkage.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-${LKJSCRIPT_BINARY:-target/lkjscript}}")
library="$PWD/library"
mkdir -p target
work=$(mktemp -d "$PWD/target/borrowed-callable.XXXXXX")
cleanup() {
    local status=$?
    if ((status)); then
        for log in "$work"/*.err "$work"/*.out; do
            [[ -s $log ]] || continue
            printf 'borrowed-callable diagnostic=%s\n' "${log##*/}" >&2
            head -n 60 "$log" >&2
        done
    fi
    if [[ ${LKJSCRIPT_CALLABLE_KEEP:-0} == 1 ]]; then
        rm -rf -- "$work/key-state"
        printf 'retained_fixture=%s\n' "$work" >&2
    else
        rm -rf -- "$work"
    fi
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$work/contracts"
for module in diagnostic facts ir ir_text ir_binary reuse check interpreter runtime_memory; do
    cp "src/$module.lkj" "$work/contracts/$module.lkj"
done
cp -R src/check src/interpreter "$work/contracts/"
cp tests/fixtures/borrowed_callable.lkj "$work/contracts/main.lkj"
cp tests/fixtures/borrowed_callable.lkjir "$work/contracts/borrowed_callable.lkjir"
"$compiler" build main.lkj --source-dir "$work/contracts" --library-dir "$library" \
    --grant out --no-store -o "$work/contracts-program" > "$work/contracts-build.out" 2> "$work/contracts-build.err"
"$work/contracts-program" > "$work/contracts.out" 2> "$work/contracts.err"
grep -Fxq 'ok borrowed callable 23 controls' "$work/contracts.out"
cat "$work/contracts.out"
# The CLI routes exercise emitted descriptor words, stack arguments, aggregate
# returns, enum storage, reborrowing and adjacent sentinels in both optimization modes.
for inline in 0 64; do
    "$compiler" test tests/fixtures/borrowed_callable.lkjir --jobs 1 \
        --inline-limit "$inline" --no-store --stats \
        > "$work/abi-$inline.out" 2> "$work/abi-$inline.err"
    cat "$work/abi-$inline.out"
done

# A descriptor constructed by a private producer helper crosses an authenticated
# source-unit boundary. Two environment shapes share the public Arguments type.
mkdir "$work/units" "$work/store" "$work/key-state"
key=$(XDG_STATE_HOME="$work/key-state" sh tools/store-key.sh)
cat > "$work/units/units.lkj" <<'LKJ'
const UNITS: &[&str] = &["render=render.lkj", "producer=producer.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["producer=render", "app=producer"];
const ENTRY: &str = "app::main";
LKJ
cat > "$work/units/render.lkj" <<'LKJ'
pub fn consume(arguments: std::fmt::Arguments<'_>) -> String { format!("{arguments}") }
LKJ
cat > "$work/units/producer.lkj" <<'LKJ'
pub fn numbers() -> String {
    let left = 20i64;
    let right = 22i64;
    let arguments = format_args!("{left} + {right}");
    crate::render::consume(arguments)
}
pub fn label() -> String {
    let value = String::from("value");
    let arguments = format_args!("label={value}");
    crate::render::consume(arguments)
}
LKJ
cat > "$work/units/app.lkj" <<'LKJ'
pub fn main(mut out: Stdout) {
    let numbers = crate::producer::numbers();
    let label = crate::producer::label();
    out.write(format!("{numbers}|{label}").as_bytes());
}
LKJ
options=(units.lkj --source-dir "$work/units" --library-dir "$library" \
    --store "$work/store" --store-key "$key" --grant out --stats)
"$compiler" build-units "${options[@]}" -o "$work/unit-cold" > "$work/unit-cold.out" 2> "$work/unit-cold.err"
[[ $("$work/unit-cold") == '20 + 22|label=value' ]]
"$compiler" build-units "${options[@]}" -o "$work/unit-warm" > "$work/unit-warm.out" 2> "$work/unit-warm.err"
grep -Fq 'checked=0 reused=3 emitted=0' "$work/unit-warm.out"
cmp "$work/unit-cold" "$work/unit-warm"
sed -i 's/22i64/23i64/' "$work/units/producer.lkj"
"$compiler" build-units "${options[@]}" -o "$work/unit-edited" > "$work/unit-edited.out" 2> "$work/unit-edited.err"
[[ $("$work/unit-edited") == '20 + 23|label=value' ]]
printf 'ok borrowed callable: four ABI routes and three unit-linkage builds\n'
