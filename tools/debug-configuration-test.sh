#!/usr/bin/env bash
# Explicit configuration is part of source selection, replay and reusable identities.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
witness=${2:-}
mkdir -p target
work=$(mktemp -d "$PWD/target/debug-configuration.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'debug-configuration: %s\n' "$*" >&2; exit 1; }
mkdir "$work/source" "$work/store"
head -c 32 /dev/urandom > "$work/key"
cat > "$work/source/provider.lkj" <<'LKJ'
#[cfg(debug_assertions)]
pub fn provider_number() -> i64 { 7i64 }
#[cfg(not(debug_assertions))]
pub fn provider_number() -> i64 { 3i64 }
#[macro_export]
macro_rules! leaf_config { () => { cfg!(debug_assertions) }; }
#[macro_export]
macro_rules! outer_config { () => { $crate::leaf_config!() }; }
#[macro_export]
macro_rules! provider_config { () => { $crate::provider_number() }; }
#[macro_export]
macro_rules! configured_item {
    ($name:ident) => {
        #[cfg(debug_assertions)] pub fn $name() -> i64 { 5i64 }
        #[cfg(not(debug_assertions))] pub fn $name() -> i64 { 2i64 }
    };
}
LKJ
cat > "$work/source/configured.lkj" <<'LKJ'
#[macro_use] extern crate cfg_provider as configuration;
#[cfg(debug_assertions)] fn selected() -> i64 { 29i64 }
#[cfg(not(debug_assertions))] fn selected() -> i64 { 12i64 }
#[cfg(debug_assertions)] fn debug_only() {}
#[cfg(not(debug_assertions))] fn release_only() {}
configured_item!(generated);
const CONFIGURATION: bool = cfg!(debug_assertions);
macro_rules! forward_config { ($($tokens:tt)*) => { cfg!($($tokens)*) }; }
pub fn probe() -> i64 {
    let debug_assertions = 99i64;
    let cfg = 0i64;
    assert_eq!(debug_assertions, 99i64);
    assert_eq!(cfg, 0i64);
    assert_eq!(CONFIGURATION, cfg!(debug_assertions));
    assert!(cfg!(all()));
    assert!(!cfg!(any()));
    assert!(cfg!(all(unix, target_os = "linux", target_arch = "x86_64", target_family = "unix", target_pointer_width = "64", target_endian = "little",)));
    assert_eq!(cfg!(debug_assertions,), outer_config!());
    assert_eq!(cfg![debug_assertions], forward_config!(debug_assertions));
    assert_eq!(cfg!{not(debug_assertions)}, !cfg!(debug_assertions));
    let arm = if cfg!(debug_assertions) { 0i64 } else { 1i64 };
    let arm = match arm {
        #[cfg(debug_assertions)] 0i64 => 0i64,
        #[cfg(not(debug_assertions))] 1i64 => 0i64,
        _ => 100i64,
    };
    selected() + provider_config!() + generated() + arm
}
LKJ
args=(configured.lkj --source-dir "$work/source" --library-dir "$PWD/library" --macro-provider cfg_provider:configuration=provider.lkj --entry probe)
cat > "$work/source/snapshot.lkj" <<'LKJ'
#[cfg(debug_assertions)] fn debug_only() -> i64 { 41i64 }
#[cfg(not(debug_assertions))] fn release_only() -> i64 { 17i64 }
#[cfg(debug_assertions)] fn probe() -> i64 { debug_only() }
#[cfg(not(debug_assertions))] fn probe() -> i64 { release_only() }
LKJ
snapshot_args=(snapshot.lkj --source-dir "$work/source" --library-dir "$PWD/library" --entry probe)
for setting in on off; do
    expected=41; bool=true
    if [[ $setting == off ]]; then expected=17; bool=false; fi
    for limit in 0 64; do
        for engine in native interpreter; do
            actual=$("$compiler" run "${args[@]}" --debug-assertions "$setting" --inline-limit "$limit" --engine "$engine" --no-store)
            [[ $actual == "$expected" ]] || fail "$setting/$limit/$engine expected $expected got $actual"
        done
    done
    "$compiler" ir "${args[@]}" --debug-assertions "$setting" -o "$work/$setting.lkjir" --no-store
    for engine in native interpreter; do
        [[ $("$compiler" run "$work/$setting.lkjir" --engine "$engine" --no-store) == "$expected" ]] || fail "serialized $setting/$engine"
    done
    printf '// expect: value %s\n// witness: {}\npub fn probe() -> bool { cfg!(debug_assertions) }\n' "$bool" > "$work/source/worker-$setting.lkj"
    "$compiler" test "$work/source/worker-$setting.lkj" --library-dir "$PWD/library" --debug-assertions "$setting" --jobs 1 --no-store > "$work/worker-$setting.out"
    grep -q '^ok ' "$work/worker-$setting.out" || fail "configured test worker $setting"
    for pass in cold warm; do
        [[ $("$compiler" run "${args[@]}" --debug-assertions "$setting" --store "$work/store" --store-key "$work/key") == "$expected" ]] || fail "$setting $pass cache"
    done
    "$compiler" build "${snapshot_args[@]}" --debug-assertions "$setting" --index --store "$work/store" --store-key "$work/key" -o "$work/image-$setting" > "$work/index-$setting"
done
on_snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" {print $2}' "$work/index-on")
off_snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" {print $2}' "$work/index-off")
[[ ${#on_snapshot} == 64 && ${#off_snapshot} == 64 && $on_snapshot != "$off_snapshot" ]] || fail 'configuration snapshot identity collision'
on_source=$(sed -n 's/.* source=\([^ ]*\).*/\1/p' "$work/index-on")
off_source=$(sed -n 's/.* source=\([^ ]*\).*/\1/p' "$work/index-off")
[[ ${#on_source} == 64 && ${#off_source} == 64 && $on_source != "$off_source" ]] || fail 'configuration source identity collision'
# Both config snapshots must remain independently selectable after the opposite build.
for setting in on off; do
    "$compiler" query "${snapshot_args[@]}" --debug-assertions "$setting" --store "$work/store" --store-key "$work/key" --kind outline --limit 100 --bytes 32768 > "$work/query-$setting"
    active=debug_only; inactive=release_only
    if [[ $setting == off ]]; then active=release_only; inactive=debug_only; fi
    grep -q "$active" "$work/query-$setting" || fail "$setting snapshot lost active declaration"
    if grep -q "$inactive" "$work/query-$setting"; then fail "$setting snapshot used opposite configuration"; fi
    selected_snapshot=$on_snapshot; opposite_snapshot=$off_snapshot
    if [[ $setting == off ]]; then selected_snapshot=$off_snapshot; opposite_snapshot=$on_snapshot; fi
    "$compiler" query "${snapshot_args[@]}" --debug-assertions "$setting" --store "$work/store" --store-key "$work/key" --snapshot "$selected_snapshot" --kind outline --limit 100 --bytes 32768 > "$work/pinned-$setting"
    grep -q "$selected_snapshot" "$work/pinned-$setting" || fail 'pinned snapshot identity'
    grep -q "$active" "$work/pinned-$setting" || fail 'pinned active declaration'
    if grep -q "$inactive" "$work/pinned-$setting"; then fail 'pinned opposite declaration'; fi
    if "$compiler" query "${snapshot_args[@]}" --debug-assertions "$setting" --store "$work/store" --store-key "$work/key" --snapshot "$opposite_snapshot" > "$work/out" 2> "$work/err"; then fail 'snapshot crossed configurations'; fi
    grep -q E_QUERY_SNAPSHOT "$work/err" || fail 'opposite snapshot diagnostic'
done
[[ $("$compiler" run "${args[@]}" --no-opt --no-store) == 41 ]] || fail 'default differs from on'
[[ $(LKJSCRIPT_DEBUG_ASSERTIONS=off RUSTFLAGS='-C debug-assertions=off' "$compiler" run "${args[@]}" --no-store) == 41 ]] || fail 'ambient configuration leaked'
negative() {
    local name=$1 source=$2 code=$3
    printf '%s\n' "$source" > "$work/source/$name.lkj"
    for setting in on off; do
        if "$compiler" check "$name.lkj" --source-dir "$work/source" --debug-assertions "$setting" --no-store > "$work/out" 2> "$work/err"; then fail "accepted $name/$setting"; fi
        grep -q "$code" "$work/err" || { cat "$work/err" >&2; fail "$name diagnostic"; }
    done
}
negative empty 'fn probe()->bool { cfg!() }' E_PARSE
negative missing_comma 'fn probe()->bool { cfg!(all(debug_assertions unix)) }' E_PARSE
negative extra_comma 'fn probe()->bool { cfg!(all(debug_assertions,,unix)) }' E_PARSE
negative not_arity 'fn probe()->bool { cfg!(not(debug_assertions,unix)) }' E_PARSE
negative attribute_comma '#[cfg(all(debug_assertions unix))] fn probe()->i64 { 42 }' E_PARSE
negative non_string 'fn probe()->bool { cfg!(target_os=linux) }' E_PARSE
negative attribute_equals '#[cfg = debug_assertions] fn probe()->i64 { 42 }' E_PARSE
negative attribute_string '#[cfg = "debug_assertions"] fn probe()->i64 { 42 }' E_PARSE
negative extra_predicate 'fn probe()->bool { cfg!(debug_assertions,unix) }' E_PARSE
negative generic 'fn probe()->bool { cfg::<i64>!(debug_assertions) }' E_PARSE
negative qualified 'fn probe()->bool { std::cfg!(debug_assertions) }' E_UNSUPPORTED_MACRO
negative unknown 'fn probe()->bool { cfg!(unknown_configuration) }' E_UNSUPPORTED
negative ordinary_check 'fn probe()->i64 { if cfg!(debug_assertions) { unknown() } else { 42 } }' E_NAME
for command in \
    '--debug-assertions' \
    '--debug-assertions true' \
    '--debug-assertions on --debug-assertions off'; do
    read -r -a options <<< "$command"
    if "$compiler" check "${args[@]}" "${options[@]}" > "$work/out" 2> "$work/err"; then fail "accepted invalid option $command"; fi
    grep -q E_USAGE "$work/err" || fail 'configuration CLI diagnostic'
done
if "$compiler" run "$work/off.lkjir" --debug-assertions on > "$work/out" 2> "$work/err"; then fail 'IR accepted a new source configuration'; fi
grep -q E_USAGE "$work/err"
# Configured statements use ordinary lowering, preserve tails and discard speculative captures.
for specimen in selection tails capture_rollback; do
    cp "tests/conformance/pass/cfg_statement_$specimen.lkj" "$work/source/statement_$specimen.lkj"
done
for setting in on off; do
    for specimen in selection tails capture_rollback; do
        statement_args=("statement_$specimen.lkj" --source-dir "$work/source" --library-dir "$PWD/library" --entry probe --debug-assertions "$setting")
        for limit in 0 64; do
            for engine in native interpreter; do
                [[ $("$compiler" run "${statement_args[@]}" --inline-limit "$limit" --engine "$engine" --no-store) == 42 ]] || fail "configured statement $specimen/$setting/$limit/$engine"
            done
        done
        "$compiler" ir "${statement_args[@]}" --no-store -o "$work/statement-$specimen-$setting.lkjir"
        for engine in native interpreter; do
            [[ $("$compiler" run "$work/statement-$specimen-$setting.lkjir" --engine "$engine" --no-store) == 42 ]] || fail "statement IR $specimen/$setting/$engine"
        done
        "$compiler" build "${statement_args[@]}" --index --store "$work/store" --store-key "$work/key" -o "$work/statement-image" > "$work/statement-$specimen-$setting.index"
        if [[ -n $witness ]]; then
            { cat "$work/source/statement_$specimen.lkj"; printf '\nfn main() { println!("{}", probe()); }\n'; } > "$work/source/statement_$specimen.rs"
            for optimization in 0 3; do
                "$witness" --edition=2021 -C "debug-assertions=$setting" -C "opt-level=$optimization" "$work/source/statement_$specimen.rs" -o "$work/statement-rust" 2> "$work/statement-rust.err"
                [[ $("$work/statement-rust") == 42 ]] || fail "statement Rust $specimen/$setting/$optimization"
            done
        fi
    done
done
negative statement_broken 'fn probe()->i64 { #[cfg(any())] if true { let = 1i64; } 42i64 }' E_PARSE
negative statement_attribute 'fn probe()->i64 { #[allow(unused)] 1i64; 42i64 }' E_UNSUPPORTED
negative statement_name 'fn probe()->i64 { #[cfg(all())] if false { missing_name(); } 42i64 }' E_NAME
negative statement_semicolon 'fn probe()->i64 { #[cfg(all())] if true { 42i64 } else { 17i64 }; }' E_TYPE
negative statement_binding 'fn probe()->i64 { #[cfg(any())] let removed = 42i64; removed }' E_NAME
negative statement_separator 'fn probe()->i64 { #[cfg(all())] 1i64 42i64 }' E_PARSE
negative statement_empty_semicolon 'fn probe()->i64 { if true { 42i64 } else { 9i64 } #[cfg(any())] {}; ; }' E_TYPE
negative statement_empty_macro_semicolon 'macro_rules! gone { () => { #[cfg(any())] { 17i64 } }; } fn probe()->i64 { if true { 42i64 } else { 9i64 } gone!(); }' E_TYPE
negative statement_macro_empty_statement 'macro_rules! barrier { () => { ; }; } fn probe()->i64 { if true { 42i64 } else { 9i64 } barrier!{} }' E_TYPE
# The selected configuration applies to literal program directories too.
mkdir "$work/program"
cat > "$work/program/program.lkj" <<'LKJ'
const SOURCE: &str = "main.lkj";
const ENTRY: &str = "probe";
const GRANTS: &[&str] = &[];
const CASES: &[&str] = &[];
LKJ
printf 'fn probe()->bool { cfg!(debug_assertions) }\n' > "$work/program/main.lkj"
[[ $("$compiler" run "$work/program" --debug-assertions off --no-store) == false ]] || fail 'program directory config'
# Conditional literal manifests choose their source under the same explicit input.
cat > "$work/program/program.lkj" <<'LKJ'
#[cfg(debug_assertions)] const SOURCE: &str = "on.lkj";
#[cfg(not(debug_assertions))] const SOURCE: &str = "off.lkj";
const ENTRY: &str = "probe";
const GRANTS: &[&str] = &[];
const CASES: &[&str] = &[];
LKJ
printf 'fn probe()->i64 { if cfg!(debug_assertions) { 41i64 } else { 98i64 } }\n' > "$work/program/on.lkj"
printf 'fn probe()->i64 { if cfg!(debug_assertions) { 99i64 } else { 17i64 } }\n' > "$work/program/off.lkj"
for setting in on off; do
    expected=41; if [[ $setting == off ]]; then expected=17; fi
    for engine in native interpreter; do
        [[ $("$compiler" run "$work/program" --debug-assertions "$setting" --engine "$engine" --no-store) == "$expected" ]] || fail "program manifest selected wrong source $setting/$engine"
    done
done
# A source runtime is a separate explicit input even when program IR is frozen.
cp library/runtime.lkj "$work/runtime.lkj"
cat >> "$work/runtime.lkj" <<'LKJ'
#[cfg(debug_assertions)]
fn cfg_runtime_on_requires_definition() { unsafe {} }
LKJ
for engine in native interpreter; do
    [[ $("$compiler" run "${args[@]}" --debug-assertions off --runtime-source "$work/runtime.lkj" --engine "$engine" --no-store) == 17 ]] || fail 'source runtime config'
    [[ $("$compiler" run "$work/off.lkjir" --debug-assertions off --runtime-source "$work/runtime.lkj" --engine "$engine" --no-store) == 17 ]] || fail 'serialized runtime config'
    [[ $("$compiler" run "$work/on.lkjir" --debug-assertions off --runtime-source "$work/runtime.lkj" --engine "$engine" --no-store) == 41 ]] || fail 'runtime config reselected frozen program IR'
    if [[ $engine == native ]]; then
        if "$compiler" run "$work/off.lkjir" --debug-assertions on --runtime-source "$work/runtime.lkj" --engine "$engine" --no-store > "$work/out" 2> "$work/err"; then fail 'custom runtime ignored selected on mode'; fi
        grep -q 'E_UNSUPPORTED.*unsafe' "$work/err" || { cat "$work/err" >&2; fail 'custom runtime refusal'; }
    fi
done

# Unit caches bind config independently of whole-program request keys.
mkdir "$work/units" "$work/unit-store"
cat > "$work/units/units.lkj" <<'LKJ'
const UNITS: &[&str] = &["base=base.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["app=base"];
const ENTRY: &str = "app::main";
LKJ
cat > "$work/units/base.lkj" <<'LKJ'
#[cfg(debug_assertions)] pub fn number() -> i64 { 41i64 }
#[cfg(not(debug_assertions))] pub fn number() -> i64 { 17i64 }
LKJ
cat > "$work/units/app.lkj" <<'LKJ'
pub fn main(mut out: Stdout) { out.write(format!("{}\n", crate::base::number()).as_bytes()); }
LKJ
for setting in on off on; do
    expected=41; if [[ $setting == off ]]; then expected=17; fi
    "$compiler" build-units units.lkj --source-dir "$work/units" --debug-assertions "$setting" --store "$work/unit-store" --store-key "$work/key" --grant out -o "$work/unit-image" > "$work/unit-$setting.out"
    [[ $("$work/unit-image") == "$expected" ]] || fail "unit $setting cached opposite configuration"
done
# An edit selects the off body and keeps the on body intact; malformed edits roll back.
mkdir "$work/edits"
cat > "$work/edits/edit.lkj" <<'LKJ'
#[cfg(debug_assertions)] fn chosen() -> i64 { 41i64 }
#[cfg(not(debug_assertions))] fn chosen() -> i64 { 17i64 }
fn probe() -> i64 { chosen() }
LKJ
edit_args=(edit.lkj --source-dir "$work/edits" --library-dir "$PWD/library" --debug-assertions off --entry probe --no-store)
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"chosen","source":"{ if cfg!(debug_assertions) { 99i64 } else { 19i64 } }"}]}' > "$work/request"
"$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/edit-plan"
for engine in native interpreter; do
    [[ $("$compiler" run "${edit_args[@]}" --engine "$engine") == 19 ]] || fail 'edit lost off config'
    [[ $("$compiler" run edit.lkj --source-dir "$work/edits" --debug-assertions on --engine "$engine" --no-store) == 41 ]] || fail 'edit changed inactive on body'
done
cp "$work/edits/edit.lkj" "$work/unchanged"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"chosen","source":"{ cfg!(not()) }"}]}' > "$work/request"
if "$compiler" edit "${edit_args[@]}" --request "$work/request" --store-key "$work/key" > "$work/out" 2> "$work/err"; then fail 'published malformed cfg edit'; fi
cmp "$work/unchanged" "$work/edits/edit.lkj"
grep -q E_EDIT_SYNTAX "$work/err"
# Shape validation for authored snippets uses the selected configuration too.
mkdir "$work/snippets"
cat > "$work/snippets/main.lkj" <<'LKJ'
#[cfg(debug_assertions)] fn probe() -> i64 { 41i64 }
#[cfg(not(debug_assertions))] fn probe() -> i64 { 17i64 }
LKJ
snippet_args=(main.lkj --source-dir "$work/snippets" --library-dir "$PWD/library" --entry probe --no-store)
cp "$work/snippets/main.lkj" "$work/unchanged"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"crate","source":"#[cfg(not(debug_assertions))] fn extra() -> i64 { 23i64 }"}]}' > "$work/request"
if "$compiler" edit "${snippet_args[@]}" --debug-assertions on --request "$work/request" --store-key "$work/key" > "$work/snippet-plan" 2> "$work/err"; then fail 'on config admitted a disabled add-item'; fi
grep -q E_EDIT_ITEM "$work/err"
cmp "$work/unchanged" "$work/snippets/main.lkj"
"$compiler" edit "${snippet_args[@]}" --debug-assertions off --request "$work/request" --store-key "$work/key" > "$work/snippet-plan"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"crate","source":"#[cfg(not(debug_assertions))] mod included;","projection":{"path":"included.lkj","source":"pub fn number() -> i64 { 19i64 }"}}]}' > "$work/request"
cp "$work/snippets/main.lkj" "$work/unchanged"
if "$compiler" edit "${snippet_args[@]}" --debug-assertions on --request "$work/request" --store-key "$work/key" > "$work/snippet-plan" 2> "$work/err"; then fail 'on config admitted a disabled projection'; fi
grep -q E_EDIT_PROJECTION "$work/err"
cmp "$work/unchanged" "$work/snippets/main.lkj"
[[ ! -e "$work/snippets/included.lkj" ]] || fail 'refused projection created a file'
"$compiler" edit "${snippet_args[@]}" --debug-assertions off --request "$work/request" --store-key "$work/key" > "$work/snippet-plan"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"probe","source":"{ extra() + included::number() }"}]}' > "$work/request"
"$compiler" edit "${snippet_args[@]}" --debug-assertions off --request "$work/request" --store-key "$work/key" > "$work/snippet-plan"
for engine in native interpreter; do
    [[ $("$compiler" run "${snippet_args[@]}" --debug-assertions off --engine "$engine") == 42 ]] || fail "off-only snippet publication $engine"
    [[ $("$compiler" run "${snippet_args[@]}" --debug-assertions on --engine "$engine") == 41 ]] || fail "off-only snippet changed on projection $engine"
