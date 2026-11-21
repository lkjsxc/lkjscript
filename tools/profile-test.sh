#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
profile_test=$(mktemp -d target/profile-test.XXXXXX)
profile_key_directory=
trap 'chmod u+w "$profile_test/publish-stage/blocked" 2>/dev/null || :; rm -rf "$profile_test"; if [ -n "$profile_key_directory" ]; then rm -rf "$profile_key_directory"; fi' EXIT HUP INT TERM
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
# The copy-site entry is separate: the original arithmetic/default-report test
# above keeps its existing workload and assertions.
"$compiler" build tools/profile-fixtures.lkj --no-store \
    --grant argv --grant files=. --grant output --grant error --grant status \
    -o "$profile_test/copy-data"
profile_fixture() (
    cd "$profile_test" || exit 1
    ./copy-data "$@"
)
# The shell checks these refusals independently of the native helper.
profile_fixture_refuse() {
    fixture_case=$1
    fixture_reason=$2
    shift 2
    fixture_status=0
    profile_fixture "$@" > "$profile_test/$fixture_case.out" 2> "$profile_test/$fixture_case.err" || fixture_status=$?
    if [ "$fixture_status" -ne 255 ] || [ -s "$profile_test/$fixture_case.out" ] ||
        ! printf '%s\n' "$fixture_reason" | cmp -s - "$profile_test/$fixture_case.err"; then
        printf 'profiler helper %s returned %s, expected 255 with empty stdout and stderr "%s"\n' \
            "$fixture_case" "$fixture_status" "$fixture_reason" >&2
        cat "$profile_test/$fixture_case.err" >&2
        exit 1
    fi
}
profile_digest() {
    sha256sum "$1" | cut -d ' ' -f 1
}
"$compiler" run tests/fixtures/profile.lkj --entry copy_once --engine interpreter --no-store \
    > "$profile_test/copy-once.interpreter"
"$compiler" run tests/fixtures/profile.lkj --entry copy_once --engine native --inline-limit 64 --no-store \
    > "$profile_test/copy-once.native"
cmp "$profile_test/copy-once.interpreter" "$profile_test/copy-once.native"
"$compiler" run tests/fixtures/profile.lkj --entry copy_roles --engine interpreter --inline-limit 0 --no-store \
    > "$profile_test/copy-roles.interpreter"
"$compiler" run tests/fixtures/profile.lkj --entry copy_roles --engine native --inline-limit 0 --no-store \
    > "$profile_test/copy-roles.native"
printf '297\n' > "$profile_test/copy-roles.expected"
cmp "$profile_test/copy-roles.expected" "$profile_test/copy-roles.interpreter"
cmp "$profile_test/copy-roles.interpreter" "$profile_test/copy-roles.native"
"$compiler" build tests/fixtures/profile.lkj --entry copy_roles --inline-limit 0 --no-store \
    --profile --profile-ir "$profile_test/copy-roles.lkjir" -o "$profile_test/copy-roles"
"$profile_test/copy-roles" > "$profile_test/copy-roles.out"
# Standalone scalar images discard their result; run above owns the value ABI.
test ! -s "$profile_test/copy-roles.out"
set -- "$profile_test"/copy-roles.samples.*
test "$#" -eq 1
"$compiler" profile "$1" --copy-sites --profile-ir "$profile_test/copy-roles.lkjir" \
    --limit 100000 --bytes 16777216 > "$profile_test/copy-roles.report"
grep -q '^continuation cursor=none ' "$profile_test/copy-roles.report"
copy_roles_ir_id=$(profile_digest "$profile_test/copy-roles.lkjir")
profile_fixture roles "${1##*/}" copy-roles.lkjir "$copy_roles_ir_id"
"$compiler" build tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --no-store \
    -o "$profile_test/copy-plain"
"$profile_test/copy-plain" > "$profile_test/copy-plain.out"
"$compiler" ir tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --no-store \
    -o "$profile_test/original-inline.lkjir"
"$compiler" ir tests/fixtures/profile.lkj --entry copy_samples --no-opt --no-store \
    -o "$profile_test/original-no-opt.lkjir"
