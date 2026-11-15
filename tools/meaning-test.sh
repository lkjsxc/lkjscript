#!/usr/bin/env bash
# Compare admitted evidence; diagnostic text and source declarations are separate.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-meaning-test.XXXXXX)
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
mkdir "$fixture/old" "$fixture/new"
compare() {
    "$compiler" compare "$fixture/new/main.lkj" --against "$fixture/old/main.lkj" --no-store "$@" > "$fixture/report"
    test "$(wc -l < "$fixture/report")" -le 30
}
cat > "$fixture/old/main.lkj" <<'SOURCE'
struct Message { tag: u8, value: i32 }
fn compute(value: i32) -> i32 { value + 1 }
pub fn probe() -> i32 { let message = Message { tag: 1u8, value: 40 }; compute(message.value) }
SOURCE
cp "$fixture/old/main.lkj" "$fixture/new/main.lkj"
compare --no-library
grep -q 'text identical, checked IR identical' "$fixture/report"
grep -q 'changed-facts=0 ' "$fixture/report"
sed 's/i32/i64/g' "$fixture/old/main.lkj" > "$fixture/new/main.lkj"
compare --no-library
grep -q 'level=checked-ir changed=layout' "$fixture/report"
grep -q '"ir:type:Message".*bytes=8.*bytes=16' "$fixture/report"
grep -q 'level=checked-ir changed=instance' "$fixture/report"
grep -q 'level=source-claim changed=signature' "$fixture/report"
compare --no-library --id ir:type:Message
grep -q 'changed=type-identity' "$fixture/report"
test "$(grep -c '^"ir:type:Message"' "$fixture/report")" -eq 2
cat > "$fixture/old/main.lkj" <<'SOURCE'
pub fn main() {}
SOURCE
cat > "$fixture/new/main.lkj" <<'SOURCE'
pub fn main(mut output: Stdout) { output.write(b"hello"); }
SOURCE
compare --library-dir library
grep -q '"ir:main" level=checked-ir changed=authority' "$fixture/report"
grep -q 'grant:0:Stdout' "$fixture/report"
grep -q 'effect:stdout_write' "$fixture/report"
cat > "$fixture/old/main.lkj" <<'SOURCE'
struct Node { next: Option<Box<Node>>, value: i64 }
pub fn probe() -> i64 { let node = Node { next: None, value: 7 }; node.value }
SOURCE
cp "$fixture/old/main.lkj" "$fixture/new/main.lkj"
printf '%s\n' '// cyclic type identity is unchanged by a comment' >> "$fixture/new/main.lkj"
compare --library-dir library
grep -q 'text differs, checked IR identical' "$fixture/report"
if grep -q 'level=checked-ir changed=' "$fixture/report"; then cat "$fixture/report" >&2; exit 1; fi
# An unrelated declaration cannot renumber the type/instance fingerprints.
cat > "$fixture/old/main.lkj" <<'SOURCE'
fn stable(value: i64) -> i64 { value + 1 }
pub fn probe() -> i64 { stable(41) }
SOURCE
{ printf '%s\n' 'fn unrelated(flag: bool) -> u8 { if flag { 3u8 } else { 4u8 } }'; cat "$fixture/old/main.lkj"; } > "$fixture/new/main.lkj"
compare --no-library --id ir:stable
if grep -q 'changed=instance' "$fixture/report"; then cat "$fixture/report" >&2; exit 1; fi
# A bounded report remains bounded even when many source and IR identities change.
: > "$fixture/old/main.lkj"
: > "$fixture/new/main.lkj"
for ((index=0; index<64; index++)); do
    printf 'fn item_%s() -> i64 { %s }\n' "$index" "$index" >> "$fixture/old/main.lkj"
    printf 'fn item_%s() -> i64 { %s }\n' "$index" "$((index+1))" >> "$fixture/new/main.lkj"
done
compare --no-library
grep -Eq 'omitted=[1-9][0-9]*' "$fixture/report"
printf 'ok checked meaning: layout, type graph, authority, stable instances, cyclic graphs and bounded review\n'
