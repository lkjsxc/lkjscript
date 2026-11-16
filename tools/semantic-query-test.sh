#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
compiler=$(realpath "$compiler")
fixture=$(mktemp -d target/semantic-query.XXXXXX)
finish() {
    local result=$?
    trap - EXIT HUP INT TERM
    if [[ -n "${LKJSCRIPT_QUERY_EVIDENCE:-}" ]]; then
        mkdir -p "$LKJSCRIPT_QUERY_EVIDENCE"
        while IFS= read -r -d '' evidence; do
            [[ $(basename "$evidence") == key ]] && continue
            if [[ ! -s "$evidence" ]] || LC_ALL=C grep -Iq '' "$evidence"; then
                cp "$evidence" "$LKJSCRIPT_QUERY_EVIDENCE/"
            fi
        done < <(find "$fixture" -maxdepth 1 -type f -print0)
    fi
    rm -rf "$fixture"
    exit "$result"
}
trap finish EXIT HUP INT TERM
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
pub fn borrowed_type<'a>(value: &'a mut helpers::Counter) -> i64 { value.value }
pub fn double_borrowed(value: &&helpers::Counter) -> i64 { value.value }
pub fn borrowed_alias(value: &CounterAlias) -> i64 { value.value }

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
mod self_function {
    use crate::helpers::square::{self};
    pub fn run() -> i64 { square(3) }
}
mod self_terminal {
    use crate::helpers::{square::self};
    pub fn run() -> i64 { square(3) }
}
mod self_transparent {
    use crate::helpers::square::{{self}};
    pub fn run() -> i64 { square(3) }
}
mod self_sibling {
    use crate::{helpers::{Counter as Other}, helpers::square::{self}};
    pub fn run() -> i64 { square(3) }
}
mod self_repeated {
    use crate::helpers::square::self::self;
    pub fn run() -> i64 { square(3) }
}
mod self_alias {
    use crate::helpers::{square::{self as renamed}};
    pub fn run() -> i64 { renamed(3) }
}
mod self_reexport {
    use crate::exports::square::{self};
    pub fn run() -> i64 { square(3) }
}
pub fn namespace_probe() -> i64 { let mut value = 1i64; std::mem::replace(&mut value, 2i64) }
pub fn turbofish_probe() -> i64 { helpers::identity::<i64>(1i64) }
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
pub fn anonymous(_: u8, named: i64, _: bool) -> i64 { named }
pub fn anonymous_reference(_: i64, _: i64) -> i64 { _ }
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
    grep -Eq 'E_QUERY_' "$fixture/refused.err" || { cat "$fixture/refused.err" >&2; fail 'missing structural refusal'; }
    { printf 'refused:'; printf ' %q' "$@"; printf '\n'; cat "$fixture/refused.err"; } >> "$fixture/refusals.log"
}
flip_byte() {
    perl - "$1" "$2" <<'PERL'
use strict;
use warnings;
my ($path, $offset) = @ARGV;
open my $file, '+<:raw', $path or die $!;
seek($file, $offset, 0) or die $!;
read($file, my $byte, 1) == 1 or die "missing mutation byte\n";
seek($file, $offset, 0) or die $!;
print {$file} chr(ord($byte) ^ 1) or die $!;
close $file or die $!;
PERL
}
build
first=$snapshot
# A protocol-compatible reader can inspect another authenticated producing
# generation, while admission and image reuse remain exact-producer claims.
producer_a=$(sha256sum "$compiler" | cut -d' ' -f1)
producer_a_root=$(find "$fixture/store" -maxdepth 1 -type f -name "semantic-container-v3-*-$first" -print -quit)
[[ -n "$producer_a_root" ]] || fail 'producer A snapshot root missing'
producer_a_slot=$(basename "$producer_a_root" | cut -d- -f4)
reader="$fixture/another-reader"
cp "$compiler" "$reader"
printf 'query reader protocol generation test\n' >> "$reader"
producer_b=$(sha256sum "$reader" | cut -d' ' -f1)
[[ "$producer_a" != "$producer_b" ]] || fail 'reader producer did not change'
query meta crate > "$fixture/producer-a"
grep -Eq "version=3 producer=$producer_a validation=authenticated-producing-compiler current-check=not-performed" "$fixture/producer-a" || fail 'producer attestation label'
"$reader" query "${common[@]}" --snapshot "$first" --kind meta --id crate > "$fixture/cross-reader"
cmp "$fixture/producer-a" "$fixture/cross-reader" || fail 'cross-reader changed the pinned answer'
refuse "$reader" query "${common[@]}" --kind meta --id crate
refuse "$reader" query main.lkj --source-dir "$fixture/not-selected" --no-library --store "$fixture/store" --store-key "$fixture/key" --snapshot "$first" --kind meta --id crate
query outline helpers --limit 1 > "$fixture/producer-a-cursor"
producer_a_cursor=$(sed -n 's/.*cursor=\([0-9a-f]*\) facts=.*/\1/p' "$fixture/producer-a-cursor")
[[ -n "$producer_a_cursor" ]] || fail 'missing producer A cursor'
"$reader" query "${common[@]}" --snapshot "$first" --kind outline --id helpers --limit 1 --cursor "$producer_a_cursor" > "$fixture/cross-reader-cursor"
"$reader" build "${common[@]}" --index --stats -o "$fixture/producer-b-program" > "$fixture/producer-b-build.log" 2>&1 || { cat "$fixture/producer-b-build.log" >&2; fail 'producer B build'; }
second=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/producer-b-build.log")
[[ ${#second} == 64 && "$second" != "$first" ]] || fail 'snapshot identity did not bind producer'
for statistic in checked_reused build_reused; do
    count=$(sed -n "s/.*\"$statistic\":\([0-9][0-9]*\).*/\1/p" "$fixture/producer-b-build.log")
    [[ "$count" == 0 ]] || fail "producer B reused producer A $statistic"
done
"$compiler" query "${common[@]}" --snapshot "$second" --kind meta --id crate > "$fixture/producer-b"
grep -Eq "version=3 producer=$producer_b validation=authenticated-producing-compiler current-check=not-performed" "$fixture/producer-b" || fail 'reader substituted its own producer'
for fact in source checked-ir; do
    a=$(sed -n "s/.* $fact=\([^ ]*\) .*/\1/p" "$fixture/producer-a" | head -n 1)
    b=$(sed -n "s/.* $fact=\([^ ]*\) .*/\1/p" "$fixture/producer-b" | head -n 1)
    [[ -n "$a" && "$a" == "$b" ]] || fail "unchanged cross-producer $fact mismatch"
done
tail -c 32 "$fixture/store/semantic-locator-v3-$first" > "$fixture/root-a"
tail -c 32 "$fixture/store/semantic-locator-v3-$second" > "$fixture/root-b"
cmp "$fixture/root-a" "$fixture/root-b" || fail 'unchanged cross-producer fact root mismatch'
refuse "$reader" query "${common[@]}" --snapshot "$second" --kind outline --id helpers --limit 1 --cursor "$producer_a_cursor"
mkdir "$fixture/small-store"
refuse "$compiler" build main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/small-store" --store-key "$fixture/key" --store-bytes 32768 --index -o "$fixture/small-program"
refuse "$compiler" query main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/small-store" --store-key "$fixture/key" --kind outline
query outline helpers --depth 2 --limit 80 > "$fixture/outline"
grep -Eq '"helpers::square".*level=source-claim.*fact=signature' "$fixture/outline" || fail outline
query item helpers::square > "$fixture/item"
grep -Eq 'fn square' "$fixture/item" || fail 'item source'
query uses helpers::square > "$fixture/uses"
grep -Eq 'kind=import' "$fixture/uses" || fail 'resolved import reference'
query references probe > "$fixture/references"
grep -Eq 'kind=alias-value' "$fixture/references" || fail 'alias use is not a rename site'
query references shadowed --limit 80 > "$fixture/shadowed"
[[ $(grep -Ec '"scalar".*kind=value' "$fixture/shadowed") == 2 ]] || fail 'lexical scope lost a free call'
grep -Eq 'binding:scalar:0.*kind=local-value' "$fixture/shadowed" || fail 'local binding target'
if grep -Eq 'kind=lexical-name' "$fixture/shadowed"; then fail 'typed local kept lexical ambiguity'; fi
query references alias_type > "$fixture/alias-type"
grep -Eq 'helpers::Counter.*kind=alias-type' "$fixture/alias-type" || fail 'resolved type alias use'
for identity in borrowed_type double_borrowed; do
    query references "$identity" > "$fixture/borrowed-type"
    grep -Eq '"helpers::Counter".*kind=type' "$fixture/borrowed-type" || fail 'borrowed nominal type site omitted'
    grep -Eq '"helpers".*kind=qualification' "$fixture/borrowed-type" || fail 'borrowed qualification omitted'
    if grep -Eq 'kind=implicit-source' "$fixture/borrowed-type"; then fail 'reference prefix confused a type path'; fi
done
query references borrowed_alias > "$fixture/borrowed-alias"
grep -Eq '"helpers::Counter".*kind=alias-type' "$fixture/borrowed-alias" || fail 'borrowed alias site omitted'
query uses 'helpers::Counter::field:value' --limit 80 > "$fixture/field-uses"
for kind in field constructor-field field-shorthand pattern-field pattern-field-shorthand; do
    grep -Eq "kind=$kind( |$)" "$fixture/field-uses" || fail "missing $kind reference"
done
query uses 'helpers::Color::variant:Blue' --limit 80 > "$fixture/variant-uses"
grep -Eq 'kind=pattern' "$fixture/variant-uses" || fail 'variant pattern target'
grep -Eq 'kind=value' "$fixture/variant-uses" || fail 'variant constructor target'
query references helpers::applied > "$fixture/method"
grep -Eq 'Counter as helpers::Apply>::apply.*kind=method' "$fixture/method" || fail 'resolved source method'
query outline helpers::local_generic --limit 80 > "$fixture/local-outline"
held=$(sed -n 's/^"\([^"]*::binding:held:0\)".*/\1/p' "$fixture/local-outline" | head -n 1)
[[ -n "$held" ]] || fail 'generic binding identity'
query type "$held" > "$fixture/generic-binding"
grep -Eq 'fact=inferred-type.*value="T"' "$fixture/generic-binding" || fail 'generic definition type'

query references probe > "$fixture/qualified"
grep -Eq '"helpers".*kind=qualification' "$fixture/qualified" || fail 'module qualification omitted'
query references helpers::patterned > "$fixture/qualified-pattern"
grep -Eq '"helpers::Color".*kind=qualification' "$fixture/qualified-pattern" || fail 'pattern type qualification omitted'
query references same_alias::run > "$fixture/same-alias"
grep -Eq '"helpers::square".*kind=alias-value' "$fixture/same-alias" || fail 'same-spelling explicit alias lost'
query references direct_import::run > "$fixture/direct-import"
grep -Eq '"helpers::square".*kind=value' "$fixture/direct-import" || fail 'ordinary import became an alias'
query references wildcard::run > "$fixture/wildcard"
grep -Eq '"helpers::square".*kind=alias-value' "$fixture/wildcard" || fail 'reexport/glob alias provenance lost'
query references same_type::run > "$fixture/same-type"
grep -Eq '"helpers::Counter".*kind=alias-type' "$fixture/same-type" || fail 'same-spelling type alias lost'
query references module_alias::run > "$fixture/module-alias"
grep -Eq '"helpers".*kind=alias-qualification' "$fixture/module-alias" || fail 'module alias provenance lost'
grep -Eq '"helpers::square".*kind=value' "$fixture/module-alias" || fail 'owner alias changed member spelling'
query references grouped::run > "$fixture/grouped"
grep -Eq '"helpers".*kind=alias-qualification' "$fixture/grouped" || fail 'grouped self alias lost'
grep -Eq '"helpers::square".*kind=alias-value' "$fixture/grouped" || fail 'grouped reexport alias lost'
query uses helpers::square --limit 120 > "$fixture/reexport-uses"
grep -Eq 'kind=alias-import' "$fixture/reexport-uses" || fail 'reexport import treated as original declaration spelling'
for identity in self_function self_terminal self_transparent self_sibling self_repeated self_alias; do
    query references "$identity" > "$fixture/self-import"
    grep -Eq '^"helpers::square".*kind=import( |$)' "$fixture/self-import" || fail "missing exact self import: $identity"
    if grep -Eq 'unresolved-reference' "$fixture/self-import"; then fail "self import retained a false namespace gap: $identity"; fi
    perl - "$fixture/source/main.lkj" "$fixture/self-import" <<'PERL'
use strict;
use warnings;
open my $source_file, '<:raw', shift @ARGV or die $!;
my $source = do { local $/; <$source_file> };
open my $facts, '<:raw', shift @ARGV or die $!;
my $found = 0;
while (<$facts>) {
    next unless /^"helpers::square".* start=(\d+) end=(\d+) kind=import(?: |$)/;
    die "wrong inherited import span\n" unless substr($source, $1, $2 - $1) eq 'square';
    ++$found;
}
die "missing inherited import span\n" unless $found == 1;
PERL
done
query references self_alias::run > "$fixture/self-alias-use"
grep -Eq '^"helpers::square".*kind=alias-value' "$fixture/self-alias-use" || fail 'self leaf alias provenance lost'
query references self_reexport > "$fixture/self-reexport"
grep -Eq '^"helpers::square".*kind=alias-import' "$fixture/self-reexport" || fail 'self reexport alias provenance lost'
if grep -Eq 'unresolved-reference' "$fixture/self-reexport"; then fail 'self reexport retained a false namespace gap'; fi
query references namespace_probe > "$fixture/namespace-probe"
grep -Eq 'kind=unresolved-namespace-qualification' "$fixture/namespace-probe" || fail 'exact nonterminal namespace role missing'
query references turbofish_probe > "$fixture/turbofish-probe"
grep -Eq '^"helpers::identity".*kind=value' "$fixture/turbofish-probe" || fail 'turbofish terminal value binding missing'
if grep -Eq 'unresolved-namespace-qualification' "$fixture/turbofish-probe"; then fail 'turbofish terminal became a namespace'; fi

query references visibility::hidden > "$fixture/visibility"
grep -Eq 'kind=visibility-qualification.*claim=partial' "$fixture/visibility" || fail 'visibility qualification gap omitted'

query outline helpers::record --limit 80 > "$fixture/nominal-outline"
record_value=$(sed -n 's/^"\([^"]*::binding:value:0\)".*/\1/p' "$fixture/nominal-outline" | head -n 1)
[[ -n "$record_value" ]] || fail 'nominal binding identity'
query type "$record_value" > "$fixture/nominal-binding"
grep -Eq 'fact=inferred-type.*value="helpers::Counter"' "$fixture/nominal-binding" || fail 'nominal type was not canonically qualified'

query callees probe > "$fixture/callees"
grep -Eq 'helpers::square' "$fixture/callees" || fail callees
query callers helpers::square > "$fixture/callers"
grep -Eq '"probe"' "$fixture/callers" || fail callers
query instances helpers::identity > "$fixture/instances"
grep -Eq 'arguments="<T=i64>"' "$fixture/instances" || fail 'generic instance arguments'
grep -Eq 'native-bytes=[0-9].*native-size=producer-measurement' "$fixture/instances" || fail 'native instance bytes'
query type 'scalar::param:value' > "$fixture/type"
grep -Eq 'value="i64"' "$fixture/type" || fail type
query outline helpers::anonymous --limit 8 > "$fixture/anonymous-outline"
for parameter in 0 named 2; do
    grep -Fq "\"helpers::anonymous::param:$parameter\"" "$fixture/anonymous-outline" || fail 'anonymous parameter identity missing'
done
query type helpers::anonymous::param:0 > "$fixture/anonymous-first"
grep -Eq 'value="u8"' "$fixture/anonymous-first" || fail 'first anonymous parameter type'
query type helpers::anonymous::param:2 > "$fixture/anonymous-last"
grep -Eq 'value="bool"' "$fixture/anonymous-last" || fail 'last anonymous parameter type'
query references helpers::anonymous > "$fixture/anonymous-named"
grep -Eq '"helpers::anonymous::param:named".*kind=local-value' "$fixture/anonymous-named" || fail 'named parameter identity changed'
query references helpers::anonymous_reference > "$fixture/anonymous-reference"
grep -Eq '"helpers::anonymous_reference::param:1".*kind=local-value' "$fixture/anonymous-reference" || fail 'anonymous reference lost its exact parameter position'
refuse query type helpers::anonymous::param:_
refuse query type helpers::anonymous::param:1
query layout helpers::Counter > "$fixture/layout"
grep -Eq 'level=checked-ir.*bytes=8 alignment=8' "$fixture/layout" || fail 'checked layout'
query authority scalar > "$fixture/pure"
grep -Eq 'level=checked-ir.*reachable-effects=""' "$fixture/pure" || fail 'ordinary scalar authority'
query authority writes > "$fixture/effect"
grep -Eq 'level=checked-ir.*stdout_write' "$fixture/effect" || fail 'ordinary effectful authority'
query impls helpers::Apply > "$fixture/impls"
grep -Eq 'helpers::Counter as helpers::Apply' "$fixture/impls" || fail implementations
query dependents helpers::square > "$fixture/dependents"
grep -Eq '"probe"' "$fixture/dependents" || fail dependents
query slice probe > "$fixture/slice"
grep -Eq 'fn probe' "$fixture/slice" || fail slice
query outline helpers --limit 1 > "$fixture/page"
cursor=$(sed -n 's/.*cursor=\([0-9a-f]*\) facts=.*/\1/p' "$fixture/page")
[[ -n "$cursor" ]] || fail 'missing bounded continuation'
query outline helpers --limit 1 --cursor "$cursor" > "$fixture/next"
[[ $(grep -Ec 'level=source-claim' "$fixture/next") == 1 ]] || fail 'continuation bound'
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
latest=$(find "$fixture/store" -maxdepth 1 -type f -name "semantic-latest-v3-$producer_a_slot-*" -print -quit)
cp "$latest" "$fixture/old-latest"
mkdir "$fixture/source/helpers"
mv "$fixture/source/helpers.lkj" "$fixture/source/helpers/mod.lkj"
sed -i 's/value \* value/value  *  value/' "$fixture/source/helpers/mod.lkj"
build
[[ "$snapshot" != "$first" ]] || fail 'changed source did not change snapshot'
query meta helpers::square > "$fixture/after"
[[ $(sed -n 's/.*signature=\([^ ]*\).*/\1/p' "$fixture/after") == "$before_signature" ]] || fail 'signature identity changed with physical move/formatting'
[[ $(sed -n 's/.*body=\([^ ]*\).*/\1/p' "$fixture/after") == "$before_body" ]] || fail 'body epoch changed with formatting'
# Cached indexed builds must repair a missing or corrupt locator even when
# the immutable snapshot root and complete image proof are still present.
repair_snapshot=$snapshot
for attack in missing corrupt; do
    locator="$fixture/store/semantic-locator-v3-$snapshot"
    if [[ "$attack" == missing ]]; then rm "$locator"; else flip_byte "$locator" 98; fi
    refuse query authority writes
    build
    [[ "$snapshot" == "$repair_snapshot" ]] || fail 'unchanged locator repair changed the snapshot'
    query authority writes > "$fixture/locator-repaired"
    grep -Eq stdout_write "$fixture/locator-repaired" || fail 'indexed build did not repair its locator'
done
refuse query outline helpers --cursor "$cursor"
mv "$fixture/source" "$fixture/hidden-source"
query authority writes > "$fixture/no-source"
grep -Eq stdout_write "$fixture/no-source" || fail 'query read source files'
cp "$fixture/old-latest" "$latest"
"$compiler" query "${common[@]}" --kind meta --id crate > "$fixture/replayed"
grep -Eq "snapshot=$first .*freshness=not-asserted" "$fixture/replayed" || fail 'replayed pointer made a freshness claim'
query authority writes > "$fixture/explicit"
grep -Eq "snapshot=$snapshot" "$fixture/explicit" || fail 'explicit snapshot followed old pointer'
cp -a "$fixture/store" "$fixture/saved-store"
root_record=$(find "$fixture/store" -maxdepth 1 -type f -name "semantic-container-v3-*-$snapshot" -print -quit)
read -r root_page root_offset root_length container_bytes < <(perl - "$root_record" <<'PERL'
use strict;
use warnings;
open my $file, '<:raw', shift @ARGV or die $!;
read($file, my $data, 490) == 490 or die "short container manifest\n";
my $at = 98;
sub field { my $n = unpack('Q<', substr($data, $at, 8)); $at += 8; my $v = substr($data, $at, $n); $at += $n; return $v; }
field(); field(); field();
my $root = field(); field();
my ($facts, $offset, $length, $total) = unpack('Q<4', substr($data, $at, 32));
die "manifest framing\n" unless $at + 32 == 490 && $offset + $length == $total;
print "$root $offset $length $total\n";
PERL
)
page_name=$(basename "$root_record")
[[ "$root_offset" -ge 490 && "$root_length" -ge 122 ]] || fail 'root extent not found'
locator_name="semantic-locator-v3-$snapshot"
[[ -f "$fixture/store/$locator_name" ]] || fail 'published snapshot has no locator'
for attack in producer selector root truncate substitute missing fifo symlink; do
    rm -rf "$fixture/store"
    cp -a "$fixture/saved-store" "$fixture/store"
    target="$fixture/store/$locator_name"
    case "$attack" in
        producer) flip_byte "$target" 98 ;;
        selector) flip_byte "$target" 130 ;;
        root) flip_byte "$target" 162 ;;
        truncate) truncate -s 40 "$target" ;;
        substitute) cp "$fixture/store/semantic-locator-v3-$first" "$target" ;;
        missing) rm "$target" ;;
        fifo) rm "$target"; mkfifo "$target" ;;
        symlink) rm "$target"; ln -s ../old-latest "$target" ;;
    esac
    refuse timeout --kill-after=2s 10s "$reader" query "${common[@]}" --snapshot "$snapshot" --kind authority --id writes
