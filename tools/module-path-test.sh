#!/usr/bin/env bash
# Destination-module strings use ordinary checked IR and authenticated source edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
if (($#)); then compiler=$1; fi
compiler=$(realpath "$compiler")
witness=
if (($# > 1)); then witness=$2; fi
mkdir -p target
work=$(mktemp -d "$PWD/target/module-path.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'module-path: %s\n' "$*" >&2; exit 1; }

cat > "$work/provider.lkj" <<'LKJ'
pub mod origin {
    pub fn own() -> &'static str { module_path!() }
}
#[macro_export]
macro_rules! leaf_path { () => { module_path!() }; }
#[macro_export]
macro_rules! provider_path { () => { $crate::origin::own() }; }
#[macro_export]
macro_rules! outer_path { () => { $crate::leaf_path!() }; }
#[macro_export]
macro_rules! statement_path { ($name:ident) => { let $name = module_path!(); }; }
#[macro_export]
macro_rules! generated_paths {
    ($name:ident, $value:expr, $body:block, $($tokens:tt)*) => {
        pub mod $name {
            pub fn template() -> &'static str { module_path!() }
            pub fn expression() -> &'static str { $value }
            pub fn block() -> &'static str { $body }
            pub fn tokens() -> &'static str { $($tokens)* }
            pub fn nested() -> &'static str { $crate::outer_path!() }
        }
    };
}
#[macro_export(local_inner_macros)]
macro_rules! qualified_path { () => { module_path!() }; }
LKJ
cat > "$work/module-path-fixture.lkj" <<'LKJ'
#[macro_use] extern crate path_provider as alias;
mod external;
const ROOT: &str = module_path!();
macro_rules! forward_path { () => { outer_path!() }; }
mod alpha {
    pub fn value() -> &'static str { forward_path!() }
    generated_paths!(made, module_path!(), { module_path!() }, module_path!());
}
mod beta {
    pub fn value() -> &'static str { ::alias::outer_path!() }
}
fn probe() -> i64 {
    let module_path = 7i64;
    assert_eq!(ROOT, "module_path_fixture");
    assert_eq!(module_path![], "module_path_fixture");
    assert_eq!(module_path!{}, "module_path_fixture");
    assert_eq!(alpha::value(), "module_path_fixture::alpha");
    assert_eq!(beta::value(), "module_path_fixture::beta");
    assert_eq!(external::value(), "module_path_fixture::external");
    assert_eq!(external::inner::value(), "module_path_fixture::external::inner");
    assert_eq!(provider_path!(), "path_provider::origin");
    assert_eq!(alpha::made::template(), "module_path_fixture::alpha::made");
    assert_eq!(alpha::made::expression(), "module_path_fixture::alpha::made");
    assert_eq!(alpha::made::block(), "module_path_fixture::alpha::made");
    assert_eq!(alpha::made::tokens(), "module_path_fixture::alpha::made");
    assert_eq!(alpha::made::nested(), "module_path_fixture::alpha::made");
    statement_path!(name);
    assert_eq!(name, "module_path_fixture");
    35i64 + module_path
}
LKJ
cat > "$work/external.lkj" <<'LKJ'
pub fn value() -> &'static str { outer_path!() }
pub mod inner { pub fn value() -> &'static str { module_path!() } }
LKJ
args=(--source-dir "$work" --library-dir "$PWD/library" --entry probe --no-store)
provider=(--macro-provider path_provider:alias=provider.lkj)
for limit in 0 64; do
    for engine in native interpreter; do
        result=$("$compiler" run module-path-fixture.lkj "${args[@]}" "${provider[@]}" --inline-limit "$limit" --engine "$engine")
        [[ $result == 42 ]] || fail "destination strings on $engine inline=$limit"
    done
done
# Provider selectors require a relative source; compare ordinary absolute roots separately.
cat > "$work/root-name.lkj" <<'LKJ'
fn probe() -> i64 { assert_eq!(module_path!(), "root_name"); 42i64 }
LKJ
result=$("$compiler" run root-name.lkj "${args[@]}")
[[ $result == 42 ]] || fail 'relative source identity'
result=$("$compiler" run "$work/root-name.lkj" --library-dir "$PWD/library" --entry probe --no-store)
[[ $result == 42 ]] || fail 'absolute source identity'
"$compiler" ir module-path-fixture.lkj "${args[@]}" "${provider[@]}" -o "$work/program.lkjir"
for engine in native interpreter; do
    result=$("$compiler" run "$work/program.lkjir" --entry probe --engine "$engine" --no-store)
    [[ $result == 42 ]] || fail "serialized string IR on $engine"
done

negative() {
    local name=$1 expression=$2 diagnostic=$3
    printf '#[macro_use] extern crate path_provider as alias;\nfn probe() -> i64 { let _value = %s; 42i64 }\n' "$expression" > "$work/$name.lkj"
    for engine in native interpreter; do
        if "$compiler" run "$name.lkj" "${args[@]}" "${provider[@]}" --engine "$engine" > "$work/out" 2> "$work/err"; then
            fail "accepted $name on $engine"
        fi
        [[ ! -s $work/out ]] && grep -Fq ": $diagnostic " "$work/err" || { cat "$work/err" >&2; fail "$name diagnostic"; }
    done
}
negative argument 'module_path!(1)' E_PARSE
negative comma 'module_path!(,)' E_PARSE
negative generic 'module_path::<i64>!()' E_PARSE
negative empty_generic 'module_path::<>!()' E_PARSE
negative qualified '<i64 as Missing>::module_path!()' E_PARSE
negative local_inner 'qualified_path!()' E_UNSUPPORTED_MACRO

mkdir "$work/edits"
cat > "$work/edits/module-path-edit.lkj" <<'LKJ'
macro_rules! leaf_path { () => { module_path!() }; }
macro_rules! outer_path { () => { leaf_path!() }; }
mod alpha { pub fn module_label_before() -> &'static str { outer_path!() } }
fn probe() -> i64 {
    let name = alpha::module_label_before();
    if name == "module_path_edit::alpha" { 1i64 }
    else if name == "module_path_edit::renamed" { 2i64 }
    else { 0i64 }
}
LKJ
head -c 32 /dev/urandom > "$work/key"
edit_args=(module-path-edit.lkj --source-dir "$work/edits" --library-dir "$PWD/library" --entry probe --no-store)
for engine in native interpreter; do
    [[ $("$compiler" run "${edit_args[@]}" --engine "$engine") == 1 ]] || fail 'initial edit value'
done
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"alpha","name":"renamed"}]}
JSON
cp "$work/edits/module-path-edit.lkj" "$work/unchanged"
if "$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/plan" 2> "$work/err"; then
    fail 'meaning-preserving rename changed module_path output'
fi
grep -q 'E_EDIT_MEANING' "$work/err"
cmp "$work/unchanged" "$work/edits/module-path-edit.lkj"
# Mixed requests still check the meaning-preserving rename subset independently.
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"alpha","name":"renamed"},{"op":"replace-body","id":"alpha::module_label_before","source":"{ module_path!() }"}]}
JSON
if "$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/plan" 2> "$work/err"; then
    fail 'mixed body edit bypassed module rename meaning check'
fi
grep -q 'E_EDIT_MEANING' "$work/err"
cmp "$work/unchanged" "$work/edits/module-path-edit.lkj"
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"alpha::module_label_before","source":"{ module_path!() }"}]}
JSON
"$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/plan"
grep -q 'mode=apply' "$work/plan"
for engine in native interpreter; do
    [[ $("$compiler" run "${edit_args[@]}" --engine "$engine") == 1 ]] || fail 'reparsed builtin destination'
