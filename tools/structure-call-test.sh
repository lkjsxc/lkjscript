#!/usr/bin/env bash
# Authored call identities select exact checked edits through the existing writer.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
mkdir -p target
fixture=$(mktemp -d "$PWD/target/compiler-structure-call.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for output in "$fixture/plan" "$fixture/error"; do
            [[ ! -f "$output" ]] || cat "$output" >&2
        done
    fi
    rm -rf -- "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
umask 077
head -c 32 /dev/urandom > "$fixture/key"
mkdir "$fixture/store"
fail() { printf 'structure-call: %s\n' "$*" >&2; exit 1; }
setup() {
    project="$fixture/$1"
    mkdir -p "$project"
    common=(main.lkj --source-dir "$project" --no-library)
}
index() {
    "$compiler" build "${common[@]}" --index --store "$fixture/store" --store-key "$fixture/key" \
        -o "$fixture/image" > "$fixture/index" 2> "$fixture/error"
    pin=$(awk '$1=="LKJSCRIPT_SNAPSHOT" { print $2 }' "$fixture/index")
    [[ ${#pin} == 64 ]] || fail 'missing snapshot identity'
}
query() {
    "$compiler" query "${common[@]}" --store "$fixture/store" --store-key "$fixture/key" \
        --snapshot "$pin" --kind "$1" --id "$2" --depth 2 --limit 256 --bytes 131072
}
call_id() {
    query outline "$1" > "$fixture/outline"
    grep -q 'cursor=none' "$fixture/outline" || fail 'fixture outline needs continuation'
    perl -MJSON::PP - "$2" "${3:-1}" "$fixture/outline" <<'PERL'
use strict; use warnings;
my ($wanted, $ordinal, $path) = @ARGV;
open my $file, '<:raw', $path or die "$path: $!";
my @ids;
while (<$file>) {
    my ($id, $rest) = split /\t/, $_, 2;
    next unless defined $rest && $rest =~ /\bfact=signature\tkind=call-site text=("(?:[^"\\]|\\.)*")/;
    push @ids, decode_json($id) if decode_json($1) eq $wanted;
}
die "call not found: $wanted occurrence $ordinal\n" unless $ordinal > 0 && $ordinal <= @ids;
print $ids[$ordinal - 1], "\n";
PERL
}
generated_id() {
    query outline "$1" > "$fixture/generated.out"
    grep -q 'cursor=none' "$fixture/generated.out" || fail 'generated outline needs continuation'
    perl -MJSON::PP -ne 'if (/\bkind=call-site\b/ && /origin=generated/) { my ($id)=split /\t/; print decode_json($id), "\n"; $found=1; last } END { die "missing generated call\n" unless $found }' "$fixture/generated.out"
}
digest() {
    query meta "$1" > "$fixture/meta"
    sed -n 's/.* item-sha256=\([0-9a-f]*\).*/\1/p' "$fixture/meta"
}
operation() {
    perl -MJSON::PP -e 'my ($id,$source,$expect)=@ARGV; my %op=(op=>"replace-call",id=>$id,source=>$source); $op{expect}=$expect if length $expect; print encode_json(\%op)' "$1" "$2" "${3:-}"
}
request() { printf '{"version":1,"operations":[%s]}\n' "$1" > "$fixture/request.json"; }
snapshot() {
    (
        cd "$project"
        find . -name '*.lkj' -printf '%y %P %l\n'
        find . -type f -name '*.lkj' -exec sha256sum {} +
    ) | LC_ALL=C sort > "$1"
}
edit() {
    "$compiler" edit "${common[@]}" --no-store --store-key "$fixture/key" \
        --request "$fixture/request.json" "$@" > "$fixture/plan" 2> "$fixture/error"
}
apply() {
    edit
    grep -q 'mode=apply' "$fixture/plan" || fail 'missing apply receipt'
    [[ ! -e "$project/.lkjscript-edit.pending" ]] || fail 'successful edit left pending state'
}
refuse() {
    local expected=$1 status=0
    snapshot "$fixture/before"
    edit || status=$?
    [[ $status == 1 ]] || fail "expected refusal $expected, got exit $status"
    [[ ! -s "$fixture/plan" ]] || fail 'refusal emitted a success plan'
    grep -Fq "$expected" "$fixture/error" || fail "missing refusal $expected"
    snapshot "$fixture/after"
    cmp "$fixture/before" "$fixture/after"
    [[ ! -e "$project/.lkjscript-edit.pending" ]] || fail 'refusal left pending state'
}
value() {
    local engine actual
    for engine in native interpreter; do
        actual=$("$compiler" run "${common[@]}" --no-store --engine "$engine")
        [[ $actual == "$1" ]] || fail "$engine returned $actual, expected $1"
    done
}

# One of two equal calls, precedence, a trailing comment, and three files.
setup primary
cat > "$project/main.lkj" <<'SOURCE'
mod left;
mod right;
mod untouched;
fn step(value: i64) -> i64 { value + 1i64 }
fn main() -> i64 { left::run() + right::run() + step(4i64) + step(4i64) }
SOURCE
printf '%s\n' 'pub fn run() -> i64 { 2i64 * crate::step(1i64) }' > "$project/left.lkj"
printf '%s\n' 'pub fn run() -> i64 { crate::step(2i64) }' > "$project/right.lkj"
printf '%s\n' '// untouched bytes: π' 'pub fn unused() -> i64 { 7i64 }' > "$project/untouched.lkj"
index
left=$(call_id left::run 'crate::step(1i64)')
right=$(call_id right::run 'crate::step(2i64)')
second=$(call_id main 'step(4i64)' 2)
expected=$(digest "$second")
[[ ${#expected} == 64 ]] || fail 'missing call item digest'
original_source=$(awk '{ for (i=1;i<=NF;i++) if ($i ~ /^source=[0-9a-f]+$/) print substr($i,8) }' "$fixture/meta")
[[ ${#original_source} == 64 ]] || fail 'missing original source digest'
request "$(operation "$left" '3i64 + 4i64'),$(operation "$right" 'crate::step(4i64) // keep the closing group on its own line'),$(operation "$second" '10i64' "$expected")"
snapshot "$fixture/before"
edit --dry-run
grep -q 'mode=dry-run' "$fixture/plan"
snapshot "$fixture/after"
cmp "$fixture/before" "$fixture/after"
[[ ! -e "$project/.lkjscript-edit.pending" ]]
apply
mkdir "$fixture/desired"
cat > "$fixture/desired/main.lkj" <<'SOURCE'
mod left;
mod right;
mod untouched;
fn step(value: i64) -> i64 { value + 1i64 }
fn main() -> i64 { left::run() + right::run() + step(4i64) + (
10i64
) }
SOURCE
cat > "$fixture/desired/left.lkj" <<'SOURCE'
pub fn run() -> i64 { 2i64 * (
3i64 + 4i64
) }
SOURCE
cat > "$fixture/desired/right.lkj" <<'SOURCE'
pub fn run() -> i64 { (
crate::step(4i64) // keep the closing group on its own line
) }
SOURCE
printf '%s\n' '// untouched bytes: π' 'pub fn unused() -> i64 { 7i64 }' > "$fixture/desired/untouched.lkj"
for file in main left right untouched; do cmp "$project/$file.lkj" "$fixture/desired/$file.lkj"; done
"$compiler" compare "${common[@]}" --against main.lkj --against-source-dir "$fixture/desired" \
    --no-store > "$fixture/comparison"
grep -Fq 'verdict="text identical, checked IR identical"' "$fixture/comparison"
value 34

# Every refusal checks all source files, not only the selected one.
index
current=$(call_id main 'step(4i64)')
right=$(call_id right::run 'crate::step(4i64)')
printf '{"version":1,"snapshot":"%s","operations":[%s]}\n' "$original_source" "$(operation "$current" '0i64')" > "$fixture/request.json"
refuse E_EDIT_STALE
request "$(operation "$current" '0i64' wrong)"
refuse E_EDIT_STALE
request "$(operation main '0i64')"
refuse E_EDIT_CALL_SITE
request "$(operation "$current" '0i64'),$(operation "$right" 'false')"
refuse E_EDIT_CHECK
request "{\"op\":\"replace-call\",\"id\":\"$current\",\"source\":\"0i64\",\"unexpected\":true}"
refuse E_EDIT_REQUEST
while IFS= read -r fragment; do
    request "$(operation "$current" "$fragment")"
    refuse E_EDIT_EXPRESSION
done <<'FRAGMENTS'

1i64; 2i64
let value = 1i64;
fn escaped() -> i64 { 9i64 }
1i64 } // escaped wrapper
1i64) } // escaped parenthesized wrapper
FRAGMENTS
# A token edit elsewhere in the same function invalidates the old body epoch.
perl -0pi -e 's/left::run\(\)/left::run() + 0i64/' "$project/main.lkj"
request "$(operation "$current" '0i64')"
refuse E_EDIT_IDENTITY


# Token epochs survive trivia, while expect still binds exact selected bytes.
setup trivia
cat > "$project/main.lkj" <<'SOURCE'
fn step(value: i64) -> i64 { value + 1i64 }
fn main() -> i64 { step(1i64) }
SOURCE
index
selected=$(call_id main 'step(1i64)')
expected=$(digest "$selected")
perl -0pi -e 's/fn main\(\) -> i64 \{/fn main() -> i64 { \/\* body trivia \*\//' "$project/main.lkj"
request "$(operation "$selected" 'step(2i64)' "$expected")"
apply
value 3
index
selected=$(call_id main 'step(2i64)')
expected=$(digest "$selected")
perl -0pi -e 's/step\(2i64\)/step(\/\* call trivia \*\/ 2i64)/' "$project/main.lkj"
request "$(operation "$selected" 'step(4i64)' "$expected")"
refuse E_EDIT_STALE
request "$(operation "$selected" 'step(4i64)')"
apply
value 5

setup nested
cat > "$project/main.lkj" <<'SOURCE'
fn step(value: i64) -> i64 { value + 1i64 }
fn main() -> i64 { step(step(1i64)) }
SOURCE
index
outer=$(call_id main 'step(step(1i64))')
inner=$(call_id main 'step(1i64)')
request "$(operation "$outer" '8i64'),$(operation "$inner" '4i64')"
refuse E_EDIT_OVERLAP

# The parser's grouping-free AST must not authorize an incomplete written span.
setup grouped
cat > "$project/main.lkj" <<'SOURCE'
struct Number { value: i64 }
impl Number { fn read(&self) -> i64 { self.value } }
fn step(value: i64) -> i64 { value + 1i64 }
fn main() -> i64 { let value = Number { value: 2i64 }; (step)(1i64) + (value).read() }
SOURCE
index
for text in 'step)(1i64)' 'value).read()'; do
    selected=$(call_id main "$text")
    request "$(operation "$selected" '3i64')"
    refuse E_EDIT_CALL_SITE
done

# Generic and method scopes are checked in place, with no detached name resolution.
setup scoped
cat > "$project/main.lkj" <<'SOURCE'
struct Number { value: i64 }
impl Number { fn plus(&self, extra: i64) -> i64 { self.value + extra } }
fn identity<T>(value: T) -> T { value }
fn transform<T>(value: T) -> T { identity(value) }
fn main() -> i64 { let number = Number { value: 2i64 }; number.plus(transform(3i64)) }
SOURCE
index
generic=$(call_id transform 'identity(value)')
method=$(call_id main 'number.plus(transform(3i64))')
request "$(operation "$generic" value),$(operation "$method" 'number.plus(transform(5i64))')"
apply
value 7

setup macro_replacement
cat > "$project/main.lkj" <<'SOURCE'
macro_rules! increase { ($value:expr) => { $value + 1i64 }; }
fn step(value: i64) -> i64 { value }
fn main() -> i64 { step(1i64) }
SOURCE
index
selected=$(call_id main 'step(1i64)')
request "$(operation "$selected" 'increase!(5i64)')"
apply
value 6

# Expansion anchors, including derive calls, are never writable call expressions.
setup generated
common=(main.lkj --source-dir "$project" --library-dir "$PWD/library")
cat > "$project/main.lkj" <<'SOURCE'
macro_rules! make { () => { fn generated() -> i64 { step(1i64) } }; }
make!();
#[derive(Debug)]
struct Packet { value: i64 }
fn step(value: i64) -> i64 { value + 1i64 }
fn main() -> i64 { let _ = format!("{:?}", Packet { value: 0i64 }); generated() }
SOURCE
index
for owner in macro-expansion:0 '<Packet as std::fmt::Debug>::fmt'; do
    selected=$(generated_id "$owner")
    request "$(operation "$selected" '0i64')"
    refuse E_EDIT_GENERATED_SOURCE
done

# Editing an undemanded library call promotes the full library checking frontier.
setup library_promotion
mkdir "$project/library"
common=(main.lkj --source-dir "$project" --library-dir "$project/library")
printf '%s\n' 'fn main() -> i64 { 0i64 }' > "$project/main.lkj"
cat > "$project/library/lib.lkj" <<'SOURCE'
pub fn helper(value: i64) -> i64 { value + 1i64 }
pub fn unused() -> i64 { helper(1i64) }
SOURCE
index
selected=$(call_id std::unused 'helper(1i64)')
request "$(operation "$selected" false)"
refuse E_EDIT_CHECK
request "$(operation "$selected" 'helper(4i64)')"
apply
grep -q 'library-validation=all' "$fixture/plan"
printf '%s\n' 'fn main() -> i64 { std::unused() }' > "$project/probe.lkj"
for engine in native interpreter; do
    [[ $("$compiler" run probe.lkj --source-dir "$project" --library-dir "$project/library" --no-store --engine "$engine") == 5 ]]
done
printf 'ok call transactions: exact checked meaning, source bytes, both engines, scopes, epochs, generated origins and atomic refusals\n'
