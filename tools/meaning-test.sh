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
grep -q 'authority-gained=.*effect:stdout_write' "$fixture/report"
grep -q 'authority-lost=""' "$fixture/report"
cp "$fixture/new/main.lkj" "$fixture/swap"
cp "$fixture/old/main.lkj" "$fixture/new/main.lkj"
mv "$fixture/swap" "$fixture/old/main.lkj"
compare --library-dir library
grep -q 'authority-lost=.*effect:stdout_write' "$fixture/report"
grep -q 'authority-gained=""' "$fixture/report"
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
grep -q 'level=source-claim changed=body' "$fixture/report"
grep -q 'level=checked-ir changed=instance' "$fixture/report"
grep -Eq 'instance-facts=[1-9][0-9]*' "$fixture/report"
compare --no-library --limit 1
test "$(wc -l < "$fixture/report")" -eq 4
compare --no-library --limit 0
test "$(wc -l < "$fixture/report")" -eq 3
# Transfer-only entries have authority even when the host CLI cannot grant them.
cat > "$fixture/old/main.lkj" <<'SOURCE'
pub fn main() {}
SOURCE
for kind in Channel File ReadFile Image; do
    case "$kind" in
        Channel) body='let _ = value.send(b"x", !0usize);'; effect=channel_send ;;
        File|ReadFile) body='let mut bytes = [0u8; 1]; let _ = value.read(&mut bytes);'; effect=file_read ;;
        Image) body='let _ = value.buffer(b"x");'; effect=image_buffer ;;
    esac
    printf 'pub fn main(mut value: %s) { %s }\n' "$kind" "$body" > "$fixture/new/main.lkj"
    compare --library-dir library --id ir:main
    grep -Eq '"ir:main".*changed=authority' "$fixture/report"
    grep -q "grant:0:$kind" "$fixture/report"
    grep -q "effect:$effect" "$fixture/report"
done
cat > "$fixture/new/main.lkj" <<'SOURCE'
pub fn main(channel: Channel, ordinary: i64) { let _ = ordinary; }
SOURCE
compare --library-dir library --id ir:main
if grep -Eq 'authority-gained="[^"]*grant:0:Channel' "$fixture/report"; then
    printf 'ordinary mixed parameters became a transferable entry\n' >&2; exit 1
fi
cp "$fixture/old/main.lkj" "$fixture/new/main.lkj"
cat > "$fixture/child-request.json" <<'REQUEST'
{"version":1,"operations":[{"op":"replace-item","id":"main","source":"pub fn main(mut channel: Channel) { let _ = channel.send(b\"x\", !0usize); }"}]}
REQUEST
"$compiler" edit main.lkj --source-dir "$fixture/new" --library-dir library --no-store \
    --request "$fixture/child-request.json" --dry-run > "$fixture/child-plan"
grep -Eq '"ir:main".*authority-gained="[^"]*effect:channel_send' "$fixture/child-plan"
grep -Eq '"ir:main".*authority-gained="[^"]*grant:0:Channel' "$fixture/child-plan"
cmp "$fixture/old/main.lkj" "$fixture/new/main.lkj"
printf 'ok checked meaning: layout, type graph, authority, stable instances, cyclic graphs and bounded review\n'