done
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"alpha::module_label_before","name":"module_label_after"}]}
JSON
"$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/plan"
grep -q 'mode=apply' "$work/plan"
for engine in native interpreter; do
    [[ $("$compiler" run "${edit_args[@]}" --engine "$engine") == 1 ]] || fail 'function rename changed module path'
done
cp "$work/edits/module-path-edit.lkj" "$work/unchanged"
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"alpha::module_label_after","source":"{ module_path!(1) }"}]}
JSON
if "$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/out" 2> "$work/err"; then
    fail 'published malformed replacement'
fi
cmp "$work/unchanged" "$work/edits/module-path-edit.lkj"
grep -q 'E_EDIT_SYNTAX' "$work/err" || { cat "$work/err" >&2; fail 'malformed replacement diagnostic'; }
printf 'ok module-path native/interpreter: 14 exact strings, 6 refusals, serialized IR, module rename refusals, body replacement, function rename and rollback\n'

if [[ -n $witness ]]; then
    "$witness" --edition=2021 --crate-name path_provider --crate-type rlib "$work/provider.lkj" -o "$work/libpath_provider.rlib"
    cp "$work/external.lkj" "$work/external.rs"
    {
        cat "$work/module-path-fixture.lkj"
        printf '\nfn main() { println!("{}", probe()); }\n'
    } > "$work/module-path-fixture.rs"
    "$witness" --edition=2021 "$work/module-path-fixture.rs" --extern "path_provider=$work/libpath_provider.rlib" -o "$work/rust"
    [[ $("$work/rust") == 42 ]] || fail 'Rust destination strings'
    for name in argument comma generic empty_generic qualified local_inner; do
        if "$witness" --edition=2021 --crate-type lib "$work/$name.lkj" --extern "path_provider=$work/libpath_provider.rlib" -o "$work/refused.rlib" > "$work/out" 2> "$work/err"; then
            fail "Rust accepted malformed $name"
        fi
    done
    printf 'ok module-path Rust agreement: 14 exact strings and 6 refusals\n'
fi
