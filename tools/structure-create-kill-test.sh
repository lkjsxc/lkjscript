#!/usr/bin/env bash
# Kill mixed create/replace publication; authenticated recovery must choose one complete projection.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d "$PWD/target/compiler-structure-create-kill.XXXXXX")
child=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$child" ]]; then kill -KILL "$child" 2>/dev/null || true; wait "$child" 2>/dev/null || true; fi
    if ((code)); then
        for output in "$fixture/plan" "$fixture/recovery" "$fixture/error"; do
            if [[ -f "$output" ]]; then cat "$output" >&2; fi
        done
    fi
    rm -rf "$fixture"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
mkdir "$fixture/old" "$fixture/new"
head -c 32 /dev/urandom > "$fixture/key"
head -c 32 /dev/urandom > "$fixture/wrong-key"
perl - "$fixture" <<'PERL'
use strict; use warnings; use JSON::PP;
my $root=shift; my $main=''; my @ops;
for my $i (0..15) {
    $main.="mod old$i;\n";
    open my $old,'>:raw',"$root/old/old$i.lkj" or die $!;
    print {$old} '// ',('x'x16384),"\npub fn value() -> i64 { ",$i,"i64 }\n"; close $old;
    push @ops,{op=>'replace-body',id=>"old$i\::value",source=>'{ '.(1000+$i).'i64 }'};
    push @ops,{op=>'add-item',id=>'crate',source=>"mod fresh$i;",
        projection=>{path=>"fresh$i.lkj",source=>'// '.('y'x16384)."\npub fn value() -> i64 { ".(2000+$i)."i64 }\n"}};
}
$main.="pub fn main() -> i64 { old0::value() }\n";
open my $out,'>:raw',"$root/old/main.lkj" or die $!; print {$out} $main; close $out;
push @ops,{op=>'replace-body',id=>'main',source=>'{ old0::value() + fresh0::value() }'};
open my $request,'>:raw',"$root/request.json" or die $!;
print {$request} JSON::PP->new->canonical->encode({version=>1,operations=>\@ops}),"\n";
close $request;
PERL
manifest() {
    (cd "$1"; find . -type f -name '*.lkj' -exec sha256sum {} +) | LC_ALL=C sort > "$2"
}
project="$fixture/source"
common=(main.lkj --source-dir "$project" --no-library --no-store)
cp -a "$fixture/old" "$project"
"$compiler" edit "${common[@]}" --request "$fixture/request.json" --store-key "$fixture/key" > "$fixture/control-plan"
grep -q 'mode=apply' "$fixture/control-plan"
[[ "$("$compiler" run "${common[@]}")" == 3000 ]]
cp "$project/"*.lkj "$fixture/new/"
manifest "$fixture/old" "$fixture/old-manifest"
manifest "$fixture/new" "$fixture/new-manifest"
interrupted=0
committed=0
old=0
new=0
attacked=0
for ((trial=0; trial<32; trial++)); do
    rm -rf "$project"
    cp -a "$fixture/old" "$project"
    "$compiler" edit "${common[@]}" --request "$fixture/request.json" --store-key "$fixture/key" \
        > "$fixture/plan" 2> "$fixture/error" &
    child=$!
    if [[ "$trial" == 0 ]]; then
        kill -KILL "$child" 2>/dev/null || true
    else
        for ((poll=0; poll<200000; poll++)); do
            kill -0 "$child" 2>/dev/null || break
            if [[ -f "$project/.lkjscript-edit.pending" ]]; then
                if ((trial%2==1)) || [[ -f "$project/fresh0.lkj" ]]; then
                    kill -KILL "$child" 2>/dev/null || true
                    interrupted=$((interrupted+1))
                    break
                fi
            fi
        done
    fi
    wait "$child" 2>/dev/null || true
    child=
    had_intent=0
    if [[ -f "$project/.lkjscript-edit.pending" ]]; then
        had_intent=1
        committed=$((committed+1))
        if [[ "$attacked" == 0 ]]; then
            cp "$project/.lkjscript-edit.pending" "$fixture/intent"
            manifest "$project" "$fixture/before"
            status=0
            "$compiler" edit "${common[@]}" --recover --store-key "$fixture/wrong-key" \
                > "$fixture/recovery" 2> "$fixture/error" || status=$?
            [[ "$status" == 1 && ! -s "$fixture/recovery" ]]
            manifest "$project" "$fixture/after"
            cmp "$fixture/before" "$fixture/after"
            size=$(stat -c %s "$project/.lkjscript-edit.pending")
            printf '\377' | dd of="$project/.lkjscript-edit.pending" bs=1 seek="$((size-1))" conv=notrunc status=none
            status=0
            "$compiler" edit "${common[@]}" --recover --store-key "$fixture/key" \
                > "$fixture/recovery" 2> "$fixture/error" || status=$?
            [[ "$status" == 1 && ! -s "$fixture/recovery" ]]
            manifest "$project" "$fixture/after"
            cmp "$fixture/before" "$fixture/after"
            cp "$fixture/intent" "$project/.lkjscript-edit.pending"
            status=0
            "$compiler" check "${common[@]}" > "$fixture/check" 2> "$fixture/error" || status=$?
            [[ "$status" == 1 ]]
            grep -q E_EDIT_RECOVERY "$fixture/error"
            attacked=1
        fi
    fi
    "$compiler" edit "${common[@]}" --recover --store-key "$fixture/key" > "$fixture/recovery" 2> "$fixture/error"
    manifest "$project" "$fixture/after"
    if cmp -s "$fixture/after" "$fixture/new-manifest"; then
        new=$((new+1))
    else
        cmp "$fixture/after" "$fixture/old-manifest"
        [[ "$had_intent" == 0 ]]
        old=$((old+1))
    fi
    [[ ! -e "$project/.lkjscript-edit.pending" ]]
    [[ -z "$(find "$project" -name '*.lkjscript-next' -print)" ]]
done
((interrupted>0 && committed>0 && attacked==1 && old>0 && new>0))
printf 'ok mixed module publication kills: trials=32 interrupted=%d committed=%d old=%d new=%d authenticated-attacks=%d\n' \
    "$interrupted" "$committed" "$old" "$new" "$attacked"