cmp "$profile_test/original-inline.lkjir" "$profile_test/original-no-opt.lkjir"
# The key is outside the default '.' source grant and every repository library
# or runtime path. This fixture owns the store and the temporary key directory.
profile_key_directory=$(mktemp -d /tmp/compiler-profile-key.XXXXXX)
chmod 700 "$profile_key_directory"
(umask 077; head -c 32 /dev/urandom > "$profile_key_directory/key")
mkdir "$profile_test/copy-store"
"$compiler" build tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/copy.lkjir" --store "$profile_test/copy-store" \
    --store-key "$profile_key_directory/key" --stats -o "$profile_test/copy" \
    > "$profile_test/copy-cold.out" 2> "$profile_test/copy-cold.log"
profile_fixture selection original-inline.lkjir copy.lkjir copy-cold.log
profile_fixture_refuse helper-selection 'original call missing' \
    selection copy.lkjir original-inline.lkjir copy-cold.log
cp "$profile_test/copy" "$profile_test/copy-cold"
cp "$profile_test/copy.lkjir" "$profile_test/copy-cold.lkjir"
rm "$profile_test/copy.lkjir"
"$compiler" build tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/copy.lkjir" --store "$profile_test/copy-store" \
    --store-key "$profile_key_directory/key" --stats -o "$profile_test/copy" \
    > "$profile_test/copy-warm.out" 2> "$profile_test/copy-warm.log"
cmp "$profile_test/copy-cold" "$profile_test/copy"
cmp "$profile_test/copy-cold.lkjir" "$profile_test/copy.lkjir"
profile_fixture warm copy-cold.log copy-warm.log copy.lkjir
profile_fixture_refuse helper-warm 'cold fragments were not isolated' \
    warm copy-warm.log copy-cold.log copy.lkjir
"$profile_test/copy" > "$profile_test/copy.out"
cmp "$profile_test/copy-plain.out" "$profile_test/copy.out"
set -- "$profile_test"/copy.samples.*
test "$#" -eq 1
copy_samples=$1
copy_profile_id=$(profile_digest "$copy_samples")
copy_ir_id=$(profile_digest "$profile_test/copy.lkjir")
profile_fixture_refuse helper-roles 'missing retained role copy_role_byte incoming' \
    roles "${copy_samples##*/}" copy.lkjir "$copy_ir_id"
copy_count=$(profile_fixture binding "${copy_samples##*/}" "$copy_ir_id")
copy_report_page() {
    profile_fixture page "$1" "${copy_samples##*/}" "$copy_profile_id" "$copy_ir_id" "$2" "$3" "$4" "$5"
}
"$compiler" profile "$copy_samples" > "$profile_test/copy-default.report"
test "$(wc -l < "$profile_test/copy-default.report")" -le 37
"$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
    --limit 100000 --bytes 16777216 > "$profile_test/copy-all.report"
: > "$profile_test/copy-all.rows"
copy_page_meta=$(copy_report_page copy-all.report 0 100000 16777216 copy-all.rows)
set -- $copy_page_meta
test "$1" = none
test "$2" -eq "$copy_count"

# First page forces a continuation. Later requests widen only the output bound;
# concatenated rows must equal the same fixed full ordering.
"$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
    --limit 1 --bytes 4096 > "$profile_test/copy-first.report"
: > "$profile_test/copy-pages.rows"
copy_page_meta=$(copy_report_page copy-first.report 0 1 4096 copy-pages.rows)
set -- $copy_page_meta
copy_cursor=$1
copy_next=$2
test "$copy_cursor" != none
copy_first_cursor=$copy_cursor
copy_page=1
while [ "$copy_cursor" != none ]; do
    test "$copy_page" -le 64
    "$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
        --limit 64 --bytes 65536 --cursor "$copy_cursor" > "$profile_test/copy-page-$copy_page.report"
    copy_page_meta=$(copy_report_page "copy-page-$copy_page.report" "$copy_next" 64 65536 copy-pages.rows)
    set -- $copy_page_meta
    copy_cursor=$1
    copy_next=$2
    copy_page=$((copy_page + 1))
done
cmp "$profile_test/copy-all.rows" "$profile_test/copy-pages.rows"
copy_first_bytes=$(wc -c < "$profile_test/copy-first.report")
"$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
    --limit 1 --bytes "$copy_first_bytes" > "$profile_test/copy-exact.report"
