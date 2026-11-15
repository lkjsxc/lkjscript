#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
profile_test=$(mktemp -d target/profile-test.XXXXXX)
trap 'rm -rf "$profile_test"' EXIT HUP INT TERM
"$compiler" build tests/fixtures/profile.lkj -o "$profile_test/plain"
"$compiler" build tests/fixtures/profile.lkj --profile -o "$profile_test/sampled"
"$profile_test/plain" > "$profile_test/plain.out"
"$profile_test/sampled" > "$profile_test/sampled.out"
cmp "$profile_test/plain.out" "$profile_test/sampled.out"
# The explicit sampling prefix is a grant. If its directory disappears, the
# authorized entry's one diagnostic names that prefix and the refused syscall.
cat > "$profile_test/refusal.lkj" <<'SOURCE'
pub fn main(mut world: World) { world.stdout().write(b"unreached\n"); }
SOURCE
mkdir "$profile_test/removed"
"$compiler" build "$profile_test/refusal.lkj" --profile -o "$profile_test/removed/sampled"
mv "$profile_test/removed/sampled" "$profile_test/refused"
rmdir "$profile_test/removed"
profile_refusal_status=0
"$profile_test/refused" > "$profile_test/refused.out" 2> "$profile_test/refused.err" || profile_refusal_status=$?
test "$profile_refusal_status" -eq 1
test ! -s "$profile_test/refused.out"
test "$(wc -l < "$profile_test/refused.err")" -eq 1
grep -qxF "$profile_test/refusal.lkj:1: R0007 Profile $profile_test/removed/sampled.samples openat: ENOENT (2)" "$profile_test/refused.err"
"$compiler" profile "$profile_test"/sampled.samples.* > "$profile_test/report"
awk '
  /^CPU samples:/ { total=$3+0; if (total < 10) exit 1 }
  /^kind [0-9]/ {
    if ($2 < 0 || $2 > 100 || $3 > total) exit 1
    kinds++
    kind_samples += $3
    if ($4 == "arithmetic" && $3 > 0) arithmetic=1
  }
  /^[0-9]/ {
    if ($1 < 0 || $1 > 100 || $2 < 0 || $2 > 100 || $3 > total) exit 1
    if ($4 == "work" && $1 > 50) hot=1
    if ($4 == "recurse" && $2 > 50) recursive=1
    if ($4 == "main" && $2 > 50) root=1
  }
  END { if (NR > 37 || !hot || !recursive || !root || kinds != 16 || kind_samples != total || !arithmetic) exit 1 }
' "$profile_test/report"
"$compiler" ir tests/fixtures/profile.lkj -o "$profile_test/plain.lkjir"
sed 's/^fn "work" /fn "work\\ncontrol" /' "$profile_test/plain.lkjir" > "$profile_test/named.lkjir"
"$compiler" build "$profile_test/named.lkjir" --profile -o "$profile_test/named"
"$profile_test/named" > "$profile_test/named.out"
cmp "$profile_test/plain.out" "$profile_test/named.out"
"$compiler" profile "$profile_test"/named.samples.* > "$profile_test/named.report"
grep -qF 'work\u{a}control' "$profile_test/named.report"
test "$(wc -l < "$profile_test/named.report")" -le 37
cp "$profile_test"/sampled.samples.* "$profile_test/invalid-metadata"
printf '\377\377\377\377\377\377\377\377' |
    dd of="$profile_test/invalid-metadata" bs=1 seek=48 conv=notrunc status=none
if "$compiler" profile "$profile_test/invalid-metadata" > "$profile_test/out" 2> "$profile_test/error"; then
    echo 'profiler accepted invalid category metadata' >&2
    exit 1
fi
grep -q E_PROFILE "$profile_test/error"
cp "$profile_test"/sampled.samples.* "$profile_test/invalid-count"
printf '\377\377\377\377\377\377\377\377' |
    dd of="$profile_test/invalid-count" bs=1 seek=24 conv=notrunc status=none
if "$compiler" profile "$profile_test/invalid-count" > "$profile_test/out" 2> "$profile_test/error"; then
    echo 'profiler accepted inconsistent sample totals' >&2
    exit 1
fi
grep -q E_PROFILE "$profile_test/error"
# Termination can interrupt the handler after its serial advances but before
# its category counter. Such a file reports the same completed samples.
profile_samples=$(awk '/^CPU samples:/ { print $3+0 }' "$profile_test/report")
for gap in -1 1 2; do
    cp "$profile_test"/sampled.samples.* "$profile_test/serial-$gap"
    LC_ALL=C awk -v serial="$((profile_samples + gap))" '
        BEGIN {
            for (byte=0; byte<8; byte++) {
                printf "%c", serial % 256
                serial=int(serial / 256)
            }
        }
    ' | dd of="$profile_test/serial-$gap" bs=1 seek=24 conv=notrunc status=none
    if [ "$gap" = 1 ]; then
        "$compiler" profile "$profile_test/serial-$gap" > "$profile_test/interrupted.report"
        cmp "$profile_test/report" "$profile_test/interrupted.report"
    else
        if "$compiler" profile "$profile_test/serial-$gap" > "$profile_test/out" 2> "$profile_test/error"; then
            echo "profiler accepted invalid sample serial gap $gap" >&2
            exit 1
        fi
        grep -q E_PROFILE "$profile_test/error"
    fi
done
printf 'invalid samples\n' > "$profile_test/invalid"
if "$compiler" profile "$profile_test/invalid" > "$profile_test/out" 2> "$profile_test/error"; then
    echo 'profiler accepted invalid input' >&2
    exit 1
fi
grep -q E_PROFILE "$profile_test/error"
printf 'ok profiler (native value, sampled functions and operation kinds, unbounded recursive inclusion, escaped names, refused sampling grant, interrupted sample, invalid input)\n'
