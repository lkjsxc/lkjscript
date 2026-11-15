#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-replay-test.XXXXXX)
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
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
common=(main.lkj --source-dir "$fixture/old" --no-library --no-store --against "$fixture/new/main.lkj" --store-key "$fixture/key")
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
test "$("$compiler" run "$fixture/old/main.lkj" --no-library --no-store)" = 9
grep -Fq 'fn kept() -> i64 { 2 }' "$fixture/old/main.lkj"
"$compiler" compare "$fixture/old/main.lkj" --against "$fixture/new/main.lkj" --no-library --no-store > "$fixture/comparison"
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
test "$("$compiler" run "$fixture/old/main.lkj" --no-library --no-store)" = 4
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
printf 'ok history replay: checked target identity, multiple operations, exact kept bytes, dry run, framing and private-key refusals\n'