done
rm -rf "$fixture/store"
cp -a "$fixture/saved-store" "$fixture/store"
for attack in manifest mutate truncate append substitute fifo symlink; do
    rm -rf "$fixture/store"
    cp -a "$fixture/saved-store" "$fixture/store"
    target="$fixture/store/$page_name"
    case "$attack" in
        manifest) flip_byte "$target" 40 ;;
        mutate) flip_byte "$target" "$((root_offset + 40))" ;;
        truncate) truncate -s "$((container_bytes - 1))" "$target" ;;
        append) printf x >> "$target" ;;
        substitute)
            other=$(find "$fixture/store" -maxdepth 1 -type f -name 'semantic-container-v3-*' ! -name "$page_name" -print -quit)
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
grep -Eq stdout_write "$fixture/repaired" || fail 'indexed rebuild did not repair the missing page'
# Library definitions remain undemanded: lexical evidence must not invent types.
mkdir "$fixture/untyped" "$fixture/untyped-library" "$fixture/untyped-store"
cat > "$fixture/untyped/main.lkj" <<'SOURCE'
fn main() -> i64 { 0i64 }
SOURCE
cat > "$fixture/untyped-library/lib.lkj" <<'SOURCE'
pub mod scope {
    pub fn target() -> i64 { 7i64 }
    pub fn unshadowed(target: i64) -> i64 { target }
    pub fn top_local() -> i64 { let target = 3i64; target }
    pub fn shadowed(target: i64) -> i64 { let target = target + 1i64; target }
    pub fn before() -> i64 { let first = target(); let target = 1i64; first + target }
    pub fn initializer() -> i64 { let target = target(); target }
    pub fn nested_block() -> i64 { { let target = 1i64; let _ = target; } target() }
    pub fn closure_shadow(target: i64) -> i64 { (|target: i64| -> i64 { target })(1i64) }
    pub fn outer(target: i64) -> i64 { fn inner() -> i64 { target() } inner() + target }
    pub const SOURCE: i64 = 9i64;
    pub fn item_boundary(SOURCE: i64) -> i64 { const N: i64 = SOURCE; N + SOURCE }
}
SOURCE
untyped_common=(main.lkj --source-dir "$fixture/untyped" --library-dir "$fixture/untyped-library" --store "$fixture/untyped-store" --store-key "$fixture/key")
"$compiler" build "${untyped_common[@]}" --index -o "$fixture/untyped-image" > "$fixture/untyped-build.log" 2>&1 || { cat "$fixture/untyped-build.log" >&2; fail 'untyped library build'; }
untyped_snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/untyped-build.log")
[[ ${#untyped_snapshot} == 64 ]] || fail 'untyped library snapshot'
untyped_query() { "$compiler" query "${untyped_common[@]}" --snapshot "$untyped_snapshot" --kind "$1" --id "$2" --limit 80; }
untyped_query references std::scope::unshadowed > "$fixture/untyped-param"
grep -Eq '^"std::scope::unshadowed::param:target".*kind=local-value' "$fixture/untyped-param" || fail 'unshadowed library parameter identity missing'
if grep -Eq 'kind=lexical-name' "$fixture/untyped-param"; then fail 'proved parameter remained ambiguous'; fi
untyped_query references std::scope::top_local > "$fixture/untyped-local"
local_identity=$(sed -n 's/^"\(std::scope::top_local::body:[^"]*::binding:target:0\)".*kind=local-value.*/\1/p' "$fixture/untyped-local")
[[ -n "$local_identity" ]] || fail 'unique top-level library binding identity missing'
untyped_query type "$local_identity" > "$fixture/untyped-local-type"
grep -Eq 'fact=type-unavailable' "$fixture/untyped-local-type" || fail 'lexical identity invented an inferred type'
for identity in shadowed before initializer nested_block closure_shadow; do
    untyped_query references "std::scope::$identity" > "$fixture/untyped-ambiguous"
    grep -Eq 'kind=lexical-name.*claim=partial' "$fixture/untyped-ambiguous" || fail "unproved lexical scope was accepted: $identity"
done
untyped_query references std::scope::outer > "$fixture/untyped-outer"
grep -Eq '^"std::scope::outer::param:target".*kind=local-value' "$fixture/untyped-outer" || fail 'outer parameter identity missing'
untyped_query uses std::scope::target > "$fixture/untyped-inner"
grep -Eq 'inner".*kind=value' "$fixture/untyped-inner" || fail 'nested function captured the outer parameter'
untyped_query uses std::scope::SOURCE > "$fixture/untyped-item"
grep -Eq '::N".*kind=value' "$fixture/untyped-item" || fail 'nested constant captured the outer parameter'
cat > "$fixture/untyped-refuse.json" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"std::scope::target","name":"renamed"}]}
REQUEST
if "$compiler" edit main.lkj --source-dir "$fixture/untyped" --library-dir "$fixture/untyped-library" --no-store --store-key "$fixture/key" --dry-run --request "$fixture/untyped-refuse.json" > "$fixture/untyped-refuse.out" 2> "$fixture/untyped-refuse.err"; then
    fail 'rename ignored an unproved library lexical use'
