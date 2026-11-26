#!/usr/bin/env bash
# Source positions stay attached to authored invocation spans through expansion and edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
library=$PWD/library
mkdir -p target
work=$(mktemp -d "$PWD/target/file-path.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'file-path: %s\n' "$*" >&2; exit 1; }
cat > "$work/provider.lkj" <<'LKJ'
pub fn own() -> &'static str { file!() }
#[macro_export] macro_rules! leaf_file { () => { file!() }; }
#[macro_export] macro_rules! outer_file { () => { $crate::leaf_file!() }; }
#[macro_export] macro_rules! provider_file { () => { $crate::own() }; }
#[macro_export] macro_rules! statement_file { ($name:ident) => { let $name = file!(); }; }
#[macro_export] macro_rules! leaf_line { () => { line!() }; }
#[macro_export] macro_rules! outer_line { () => { $crate::leaf_line!() }; }
#[macro_export] macro_rules! generated_files {
    ($name:ident, $value:expr, $body:block, $($tokens:tt)*) => {
        pub mod $name {
            pub fn template() -> &'static str { file!() }
            pub fn expression() -> &'static str { $value }
            pub fn block() -> &'static str { $body }
            pub fn tokens() -> &'static str { $($tokens)* }
            pub fn nested() -> &'static str { $crate::outer_file!() }
        }
    };
}
#[macro_export(local_inner_macros)] macro_rules! qualified_file { () => { file!() }; }
LKJ
cat > "$work/file-fixture.lkj" <<'LKJ'
#[macro_use] extern crate file_provider as alias;
mod external;
const ROOT: &str = file!();
macro_rules! forward_file { () => { outer_file!() }; }
macro_rules! pass_line { ($value:expr) => { $value }; }
macro_rules! pass_tokens { ($($value:tt)*) => { $($value)* }; }
mod alpha {
    pub fn value() -> &'static str { forward_file!() }
    generated_files!(made, file!(), { file!() }, file!());
}
fn probe() -> i64 {
    let file = 7i64;
    assert_eq!(ROOT, "@ROOT@");
    assert_eq!(file![], ROOT);
    assert_eq!(file!{}, ROOT);
    assert_eq!(alpha::value(), ROOT);
    assert_eq!(::alias::outer_file!(), ROOT);
    assert_eq!(external::value(), "@EXTERNAL@");
    assert_eq!(external::inner::value(), "@EXTERNAL@");
    assert_eq!(provider_file!(), "@PROVIDER@");
    assert_eq!(alpha::made::template(), ROOT);
    assert_eq!(alpha::made::expression(), ROOT);
    assert_eq!(alpha::made::block(), ROOT);
    assert_eq!(alpha::made::tokens(), ROOT);
    assert_eq!(alpha::made::nested(), ROOT);
    statement_file!(name);
    assert_eq!(name, ROOT);
    assert_eq!(format!("{}", file!()), ROOT);
    let direct: u32 = line!();
    let nested: u32 = outer_line!();
    assert_eq!(nested, direct + 1u32);
    let before: u32 = line!();
    let captured = pass_line!(
        line!()
    );
    assert_eq!(captured, before + 2u32);
    let before: u32 = line!();
    let tokens = pass_tokens!(
        line!()
    );
    assert_eq!(tokens, before + 2u32);
    35i64 + file
}
LKJ
cat > "$work/external.lkj" <<'LKJ'
pub fn value() -> &'static str { outer_file!() }
pub mod inner { pub fn value() -> &'static str { file!() } }
LKJ
sed -i "s|@ROOT@|$work/file-fixture.lkj|g; s|@EXTERNAL@|$work/external.lkj|g; s|@PROVIDER@|$work/provider.lkj|g" "$work/file-fixture.lkj"
args=(--source-dir "$work" --library-dir "$library" --entry probe --no-store)
provider=(--macro-provider file_provider:alias=provider.lkj)
for limit in 0 64; do
    for engine in native interpreter; do
        [[ $("$compiler" run file-fixture.lkj "${args[@]}" "${provider[@]}" --inline-limit "$limit" --engine "$engine") == 42 ]] || fail "selected paths on $engine inline=$limit"
    done
done
"$compiler" ir file-fixture.lkj "${args[@]}" "${provider[@]}" -o "$work/program.lkjir"
for engine in native interpreter; do
    [[ $("$compiler" run "$work/program.lkjir" --entry probe --engine "$engine" --no-store) == 42 ]] || fail "serialized file/line constants on $engine"
done
mkdir "$work/paths"
for selected in paths/leaf.lkj ./paths/leaf.lkj "$work/paths/leaf.lkj"; do
    printf 'fn probe() -> i64 { assert_eq!(file!(), "%s"); 42i64 }\n' "$selected" > "$work/paths/leaf.lkj"
    for engine in native interpreter; do
        [[ $(cd "$work" && "$compiler" run "$selected" --library-dir "$library" --entry probe --engine "$engine" --no-store) == 42 ]] || fail "exact selected spelling $selected on $engine"
    done
    if [[ -n $witness ]]; then
        printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/paths/leaf.lkj"
        (cd "$work" && "$witness" --edition=2021 --crate-name file_leaf "$selected" -o "$work/rust-leaf")
        [[ $("$work/rust-leaf") == 42 ]] || fail "Rust selected spelling $selected"
    fi
done
# A relative source directory has the same presentation as that selected Rust path.
printf 'fn probe() -> i64 { assert_eq!(file!(), "./paths/leaf.lkj"); 42i64 }\n' > "$work/paths/leaf.lkj"
for engine in native interpreter; do
    [[ $(cd "$work" && "$compiler" run leaf.lkj --source-dir ./paths --library-dir "$library" --entry probe --engine "$engine" --no-store) == 42 ]] || fail "relative mounted path on $engine"
done
negative() {
    local name=$1 expression=$2 diagnostic=$3
    printf '#[macro_use] extern crate file_provider as alias;\nfn probe() -> i64 { let _value = %s; 42i64 }\n' "$expression" > "$work/$name.lkj"
    for engine in native interpreter; do
        if "$compiler" run "$name.lkj" "${args[@]}" "${provider[@]}" --engine "$engine" > "$work/out" 2> "$work/err"; then fail "accepted $name on $engine"; fi
        [[ ! -s $work/out ]] && grep -Fq ": $diagnostic " "$work/err" || { cat "$work/err" >&2; fail "$name diagnostic"; }
    done
}
for builtin in file line; do
    negative "${builtin}_argument" "$builtin!(1)" E_PARSE
    negative "${builtin}_comma" "$builtin!(,)" E_PARSE
    negative "${builtin}_generic" "$builtin::<i64>!()" E_PARSE
    negative "${builtin}_empty_generic" "$builtin::<>!()" E_PARSE
    negative "${builtin}_qualified" "<i64 as Missing>::$builtin!()" E_PARSE
done
negative file_local_inner 'qualified_file!()' E_UNSUPPORTED_MACRO
# FilePath stays a source leaf until the selected loader identity is known.
# These two source-folding positions deliberately require an ordinary literal.
negative file_format_literal 'format!(file!())' E_FORMAT
negative file_concat_literal 'concat!(file!())' E_MACRO_EXPANSION
mkdir "$work/edits"
cat > "$work/edits/file-edit.lkj" <<'LKJ'
macro_rules! leaf_file { () => { file!() }; }
mod alpha { pub fn name_before() -> &'static str { leaf_file!() } }
fn probe() -> i64 { assert_eq!(alpha::name_before(), file!()); 42i64 }
LKJ
head -c 32 /dev/urandom > "$work/key"
edit_args=(file-edit.lkj --source-dir "$work/edits" --library-dir "$library" --entry probe --no-store)
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"alpha","name":"renamed"}]}
JSON
"$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/plan"
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"renamed::name_before","source":"{ file!() }"}]}
JSON
"$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/plan"
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"renamed::name_before","name":"name_after"}]}
JSON
"$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/plan"
for engine in native interpreter; do
    [[ $("$compiler" run "${edit_args[@]}" --engine "$engine") == 42 ]] || fail "checked rename/body edit source path on $engine"
