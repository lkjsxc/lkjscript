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
use helpers::{Counter as CounterAlias, Color as ColorAlias};
pub fn main() {}
pub fn probe() -> i64 { helpers::identity(times(7)) }
pub fn scalar(value: i64) -> i64 { value + 1 }
pub fn shadowed() -> i64 { let before = scalar(1); { let scalar = before; let _ = scalar; } scalar(before) }
pub fn alias_type(value: CounterAlias) -> i64 { value.value }

pub fn writes(value: i64, mut output: Stdout) -> i64 { output.write(b"x"); value }
mod same_alias {
    use crate::helpers::square as square;
    pub fn run() -> i64 { square(3) }
}
mod direct_import {
    use crate::helpers::square;
    pub fn run() -> i64 { square(3) }
}
mod exports { pub use crate::helpers::square as square; }
mod wildcard {
    use crate::exports::*;
    pub fn run() -> i64 { square(3) }
}
mod same_type {
    use crate::helpers::Counter as Counter;
    pub fn run(value: Counter) -> i64 { value.value }
}
mod module_alias {
    use crate::helpers as helpers;
    pub fn run() -> i64 { helpers::square(3) }
}
mod grouped {
    use crate::{helpers::{self as helpers, Color as Color}, exports::square};
    pub fn run(value: Color) -> i64 { helpers::square(square(3)) }
}
mod visibility {
    pub(in crate::visibility) fn hidden() -> i64 { 1 }
}
SOURCE
cat > "$fixture/source/helpers.lkj" <<'SOURCE'
pub struct Counter { pub value: i64 }
pub enum Color { Red, Blue(i64) }
pub trait Apply { fn apply(&self, value: i64) -> i64; }
impl Apply for Counter { fn apply(&self, value: i64) -> i64 { self.value + value } }
pub fn identity<T>(value: T) -> T { value }
pub fn square(value: i64) -> i64 { value * value }
pub fn record() -> i64 { let value = Counter { value: 9 }; value.value }
pub fn applied(value: Counter) -> i64 { value.apply(4) }
pub fn named(value: i64) -> Counter { Counter { value } }
pub fn destructure(value: Counter) -> i64 { let Counter { value: observed } = value; observed }
pub fn shorthand(value: Counter) -> i64 { let Counter { value } = value; value }
pub fn patterned(value: Color) -> i64 { match value { Color::Red => 0, Color::Blue(n) => n } }
pub fn blue(value: i64) -> Color { Color::Blue(value) }
pub fn local_generic<T>(value: T) -> T { let held = value; held }

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
mkdir "$fixture/small-store"
refuse "$compiler" build main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/small-store" --store-key "$fixture/key" --store-bytes 32768 --index -o "$fixture/small-program"
refuse "$compiler" query main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/small-store" --store-key "$fixture/key" --kind outline
query outline helpers --depth 2 --limit 80 > "$fixture/outline"
rg -q '"helpers::square".*level=source-claim.*fact=signature' "$fixture/outline" || fail outline
query item helpers::square > "$fixture/item"
rg -q 'fn square' "$fixture/item" || fail 'item source'
query uses helpers::square > "$fixture/uses"
rg -q 'kind=import' "$fixture/uses" || fail 'resolved import reference'
query references probe > "$fixture/references"
rg -q 'kind=alias-value' "$fixture/references" || fail 'alias use is not a rename site'
query references shadowed --limit 80 > "$fixture/shadowed"
[[ $(rg -c '"scalar".*kind=value' "$fixture/shadowed") == 2 ]] || fail 'lexical scope lost a free call'
rg -q 'binding:scalar:0.*kind=local-value' "$fixture/shadowed" || fail 'local binding target'
if rg -q 'kind=lexical-name' "$fixture/shadowed"; then fail 'typed local kept lexical ambiguity'; fi
query references alias_type > "$fixture/alias-type"
rg -q 'helpers::Counter.*kind=alias-type' "$fixture/alias-type" || fail 'resolved type alias use'
query uses 'helpers::Counter::field:value' --limit 80 > "$fixture/field-uses"
for kind in field constructor-field field-shorthand pattern-field pattern-field-shorthand; do
    rg -q "kind=$kind( |$)" "$fixture/field-uses" || fail "missing $kind reference"