fi
grep -Eq 'E_EDIT_REFERENCES' "$fixture/untyped-refuse.err" || { cat "$fixture/untyped-refuse.err" >&2; fail 'missing lexical rename refusal'; }
mkdir "$fixture/library-rename"
cat > "$fixture/library-rename/main.lkj" <<'SOURCE'
mod edit { pub fn target(value: u64) -> u64 { value ^ 1u64 } }
fn main() -> u64 { edit::target(41u64) }
SOURCE
cat > "$fixture/library-rename.json" <<'REQUEST'
{"version":1,"operations":[{"op":"rename","id":"edit::target","name":"target_renamed"}]}
REQUEST
"$compiler" edit main.lkj --source-dir "$fixture/library-rename" --library-dir "$PWD/library" --no-store --store-key "$fixture/key" --dry-run --request "$fixture/library-rename.json" > "$fixture/library-rename.out" 2> "$fixture/library-rename.err" || { cat "$fixture/library-rename.err" >&2; fail 'real-library free-function rename'; }
grep -Eq 'mode=dry-run' "$fixture/library-rename.out" || fail 'missing checked real-library rename plan'

# Self record constructors follow the resolved implementation owner and keep
# their spelling under rename; generic target arguments remain in type claims.
mkdir "$fixture/self-source" "$fixture/self-store"
cat > "$fixture/self-source/main.lkj" <<'SOURCE'
mod types { pub struct Item { pub value: i64 } impl Item { pub fn new() -> Self { Self { value: 21i64 } } } }
mod generic { pub struct Record<T> { pub value: T } impl<T> Record<T> { pub fn new(value: T) -> Self { Self { value } } } }
use types::Item as Alias;
fn main() -> i64 { (types::Item { value: 21i64 }).value + (Alias { value: 21i64 }).value + (generic::Record { value: 5i64 }).value }
SOURCE
self_common=(main.lkj --source-dir "$fixture/self-source" --no-library --store "$fixture/self-store" --store-key "$fixture/key")
"$compiler" build "${self_common[@]}" --index -o "$fixture/self-image" > "$fixture/self-build.log" 2>&1 || { cat "$fixture/self-build.log" >&2; fail 'Self record index'; }
self_snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/self-build.log")
self_query() { "$compiler" query "${self_common[@]}" --snapshot "$self_snapshot" --kind "$1" --id "$2" --limit 80; }
self_query references types::Item::new > "$fixture/self-references"
grep -Eq '^"types::Item".*kind=alias-constructor$' "$fixture/self-references" || fail 'Self nominal constructor identity missing'
grep -Eq '^"types::Item::field:value".*kind=constructor-field$' "$fixture/self-references" || fail 'Self field identity missing'
read -r self_start self_end <<< "$(sed -n 's/^"types::Item".*start=\([0-9]*\) end=\([0-9]*\) kind=alias-constructor$/\1 \2/p' "$fixture/self-references")"
[[ $(dd if="$fixture/self-source/main.lkj" bs=1 skip="$self_start" count="$((self_end - self_start))" status=none) == Self ]] || fail 'Self constructor span was not the exact token'
self_query references 'generic::Record<T>::new' > "$fixture/self-generic"
grep -Eq '^"generic::Record".*kind=alias-constructor$' "$fixture/self-generic" || fail 'generic Self nominal identity missing'
printf '%s\n' '{"version":1,"operations":[{"op":"rename","id":"types::Item","name":"Renamed"}]}' > "$fixture/self-request"
"$compiler" edit main.lkj --source-dir "$fixture/self-source" --no-library --no-store --store-key "$fixture/key" --request "$fixture/self-request" > "$fixture/self-plan" 2> "$fixture/self-error" || { cat "$fixture/self-error" >&2; fail 'Self nominal rename'; }
grep -Fq 'Self { value: 21i64 }' "$fixture/self-source/main.lkj" || fail 'nominal rename changed Self spelling'
test "$("$compiler" run "$fixture/self-source/main.lkj" --no-library --no-store)" = 47
printf '%s\n' '{"version":1,"operations":[{"op":"add-field","id":"generic::Record","source":"pub tag: i64","value":"0i64"}]}' > "$fixture/self-request"
"$compiler" edit main.lkj --source-dir "$fixture/self-source" --no-library --no-store --store-key "$fixture/key" --request "$fixture/self-request" > "$fixture/self-plan" 2> "$fixture/self-error" || { cat "$fixture/self-error" >&2; fail 'generic Self field propagation'; }
test "$("$compiler" run "$fixture/self-source/main.lkj" --no-library --no-store)" = 47
# Undemanded library bodies expose source claims without inventing the owner
# of Self inside a nested free function or an abstract trait default.
mkdir "$fixture/self-untyped" "$fixture/self-library" "$fixture/self-untyped-store"
printf '%s\n' 'fn main() -> i64 { 0i64 }' > "$fixture/self-untyped/main.lkj"
cat > "$fixture/self-library/lib.lkj" <<'SOURCE'
pub struct Actual { pub value: i64 }
impl Actual {
    pub fn construct() -> Self { Self { value: 1i64 } }
    pub fn outer() -> i64 { fn nested_self_constructor() -> i64 { let _ = Self { value: 2i64 }; 0i64 } 0i64 }
}
pub trait Abstract { fn make() -> Self { Self { value: 3i64 } } }
SOURCE
self_untyped_common=(main.lkj --source-dir "$fixture/self-untyped" --library-dir "$fixture/self-library" --store "$fixture/self-untyped-store" --store-key "$fixture/key")
"$compiler" build "${self_untyped_common[@]}" --index -o "$fixture/self-untyped-image" > "$fixture/self-untyped-build.log" 2>&1 || { cat "$fixture/self-untyped-build.log" >&2; fail 'undemanded Self index'; }
self_untyped_snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/self-untyped-build.log")
for identity in std::nested_self_constructor std::Abstract::make; do
    "$compiler" query "${self_untyped_common[@]}" --snapshot "$self_untyped_snapshot" --kind references --id "$identity" > "$fixture/self-unresolved"
    grep -Eq 'kind=constructor claim=partial' "$fixture/self-unresolved" || fail "Self borrowed an unproved nominal owner: $identity"
    if grep -Eq 'kind=alias-constructor' "$fixture/self-unresolved"; then fail "Self acquired an enclosing implementation: $identity"; fi
