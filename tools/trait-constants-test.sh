#!/usr/bin/env bash
# Exact trait-constant source owners, member uses and conservative checked edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target
fixture=$(mktemp -d "$PWD/target/trait-constants.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        for file in "$fixture"/*.log "$fixture"/*.query; do
            [[ ! -f $file ]] || cat "$file" >&2
        done
    fi
    rm -rf "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
specimen=tests/fixtures/trait-constants
cp "$specimen/before.lkj" "$fixture/source/main.lkj"
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
"$compiler" build "${common[@]}" --index -o "$fixture/program" > "$fixture/index.log"
snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index.log")
[[ ${#snapshot} == 64 ]]
query() {
    local kind=$1 identity=$2
    shift 2
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$kind" --id "$identity" --limit 128 --bytes 100000 "$@"
}
query outline crate --depth 8 > "$fixture/outline.query"
query type left::Flag::VALUE > "$fixture/required.query"
query type '<Ready as left::Flag>::VALUE' > "$fixture/provided.query"
query references value@main > "$fixture/references.query"
query uses '<Ready as left::Flag>::VALUE' > "$fixture/left.query"
query uses '<Ready as right::Flag>::VALUE' > "$fixture/right.query"
query uses '<Other as left::Flag>::VALUE' > "$fixture/other.query"
compare_facts() {
    local name=$1
    # Keep the semantic rows and omit the scratch path and opaque impl discriminator.
    awk -F '\t' -v name="$name" '
        BEGIN { OFS = FS }
        $3 == "fact=declared-type" ||
        ($3 == "fact=signature" && $4 ~ /^kind=associated-constant /) ||
        ($3 == "fact=references" && $4 ~ / kind=associated-constant$/) ||
        $3 == "fact=uses" ||
        ($3 == "fact=empty-indexed-result" && $4 ~ /^kind=uses /) {
            sub(/ file="([^"\\]|\\.)*"/, "", $0)
            print
        }
        name == "collision-outline" && $3 == "fact=signature" && $4 ~ /^kind=implementation / {
            if (!sub(/#[0-9a-f]+"$/, "\"", $1)) exit 1
            print
        }
    ' "$fixture/$name.query" > "$fixture/$name.facts"
    cmp - "$fixture/$name.facts"
}
compare_facts required <<'CHECK'
"left::Flag::VALUE"	level=source-claim	fact=declared-type	value="u16"
CHECK
compare_facts provided <<'CHECK'
"<Ready as left::Flag>::VALUE"	level=source-claim	fact=declared-type	value="u16"
CHECK
compare_facts outline <<'CHECK'
"<Other as left::Flag>::VALUE"	level=source-claim	fact=signature	kind=associated-constant text="const VALUE: u16 = 999;" bytes=23 complete=true
"<Ready as left::Flag>::VALUE"	level=source-claim	fact=signature	kind=associated-constant text="const VALUE: u16 = 40;" bytes=22 complete=true
"<Ready as right::Flag>::VALUE"	level=source-claim	fact=signature	kind=associated-constant text="const VALUE: u16 = 2;" bytes=21 complete=true
"left::Flag::VALUE"	level=source-claim	fact=signature	kind=associated-constant text="const VALUE: u16;" bytes=17 complete=true
"right::Flag::VALUE"	level=source-claim	fact=signature	kind=associated-constant text="const VALUE: u16;" bytes=17 complete=true
CHECK
compare_facts references <<'CHECK'
"<Ready as left::Flag>::VALUE"	level=source-claim	fact=references	from="value@main" start=386 end=391 kind=associated-constant
"<Ready as right::Flag>::VALUE"	level=source-claim	fact=references	from="value@main" start=418 end=423 kind=associated-constant
CHECK
compare_facts left <<'CHECK'
"value@idle"	level=source-claim	fact=uses	target="<Ready as left::Flag>::VALUE" start=331 end=336 kind=associated-constant
"value@main"	level=source-claim	fact=uses	target="<Ready as left::Flag>::VALUE" start=386 end=391 kind=associated-constant
CHECK
compare_facts right <<'CHECK'
"value@main"	level=source-claim	fact=uses	target="<Ready as right::Flag>::VALUE" start=418 end=423 kind=associated-constant
CHECK
compare_facts other <<'CHECK'
"<Other as left::Flag>::VALUE"	level=protocol	fact=empty-indexed-result	kind=uses claim=recorded-facts-only
CHECK
check_member() {
    local source=$1 start=$2
    dd if="$source" bs=1 skip="$start" count=5 status=none | cmp - <(printf VALUE)
}
for start in 331 386 418; do
    check_member "$fixture/source/main.lkj" "$start"
done
for identity in 'left::Flag::VALUE' '<Ready as left::Flag>::VALUE'; do
    cp "$fixture/source/main.lkj" "$fixture/unchanged"
    printf '{"version":1,"operations":[{"op":"rename","id":"%s","name":"CHANGED"}]}\n' \
        "$identity" > "$fixture/request.json"
    if "$compiler" edit "${common[@]}" --request "$fixture/request.json" > "$fixture/rename.log" 2>&1; then
        echo 'trait constant rename published an incomplete member cohort' >&2; exit 1
    fi
    grep -Fq E_EDIT_REFERENCES "$fixture/rename.log"
    cmp "$fixture/unchanged" "$fixture/source/main.lkj"
    test ! -e "$fixture/source/.lkjscript-edit.pending"
done
for engine in native interpreter; do
    [[ $("$compiler" run "${common[@]}" --engine "$engine") == 42 ]]
done
# Constant initializers are interface inputs, including a body-independent build.
mkdir "$fixture/header-store"
headers=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/header-store" --store-key "$fixture/key" --stats -o "$fixture/header-program")
"$compiler" build "${headers[@]}" > "$fixture/cold.log" 2>&1
"$compiler" build "${headers[@]}" > "$fixture/warm.log" 2>&1
grep -Eq '"build_reused":1' "$fixture/warm.log"
cmp "$specimen/before.lkj" "$fixture/source/main.lkj"
cp "$specimen/after.lkj" "$fixture/source/main.lkj"
"$compiler" build "${headers[@]}" > "$fixture/changed.log" 2>&1
grep -Eq '"build_reused":0' "$fixture/changed.log"
grep -Eq '"front_reused":0' "$fixture/changed.log"
for engine in native interpreter; do
    [[ $("$compiler" run main.lkj --source-dir "$fixture/source" --no-library \
        --store "$fixture/header-store" --store-key "$fixture/key" --engine "$engine") == 43 ]]
done
printf 'ok trait constants: exact owners/types/member spans, idle uses, rename rollback and warm initializer invalidation\n'
# Equal spelling cannot merge primitive and nominal implementation identities.
mkdir "$fixture/collisions"
cp tests/conformance/pass/trait_constant_receiver_identity.lkj "$fixture/collisions/main.lkj"
"$compiler" build main.lkj --source-dir "$fixture/collisions" --no-library --store "$fixture/store" --store-key "$fixture/key" --index \
    -o "$fixture/collision-program" > "$fixture/collision-index.log"
collision_snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/collision-index.log")
[[ "$collision_snapshot" =~ ^[0-9a-f]{64}$ ]]
"$compiler" query main.lkj --source-dir "$fixture/collisions" --no-library --store "$fixture/store" --store-key "$fixture/key" \
    --snapshot "$collision_snapshot" --kind outline --id crate --depth 8 --limit 128 --bytes 100000 > "$fixture/collision-outline.query"
"$compiler" query main.lkj --source-dir "$fixture/collisions" --no-library --store "$fixture/store" --store-key "$fixture/key" \
    --snapshot "$collision_snapshot" --kind uses --id '<crate::u8 as Flag>::VALUE' --limit 128 --bytes 100000 > "$fixture/collision-nominal.query"
"$compiler" query main.lkj --source-dir "$fixture/collisions" --no-library --store "$fixture/store" --store-key "$fixture/key" \
    --snapshot "$collision_snapshot" --kind uses --id '<u8 as Flag>::VALUE' --limit 128 --bytes 100000 > "$fixture/collision-primitive.query"
compare_facts collision-outline <<'COLLISIONS'
"impl:<crate::u8 as Flag>"	level=source-claim	fact=signature	kind=implementation text="impl Flag for u8" bytes=16 complete=true
"<crate::u8 as Flag>::VALUE"	level=source-claim	fact=signature	kind=associated-constant text="const VALUE: i64 = 40;" bytes=22 complete=true
"Flag::VALUE"	level=source-claim	fact=signature	kind=associated-constant text="const VALUE: i64;" bytes=17 complete=true
"impl:<u8 as Flag>"	level=source-claim	fact=signature	kind=implementation text="impl super::Flag for u8" bytes=23 complete=true
"<u8 as Flag>::VALUE"	level=source-claim	fact=signature	kind=associated-constant text="const VALUE: i64 = 2;" bytes=21 complete=true
COLLISIONS
compare_facts collision-nominal <<'COLLISIONS'
"value@probe"	level=source-claim	fact=uses	target="<crate::u8 as Flag>::VALUE" start=286 end=291 kind=associated-constant
COLLISIONS
compare_facts collision-primitive <<'COLLISIONS'
"value@child::probe"	level=source-claim	fact=uses	target="<u8 as Flag>::VALUE" start=238 end=243 kind=associated-constant
COLLISIONS
# Include the qualifier in the independent span check for each receiver.
dd if="$fixture/collisions/main.lkj" bs=1 skip=272 count=19 status=none | cmp - <(printf '%s' '<u8 as Flag>::VALUE')
dd if="$fixture/collisions/main.lkj" bs=1 skip=217 count=26 status=none | cmp - <(printf '%s' '<u8 as super::Flag>::VALUE')
printf 'ok trait constants: distinct nominal/primitive catalog identities and exact receiver uses\n'
