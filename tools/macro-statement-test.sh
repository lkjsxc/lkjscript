#!/usr/bin/env bash
# Captured local edits and statement-generated bindings use authenticated replay.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
root=$PWD
work=$(mktemp -d "$root/target/macro-statements.XXXXXX")
cleanup() {
    code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then [[ ! -f $work/error ]] || cat "$work/error" >&2; fi
    if [[ ${LKJSCRIPT_MACRO_KEEP:-0} == 1 ]]; then
        rm -f "$work/key"
        printf 'retained_fixture=%s\n' "$work" >&2
    else rm -rf -- "$work"; fi
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
umask 077
mkdir "$work/store"
head -c 32 /dev/urandom > "$work/key"
fail() { printf 'macro-statements: %s\n' "$*" >&2; exit 1; }
publish() {
    common=(main.lkj --source-dir "$project" --no-library --store "$work/store" --store-key "$work/key")
    "$compiler" build "${common[@]}" --index -o "$work/image" > "$work/index" 2> "$work/error"
    "$compiler" query "${common[@]}" --kind outline --id crate --depth 8 --limit 128 --bytes 65536 > "$work/query"
}
identity() {
    awk -F '\t' -v suffix="$1" '$1 ~ /^"value@/ && index($1, suffix "\"") { print substr($1,2,length($1)-2); exit }' "$work/query"
}
refuse() {
    local expected=$1
    cp "$project/main.lkj" "$work/before"
    rm -f "$work/generation"
    if [[ -f $project/.lkjscript-edit.generation ]]; then cp "$project/.lkjscript-edit.generation" "$work/generation"; fi
    if "$compiler" edit "${common[@]}" --request "$work/request" > "$work/edit" 2> "$work/error"; then fail "accepted $expected refusal"; fi
    grep -Fq "$expected" "$work/error" || fail "missing $expected"
    cmp "$work/before" "$project/main.lkj"
    if [[ -f $work/generation ]]; then cmp "$work/generation" "$project/.lkjscript-edit.generation"
    else [[ ! -e $project/.lkjscript-edit.generation ]]; fi
    [[ ! -e $project/.lkjscript-edit.pending ]]
}
value() {
    for engine in native interpreter; do
        "$compiler" run main.lkj --source-dir "$project" --no-library --no-store --engine "$engine" > "$work/value" 2> "$work/error"
        [[ $(cat "$work/value") == "$1" ]] || fail "$engine value"
    done
}
while read -r name verdict from to expected; do
    for mode in pure mixed; do
        project=$work/$name-$mode
        mkdir "$project"
        cp "tests/fixtures/macro_statements/$name.lkj" "$project/main.lkj"
        printf '\nfn spare() -> i64 { 0i64 }\n' >> "$project/main.lkj"
        publish
        suffix=::binding:$from:0
        [[ $name != accept_parameter ]] || suffix=::param:$from
        id=$(identity "$suffix")
        [[ -n $id ]] || fail "missing $name identity"
        extra=
        [[ $mode != mixed ]] || extra=',{"op":"replace-body","id":"spare","source":"{ 1i64 }"}'
        printf '{"version":1,"operations":[{"op":"rename","id":"%s","name":"%s"}%s]}\n' "$id" "$to" "$extra" > "$work/request"
        if [[ $verdict == accept ]]; then
            "$compiler" edit "${common[@]}" --request "$work/request" > "$work/edit" 2> "$work/error"
            grep -Fq 'mode=apply' "$work/edit"
            value "$expected"
        else
            refuse E_EDIT_REFERENCES
            grep -Eq 'capture (role|target|rule)' "$work/error"
            grep -Fq 'decision site' "$work/error"
        fi
        printf 'ok macro capture %s %s %s\n' "$name" "$mode" "$verdict"
    done
done <<'CASES'
accept_twice accept x y 42
accept_hygiene accept x y 42
accept_parameter accept x y 42
accept_other_role_argument accept p q 6
refuse_binder refuse x y 6
refuse_arm refuse x y 5
refuse_role refuse x y 6
nested_forward_arm refuse x y 5
same_meaning_arm refuse x y 5
same_capture_arm refuse x y 5
literal_old refuse x y 5
CASES
for version in 1 2; do
    project=$work/binders-$version
    mkdir "$project"
    cp "tests/fixtures/macro_statements/binder_order-$version.lkj" "$project/main.lkj"
    publish
    identity '::binding:t:0' > "$work/authored-$version"
    awk -F '\t' '$1 ~ /^"macro-expansion:.*::binding:t:/ { print substr($1,2,length($1)-2) }' "$work/query" | sort -u > "$work/generated-$version"
    [[ $(wc -l < "$work/generated-$version") == 2 ]]
    generated=$(head -1 "$work/generated-$version")
    "$compiler" query "${common[@]}" --kind meta --id "$generated" > "$work/meta"
    grep -Fq 'source-projection=false' "$work/meta"
    "$compiler" query "${common[@]}" --kind text --id "$generated" > "$work/text"
    grep -Fq 'fact=text-unavailable' "$work/text"
    printf '{"version":1,"operations":[{"op":"rename","id":"%s","name":"invented"}]}\n' "$generated" > "$work/request"
    refuse E_EDIT_GENERATED_SOURCE
    expected=22
    [[ $version != 2 ]] || expected=11
    value "$expected"
done
cmp "$work/authored-1" "$work/authored-2"
[[ -s $work/authored-1 ]]
if cmp -s "$work/generated-1" "$work/generated-2"; then fail 'template binder epoch unchanged'; fi
printf 'ok macro bindings: authored identities stable; generated epochs differ and refuse source edits\n'

for case in semi_stmt cap_let use_stmt nom_permutation; do
    "$compiler" compare "tests/fixtures/macro_statements/$case-macro.lkj" --against "tests/fixtures/macro_statements/$case-twin.lkj" --library-dir library --no-store > "$work/compare" 2> "$work/error"
    grep -Fq 'text differs, checked IR identical' "$work/compare" || { cat "$work/compare" >&2; fail "$case expanded meaning differs"; }
    printf 'ok statement macro checked-IR twin: %s\n' "$case"
done
