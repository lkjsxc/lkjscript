#!/bin/bash
# Exercise an installed native compiler as processes, without a bootstrap.
set -Eeuo pipefail
export LC_ALL=C
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
toolchain=$(realpath -e -- "${1:?usage: native_workflow.sh TOOLCHAIN [OUTPUT_DIR]}")
work=${2:-$repo/target/native-workflow}
[[ ! -e $work ]] || { printf 'Output directory must be new: %s\n' "$work" >&2; exit 1; }
mkdir -p -- "$work"
work=$(cd -- "$work" && pwd -P)
lkjscript=$toolchain/bin/lkjscript
count=0
report_failure() {
    local status=$? file
    printf 'Native workflow failed near line %s (status %s)\n' "$1" "$status" >&2
    for file in "$work"/*.stderr; do
        if [[ -s $file ]]; then printf '%s:\n' "$file" >&2; tail -c 2048 -- "$file" >&2; fi
    done
    exit "$status"
}
trap 'report_failure "$LINENO"' ERR
capture() {
    local expected=$1 name=$2 status=0
    shift 2
    "$@" >"$work/$name.stdout" 2>"$work/$name.stderr" || status=$?
    printf '%s\n' "$status" >"$work/$name.status"
    [[ $status == "$expected" ]]
    count=$((count+1))
}
text_is() { printf '%s' "$2" | cmp -s - "$work/$1.stdout"; }
silent_errors() { [[ ! -s $work/$1.stderr ]]; }
mkdir -p "$work/app" "$work/caller" "$work/without-bootstrap"
# Keep normal system commands available; fail if a native operation calls one
# of the bootstrap/development tools, and retain the attempted name.
for name in cargo rustc node; do
    cat >"$work/without-bootstrap/$name" <<'STUB'
#!/bin/bash
printf '%s\n' "$0" >>"$LKJSCRIPT_FORBIDDEN_LOG"
exit 97
STUB
    chmod 755 "$work/without-bootstrap/$name"
done
export LKJSCRIPT_FORBIDDEN_LOG=$work/forbidden.log
rm -f -- "$LKJSCRIPT_FORBIDDEN_LOG"
export PATH="$work/without-bootstrap:$PATH"
cat >"$work/app/main.rs" <<'SOURCE'
pub mod calc;
pub fn probe() -> i64 { calc::answer() }
SOURCE
printf 'pub fn answer() -> i64 { 42 }\n' >"$work/app/calc.rs"
cd -- "$work"
capture 0 source-check "$lkjscript" check app/main.rs --no-library
text_is source-check ''
silent_errors source-check
capture 0 source-ir "$lkjscript" ir app/main.rs --no-library -o app.lkjir
capture 0 pack "$lkjscript" pack app/main.rs --no-library -o app.lkjsb
capture 0 bundle-check "$lkjscript" check app.lkjsb
capture 0 bundle-ir "$lkjscript" ir app.lkjsb -o bundle.lkjir
cmp app.lkjir bundle.lkjir
capture 0 direct-build "$lkjscript" build app/main.rs --no-library -o direct.elf
capture 0 ir-build "$lkjscript" build app.lkjir -o from-ir.elf
cmp direct.elf from-ir.elf
capture 0 standalone "$lkjscript" run app.lkjir
text_is standalone ''
capture 0 scalar "$lkjscript" eval app.lkjir
text_is scalar $'42\n'
silent_errors scalar
capture 0 runtime "$lkjscript" runtime-ir -o runtime.lkjir
cmp runtime.lkjir "$toolchain/runtime.lkjir"

cat >values.rs <<'SOURCE'
pub fn unit() {}
pub fn boolean() -> bool { true }
pub fn character() -> char { '🦀' }
pub fn i8_min() -> i8 { -128i8 }
pub fn i64_min() -> i64 { -9223372036854775808i64 }
pub fn u8_max() -> u8 { 255u8 }
pub fn u64_max() -> u64 { 18446744073709551615u64 }
pub fn float() -> f64 { -0.0 }
pub fn trap() -> i64 { panic!("native custom trap") }
SOURCE
capture 0 values "$lkjscript" ir values.rs --no-library -o values.lkjir
for pair in unit: boolean:true character:🦀 i8_min:-128 i64_min:-9223372036854775808 u8_max:255 u64_max:18446744073709551615 float:f64:0x8000000000000000; do
    name=${pair%%:*}
    expected=${pair#*:}
    capture 0 "value-$name" "$lkjscript" eval values.lkjir --entry "$name"
    if [[ $name == unit ]]; then text_is "value-$name" ''
    else text_is "value-$name" "$expected"$'\n'; fi
    silent_errors "value-$name"
done
capture 1 source-trap "$lkjscript" eval values.lkjir --entry trap
grep -Eq '^values.lkjir:[0-9]+: R[0-9]+ explicit panic$' source-trap.stderr
# The current source panic lowers to a generic message. Direct IR owns its text.
sed 's/trap "explicit panic"/trap "native custom trap"/' values.lkjir >traps.lkjir
capture 1 custom-trap "$lkjscript" eval traps.lkjir --entry trap
text_is custom-trap ''
grep -Eq '^traps.lkjir:[0-9]+: R[0-9]+ native custom trap$' custom-trap.stderr
# A failed descriptor write must leave stdout without a partial ELF.
capture 1 descriptor-failure "$toolchain/libexec/backend" values.lkjir --runtime runtime.lkjir --entry i8_min --result-descriptor absent/result.desc
text_is descriptor-failure ''
grep -Fq 'create failed' descriptor-failure.stderr
capture 0 packet-build "$toolchain/libexec/backend" values.lkjir --runtime runtime.lkjir --entry i8_min --result-descriptor result.desc
cp packet-build.stdout packet.elf
chmod 755 packet.elf
./packet.elf 3>packet.raw
{ cat packet.raw; printf '\n7\n'; } >wrong-status
capture 1 result-status "$toolchain/libexec/result" result.desc values.lkjir <wrong-status
grep -Fq 'E_HOST native exit status 7 disagrees with packet status 0' result-status.stderr
capture 1 result-empty "$toolchain/libexec/result" result.desc values.lkjir </dev/null
grep -Fq 'E_HOST expected native packet' result-empty.stderr

# Preserve the previous output when source/module checks fail.
mkdir -p app/calc
printf 'pub fn answer() -> i64 { 99 }\n' >app/calc/mod.rs
printf 'previous artifact\n' >preserved
capture 1 ambiguous "$lkjscript" build app/main.rs --no-library -o preserved
grep -Fq 'E_MODULE_AMBIGUOUS' ambiguous.stderr
printf 'previous artifact\n' | cmp -s - preserved
rm app/calc/mod.rs
printf 'pub fn answer() -> i64 { false }\n' >app/calc.rs
capture 1 child-error "$lkjscript" check app/main.rs --no-library
grep -Fq 'app/calc.rs:' child-error.stderr
printf 'pub fn answer() -> i64 { 42 }\n' >app/calc.rs

# Compile existing World boundary data once; then exercise the native wrapper.
for fixture in main closed_stdin output_failure; do
    capture 0 "compile-$fixture" "$lkjscript" ir "tests/fixtures/world/$fixture.rs" --source-dir "$repo" -o "$work/$fixture.lkjir"
done
printf '\0\377A\n' >world.stdin
payload=$'argument with spaces \377'
{ printf '%s' "$payload"; cat world.stdin; } >world.expected
cd caller
for mode in run eval; do
    capture 7 "world-$mode" "$lkjscript" "$mode" "$work/main.lkjir" -- "$payload" . "written-$mode" <"$work/world.stdin"
    cmp "$work/world.expected" "$work/world-$mode.stdout"
    cmp "$work/world.stdin" "$work/world-$mode.stderr"
    cmp "$work/world.expected" "written-$mode"
    capture 0 "closed-input-$mode" "$lkjscript" "$mode" "$work/closed_stdin.lkjir" -- . 0<&-
    text_is "closed-input-$mode" $'closed ok\n'
    capture 1 "closed-output-$mode" bash -c '"$@" 1>&-' bash "$lkjscript" "$mode" "$work/output_failure.lkjir" -- . 1
    grep -Fq 'R0007' "$work/closed-output-$mode.stderr"
    printf 'owned\n' | cmp -s - owned-file
    capture 1 "closed-error-$mode" bash -c '"$@" 2>&-' bash "$lkjscript" "$mode" "$work/output_failure.lkjir" -- . 2
    text_is "closed-error-$mode" ''
    silent_errors "closed-error-$mode"
done
capture 0 verify "$lkjscript" verify
[[ ! -e $LKJSCRIPT_FORBIDDEN_LOG ]]
printf 'Native workflow: %s process outcomes; source/bundle/IR identity, runtime, scalars, traps, streams, paths, controls, hashes; no Rust or Node\n' "$count"
