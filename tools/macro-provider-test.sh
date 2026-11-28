#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
mkdir -p target
work=$(mktemp -d "$PWD/target/macro-provider.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM

cat > "$work/provider.lkj" <<'LKJ'
pub mod store {
    pub fn measure(value: i32) -> i32 { value + 2 }
}
fn secret() -> i32 { 42 }
macro_rules! hidden { () => { 42 }; }
#[macro_export]
macro_rules! count { ($value:expr) => { $crate::store::measure($value) }; }
#[macro_export]
macro_rules! incr { ($value:expr) => { count!($value) }; }
#[macro_export]
macro_rules! gauge {
    ($lhs:expr, $rhs:expr) => ({ let v = $rhs; $crate::store::measure($lhs + v) });
}
#[macro_export]
macro_rules! access { () => { $crate::secret() }; }
#[macro_export]
macro_rules! view { ($value:expr) => { &$value }; }
#[macro_export]
macro_rules! foo { ($name:ident) => { let $name = 21i32; }; }
#[macro_export]
macro_rules! ambient { () => {{ println!("ambient output"); 42 }}; }
LKJ
cat > "$work/other.lkj" <<'LKJ'
#[macro_export]
macro_rules! count { ($value:expr) => { $value + 40 }; }
LKJ

specimen() {
    local name=$1
    {
        printf '#[macro_use] extern crate fixture_provider as metrics;\n'
        cat
    } > "$work/$name.lkj"
}
specimen exact <<'LKJ'
mod store { pub fn measure(value: i32) -> i32 { value + 100 } }
fn metrics() -> i32 { 21 }
fn probe() -> i32 {
    let count = 10;
    let v = 5;
    incr!(count) + ::metrics::gauge!(v, metrics()) + metrics::count!(0)
}
LKJ
specimen precedence <<'LKJ'
fn before() -> i32 { count!(0) }
#[macro_export]
macro_rules! count { ($value:expr) => { $value + 1 }; }
macro_rules! count { ($value:expr) => { $value + 20 }; }
mod caller {
    mod metrics {}
    pub fn after() -> i32 { incr!(1) + ::metrics::count!(18) }
}
fn probe() -> i32 { before() + caller::after() }
LKJ
specimen collision <<'LKJ'
#[macro_use] extern crate other_provider;
fn probe() -> i32 { count!(0) + ::metrics::count!(0) }
LKJ
cat > "$work/qualified_keyword.lkj" <<'LKJ'
#[macro_use] extern crate fixture_provider as macro_rules;
fn probe() -> i32 {
    macro_rules::foo!(left);
    macro_rules::foo! { right }
    left + right
}
LKJ
specimen hidden <<'LKJ'
fn probe() -> i32 { metrics::hidden!() }
LKJ
specimen malformed <<'LKJ'
fn probe() -> i32 { metrics::wrong::count!(40) }
LKJ
specimen shadowed <<'LKJ'
mod caller {
    mod metrics {}
    pub fn nested() -> i32 { metrics::count!(40) }
}
fn probe() -> i32 { caller::nested() }
LKJ
specimen shadowed_type <<'LKJ'
mod caller {
    type metrics = i32;
    pub fn nested() -> i32 { metrics::count!(40) }
}
fn probe() -> i32 { caller::nested() }
LKJ
specimen duplicate <<'LKJ'
mod metrics {}
fn probe() -> i32 { ::metrics::count!(40) }
LKJ
specimen private <<'LKJ'
fn probe() -> i32 { access!() }
LKJ
specimen lifetime <<'LKJ'
fn probe() -> i32 {
    let outside: &i32;
    {
        let inside = 42;
        outside = view!(inside);
    }
    *outside
}
LKJ
specimen ambient <<'LKJ'
fn probe() -> i32 { ambient!() }
LKJ

args=(--source-dir "$work" --library-dir "$PWD/library" --entry probe)
provider=(--macro-provider fixture_provider:metrics=provider.lkj)
for engine in native interpreter; do
    for name in exact precedence; do
        result=$("$compiler" run "$name.lkj" "${args[@]}" "${provider[@]}" --engine "$engine")
        [[ $result == 42 ]]
    done
    result=$("$compiler" run qualified_keyword.lkj "${args[@]}" \
        --macro-provider fixture_provider:macro_rules=provider.lkj --engine "$engine")
    [[ $result == 42 ]]
    # Select inputs in reverse order to prove precedence comes from source.
    result=$("$compiler" run collision.lkj "${args[@]}" --macro-provider other_provider:other_provider=other.lkj \
        "${provider[@]}" --engine "$engine")
    [[ $result == 42 ]]
    for control in hidden:E_UNSUPPORTED_MACRO malformed:E_UNSUPPORTED_MACRO shadowed:E_MACRO_PROVIDER \
        shadowed_type:E_MACRO_PROVIDER \
        duplicate:E_MACRO_PROVIDER private:E_PRIVACY lifetime:E_BORROW ambient:E_CAPABILITY; do
        name=${control%:*}
        diagnostic=${control#*:}
        if "$compiler" run "$name.lkj" "${args[@]}" "${provider[@]}" --engine "$engine" > "$work/out" 2> "$work/err"; then
            printf 'unexpected %s acceptance on %s\n' "$name" "$engine" >&2
            exit 1
        fi
        if [[ -s $work/out ]] || ! grep -Fq ": $diagnostic " "$work/err"; then
            cat "$work/err" >&2
            exit 1
        fi
    done
    printf 'ok macro provider %s: four values and eight located refusals\n' "$engine"
done

if "$compiler" check exact.lkj "${args[@]}" --macro-provider fixture_provider:wrong=missing.lkj > "$work/out" 2> "$work/err"; then
    printf 'mismatched provider selector was accepted\n' >&2
    exit 1
fi
grep -Fq 'E_MACRO_PROVIDER extern crate alias differs' "$work/err"
if grep -Fq 'E_MODULE_MISSING' "$work/err"; then
    printf 'an unmatched provider file was loaded\n' >&2
    exit 1
fi
if "$compiler" check exact.lkj "${args[@]}" --macro-provider 'fixture_provider:metrics=../provider.lkj' > "$work/out" 2> "$work/err"; then
    printf 'escaping provider selector was accepted\n' >&2
    exit 1
fi
grep -Fq 'E_USAGE --macro-provider requires' "$work/err"

if [[ -n $witness ]]; then
    "$witness" --edition=2024 -Awarnings --crate-name fixture_provider --crate-type rlib \
        "$work/provider.lkj" -o "$work/libfixture_provider.rlib"
    "$witness" --edition=2024 -Awarnings --crate-name other_provider --crate-type rlib \
        "$work/other.lkj" -o "$work/libother_provider.rlib"
    for name in exact precedence collision qualified_keyword hidden malformed shadowed shadowed_type duplicate private lifetime ambient; do
        {
            cat "$work/$name.lkj"
            printf 'fn main() { println!("{}", probe()); }\n'
        } > "$work/main.rs"
        rust=(--edition=2024 -Awarnings --extern "fixture_provider=$work/libfixture_provider.rlib"
            --extern "other_provider=$work/libother_provider.rlib" "$work/main.rs" -o "$work/rust")
        case $name in
            exact|precedence|collision|qualified_keyword|ambient)
                "$witness" "${rust[@]}"
                result=$("$work/rust")
                if [[ $name == ambient ]]; then [[ $result == $'ambient output\n42' ]]
                else [[ $result == 42 ]]; fi
                ;;
            *)
                if "$witness" "${rust[@]}" > "$work/out" 2> "$work/err"; then
                    printf 'unexpected Rust acceptance of %s\n' "$name" >&2
                    exit 1
                fi
                grep -Fq 'error[' "$work/err"
                ;;
        esac
    done
    printf 'ok macro provider Rust: four matching values, seven matching refusals and the ambient-output boundary\n'
fi

# Explicit external-prelude selection shares the same provider contract.
bash tools/macro-provider-prelude-test.sh "$compiler" "$witness"
