#!/usr/bin/env bash
# Explicit provider selection supplies qualified macros without implicit macro_use.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
mkdir -p target
work=$(mktemp -d "$PWD/target/macro-provider-prelude.XXXXXX")
cleanup() {
    local status=$?
    if ((status)); then
        for log in error output origins-error origins-output; do
            if [[ -s $work/$log ]]; then cat "$work/$log" >&2; fi
        done
    fi
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'macro-provider-prelude: %s\n' "$*" >&2; exit 1; }

cat > "$work/provider.lkj" <<'LKJ'
pub fn add(value: i32) -> i32 { value + 2 }
fn secret() -> i32 { 42 }
macro_rules! hidden { () => { 42 }; }
#[macro_export]
macro_rules! count { ($value:expr) => { $crate::add($value) }; }
#[macro_export]
macro_rules! across { ($value:expr) => { ::other::count!($value) }; }
#[macro_export]
macro_rules! nested { ($value:expr) => { $crate::count!($value) }; }
#[macro_export]
macro_rules! emit { () => { fn answer() -> i32 { 42 } }; }
#[macro_export]
macro_rules! bind { ($name:ident) => { let $name = 42i32; }; }
#[macro_export]
macro_rules! pat { () => { 42 }; }
#[macro_export]
macro_rules! private { () => { $crate::secret() }; }
#[macro_export]
macro_rules! view { ($value:expr) => { &$value }; }
#[macro_export]
macro_rules! ambient { () => {{ println!("ambient output"); 42 }}; }
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! configured { () => { 42 }; }
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! configured { () => { 40 }; }
LKJ
cat > "$work/other.lkj" <<'LKJ'
#[macro_export]
macro_rules! count { ($value:expr) => { $value + 40 }; }
#[macro_export]
macro_rules! other_only { () => { 42 }; }
LKJ
cat > "$work/empty_provider.lkj" <<'LKJ'
pub fn unused() -> i32 { 7 }
LKJ

cases=()
specimen() {
    local name=$1 expected=$2 rust=$3
    cases+=("$name:$expected:$rust")
    cat > "$work/$name.lkj"
}
specimen relative 42 value <<'LKJ'
fn probe() -> i32 { chosen::count!(40) }
LKJ
specimen absolute 42 value <<'LKJ'
fn probe() -> i32 { ::chosen::count!(40) }
LKJ
specimen nested 42 value <<'LKJ'
fn probe() -> i32 { chosen::nested!(40) }
LKJ
specimen nested_scope 42 value <<'LKJ'
mod inner { pub fn value() -> i32 { chosen::count!(40) } }
fn probe() -> i32 { inner::value() }
LKJ
specimen weak_keyword 42 value <<'LKJ'
fn probe() -> i32 { macro_rules::count!(40) }
LKJ
specimen mixed 42 value <<'LKJ'
#[macro_use] extern crate counters as chosen;
fn probe() -> i32 { count!(0) + other::count!(0) }
LKJ
specimen mixed_unqualified E_UNSUPPORTED_MACRO refuse <<'LKJ'
#[macro_use] extern crate counters as chosen;
fn probe() -> i32 { other_only!() }
LKJ
specimen cross_provider 42 value <<'LKJ'
fn probe() -> i32 { chosen::across!(2) }
LKJ
specimen statement 42 value <<'LKJ'
fn probe() -> i32 { chosen::bind!(value); value }
LKJ
specimen item 42 value <<'LKJ'
chosen::emit!();
fn probe() -> i32 { answer() }
LKJ
specimen pattern 42 value <<'LKJ'
fn probe() -> i32 { match 42 { chosen::pat!() => 42, _ => 0 } }
LKJ
specimen absolute_shadow 42 value <<'LKJ'
mod chosen {}
mod inner {
    type chosen = i32;
    pub fn value() -> i32 { ::chosen::count!(40) }
}
fn probe() -> i32 { inner::value() }
LKJ
specimen value_namespace 42 value <<'LKJ'
fn chosen() -> i32 { 40 }
fn probe() -> i32 { chosen::count!(chosen()) }
LKJ
specimen two_providers 42 value <<'LKJ'
fn probe() -> i32 { chosen::count!(0) + other::count!(0) }
LKJ
specimen local_macro 42 value <<'LKJ'
macro_rules! count { () => { 40 }; }
fn probe() -> i32 { count!() + chosen::count!(0) }
LKJ
specimen empty 42 value <<'LKJ'
fn probe() -> i32 { 42 }
LKJ
specimen configured configured value <<'LKJ'
fn probe() -> i32 { chosen::configured!() }
LKJ
specimen unqualified E_UNSUPPORTED_MACRO refuse <<'LKJ'
fn probe() -> i32 { count!(40) }
LKJ
specimen hidden E_UNSUPPORTED_MACRO refuse <<'LKJ'
fn probe() -> i32 { chosen::hidden!() }
LKJ
specimen malformed E_UNSUPPORTED_MACRO refuse <<'LKJ'
fn probe() -> i32 { chosen::wrong::count!(40) }
LKJ
specimen shadow_module E_MACRO_PROVIDER refuse <<'LKJ'
mod chosen {}
fn probe() -> i32 { chosen::count!(40) }
LKJ
specimen shadow_type E_MACRO_PROVIDER refuse <<'LKJ'
type chosen = i32;
fn probe() -> i32 { chosen::count!(40) }
LKJ
specimen privacy E_PRIVACY refuse <<'LKJ'
fn probe() -> i32 { chosen::private!() }
LKJ
specimen lifetime E_BORROW refuse <<'LKJ'
fn probe() -> i32 {
    let outer: &i32;
    { let inner = 42; outer = chosen::view!(inner); }
    *outer
}
LKJ
specimen imported E_UNSUPPORTED_MACRO boundary <<'LKJ'
use chosen::count;
fn probe() -> i32 { count!(40) }
LKJ
specimen bare_extern E_UNSUPPORTED boundary <<'LKJ'
extern crate counters as chosen;
fn probe() -> i32 { chosen::count!(40) }
LKJ
specimen ambient E_CAPABILITY ambient <<'LKJ'
fn probe() -> i32 { chosen::ambient!() }
LKJ