done
cp "$work/edits/file-edit.lkj" "$work/unchanged"
cat > "$work/request" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"renamed::name_after","source":"{ file!(1) }"}]}
JSON
if "$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/out" 2> "$work/err"; then fail 'published malformed file replacement'; fi
cmp "$work/unchanged" "$work/edits/file-edit.lkj"
grep -q E_EDIT_SYNTAX "$work/err" || { cat "$work/err" >&2; fail 'edit syntax diagnostic'; }
printf 'ok file/line native/interpreter: exact selected paths, nested providers and captures, line values, 13 refusals, serialized IR, checked renames/body edits and rollback\n'
if [[ -n $witness ]]; then
    "$witness" --edition=2021 --crate-name file_provider --crate-type rlib "$work/provider.lkj" -o "$work/libfile_provider.rlib"
    # Rust's explicit module path selects the same .lkj bytes and source identity.
    sed -i 's/^mod external;/#[path = "external.lkj"] mod external;/' "$work/file-fixture.lkj"
    printf '\nfn main() { println!("{}", probe()); }\n' >> "$work/file-fixture.lkj"
    for level in 0 3; do
        "$witness" --edition=2021 --crate-name file_fixture -C "opt-level=$level" "$work/file-fixture.lkj" --extern "file_provider=$work/libfile_provider.rlib" -o "$work/rust"
        [[ $("$work/rust") == 42 ]] || fail "Rust file/line values opt=$level"
    done
    for name in file_argument file_comma file_generic file_empty_generic file_qualified file_local_inner line_argument line_comma line_generic line_empty_generic line_qualified; do
        if "$witness" --edition=2021 --crate-type lib "$work/$name.lkj" --extern "file_provider=$work/libfile_provider.rlib" -o "$work/refused.rlib" > "$work/out" 2> "$work/err"; then fail "Rust accepted malformed $name"; fi
    done
    printf 'ok file/line Rust agreement: exact source paths, nested providers/captures, opt0/opt3 line values and 11 refusals; literal-only format/concat boundary retained\n'
fi
