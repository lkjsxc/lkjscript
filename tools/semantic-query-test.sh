#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
compiler=$(realpath "$compiler")
fixture=$(mktemp -d target/semantic-query.XXXXXX)
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
umask 077
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
mod helpers;
use helpers::square as times;
pub fn main() {}
pub fn probe() -> i64 { helpers::identity(times(7)) }
pub fn scalar(value: i64) -> i64 { value + 1 }
pub fn writes(value: i64, mut output: Stdout) -> i64 { output.write(b"x"); value }
SOURCE
cat > "$fixture/source/helpers.lkj" <<'SOURCE'
pub struct Counter { pub value: i64 }
pub enum Color { Red, Blue(i64) }
pub trait Apply { fn apply(&self, value: i64) -> i64; }
impl Apply for Counter { fn apply(&self, value: i64) -> i64 { self.value + value } }
pub fn identity<T>(value: T) -> T { value }
pub fn square(value: i64) -> i64 { value * value }
pub fn record() -> i64 { let value = Counter { value: 9 }; value.value }
SOURCE
fail() { printf 'semantic-query: %s\n' "$*" >&2; exit 1; }
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
build() {
    "$compiler" build "${common[@]}" --index -o "$fixture/program" > "$fixture/build.log" 2>&1 || { cat "$fixture/build.log" >&2; fail build; }
    snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/build.log")
    [[ ${#snapshot} == 64 ]] || fail 'build did not return snapshot identity'
}
query() { "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" "${@:3}"; }
refuse() {
    if "$@" > "$fixture/refused.out" 2> "$fixture/refused.err"; then fail 'hostile query accepted'; fi
    [[ ! -s "$fixture/refused.out" ]] || fail 'refusal leaked a partial answer'
    rg -q 'E_QUERY_' "$fixture/refused.err" || { cat "$fixture/refused.err" >&2; fail 'missing structural refusal'; }
}
build
first=$snapshot
query outline helpers --depth 2 --limit 80 > "$fixture/outline"
rg -q '"helpers::square".*level=source-claim.*fact=signature' "$fixture/outline" || fail outline
query item helpers::square > "$fixture/item"
rg -q 'fn square' "$fixture/item" || fail 'item source'
query uses helpers::square > "$fixture/uses"
rg -q 'kind=import' "$fixture/uses" || fail 'resolved import reference'
query references probe > "$fixture/references"
rg -q 'kind=alias-value' "$fixture/references" || fail 'alias use is not a rename site'
query callees probe > "$fixture/callees"
rg -q 'helpers::square' "$fixture/callees" || fail callees
query callers helpers::square > "$fixture/callers"
rg -q '"probe"' "$fixture/callers" || fail callers
query instances helpers::identity > "$fixture/instances"
rg -q 'arguments="<T=i64>"' "$fixture/instances" || fail 'generic instance arguments'
query type 'scalar::param:value' > "$fixture/type"
rg -q 'value="i64"' "$fixture/type" || fail type
query layout helpers::Counter > "$fixture/layout"
rg -q 'level=checked-ir.*bytes=8 alignment=8' "$fixture/layout" || fail 'checked layout'
query authority scalar > "$fixture/pure"
rg -q 'level=checked-ir.*reachable-effects=""' "$fixture/pure" || fail 'ordinary scalar authority'
query authority writes > "$fixture/effect"
rg -q 'level=checked-ir.*stdout_write' "$fixture/effect" || fail 'ordinary effectful authority'
query impls helpers::Apply > "$fixture/impls"
rg -q 'helpers::Counter as helpers::Apply' "$fixture/impls" || fail implementations
query dependents helpers::square > "$fixture/dependents"
rg -q '"probe"' "$fixture/dependents" || fail dependents
query slice probe > "$fixture/slice"
rg -q 'fn probe' "$fixture/slice" || fail slice
query outline helpers --limit 1 > "$fixture/page"
cursor=$(sed -n 's/.*cursor=\([0-9a-f]*\) facts=.*/\1/p' "$fixture/page")
[[ -n "$cursor" ]] || fail 'missing bounded continuation'
query outline helpers --limit 1 --cursor "$cursor" > "$fixture/next"
[[ $(rg -c 'level=source-claim' "$fixture/next") == 1 ]] || fail 'continuation bound'
refuse query outline scalar --limit 1 --cursor "$cursor"
refuse query outline helpers --cursor "${cursor%?}z"
query meta helpers::square > "$fixture/before"
before_signature=$(sed -n 's/.*signature=\([^ ]*\).*/\1/p' "$fixture/before")
before_body=$(sed -n 's/.*body=\([^ ]*\).*/\1/p' "$fixture/before")
latest=$(find "$fixture/store" -maxdepth 1 -type f -name 'semantic-latest-*' -print -quit)
cp "$latest" "$fixture/old-latest"
mkdir "$fixture/source/helpers"
mv "$fixture/source/helpers.lkj" "$fixture/source/helpers/mod.lkj"
sed -i 's/value \* value/value  *  value/' "$fixture/source/helpers/mod.lkj"
build
[[ "$snapshot" != "$first" ]] || fail 'changed source did not change snapshot'
query meta helpers::square > "$fixture/after"
[[ $(sed -n 's/.*signature=\([^ ]*\).*/\1/p' "$fixture/after") == "$before_signature" ]] || fail 'signature identity changed with physical move/formatting'
[[ $(sed -n 's/.*body=\([^ ]*\).*/\1/p' "$fixture/after") == "$before_body" ]] || fail 'body epoch changed with formatting'
refuse query outline helpers --cursor "$cursor"
mv "$fixture/source" "$fixture/hidden-source"
query authority writes > "$fixture/no-source"
rg -q stdout_write "$fixture/no-source" || fail 'query read source files'
cp "$fixture/old-latest" "$latest"
"$compiler" query "${common[@]}" --kind meta --id crate > "$fixture/replayed"
rg -q "snapshot=$first .*freshness=not-asserted" "$fixture/replayed" || fail 'replayed pointer made a freshness claim'
query authority writes > "$fixture/explicit"
rg -q "snapshot=$snapshot" "$fixture/explicit" || fail 'explicit snapshot followed old pointer'
cp -a "$fixture/store" "$fixture/saved-store"
root_record=$(find "$fixture/store" -maxdepth 1 -type f -name "semantic-root-*-$snapshot" -print -quit)
root_page=$(perl - "$root_record" <<'PERL'
use strict;
use warnings;
open my $file, '<:raw', shift @ARGV or die $!;
local $/;
my $data = <$file>;
my $at = length("lkjscript-store-2\n") + 8 + 64 + 8;
sub field { my $n = unpack('Q<', substr($data, $at, 8)); $at += 8; my $v = substr($data, $at, $n); $at += $n; return $v; }
field(); field();
print field();
PERL
)
page_name=$(basename "$(find "$fixture/store" -maxdepth 1 -type f -name "semantic-page-*-$root_page" -print -quit)")
[[ -n "$page_name" ]] || fail 'root page not found'
for attack in mutate truncate substitute fifo symlink; do
    rm -rf "$fixture/store"
    cp -a "$fixture/saved-store" "$fixture/store"
    target="$fixture/store/$page_name"
    case "$attack" in
        mutate) printf X | dd of="$target" bs=1 seek=40 conv=notrunc status=none ;;
        truncate) truncate -s 40 "$target" ;;
        substitute)
            other=$(find "$fixture/store" -maxdepth 1 -type f -name 'semantic-page-*' ! -name "$page_name" -print -quit)
            cp "$other" "$target" ;;
        fifo) rm "$target"; mkfifo "$target" ;;
        symlink) rm "$target"; ln -s ../old-latest "$target" ;;
    esac
    refuse timeout --kill-after=2s 10s "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind authority --id writes
done
rm -rf "$fixture/store"
cp -a "$fixture/saved-store" "$fixture/store"
truncate -s 40 "$fixture/store/$page_name"
refuse query authority writes
mv "$fixture/hidden-source" "$fixture/source"
build
query authority writes > "$fixture/repaired"
rg -q stdout_write "$fixture/repaired" || fail 'indexed rebuild did not repair the missing page'
printf 'ok semantic-query: identities, source/IR levels, ordinary authority, all question kinds, bounded authenticated cursor, source-free lookup, physical move/formatting, explicit snapshot replay and five index attacks\n'