done
cp "$work/snippets/main.lkj" "$work/unchanged"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"crate","source":"#[cfg(all(debug_assertions unix))] fn broken() -> i64 { 0i64 }"}]}' > "$work/request"
for setting in on off; do
    if "$compiler" edit "${snippet_args[@]}" --debug-assertions "$setting" --request "$work/request" --store-key "$work/key" > "$work/snippet-plan" 2> "$work/err"; then fail "accepted malformed snippet/$setting"; fi
    grep -q E_EDIT_SYNTAX "$work/err"
    cmp "$work/unchanged" "$work/snippets/main.lkj"
done
mkdir "$work/members"
cat > "$work/members/main.lkj" <<'LKJ'
enum Choice { First }
fn selected(value: Choice) -> i64 { match value { Choice::First => 41i64, #[cfg(debug_assertions)] _ => 0i64 } }
#[cfg(debug_assertions)] fn probe() -> i64 { selected(Choice::First) }
#[cfg(not(debug_assertions))] fn probe() -> i64 { selected(Choice::First) }
LKJ
member_args=(main.lkj --source-dir "$work/members" --library-dir "$PWD/library" --entry probe --no-store)
printf '%s\n' '{"version":1,"operations":[{"op":"add-variant","id":"Choice","source":"Second","arm":"#[cfg(not(debug_assertions))] Choice::Second => 42i64"}]}' > "$work/request"
"$compiler" edit "${member_args[@]}" --debug-assertions off --request "$work/request" --store-key "$work/key" > "$work/member-plan"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"probe","source":"{ selected(Choice::Second) }"}]}' > "$work/request"
"$compiler" edit "${member_args[@]}" --debug-assertions off --request "$work/request" --store-key "$work/key" > "$work/member-plan"
for engine in native interpreter; do
    [[ $("$compiler" run "${member_args[@]}" --debug-assertions off --engine "$engine") == 42 ]] || fail "off-only variant/arm $engine"
    [[ $("$compiler" run "${member_args[@]}" --debug-assertions on --engine "$engine") == 41 ]] || fail "off-only variant/arm changed on meaning $engine"