base=(--source-dir "$work" --library-dir "$PWD/library" --entry probe --no-store)
selection=(--macro-provider counters:chosen=provider.lkj --macro-provider second:other=other.lkj)
routes=0
for mode in off on; do
    for inline in 0 64; do
        for engine in native interpreter; do
            for spec in "${cases[@]}"; do
                IFS=: read -r name expected rust <<< "$spec"
                picked=("${selection[@]}")
                if [[ $name == empty ]]; then picked=(--macro-provider counters:chosen=empty_provider.lkj); fi
                if [[ $name == weak_keyword ]]; then picked=(--macro-provider counters:macro_rules=provider.lkj); fi
                if [[ $expected == configured ]]; then
                    expected=40
                    if [[ $mode == on ]]; then expected=42; fi
                fi
                if [[ $expected == E_* ]]; then
                    if "$compiler" run "$name.lkj" "${base[@]}" "${picked[@]}" --engine "$engine" \
                        --debug-assertions "$mode" --inline-limit "$inline" > "$work/output" 2> "$work/error"; then
                        fail "unexpected acceptance: $name/$mode/$inline/$engine"
                    fi
                    [[ ! -s $work/output ]] && grep -Fq ": $expected " "$work/error" \
                        || fail "wrong refusal: $name/$mode/$inline/$engine"
                else
                    "$compiler" run "$name.lkj" "${base[@]}" "${picked[@]}" --engine "$engine" \
                        --debug-assertions "$mode" --inline-limit "$inline" > "$work/output" 2> "$work/error" \
                        || fail "value refused: $name/$mode/$inline/$engine"
                    [[ $(<"$work/output") == "$expected" ]] || fail "wrong value: $name/$mode/$inline/$engine"
                fi
                routes=$((routes + 1))
            done
        done
    done
done
printf 'ok macro provider prelude: %s cases, %s native/interpreter routes, debug off/on and inline0/64\n' "${#cases[@]}" "$routes"