done
"$compiler" query "${self_untyped_common[@]}" --snapshot "$self_untyped_snapshot" --kind references --id std::Actual::construct > "$fixture/self-resolved"
grep -Eq '^"std::Actual".*kind=alias-constructor$' "$fixture/self-resolved" || fail 'resolved owner missing from an undemanded inherent method'

# Ordinary checkpoints omit source-site evidence. Indexing the same tree must
# rebuild complete references, while a complete unchanged snapshot may be reused.
capture_checkpoint_regression() {
    local source_root="$fixture/capture-source"
    local warm_store="$fixture/capture-warm-store"
    local cold_before="$fixture/capture-cold-before"
    local cold_after="$fixture/capture-cold-after"
    local -a common=()
    local snapshot= first_snapshot= changed_snapshot=
    mkdir "$source_root" "$warm_store" "$cold_before" "$cold_after"
    cat > "$source_root/main.lkj" <<'SOURCE'
pub fn main() -> i64 { shadowed() }
pub fn scalar(value: i64) -> i64 { value + 1 }
pub fn shadowed() -> i64 { let before = scalar(1); { let scalar = before; let _ = scalar; } scalar(before) }
pub fn local_generic<T>(value: T) -> T { let held = value; held }
SOURCE
    cp "$source_root/main.lkj" "$fixture/capture-before.lkj"
    capture_stat() {
        local value
        value=$(sed -n "s/.*\"$2\":\\([0-9][0-9]*\\).*/\\1/p" "$1")
        [[ "$value" =~ ^[0-9]+$ ]] || fail "missing or repeated capture statistic $2 in $1"
        printf '%s\n' "$value"
    }
    capture_build() {
        local phase=$1 store=$2
        shift 2
        common=(main.lkj --source-dir "$source_root" --no-library --store "$store" --store-key "$fixture/key")
        "$compiler" build "${common[@]}" "$@" --stats -o "$fixture/capture-program" \
            > "$fixture/capture-$phase.log" 2>&1 || {
                cat "$fixture/capture-$phase.log" >&2
                fail "capture build $phase"
            }
    }
    capture_index() {
        local phase=$1
        capture_build "$@" --index
        snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/capture-$phase.log")
        [[ ${#snapshot} == 64 ]] || fail "capture snapshot $phase"
    }
    capture_facts() {
        local phase=$1 held scalar_binding fact cursor
        query references shadowed --limit 80 --bytes 32768 > "$fixture/capture-$phase-shadowed"
        [[ $(grep -Ec '^"scalar".*kind=value' "$fixture/capture-$phase-shadowed") == 2 ]] ||
            fail "capture lost a shadowed free call: $phase"
        grep -Eq 'binding:scalar:0.*kind=local-value' "$fixture/capture-$phase-shadowed" ||
            fail "capture lost the shadowed local: $phase"
        if grep -Eq 'kind=lexical-name' "$fixture/capture-$phase-shadowed"; then
            fail "capture left a typed local ambiguous: $phase"
        fi
        scalar_binding=$(sed -n 's/^"\([^"]*::binding:scalar:0\)".*kind=local-value.*/\1/p' "$fixture/capture-$phase-shadowed")
        [[ -n "$scalar_binding" ]] || fail "capture scalar binding identity: $phase"
        query type "$scalar_binding" --limit 80 --bytes 32768 > "$fixture/capture-$phase-shadow-type"
        grep -Eq 'fact=inferred-type.*value="i64"' "$fixture/capture-$phase-shadow-type" ||
            fail "capture lost the local inferred type: $phase"

        query references local_generic --limit 80 --bytes 32768 > "$fixture/capture-$phase-generic"
        grep -Eq '^"local_generic::param:value".*kind=local-value' "$fixture/capture-$phase-generic" ||
            fail "capture lost the generic parameter reference: $phase"
        grep -Eq 'binding:held:0.*kind=local-value' "$fixture/capture-$phase-generic" ||
            fail "capture lost the generic binding reference: $phase"
        query outline local_generic --limit 80 --bytes 32768 > "$fixture/capture-$phase-outline"
        held=$(sed -n 's/^"\([^"]*::binding:held:0\)".*/\1/p' "$fixture/capture-$phase-outline" | head -n 1)
        [[ -n "$held" ]] || fail "capture generic binding identity: $phase"
        query type "$held" --limit 80 --bytes 32768 > "$fixture/capture-$phase-held-type"
        grep -Eq 'fact=inferred-type.*value="T"' "$fixture/capture-$phase-held-type" ||
            fail "capture lost the generic definition type: $phase"
        for fact in shadowed shadow-type generic outline held-type; do
            cursor=$(sed -n 's/.*cursor=\([^ ]*\) facts=.*/\1/p' "$fixture/capture-$phase-$fact")
            [[ "$cursor" == none ]] || fail "capture fact response is incomplete: $phase/$fact"
            [[ $(wc -c < "$fixture/capture-$phase-$fact") -le 32768 ]] ||
                fail "capture fact response exceeded its bound: $phase/$fact"
        done
    }
    capture_same_facts() {
        local fact
        for fact in shadowed shadow-type generic outline held-type; do
            cmp "$fixture/capture-$1-$fact" "$fixture/capture-$2-$fact" ||
                fail "capture facts differ: $1/$2/$fact"
        done
    }
    capture_fresh_index() {
        local phase=$1
        capture_index "$@"
        [[ $(capture_stat "$fixture/capture-$phase.log" build_reused) == 0 ]] ||
            fail "capture index reused an incomplete ordinary image: $phase"
        [[ $(capture_stat "$fixture/capture-$phase.log" front_reused) == 0 ]] ||
            fail "capture index restored incomplete frontend evidence: $phase"
        (( $(capture_stat "$fixture/capture-$phase.log" front_definitions) > 0 )) ||
            fail "capture index did not perform full inference: $phase"
        capture_facts "$phase"
    }

    capture_build ordinary "$warm_store"
    [[ $(capture_stat "$fixture/capture-ordinary.log" front_reused) == 0 ]] ||
        fail 'capture ordinary cold build reused frontend work'
    capture_fresh_index from-ordinary "$warm_store"
    first_snapshot=$snapshot
    capture_fresh_index cold-before "$cold_before"
    [[ "$snapshot" == "$first_snapshot" ]] || fail 'capture cold/warm source snapshot differs'
    capture_same_facts from-ordinary cold-before

    capture_index unchanged "$warm_store"
    [[ "$snapshot" == "$first_snapshot" ]] || fail 'capture unchanged snapshot identity differs'
    [[ $(capture_stat "$fixture/capture-unchanged.log" build_reused) == 1 ]] ||
        fail 'capture unchanged complete snapshot was not reused'
    [[ $(capture_stat "$fixture/capture-unchanged.log" front_definitions) == 0 ]] ||
        fail 'capture unchanged indexed build repeated inference'
    capture_facts unchanged
    capture_same_facts from-ordinary unchanged

    sed -i 's/let before = scalar(1);/let before = scalar(2);/' "$source_root/main.lkj"
    grep -Fq 'let before = scalar(2);' "$source_root/main.lkj" || fail 'capture body edit missing'
    cp "$source_root/main.lkj" "$fixture/capture-after.lkj"
    capture_build body-ordinary "$warm_store"
    [[ $(capture_stat "$fixture/capture-body-ordinary.log" build_reused) == 0 ]] ||
        fail 'capture body edit replayed the old image'
    (( $(capture_stat "$fixture/capture-body-ordinary.log" front_reused) > 0 )) ||
        fail 'capture body edit did not resume its ordinary checkpoint'
    [[ $(capture_stat "$fixture/capture-body-ordinary.log" front_definitions) == 1 ]] ||
        fail 'capture body edit did not re-elaborate exactly one definition'
    capture_fresh_index from-body "$warm_store"
    changed_snapshot=$snapshot
    [[ "$changed_snapshot" != "$first_snapshot" ]] || fail 'capture body edit kept the old snapshot'
    capture_fresh_index cold-after "$cold_after"
    [[ "$snapshot" == "$changed_snapshot" ]] || fail 'capture changed cold/warm snapshot differs'
    capture_same_facts from-body cold-after
}
capture_checkpoint_regression

printf 'ok semantic-query: identities, source/IR levels, ordinary authority, all question kinds, bounded authenticated cursor, source-free lookup, physical move/formatting, explicit cross-producer provenance, producer-bound admission, snapshot replay, eight locator and seven container/node attacks\n'
