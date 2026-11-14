#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
profile_test=$(mktemp -d target/profile-test.XXXXXX)
trap 'rm -rf "$profile_test"' EXIT HUP INT TERM
"$compiler" build tests/fixtures/profile.rs -o "$profile_test/plain"
"$compiler" build tests/fixtures/profile.rs --profile -o "$profile_test/sampled"
"$profile_test/plain" > "$profile_test/plain.out"
"$profile_test/sampled" > "$profile_test/sampled.out"
cmp "$profile_test/plain.out" "$profile_test/sampled.out"
"$compiler" profile "$profile_test"/sampled.samples.* > "$profile_test/report"
awk '
  /^CPU samples:/ { total=$3+0; if (total < 10) exit 1 }
  /^[0-9]/ {
    if ($1 < 0 || $1 > 100 || $2 < 0 || $2 > 100 || $3 > total) exit 1
    if ($4 == "work" && $1 > 50) hot=1
    if ($4 == "recurse" && $2 > 50) recursive=1
    if ($4 == "main" && $2 > 50) root=1
  }
  END { if (NR > 40 || !hot || !recursive || !root) exit 1 }
' "$profile_test/report"
"$compiler" ir tests/fixtures/profile.rs -o "$profile_test/plain.lkjir"
sed 's/^fn "work" /fn "work\\ncontrol" /' "$profile_test/plain.lkjir" > "$profile_test/named.lkjir"
"$compiler" build "$profile_test/named.lkjir" --profile -o "$profile_test/named"
"$profile_test/named" > "$profile_test/named.out"
cmp "$profile_test/plain.out" "$profile_test/named.out"
"$compiler" profile "$profile_test"/named.samples.* > "$profile_test/named.report"
grep -qF 'work\u{a}control' "$profile_test/named.report"
test "$(wc -l < "$profile_test/named.report")" -le 40
printf 'invalid samples\n' > "$profile_test/invalid"
if "$compiler" profile "$profile_test/invalid" > "$profile_test/out" 2> "$profile_test/error"; then
    echo 'profiler accepted invalid input' >&2
    exit 1
fi
grep -q E_PROFILE "$profile_test/error"
printf 'ok profiler (native value, sampled hot function, unbounded recursive inclusion, escaped names, invalid input)\n'