# A failed authored binding must suppress only that selected root. It stays failed
# even if a later declaration matches, and includes syntax/attribute failures.
for shape in mismatch_then_valid valid_then_mismatch duplicate malformed missing_semicolon public restricted; do
    case $shape in
        mismatch_then_valid) text=$'#[macro_use] extern crate counters as wrong;\n#[macro_use] extern crate counters as chosen;' ;;
        valid_then_mismatch) text=$'#[macro_use] extern crate counters as chosen;\n#[macro_use] extern crate counters as wrong;' ;;
        duplicate) text=$'#[macro_use] extern crate counters as chosen;\n#[macro_use] extern crate counters as chosen;' ;;
        malformed) text='#[macro_use] extern crate counters as ;' ;;
        missing_semicolon) text='#[macro_use] extern crate counters as chosen' ;;
        public) text='#[macro_use] pub extern crate counters as chosen;' ;;
        restricted) text='#[macro_use(count)] extern crate counters as chosen;' ;;
    esac
    printf '%s\nfn probe() -> i32 { 42 }\n' "$text" > "$work/failed.lkj"
    if "$compiler" check failed.lkj "${base[@]}" --macro-provider counters:chosen=missing.lkj \
        --macro-provider second:other=other.lkj > "$work/output" 2> "$work/error"; then
        fail "failed declaration accepted: $shape"
    fi
    if grep -Fq 'missing.lkj' "$work/error"; then fail "failed declaration read provider: $shape"; fi
    grep -Eq ': E_(MACRO_PROVIDER|PARSE) ' "$work/error" || fail "missing binding diagnostic: $shape"
done
if "$compiler" check empty.lkj "${base[@]}" --macro-provider counters:chosen=missing.lkj \
    > "$work/output" 2> "$work/error"; then fail 'missing selected root accepted'; fi
grep -Fq 'E_MODULE_MISSING' "$work/error" || fail 'missing selected root diagnostic'
printf 'ok macro provider prelude: seven failed-binding no-read controls and one missing-root refusal\n'
# Replacing std also replaces Rust's implicit prelude, not just explicit paths.
# It needs a standard-library interface rather than this macro-only selector.
for selector in counters:std=missing.lkj std:chosen=missing.lkj; do
    if "$compiler" check empty.lkj "${base[@]}" --macro-provider "$selector" \
        > "$work/output" 2> "$work/error"; then fail 'replacement std provider accepted'; fi
    grep -Fq 'E_USAGE macro providers cannot replace the implicit std crate or prelude' "$work/error" \
        || fail 'replacement std provider diagnostic'
    if grep -Fq 'E_MODULE_MISSING' "$work/error"; then fail 'replacement std provider was read'; fi
done
printf 'ok macro provider prelude: reserved std crate/alias selectors refuse before source reads\n'


# A template emitted into the consumer may use that consumer's other aliases.
# An authored provider body has its own crate scope and no implicit dependency map.
cat > "$work/dependency_provider.lkj" <<'LKJ'
pub fn external() -> i32 { ::other::count!(2) }
LKJ
if "$compiler" check empty.lkj "${base[@]}" \
    --macro-provider counters:chosen=dependency_provider.lkj \
    --macro-provider second:other=other.lkj > "$work/output" 2> "$work/error"; then
    fail 'consumer aliases leaked into an authored provider body'
fi
grep -Fq ': E_UNSUPPORTED_MACRO ' "$work/error" || fail 'provider dependency refusal'
if [[ -n $witness ]]; then
    if "$witness" --edition=2021 -Awarnings --crate-name counters --crate-type rlib \
        "$work/dependency_provider.lkj" -o "$work/libdependency.rlib" > "$work/output" 2> "$work/error"; then
        fail 'Rust provider acquired an undeclared external alias'
    fi
    grep -Eq '^error(\[|:)' "$work/error" || fail 'Rust provider dependency diagnostic'
fi
printf 'ok macro provider dependency scope: consumer alias is absent from authored provider bodies\n'

