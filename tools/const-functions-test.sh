#!/usr/bin/env bash
# Const source qualification survives questions, cached builds and checked edits.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
mkdir -p target
fixture=$(mktemp -d "$PWD/target/const-functions.XXXXXX")
cleanup() {
    status=$?
    if ((status)); then
        cat "$fixture/source/main.lkj" >&2
        for file in "$fixture"/*.log "$fixture"/*type "$fixture"/references; do
            [[ ! -f "$file" ]] || cat "$file" >&2
        done
    fi
    rm -rf "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
const fn make(value: i64) -> i64 { value + 1i64 }
const fn answer() -> i64 { make(40i64) }
fn ordinary(value: i64) -> i64 { value + 2i64 }
fn main() -> i64 { answer() }
SOURCE
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
publish() {
    "$compiler" build "${common[@]}" --index --stats -o "$fixture/program" > "$fixture/$1.log" 2>&1
    snapshot=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/$1.log")
    [[ ${#snapshot} == 64 ]]
}
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --limit 128 --bytes 100000
}
execute() {
    for engine in native interpreter; do
        [[ $("$compiler" run "${common[@]}" --engine "$engine") == "$1" ]]
    done
}
publish cold
query type make > "$fixture/original-type"
grep -Fq 'const fn(i64) -> i64' "$fixture/original-type"
query references answer > "$fixture/references"
grep -Fq 'make' "$fixture/references"
execute 41
publish warm
grep -Eq '"build_reused":1' "$fixture/warm.log"
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"make","name":"increment"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rename.log"
grep -Fq 'const fn increment' "$fixture/source/main.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"increment","source":"{ value + 2i64 }"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/edit.log"
publish changed
query type increment > "$fixture/changed-type"
grep -Fq 'const fn(i64) -> i64' "$fixture/changed-type"
execute 42
before=$(sha256sum "$fixture/source/main.lkj")
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"increment","source":"{ ordinary(value) }"}]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/rejected.log" 2>&1; then
    echo 'checked edit published a non-const call in const fn' >&2; exit 1
fi
grep -Fq 'const fn cannot call a non-const function' "$fixture/rejected.log"
[[ $(sha256sum "$fixture/source/main.lkj") == "$before" ]]
printf '%s\n' '{"version":1,"operations":[{"op":"replace-item","id":"increment","source":"fn increment(value: i64) -> i64 { value + 2i64 }"}]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/dequalified.log" 2>&1; then
    echo 'checked edit removed const from a const callee' >&2; exit 1
fi
grep -Fq 'const fn cannot call a non-const function' "$fixture/dequalified.log"
[[ $(sha256sum "$fixture/source/main.lkj") == "$before" ]]
publish final
execute 42

# Indexed builds collect complete references and intentionally skip frontend
# checkpoints. A separate nonindexed store exercises actual header invalidation.
mkdir "$fixture/header-store"
header_build() {
    "$compiler" build main.lkj --source-dir "$fixture/source" --no-library \
        --store "$fixture/header-store" --store-key "$fixture/key" --stats \
        -o "$fixture/header-program" > "$fixture/$1.log" 2>&1
}
header_build header-base
header_build header-warm
grep -Eq '"build_reused":1' "$fixture/header-warm.log"
query type ordinary > "$fixture/header-before-type"
grep -Fq 'value="fn(i64) -> i64"' "$fixture/header-before-type"
cp "$fixture/source/main.lkj" "$fixture/header-before-source"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-item","id":"ordinary","source":"const fn ordinary(value: i64) -> i64 { value + 2i64 }"}]}' > "$fixture/request"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/header-edit.log"
perl - "$fixture/header-before-source" "$fixture/source/main.lkj" <<'SOURCE_CHECK'
use strict;
use warnings;
sub read_file { open my $input, '<:raw', $_[0] or die $!; local $/; return <$input>; }
my $expected = read_file($ARGV[0]);
my $count = ($expected =~ s/^fn ordinary/const fn ordinary/mg);
die "expected one ordinary header\n" unless $count == 1;
die "qualifier edit changed other source bytes\n" unless read_file($ARGV[1]) eq $expected;
SOURCE_CHECK
header_build header-qualified
grep -Eq '"build_reused":0' "$fixture/header-qualified.log"
grep -Eq '"front_reused":0' "$fixture/header-qualified.log"
publish qualifier-meaning
query type ordinary > "$fixture/header-after-type"
grep -Fq 'value="const fn(i64) -> i64"' "$fixture/header-after-type"
header_build header-qualified-warm
grep -Eq '"build_reused":1' "$fixture/header-qualified-warm.log"
execute 42
test ! -e "$fixture/source/.lkjscript-edit.pending"
printf 'ok const fn: qualified meaning, call references, checked rename/body edit, rejected nonconst edits, raw-header cache invalidation, warm cache and both engines\n'