done
printf 'ok debug-configuration: on/off native/interpreter, serialized IR, providers, configured statements/tails/captures, grammar refusals, workers, store/query, units, program directories and checked edits\n'
if [[ -n $witness ]]; then
    for setting in on off; do
        expected=41; if [[ $setting == off ]]; then expected=17; fi
        for optimization in 0 3; do
            "$witness" --edition=2021 --crate-name cfg_provider --crate-type rlib "$work/source/provider.lkj" -C "debug-assertions=$setting" -C "opt-level=$optimization" -o "$work/libcfg_provider.rlib"
            { cat "$work/source/configured.lkj"; printf '\nfn main() { println!("{}", probe()); }\n'; } > "$work/source/configured.rs"
            "$witness" --edition=2021 "$work/source/configured.rs" -C "debug-assertions=$setting" -C "opt-level=$optimization" --extern "cfg_provider=$work/libcfg_provider.rlib" -o "$work/rust" 2> "$work/rust.err"
            [[ $("$work/rust") == "$expected" ]] || fail "Rust $setting/$optimization"
        done
        "$compiler" test "$work/source/worker-$setting.lkj" --library-dir "$PWD/library" --debug-assertions "$setting" --jobs 1 --witness "$witness" --no-store > "$work/witness-worker.out"
        for name in statement_broken statement_name statement_semicolon statement_binding statement_separator statement_empty_semicolon statement_empty_macro_semicolon statement_macro_empty_statement empty missing_comma extra_comma not_arity attribute_comma non_string attribute_equals attribute_string extra_predicate generic ordinary_check; do
            if "$witness" --edition=2021 --crate-type lib "$work/source/$name.lkj" -C "debug-assertions=$setting" -o "$work/refused.rlib" > "$work/out" 2> "$work/err"; then fail "Rust accepted $name/$setting"; fi
        done
    done
    printf 'ok debug-configuration Rust: opt0/opt3 on/off values, malformed forms, cfg! branch checking and configured witness workers\n'
fi