if [[ -n $witness ]]; then
    values=0; refusals=0; boundaries=0
    for mode in off on; do
        rust_debug=no
        if [[ $mode == on ]]; then rust_debug=yes; fi
        for optimization in 0 3; do
            for provider in provider other empty_provider; do
                crate=counters
                if [[ $provider == other ]]; then crate=second; fi
                "$witness" --edition=2021 -Awarnings -C "debug-assertions=$rust_debug" \
                    -C "opt-level=$optimization" --crate-name "$crate" --crate-type rlib \
                    "$work/$provider.lkj" -o "$work/lib$provider.rlib"
            done
            for spec in "${cases[@]}"; do
                IFS=: read -r name expected rust <<< "$spec"
                {
                    cat "$work/$name.lkj"
                    printf 'fn main() { println!("{}", probe()); }\n'
                } > "$work/main.rs"
                provider=provider; alias=chosen
                if [[ $name == empty ]]; then provider=empty_provider; fi
                if [[ $name == bare_extern || $name == mixed || $name == mixed_unqualified ]]; then alias=counters; fi
                if [[ $name == weak_keyword ]]; then alias=macro_rules; fi
                rust_args=(--edition=2021 -Awarnings -C "debug-assertions=$rust_debug"
                    -C "opt-level=$optimization" --extern "$alias=$work/lib$provider.rlib"
                    --extern "other=$work/libother.rlib" "$work/main.rs" -o "$work/rust")
                if [[ $rust == refuse ]]; then
                    if "$witness" "${rust_args[@]}" > "$work/output" 2> "$work/error"; then
                        fail "Rust accepted refusal: $name/$mode/$optimization"
                    fi
                    grep -Eq '^error(\[|:)' "$work/error" || fail "Rust refusal has no diagnostic: $name"
                    refusals=$((refusals + 1))
                else
                    "$witness" "${rust_args[@]}" > "$work/output" 2> "$work/error" \
                        || fail "Rust rejected value: $name/$mode/$optimization"
                    "$work/rust" > "$work/output" 2> "$work/error" || fail "Rust execution: $name"
                    if [[ $rust == ambient ]]; then expected=$'ambient output\n42'
                    elif [[ $rust == boundary ]]; then expected=42
                    elif [[ $expected == configured ]]; then
                        expected=40
                        if [[ $mode == on ]]; then expected=42; fi
                    fi
                    [[ $(<"$work/output") == "$expected" ]] || fail "Rust value: $name/$mode/$optimization"
                    if [[ $rust == value ]]; then values=$((values + 1)); else boundaries=$((boundaries + 1)); fi
                fi
            done
        done
    done
    printf 'ok macro provider prelude Rust opt0/opt3 debug off/on: %s values, %s refusals, %s explicit boundaries\n' \
        "$values" "$refusals" "$boundaries"
fi


