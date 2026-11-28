#!/usr/bin/env bash
# Authenticated caller-binding sites, complete capture replay, and pattern epochs.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
root=$PWD
mkdir -p target
work=$(mktemp -d "$root/target/macro-pattern-edits.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for log in "$work/error" "$work/plan"; do [[ ! -f $log ]] || cat "$log" >&2; done
    fi
    rm -f -- "$work/key"
    if [[ ${LKJSCRIPT_MACRO_KEEP:-0} == 1 ]]; then printf 'retained_fixture=%s\n' "$work" >&2
    else rm -rf -- "$work"; fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 129' HUP
trap 'exit 143' TERM
umask 077
head -c 32 /dev/urandom > "$work/key"
fail() { printf 'macro-pattern-edits: %s\n' "$*" >&2; exit 1; }
checks=0
load() {
    case_name=$1
    project="$work/$case_name/project"
    mkdir -p "$project" "$work/$case_name/store"
    cp "tests/fixtures/macro_patterns_edits/$case_name.lkj" "$project/main.lkj"
    common=(main.lkj --source-dir "$project" --library-dir "$root/library" --store "$work/$case_name/store" --store-key "$work/key")
}
publish() {
    "$compiler" build "${common[@]}" --index -o "$work/image" > "$work/index" 2> "$work/error" || fail "$case_name indexed build"
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" {print $2}' "$work/index")
    [[ ${#snapshot} == 64 ]] || fail 'missing snapshot'
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --depth 3 --limit 256 --bytes 131072
}
binding() {
    query outline "$1" > "$work/outline"
    local identity
    identity=$(awk -F '\t' -v name="::binding:$2:" 'index($0,"kind=binding") && index($1,name) {print substr($1,2,length($1)-2); exit}' "$work/outline")
    [[ -n $identity ]] || { cat "$work/outline" >&2; fail "missing binding $1/$2"; }
    printf '%s' "$identity"
}
value() {
    for engine in native interpreter; do
        actual=$("$compiler" run main.lkj --source-dir "$project" --library-dir "$root/library" --no-store --engine "$engine" 2> "$work/error") || fail "$case_name $engine run"
        [[ $actual == "$1" ]] || fail "$case_name $engine value $actual != $1"
        checks=$((checks + 1))
    done
}
exact_uses() {
    query uses "$1" > "$work/exact-uses"
    local count=0 start end spelling line
    while IFS= read -r line; do
        [[ $line != *'origin=generated'* ]] || fail "$case_name generated bridge leaked into caller uses"
        start=$(sed -n 's/.* start=\([0-9]*\) end=.*/\1/p' <<< "$line")
        end=$(sed -n 's/.* end=\([0-9]*\) kind=.*/\1/p' <<< "$line")
        [[ -n $start && -n $end ]] || fail 'missing source reference offsets'
        spelling=$(dd if="$project/main.lkj" bs=1 skip="$start" count="$((end-start))" status=none)
        [[ $spelling == "$2" ]] || fail "$case_name source reference widened beyond $2: $spelling"
        count=$((count+1))
    done < <(awk '/fact=uses/ && /kind=local-value/' "$work/exact-uses")
    [[ $count == "$3" ]] || fail "$case_name source use count $count != $3"
    checks=$((checks+1))
}
rename_request() {
    printf '{"version":1,"operations":[{"op":"rename","id":"%s","name":"%s"}]}\n' "$1" "$2" > "$work/request"
}
apply() {
    "$compiler" edit "${common[@]}" --request "$work/request" > "$work/plan" 2> "$work/error" || fail "$case_name checked edit"
    grep -Eq 'mode=apply' "$work/plan" || fail 'edit did not apply'
    checks=$((checks + 1))
}
refuse() {
    local reason=$1
    cp "$project/main.lkj" "$work/before"
    local generation=no
    if [[ -f "$project/.lkjscript-edit.generation" ]]; then
        generation=yes; cp "$project/.lkjscript-edit.generation" "$work/before-generation"
    fi
    if "$compiler" edit "${common[@]}" --request "$work/request" > "$work/plan" 2> "$work/error"; then fail "$case_name unsafe edit accepted"; fi
    grep -Eq "$reason" "$work/error" || fail "$case_name missing refusal $reason"
    cmp "$work/before" "$project/main.lkj" || fail 'refusal changed source'
    if [[ $generation == yes ]]; then cmp "$work/before-generation" "$project/.lkjscript-edit.generation" || fail 'refusal changed generation'
    else [[ ! -e "$project/.lkjscript-edit.generation" ]] || fail 'refusal invented generation'; fi
    [[ ! -e "$project/.lkjscript-edit.pending" ]] || fail 'refusal left pending transaction'
    checks=$((checks + 1))
}
epoch() {
    query meta "$1" > "$work/meta"
    local digest
    digest=$(sed -n 's/.*derived-epoch=\([0-9a-f]*\).*/\1/p' "$work/meta" | head -1)
    [[ ${#digest} == 64 ]] || fail 'missing generated epoch'
    printf '%s' "$digest"
}
load ident
publish
identity=$(binding value@main value)
query text "$identity" > "$work/text"
grep -Eq 'value="r#value"' "$work/text" || fail 'raw captured binding lost exact caller spelling'
query references "$identity" > "$work/references"
grep -Eq 'kind=macro-capture' "$work/references" || fail 'captured binding lost expansion provenance'
query uses "$identity" > "$work/uses"
grep -Eq 'kind=local-value' "$work/uses" || fail 'authored use missing from captured binding'
query meta "$identity" > "$work/meta"
if grep -Eq 'origin=generated' "$work/meta"; then fail 'exact caller binding remains generated'; fi
checks=$((checks + 4))
rename_request "$identity" r#amount
apply
grep -Eq 'bind!\(r#amount\).*r#amount \+ spare' "$project/main.lkj" || fail 'capture and caller reference were not renamed together'
value 42
publish
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"spare","source":"{ 3i64 }"}]}' > "$work/request"
apply
value 43
publish
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"spare","source":"{ true }"}]}' > "$work/request"
refuse E_EDIT_CHECK
value 43
for case_name in pat nested_capture parameter let_else let_else_pat mixed_captures; do
    load "$case_name"
    publish
    owner=value@main
    old=value
    if [[ $case_name == parameter ]]; then owner=value@value; old=amount; fi
    if [[ $case_name == mixed_captures ]]; then old=captured; fi
    identity=$(binding "$owner" "$old")
    if [[ $case_name == let_else || $case_name == let_else_pat || $case_name == mixed_captures ]]; then exact_uses "$identity" "$old" 1; fi
    rename_request "$identity" changed
    apply
    if [[ $case_name == let_else || $case_name == let_else_pat || $case_name == mixed_captures ]]; then
        if [[ $case_name == let_else_pat ]]; then
            grep -Fq 'payload!(Some(changed))' "$project/main.lkj" || fail 'let-else pattern capture was not renamed'
        else
            grep -Fq 'payload!(changed)' "$project/main.lkj" || fail 'let-else capture token was not renamed'
        fi
        if [[ $case_name == let_else || $case_name == let_else_pat ]]; then
            grep -Eq '^    changed$' "$project/main.lkj" || fail 'let-else continuation use was not renamed'
        else
            grep -Eq 'ordinary \+ changed' "$project/main.lkj" || fail 'mixed let-else continuation use was not renamed'
        fi
        checks=$((checks+1))
    fi
    value 42
    if [[ $case_name == pat || $case_name == mixed_captures ]]; then
        publish
        sibling=other
        if [[ $case_name == mixed_captures ]]; then sibling=ordinary; fi
        identity=$(binding value@main "$sibling")
        if [[ $case_name == mixed_captures ]]; then exact_uses "$identity" "$sibling" 1; fi
        rename_request "$identity" second
        apply
        value 42
    fi
done
load body
publish
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ let payload!(value) = Some(43i64) else { return 0i64; }; value }"}]}' > "$work/request"
apply
value 43
publish
identity=$(binding value@main value)
exact_uses "$identity" value 1
rename_request "$identity" rebuilt
apply
value 43
publish
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ let payload!(rebuilt) = Some(true) else { return 0i64; }; rebuilt }"}]}' > "$work/request"
refuse E_EDIT_CHECK
value 43
load template
publish
identity=$(binding macro-expansion:0 hidden)
query text "$identity" > "$work/text"
grep -Eq 'fact=text-unavailable' "$work/text" || fail 'template binding claims authored source'
rename_request "$identity" changed
refuse E_EDIT_GENERATED
value 42
for case_name in ambiguous rule_change shorthand; do
    load "$case_name"
    publish
    old=value
    new=renamed
    if [[ $case_name == shorthand ]]; then old=x; new=y; fi
    identity=$(binding value@main "$old")
    rename_request "$identity" "$new"
    refuse E_EDIT_REFERENCES
    value 42
done
load nested_epoch
publish
before=$(epoch macro-expansion:0)
sed -i 's/=> { hidden }/=> { mut hidden }/' "$project/main.lkj"
publish
after=$(epoch macro-expansion:0)
[[ $before != "$after" ]] || fail 'outer pattern epoch ignored nested producer change'
checks=$((checks + 1))
value 42
load mixed_epoch
publish
before_identity=$(binding macro-expansion:1 hidden)
before=$(epoch macro-expansion:1)
sed -i 's/=> { _ }/=> { 0i64 }/' "$project/main.lkj"
publish
identity=$(binding macro-expansion:1 hidden)
after=$(epoch macro-expansion:1)
[[ $before != "$after" && $before_identity != "$identity" ]] || fail 'let-else binding epoch ignored sibling pattern producer change'
checks=$((checks + 1))
value 42
printf 'ok macro-pattern edits: %s checks; exact caller ident/pat/header/let-else queries and renames; template/duplicate/rule/field-role refusals; body rollback; nested and mixed-export epochs; native/interpreter values\n' "$checks"
