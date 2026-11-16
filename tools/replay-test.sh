#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-replay-test.XXXXXX)
fixture=$(realpath "$fixture")
cleanup() {
    local result=$?
    if test "$result" -ne 0; then
        for output in "$fixture/plan" "$fixture/error" "$fixture/scoped-plan" "$fixture/scoped-error"; do
            if test -f "$output"; then cat "$output" >&2; fi
        done
    fi
    rm -rf "$fixture"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/old" "$fixture/new"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/old/main.lkj" <<'SOURCE'
fn old_name() -> i64 { 1 }
// keep these exact bytes: π
fn kept() -> i64 { 2 }
pub fn probe() -> i64 { old_name() + kept() }
SOURCE
cat > "$fixture/new/main.lkj" <<'SOURCE'
fn replacement() -> i64 { 4 }
fn inserted() -> i64 { 3 }
// keep these exact bytes: π
fn kept() -> i64 { 2 }
pub fn probe() -> i64 { replacement() + kept() + inserted() }
SOURCE
common=(main.lkj --source-dir "$fixture/old" --no-library --no-store --against main.lkj --against-source-dir "$fixture/new" --store-key "$fixture/key")
cp "$fixture/old/main.lkj" "$fixture/before"
"$compiler" edit "${common[@]}" --dry-run > "$fixture/plan"
cmp "$fixture/before" "$fixture/old/main.lkj"
"$compiler" edit "${common[@]}" > "$fixture/plan"
perl - "$fixture/plan" <<'PERL'
use strict; use warnings;
open my $file, '<:raw', shift or die $!; local $/; my $text=<$file>;
$text =~ /^replay desired=([0-9a-f]{64}) candidate=([0-9a-f]{64}) verified=true operations=([0-9]+)$/m or die "missing replay proof\n";
die "unequal replay proof\n" unless $1 eq $2 && $3 > 0;
PERL
test "$("$compiler" run main.lkj --source-dir "$fixture/old" --no-library --no-store)" = 9
grep -Fq 'fn kept() -> i64 { 2 }' "$fixture/old/main.lkj"
"$compiler" compare main.lkj --source-dir "$fixture/old" --against main.lkj --against-source-dir "$fixture/new" --no-library --no-store > "$fixture/comparison"
grep -q 'checked IR identical' "$fixture/comparison"
# External modules are intentionally read twice by the loader. Unchanged binary
# includes are part of the snapshot but never tokenized as source declarations.
cat > "$fixture/old/main.lkj" <<'SOURCE'
mod helper;
pub fn probe() -> i64 { helper::value() }
SOURCE
cp "$fixture/old/main.lkj" "$fixture/new/main.lkj"
cat > "$fixture/old/helper.lkj" <<'SOURCE'
pub fn value() -> i64 { let data = include_bytes!("payload.bin"); data.len() as i64 + 1i64 }
SOURCE
sed 's/+ 1i64/+ 2i64/' "$fixture/old/helper.lkj" > "$fixture/new/helper.lkj"
printf '\377\000' > "$fixture/old/payload.bin"
cp "$fixture/old/payload.bin" "$fixture/new/payload.bin"
"$compiler" edit "${common[@]}" > "$fixture/plan"
grep -q 'verified=true' "$fixture/plan"
test "$("$compiler" run main.lkj --source-dir "$fixture/old" --no-library --no-store)" = 4
cp "$fixture/old/main.lkj" "$fixture/before"
printf '\376\000' > "$fixture/new/payload.bin"
if "$compiler" edit "${common[@]}" > "$fixture/plan" 2> "$fixture/error"; then exit 1; fi
grep -q E_REPLAY_DATA "$fixture/error"
cmp "$fixture/before" "$fixture/old/main.lkj"
# Imports and file framing are explicit decisions; a failed replay publishes nothing.
cat > "$fixture/old/main.lkj" <<'SOURCE'
mod helpers { pub fn first() -> i64 { 1 } pub fn second() -> i64 { 2 } }
use helpers::first as selected;
pub fn probe() -> i64 { selected() }
SOURCE
sed 's/use helpers::first/use helpers::second/' "$fixture/old/main.lkj" > "$fixture/new/main.lkj"
cp "$fixture/old/main.lkj" "$fixture/before"
if "$compiler" edit "${common[@]}" > "$fixture/plan" 2> "$fixture/error"; then exit 1; fi
grep -q E_REPLAY_FRAME "$fixture/error"
cmp "$fixture/before" "$fixture/old/main.lkj"
# Even the desired tree cannot use a copied signing key as ordinary source data.
cp "$fixture/key" "$fixture/new/private.bin"
cat > "$fixture/new/main.lkj" <<'SOURCE'
pub fn probe() -> i64 { let bytes = include_bytes!("private.bin"); bytes.len() as i64 }
SOURCE
if "$compiler" edit "${common[@]}" > "$fixture/plan" 2> "$fixture/error"; then exit 1; fi
grep -q 'private.*key' "$fixture/error"
cmp "$fixture/before" "$fixture/old/main.lkj"
test ! -e "$fixture/old/.lkjscript-edit.pending"
# Both trees can carry narrow source roots even when the desired entry is relative
# to another working directory. Include-only sibling data shares a directory with
# explicit library declarations, matching the compiler's embedded runtime shape.
mkdir -p "$fixture/scoped-old/src" "$fixture/scoped-new/src"
cp -R library "$fixture/scoped-old/library"
cp -R library "$fixture/scoped-new/library"
cat > "$fixture/scoped-old/src/main.lkj" <<'SOURCE'
pub fn probe() -> i64 {
    let bytes = include_bytes!("../library/runtime.lkj");
    bytes.len() as i64 + 1i64
}
SOURCE
sed 's/+ 1i64/+ 2i64/' "$fixture/scoped-old/src/main.lkj" > "$fixture/scoped-new/src/main.lkj"
cp "$fixture/scoped-old/src/main.lkj" "$fixture/scoped-before"
scoped=(src/main.lkj --source-dir "$fixture/scoped-old" --library-dir "$fixture/scoped-old/library"
    --against src/main.lkj --against-library-dir "$fixture/scoped-new/library"
    --no-store --store-key "$fixture/key")
(
    cd "$fixture/scoped-new"
    # Without an explicit desired scope, differently spelled include aliases
    # remain a refusal; this option does not invent a filename correspondence.
    if "$compiler" edit "${scoped[@]}" > "$fixture/scoped-plan" 2> "$fixture/scoped-error"; then exit 1; fi
    grep -q E_REPLAY_FILES "$fixture/scoped-error"
    grep -Fq "$fixture/scoped-old/library/runtime.lkj" "$fixture/scoped-error"
    test ! -s "$fixture/scoped-plan"
    cmp "$fixture/scoped-before" "$fixture/scoped-old/src/main.lkj"
    "$compiler" edit "${scoped[@]}" --against-source-dir "$fixture/scoped-new" --dry-run > "$fixture/scoped-plan"
    grep -q 'verified=true' "$fixture/scoped-plan"
    cmp "$fixture/scoped-before" "$fixture/scoped-old/src/main.lkj"
    "$compiler" edit "${scoped[@]}" --against-source-dir "$fixture/scoped-new" > "$fixture/scoped-plan"
    grep -q 'verified=true' "$fixture/scoped-plan"
    cmp "$fixture/scoped-old/src/main.lkj" "$fixture/scoped-new/src/main.lkj"
    "$compiler" compare "${scoped[@]}" --against-source-dir "$fixture/scoped-new" > "$fixture/scoped-comparison"
    grep -q 'checked IR identical' "$fixture/scoped-comparison"
    # A nonexistent explicit grant must fail without falling back to this cwd,
    # where an otherwise valid desired src/main.lkj is available.
    for command in edit compare; do
        if "$compiler" "$command" "${scoped[@]}" --against-source-dir "$fixture/missing" > "$fixture/scoped-plan" 2> "$fixture/scoped-error"; then exit 1; fi
        grep -q 'E_EDIT_IO open source transaction scope: errno 2' "$fixture/scoped-error"
        test ! -s "$fixture/scoped-plan"
        cmp "$fixture/scoped-old/src/main.lkj" "$fixture/scoped-new/src/main.lkj"
    done
)
for command in edit compare; do
    if "$compiler" "$command" src/main.lkj --source-dir "$fixture/scoped-old" --against-source-dir "$fixture/scoped-new" --no-store > "$fixture/scoped-plan" 2> "$fixture/scoped-error"; then exit 1; fi
    grep -Fq 'E_USAGE --against-source-dir requires --against SOURCE' "$fixture/scoped-error"
    test ! -s "$fixture/scoped-plan"
done
test ! -e "$fixture/scoped-old/.lkjscript-edit.pending"
printf 'ok history replay: checked target identity, exact kept bytes, dry run, framing, private-key exclusion, explicit desired scope and no cwd fallback\n'