# Selected external aliases must not accidentally select an intrinsic/prelude
# crate. Check the refusal and the ordinary local/generic shadowing alternatives.
cat > "$work/collision_provider.lkj" <<'LKJ'
pub mod mem { pub fn size_of<T>() -> usize { 42usize } }
pub fn value() -> i32 { 42 }
pub fn new() -> i32 { 42 }
pub const MAX: i32 = 42;
#[macro_export]
macro_rules! count { ($value:expr) => { $value + 2 }; }
LKJ
alias_cases=()
alias_specimen() {
    local name=$1 alias=$2 result=$3
    alias_cases+=("$name:$alias:$result")
    cat > "$work/alias_$name.lkj"
}
alias_specimen core core refuse <<'LKJ'
fn probe() -> usize { core::mem::size_of::<u8>() }
LKJ
alias_specimen core_absolute core refuse <<'LKJ'
fn probe() -> usize { ::core::mem::size_of::<u8>() }
LKJ
alias_specimen primitive u8 refuse <<'LKJ'
fn probe() -> i32 { u8::MAX as i32 }
LKJ
alias_specimen vector Vec refuse <<'LKJ'
fn probe() -> i32 { Vec::new() }
LKJ
alias_specimen import core refuse <<'LKJ'
use core::mem::size_of;
fn probe() -> usize { size_of::<u8>() }
LKJ
alias_specimen glob core refuse <<'LKJ'
use core::*;
fn probe() -> usize { mem::size_of::<u8>() }
LKJ
alias_specimen nested_glob core refuse <<'LKJ'
use core::mem::*;
fn probe() -> usize { size_of::<u8>() }
LKJ
alias_specimen ordinary chosen refuse <<'LKJ'
fn probe() -> i32 { chosen::value() }
LKJ
alias_specimen absolute_local chosen refuse <<'LKJ'
mod chosen { pub fn value() -> i32 { 1 } }
fn probe() -> i32 { ::chosen::value() }
LKJ
alias_specimen relative_local chosen value <<'LKJ'
mod chosen { pub fn value() -> i32 { 7 } }
fn probe() -> i32 { chosen::value() + 35 }
LKJ
alias_specimen crate_local chosen value <<'LKJ'
mod chosen { pub fn value() -> i32 { 7 } }
fn probe() -> i32 { crate::chosen::value() + 35 }
LKJ
alias_specimen self_local chosen value <<'LKJ'
mod chosen { pub fn value() -> i32 { 7 } }
fn probe() -> i32 { self::chosen::value() + 35 }
LKJ
alias_specimen generic core value <<'LKJ'
fn identity<core>(value: core) -> core { value }
fn probe() -> i32 { identity(42) }
LKJ
alias_specimen imported_local core value <<'LKJ'
mod local { pub mod core { pub fn value() -> i32 { 7 } } }
use local::core;
fn probe() -> i32 { core::value() + 35 }
LKJ
alias_routes=0; alias_values=0; alias_boundaries=0
for mode in off on; do
    for inline in 0 64; do
        for engine in native interpreter; do
            for spec in "${alias_cases[@]}"; do
                IFS=: read -r name alias result <<< "$spec"
                args=(run "alias_$name.lkj" "${base[@]}" --macro-provider "counters:$alias=collision_provider.lkj"
                    --debug-assertions "$mode" --inline-limit "$inline" --engine "$engine")
                if [[ $result == refuse ]]; then
                    if "$compiler" "${args[@]}" > "$work/output" 2> "$work/error"; then
                        fail "ordinary external alias accepted: $name/$mode/$inline/$engine"
                    fi
                    [[ ! -s $work/output ]] && grep -Fq ': E_MACRO_PROVIDER ' "$work/error" \
                        || fail "wrong alias refusal: $name/$mode/$inline/$engine"
                else
                    "$compiler" "${args[@]}" > "$work/output" 2> "$work/error" \
                        || fail "ordinary shadowing refused: $name/$mode/$inline/$engine"
                    [[ $(<"$work/output") == 42 ]] || fail "ordinary shadowing value: $name"
                fi
                alias_routes=$((alias_routes + 1))
            done
        done
    done
    if [[ -n $witness ]]; then
        rust_debug=no
        if [[ $mode == on ]]; then rust_debug=yes; fi
        for optimization in 0 3; do
            "$witness" --edition=2021 -Awarnings -C "debug-assertions=$rust_debug" -C "opt-level=$optimization" \
                --crate-name counters --crate-type rlib "$work/collision_provider.lkj" -o "$work/libcollision.rlib"
            for spec in "${alias_cases[@]}"; do
                IFS=: read -r name alias result <<< "$spec"
                {
                    cat "$work/alias_$name.lkj"
                    printf 'fn main() { println!("{}", probe()); }\n'
                } > "$work/main.rs"
                "$witness" --edition=2021 -Awarnings -C "debug-assertions=$rust_debug" -C "opt-level=$optimization" \
                    --extern "$alias=$work/libcollision.rlib" "$work/main.rs" -o "$work/rust" \
                    > "$work/output" 2> "$work/error" || fail "Rust alias compilation: $name"
                "$work/rust" > "$work/output" 2> "$work/error" || fail "Rust alias execution: $name"
                [[ $(<"$work/output") == 42 ]] || fail "Rust alias value: $name"
                if [[ $result == value ]]; then alias_values=$((alias_values + 1)); else alias_boundaries=$((alias_boundaries + 1)); fi
            done
        done
    fi
done
printf 'ok macro provider ordinary aliases: %s cases / %s native-interpreter routes\n' "${#alias_cases[@]}" "$alias_routes"
if [[ -n $witness ]]; then
    printf 'ok macro provider alias Rust opt0/opt3 debug off/on: %s shared values / %s explicit boundary values\n' \
        "$alias_values" "$alias_boundaries"
fi

# Validate the actual source snapshot, including optional declarations; expansion
# replay alone can leave a qualified call unchanged after forged macro_use state.
mkdir "$work/origins"
for module in ast diagnostic names syntax resolve source; do cp "src/$module.lkj" "$work/origins/"; done
cp -R src/ast src/syntax src/resolve "$work/origins/"
cp tests/fixtures/macro_provider_origins.lkj "$work/origins/main.lkj"
for engine in native interpreter; do
    "$compiler" run main.lkj --source-dir "$work/origins" --library-dir "$PWD/library" \
        --no-store --engine "$engine" > "$work/origins-output" 2> "$work/origins-error" \
        || fail "authenticated origins/$engine"
    [[ $(<"$work/origins-output") == 23 ]] || fail "origin control count/$engine"
    printf 'ok macro provider prelude authenticated origins: 23 controls (%s)\n' "$engine"
done