cmp "$profile_test/copy-first.report" "$profile_test/copy-exact.report"
copy_refuse() {
    case_name=$1
    case_message=$2
    shift 2
    if "$@" > "$profile_test/$case_name.out" 2> "$profile_test/$case_name.err"; then
        echo "profiler accepted refused case $case_name" >&2
        exit 1
    fi
    test ! -s "$profile_test/$case_name.out"
    grep -qF "$case_message" "$profile_test/$case_name.err"
}
copy_refuse bound-first E_PROFILE_BOUND "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes "$((copy_first_bytes - 1))"
copy_refuse bound-header E_PROFILE_BOUND "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 1
copy_end_cursor="4:$copy_profile_id:$copy_ir_id:$copy_count"
copy_refuse bound-empty E_PROFILE_BOUND "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 1 --cursor "$copy_end_cursor"
"$compiler" profile "$copy_samples" --copy-sites --profile-ir "$profile_test/copy.lkjir" \
    --limit 1 --bytes 4096 --cursor "$copy_end_cursor" > "$profile_test/copy-empty.report"
: > "$profile_test/copy-empty.rows"
copy_report_page copy-empty.report "$copy_count" 1 4096 copy-empty.rows > "$profile_test/copy-empty.meta"
test ! -s "$profile_test/copy-empty.rows"
copy_refuse cursor-noncanonical E_PROFILE_CURSOR "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 --cursor "4:$copy_profile_id:$copy_ir_id:00"
copy_refuse cursor-range E_PROFILE_CURSOR "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 \
    --cursor "4:$copy_profile_id:$copy_ir_id:$((copy_count + 1))"
copy_refuse cursor-companion E_PROFILE_CURSOR "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 \
    --cursor "4:$copy_profile_id:$copy_profile_id:1"
copy_refuse usage-bounds E_USAGE "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir"
copy_refuse usage-zero E_USAGE "$compiler" profile "$copy_samples" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 0 --bytes 4096
mkdir "$profile_test/copy-mutations"
profile_fixture mutate "${copy_samples##*/}" copy.lkjir copy-mutations
for mutation in range owner sentinel tag map tail short; do
    copy_refuse "invalid-$mutation" E_PROFILE "$compiler" profile \
        "$profile_test/copy-mutations/bad-$mutation"
done
for mutation in instruction subsite type width; do
    copy_refuse "invalid-$mutation" E_PROFILE_IR "$compiler" profile \
        "$profile_test/copy-mutations/bad-$mutation" --copy-sites \
        --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096
done
copy_refuse wrong-companion 'companion bytes differ' "$compiler" profile "$copy_samples" \
    --copy-sites --profile-ir "$profile_test/copy-mutations/wrong.lkjir" --limit 1 --bytes 4096
copy_refuse unchecked-companion 'companion failed independent IR checking' "$compiler" profile \
    "$profile_test/copy-mutations/unchecked.samples" --copy-sites \
    --profile-ir "$profile_test/copy-mutations/unchecked.lkjir" --limit 1 --bytes 4096
for state in complete serial-leading site-leading unmapped-leading; do
    "$compiler" profile "$profile_test/copy-mutations/count-$state" > "$profile_test/count-$state.default"
    "$compiler" profile "$profile_test/copy-mutations/count-$state" --copy-sites \
        --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 > "$profile_test/count-$state.detail"
    case "$state" in complete|serial-leading) flight=0;; *) flight=1;; esac
    grep -q "^copy-sites .* in-flight=$flight sites=" "$profile_test/count-$state.detail"
done
cmp "$profile_test/count-complete.default" "$profile_test/count-serial-leading.default"
for state in detail-early detail-two detail-missing serial-behind serial-two overflow; do
    copy_refuse "count-$state" E_PROFILE "$compiler" profile "$profile_test/copy-mutations/count-$state"
done
copy_refuse cursor-profile E_PROFILE_CURSOR "$compiler" profile \
    "$profile_test/copy-mutations/count-complete" --copy-sites \
    --profile-ir "$profile_test/copy.lkjir" --limit 1 --bytes 4096 --cursor "$copy_first_cursor"