done
query uses 'helpers::Color::variant:Blue' --limit 80 > "$fixture/variant-uses"
rg -q 'kind=pattern' "$fixture/variant-uses" || fail 'variant pattern target'
rg -q 'kind=value' "$fixture/variant-uses" || fail 'variant constructor target'
query references helpers::applied > "$fixture/method"
rg -q 'Counter as helpers::Apply>::apply.*kind=method' "$fixture/method" || fail 'resolved source method'
query outline helpers::local_generic --limit 80 > "$fixture/local-outline"
held=$(sed -n 's/^"\([^"]*::binding:held:0\)".*/\1/p' "$fixture/local-outline" | head -n 1)
[[ -n "$held" ]] || fail 'generic binding identity'
query type "$held" > "$fixture/generic-binding"
rg -q 'fact=inferred-type.*value="T"' "$fixture/generic-binding" || fail 'generic definition type'

query references probe > "$fixture/qualified"
rg -q '"helpers".*kind=qualification' "$fixture/qualified" || fail 'module qualification omitted'
query references helpers::patterned > "$fixture/qualified-pattern"
rg -q '"helpers::Color".*kind=qualification' "$fixture/qualified-pattern" || fail 'pattern type qualification omitted'
query references same_alias::run > "$fixture/same-alias"
rg -q '"helpers::square".*kind=alias-value' "$fixture/same-alias" || fail 'same-spelling explicit alias lost'
query references direct_import::run > "$fixture/direct-import"
rg -q '"helpers::square".*kind=value' "$fixture/direct-import" || fail 'ordinary import became an alias'
query references wildcard::run > "$fixture/wildcard"
rg -q '"helpers::square".*kind=alias-value' "$fixture/wildcard" || fail 'reexport/glob alias provenance lost'
query references same_type::run > "$fixture/same-type"
rg -q '"helpers::Counter".*kind=alias-type' "$fixture/same-type" || fail 'same-spelling type alias lost'
query references module_alias::run > "$fixture/module-alias"
rg -q '"helpers".*kind=alias-qualification' "$fixture/module-alias" || fail 'module alias provenance lost'
rg -q '"helpers::square".*kind=value' "$fixture/module-alias" || fail 'owner alias changed member spelling'
query references grouped::run > "$fixture/grouped"
rg -q '"helpers".*kind=alias-qualification' "$fixture/grouped" || fail 'grouped self alias lost'
rg -q '"helpers::square".*kind=alias-value' "$fixture/grouped" || fail 'grouped reexport alias lost'
query uses helpers::square --limit 120 > "$fixture/reexport-uses"
rg -q 'kind=alias-import' "$fixture/reexport-uses" || fail 'reexport import treated as original declaration spelling'
query references visibility::hidden > "$fixture/visibility"
rg -q 'kind=visibility-qualification.*claim=partial' "$fixture/visibility" || fail 'visibility qualification gap omitted'

query outline helpers::record --limit 80 > "$fixture/nominal-outline"
record_value=$(sed -n 's/^"\([^"]*::binding:value:0\)".*/\1/p' "$fixture/nominal-outline" | head -n 1)
[[ -n "$record_value" ]] || fail 'nominal binding identity'
query type "$record_value" > "$fixture/nominal-binding"
rg -q 'fact=inferred-type.*value="helpers::Counter"' "$fixture/nominal-binding" || fail 'nominal type was not canonically qualified'

query callees probe > "$fixture/callees"
rg -q 'helpers::square' "$fixture/callees" || fail callees
query callers helpers::square > "$fixture/callers"
rg -q '"probe"' "$fixture/callers" || fail callers
query instances helpers::identity > "$fixture/instances"
rg -q 'arguments="<T=i64>"' "$fixture/instances" || fail 'generic instance arguments'
rg -q 'native-bytes=[0-9].*native-size=producer-measurement' "$fixture/instances" || fail 'native instance bytes'
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
query authority scalar --limit 1 > "$fixture/one-fact"
end_cursor=$(sed -n 's/.*cursor=\([0-9a-f]*\) facts=.*/\1/p' "$fixture/one-fact")
[[ -n "$end_cursor" ]] || fail 'missing final-page continuation'
refuse query authority scalar --limit 1 --bytes 1 --cursor "$end_cursor"
query authority scalar --limit 1 --bytes 2048 --cursor "$end_cursor" > "$fixture/finished"
[[ $(wc -c < "$fixture/finished") -le 2048 ]] || fail 'protocol exceeded byte bound'
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