# Publication errors are deterministic owned-file fixtures. Permission denial
# must still be real under a root-run gate; only that child's DAC bypass is
# dropped. No observer, race, signal injection or global filesystem change.
copy_without_dac_override() {
    if [ "$(id -u)" -eq 0 ]; then
        command -v setpriv >/dev/null
        setpriv --bounding-set=-dac_override,-dac_read_search "$@"
    else
        "$@"
    fi
}
copy_no_publication() {
    test -z "$(find "$1" -maxdepth 1 -type f \( -name 'image-*' -o -name 'inputs-image-*' \) -print)"
    test -z "$(find "$2" -name '.lkjscript-*.tmp' -print)"
}
mkdir "$profile_test/publish-same" "$profile_test/publish-same/store"
printf 'old same output\n' > "$profile_test/publish-same/image"
cp "$profile_test/publish-same/image" "$profile_test/publish-same/expected"
copy_refuse publish-same E_PROFILE_OUTPUT "$compiler" build tests/fixtures/profile.lkj \
    --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/publish-same/image" -o "$profile_test/publish-same/image" \
    --store "$profile_test/publish-same/store" --store-key "$profile_key_directory/key"
cmp "$profile_test/publish-same/image" "$profile_test/publish-same/expected"
copy_no_publication "$profile_test/publish-same/store" "$profile_test/publish-same"
copy_refuse publish-parent-alias E_PROFILE_OUTPUT "$compiler" build tests/fixtures/profile.lkj \
    --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/publish-same/./image" -o "$profile_test/publish-same/image" \
    --store "$profile_test/publish-same/store" --store-key "$profile_key_directory/key"
cmp "$profile_test/publish-same/image" "$profile_test/publish-same/expected"
copy_no_publication "$profile_test/publish-same/store" "$profile_test/publish-same"

mkdir "$profile_test/publish-stage" "$profile_test/publish-stage/store" "$profile_test/publish-stage/blocked"
printf 'old image\n' > "$profile_test/publish-stage/image"
printf 'old companion\n' > "$profile_test/publish-stage/blocked/companion"
cp "$profile_test/publish-stage/image" "$profile_test/publish-stage/expected-image"
cp "$profile_test/publish-stage/blocked/companion" "$profile_test/publish-stage/expected-companion"
chmod 0555 "$profile_test/publish-stage/blocked"
copy_refuse publish-stage "$profile_test/publish-stage/blocked/companion:1:1: E_IO create temporary failed (errno 13)" \
    copy_without_dac_override "$compiler" build tests/fixtures/profile.lkj \
    --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/publish-stage/blocked/companion" -o "$profile_test/publish-stage/image" \
    --store "$profile_test/publish-stage/store" --store-key "$profile_key_directory/key"
chmod u+w "$profile_test/publish-stage/blocked"
cmp "$profile_test/publish-stage/image" "$profile_test/publish-stage/expected-image"
cmp "$profile_test/publish-stage/blocked/companion" "$profile_test/publish-stage/expected-companion"
copy_no_publication "$profile_test/publish-stage/store" "$profile_test/publish-stage"

mkdir "$profile_test/publish-image" "$profile_test/publish-image/store" "$profile_test/publish-image/image"
printf 'old directory image\n' > "$profile_test/publish-image/image/marker"
cp "$profile_test/publish-image/image/marker" "$profile_test/publish-image/expected-marker"
printf 'old companion\n' > "$profile_test/publish-image/companion"
copy_refuse publish-image "$profile_test/publish-image/image:1:1: E_IO rename failed (errno 21)" \
    "$compiler" build tests/fixtures/profile.lkj --entry copy_samples --inline-limit 64 --profile \
    --profile-ir "$profile_test/publish-image/companion" -o "$profile_test/publish-image/image" \
    --store "$profile_test/publish-image/store" --store-key "$profile_key_directory/key"
test -d "$profile_test/publish-image/image"
cmp "$profile_test/publish-image/image/marker" "$profile_test/publish-image/expected-marker"
# The companion rename completed before the image rename failed. Its exact
# checked bytes are valid, but the pair did not publish successfully.
cmp "$profile_test/publish-image/companion" "$profile_test/copy.lkjir"
copy_no_publication "$profile_test/publish-image/store" "$profile_test/publish-image"
printf 'ok profiler (ordinary report, selected IR, copy roles/value, fragment reuse, pagination/bounds, malformed coordinates, independent checking, interrupted accounting, publication refusals)\n'
