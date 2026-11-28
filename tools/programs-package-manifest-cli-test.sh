#!/usr/bin/env bash
# Exercise checked CLI policy. The saved Perl oracle is optional, only for
# explicit differential runs, never the native gate.
set -euo pipefail
repo=$(cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo"
source tools/programs-package-manifest-build.sh
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
parent=${2:-${TMPDIR:-/tmp}}
work=$(mktemp -d "$parent/package-manifest-cli.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
export LC_ALL=C GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_COMMON_DIR GIT_NAMESPACE GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT
build_package_manifest "$compiler" "$repo" "$work"
src="$work/source"
mkdir -p "$src/src"
printf 'permission\n' > "$src/LICENSE"
printf 'pub fn answer() -> u8 { 7 }\n' > "$work/code"
cp "$work/code" "$src/src/lib.rs"
git_at() {
    git -c core.hooksPath=/dev/null -c core.fsmonitor=false -c commit.gpgsign=false \
        -c user.name=Fixture -c user.email=fixture@example.invalid -C "$src" "$@"
}
git_at init -q
git_at add LICENSE src/lib.rs
git_at commit -qm fixture
revision=$(git_at rev-parse HEAD)
cases=0
successes=0
refusals=0
prefix="$work/package"
clear_outputs() { rm -f -- "$prefix.manifest.json" "$prefix.pin.json"; }
save_occupied() {
    local path=$1 label=$2
    if [[ -L $path ]]; then readlink -- "$path" > "$work/$label.link"
    elif [[ -e $path ]]; then cp -- "$path" "$work/$label.saved"; fi
}
restore_occupied() {
    local path=$1 label=$2
    if [[ -e $work/$label.link ]]; then ln -s -- "$(cat "$work/$label.link")" "$path"
    elif [[ -e $work/$label.saved ]]; then cp -- "$work/$label.saved" "$path"; fi
}
check_refusal() {
    local path=$1 label=$2
    if [[ -e $work/$label.link ]]; then
        [[ -L $path ]] && readlink -- "$path" | cmp -s - "$work/$label.link"
    elif [[ -e $work/$label.saved ]]; then
        [[ ! -L $path ]] && cmp -s "$path" "$work/$label.saved"
    else [[ ! -e $path && ! -L $path ]]; fi
}
produce() {
    local want=$1 label=$2; shift 2
    ((cases+=1))
    local -a args=(--source "$src" --name fixture --version 1.0 --origin https://example.invalid/fixture \
        --notice LICENSE --output "$prefix" "$@")
    local rc=0 old_rc=0
    rm -f -- "$work/"{manifest,pin}.{saved,link}
    save_occupied "$prefix.manifest.json" manifest
    save_occupied "$prefix.pin.json" pin
    timeout --kill-after=1s 30s "$work/host" "$work/manifest" "${args[@]}" \
        > "$work/stdout" 2> "$work/stderr" || rc=$?
    if [[ $want == ok ]]; then
        if [[ $rc != 0 ]]; then printf 'FAIL %s: expected success, got %s\n' "$label" "$rc" >&2; cat "$work/stderr" >&2; exit 1; fi
        test ! -s "$work/stderr"
        test -s "$prefix.manifest.json"
        test -s "$prefix.pin.json"
        test "$(stat -c %a "$prefix.manifest.json")" = 600
        test "$(stat -c %a "$prefix.pin.json")" = 600
        cp "$prefix.manifest.json" "$work/manifest.actual"
        cp "$prefix.pin.json" "$work/pin.actual"
        ((successes+=1))
    else
        if [[ $rc == 0 || $rc == 124 || $rc == 126 || $rc == 127 || ( $rc -gt 128 && $rc -le 192 ) ]]; then
            printf 'FAIL %s: expected refusal, got %s\n' "$label" "$rc" >&2; cat "$work/stderr" >&2; exit 1
        fi
        test ! -s "$work/stdout"
        check_refusal "$prefix.manifest.json" manifest
        check_refusal "$prefix.pin.json" pin
        ((refusals+=1))
    fi
    if [[ -n ${LKJSCRIPT_MANIFEST_ORACLE:-} ]]; then
        clear_outputs
        restore_occupied "$prefix.manifest.json" manifest
        restore_occupied "$prefix.pin.json" pin
        timeout --kill-after=1s 30s "$LKJSCRIPT_MANIFEST_ORACLE" "${args[@]}" \
            > "$work/oracle.stdout" 2> "$work/oracle.stderr" || old_rc=$?
        if [[ $rc != "$old_rc" ]]; then
            printf 'FAIL differential %s: checked=%s oracle=%s\n' "$label" "$rc" "$old_rc" >&2
            cat "$work/stderr" "$work/oracle.stderr" >&2
            exit 1
        fi
        cmp "$work/stdout" "$work/oracle.stdout"
        if [[ $want == ok ]]; then
            cmp "$prefix.manifest.json" "$work/manifest.actual"
            cmp "$prefix.pin.json" "$work/pin.actual"
        else
            check_refusal "$prefix.manifest.json" manifest
            check_refusal "$prefix.pin.json" pin
        fi
    fi
    clear_outputs
    printf 'ok %s (%s)\n' "$label" "$rc"
}
git_case() { local want=$1 label=$2; shift 2; produce "$want" "$label" --git --revision "$revision" "$@"; }
archive_case() {
    local want=$1 label=$2 file=$3 checksum
    shift 3
    read -r checksum _ < <(sha256sum "$file")
    produce "$want" "$label" --archive "$file" --checksum "$checksum" "$@"
}
gzip_tar() { gzip -n -c "$1" > "$2"; }
git_case ok git-sha1
cp "$work/manifest.actual" "$work/git.manifest"
cp "$work/pin.actual" "$work/child.pin"
bash tools/programs-package-fixture.sh git "$src" "$revision" fixture 1.0 "$work/independent" > "$work/tree"
cmp "$work/git.manifest" "$work/independent.manifest.json"
cmp "$work/child.pin" "$work/independent.pin.json"
printf '923c6e0caca7fed9ed39c2f2bf1646b1d1bc91c85c9d076ca048cc86c76aa007\n' | cmp - "$work/tree"
# Real public invocation including its separate build and scratch cleanup.
LKJSCRIPT_COMPILER="$compiler" bash tools/programs-package-manifest.sh --source "$src" --git --revision "$revision" \
    --name fixture --version 1.0 --origin https://example.invalid/fixture --notice LICENSE --output "$work/public" > "$work/public.stdout"
cmp "$work/public.manifest.json" "$work/git.manifest"
cmp "$work/public.pin.json" "$work/child.pin"
cmp "$work/public.stdout" "$work/stdout"

git_case refuse wrong-revision --revision 0000000000000000000000000000000000000000
mkdir "$work/nonrepo"
GIT_CEILING_DIRECTORIES="$work" git_case refuse nonrepo --source "$work/nonrepo"
git_case refuse missing-root --source "$work/not-a-source"
git_case refuse source-not-directory --source "$src/LICENSE"
git_case refuse revision-alias --revision HEAD
git_case refuse invalid-name --name 'bad/name'
git_case refuse duplicate-notice --notice LICENSE
git_case refuse unlisted-notice --notice MISSING
git_case refuse path-dot --notice ./LICENSE
git_case refuse path-empty-component --notice src//lib.rs
git_case refuse path-absolute --notice /LICENSE
git_case refuse path-invalid-utf8 --notice $'bad\377path'
git_case refuse path-parent --notice src/../LICENSE
git_case refuse path-backslash --notice 'src\lib.rs'
git_case refuse path-control --notice $'bad\tpath'
git_case refuse invalid-identity --origin $'bad\norigin'
printf 'changed\n' > "$src/src/lib.rs"
git_case refuse dirty
git_at add src/lib.rs
git_case refuse staged
git_at reset -q HEAD -- src/lib.rs
cp "$work/code" "$src/src/lib.rs"
rm "$src/src/lib.rs"
ln -s ../LICENSE "$src/src/lib.rs"
git_case refuse symlink-leaf
rm "$src/src/lib.rs"
mkfifo "$src/src/lib.rs"
git_case refuse fifo-leaf
rm "$src/src/lib.rs"
cp "$work/code" "$src/src/lib.rs"
mv "$src/src" "$src/moved"
ln -s moved "$src/src"
git_case refuse symlink-parent
rm "$src/src"
mv "$src/moved" "$src/src"
GIT_DIR="$work/not-a-repo" GIT_WORK_TREE="$work" GIT_INDEX_FILE="$work/not-an-index" \
    GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.fsmonitor GIT_CONFIG_VALUE_0=/false git_case ok sanitized-git
printf 'untouched\n' > "$prefix.manifest.json"
git_case refuse exclusive-manifest
printf 'untouched\n' > "$prefix.pin.json"
git_case refuse exclusive-pin-rollback
ln -s "$work/target-that-must-not-exist" "$prefix.pin.json"
git_case refuse exclusive-symlink-rollback
test ! -e "$work/target-that-must-not-exist"
sed 's/"fixture"/"zchild"/' "$work/child.pin" > "$work/zchild.pin"
sed 's/"fixture"/"achild"/' "$work/child.pin" > "$work/achild.pin"
git_case ok dependencies --dependency-pin "$work/zchild.pin" --dependency-pin "$work/achild.pin"
cp "$work/manifest.actual" "$work/with-deps"
git_case ok reversed-dependencies --dependency-pin "$work/achild.pin" --dependency-pin "$work/zchild.pin"
cmp "$work/manifest.actual" "$work/with-deps"
git_case refuse duplicate-dependency --dependency-pin "$work/child.pin" --dependency-pin "$work/child.pin"
{ printf ' '; cat "$work/child.pin"; } > "$work/noncanonical.pin"
git_case refuse noncanonical-pin --dependency-pin "$work/noncanonical.pin"
printf '[1,2,3]' > "$work/malformed.pin"
git_case refuse malformed-pin --dependency-pin "$work/malformed.pin"
ln -s "$work/child.pin" "$work/linked.pin"
git_case refuse symlink-pin --dependency-pin "$work/linked.pin"
mkfifo "$work/fifo.pin"
git_case refuse fifo-pin --dependency-pin "$work/fifo.pin"
# JSON::PP numeric spelling must round-trip even inside otherwise opaque
# nested pin dependency data. Real package identity fields are strings.
numeric_pin() {
    local want=$1 label=$2 number=$3
    sed "s/,\[\]]$/,[${number}]]/" "$work/child.pin" > "$work/number.pin"
    git_case "$want" "$label" --dependency-pin "$work/number.pin"
}
numeric_pin ok canonical-integer 18446744073709551615
numeric_pin ok canonical-fraction 1.5
numeric_pin ok canonical-scientific 1.2e+15
numeric_pin ok canonical-small 1e-05
numeric_pin ok canonical-subnormal 4.94065645841247e-324
numeric_pin refuse noncanonical-fraction 1.0
numeric_pin refuse noncanonical-negative-zero -0
numeric_pin refuse noncanonical-exponent 1e-5
numeric_pin refuse noncanonical-exponent-integer 1e+19
numeric_pin refuse noncanonical-huge-integer 18446744073709551616
numeric_pin refuse noncanonical-rounded-fraction 1.2345678901234567
numeric_pin refuse noncanonical-subnormal 1e-323
gzip -dc tools/fixtures/programs-package.tar.gz > "$work/original.tar"
gzip_tar "$work/original.tar" "$work/original.crate"
archive_case ok archive "$work/original.crate"
grep -Fq "$(cat "$work/tree")" "$work/pin.actual"
mkdir "$work/fixture-1.0"
cp -a "$src/LICENSE" "$src/src" "$work/fixture-1.0/"
tar --format=ustar -cf "$work/dirs.tar" -C "$work" fixture-1.0
gzip_tar "$work/dirs.tar" "$work/dirs.crate"
archive_case ok archive-directories "$work/dirs.crate"
# Prove the first block type, rather than assuming a gzip compression level
# selects a particular DEFLATE representation.
block_type() {
    local byte
    read -r byte < <(od -An -tu1 -j10 -N1 "$1")
    test "$(((byte >> 1) & 3))" = "$2"
}
block_type "$work/original.crate" 1
archive_case ok gzip-fixed "$work/original.crate"

# Make an exact single stored block with an independently generated gzip
# envelope and trailer. The known fixture fits in one 16-bit LEN field.
emit_byte() { printf '%b' "\\$(printf '%03o' "$(($1 & 255))")"; }
size=$(wc -c < "$work/original.tar")
test "$size" -lt 65536
{
    head -c10 "$work/original.crate"
    printf '\001'
    emit_byte "$size"; emit_byte "$((size >> 8))"
    emit_byte "$((65535 ^ size))"; emit_byte "$(((65535 ^ size) >> 8))"
    cat "$work/original.tar"
    tail -c8 "$work/original.crate"
} > "$work/stored.crate"
block_type "$work/stored.crate" 0
archive_case ok gzip-stored "$work/stored.crate"

# Exercise dynamic codes, multiple blocks, long backreferences, >128 KiB
# binary archive/source responses and >128 KiB request/output frames. The
# fixed-seed integer generator emits a reproducible 6-bit byte distribution.
awk 'BEGIN { x=1; for(i=0;i<262144;i++){x=(48271*x)%2147483647;printf "%c",int(x/33554432)} }' > "$src/src/large.bin"
cp "$src/src/large.bin" "$work/fixture-1.0/src/large.bin"
tar --format=ustar -cf "$work/large.tar" -C "$work" fixture-1.0
gzip_tar "$work/large.tar" "$work/large.crate"
test "$(wc -c < "$work/large.crate")" -gt 131072
test "$(wc -c < "$src/src/large.bin")" -gt 131072
block_type "$work/large.crate" 2
awk '{sub(/https:\/\/example.invalid\/fixture/,"@");split($0,a,"@");printf "%s",a[1];for(i=0;i<140000;i++)printf "x";printf "%s",a[2]}' "$work/child.pin" > "$work/large.pin"
test "$(wc -c < "$work/large.pin")" -gt 131072
archive_case ok gzip-dynamic-large "$work/large.crate" --dependency-pin "$work/large.pin"
test "$(wc -c < "$work/manifest.actual")" -gt 131072
test "$(wc -c < "$work/stdout")" -gt 131072
rm "$src/src/large.bin" "$work/fixture-1.0/src/large.bin"

# Exercise optional gzip header fields, including the low 16 bits of the
# header CRC32, independently computed with gzip's normal data trailer.
{
    printf '\037\213\010\036\000\000\000\000\000\003'
    printf '\007\000XY\003\000xyzfixture\000comment\000'
} > "$work/optional.header"
{
    cat "$work/optional.header"
    gzip -n -c "$work/optional.header" | tail -c8 | head -c2
    tail -c+11 "$work/original.crate"
} > "$work/optional.crate"
gzip -t "$work/optional.crate"
LKJSCRIPT_MANIFEST_ORACLE= archive_case ok gzip-optional-header-crc "$work/optional.crate"
{
    printf "\037\213\010\034"
    tail -c+5 "$work/optional.header"
    tail -c+11 "$work/original.crate"
} > "$work/optional-no-crc.crate"
archive_case ok gzip-optional-header "$work/optional-no-crc.crate"
cp "$work/optional.crate" "$work/bad-header-crc.crate"
header_size=$(wc -c < "$work/optional.header")
read -r byte < <(od -An -tu1 -j"$header_size" -N1 "$work/optional.crate")
emit_byte "$((byte ^ 1))" | dd of="$work/bad-header-crc.crate" bs=1 seek="$header_size" conv=notrunc status=none
archive_case refuse gzip-bad-header-crc "$work/bad-header-crc.crate"

# Isolate invalid CRC32 and ISIZE (archive checksum is recomputed per case).
gzip_size=$(wc -c < "$work/original.crate")
for field in crc isize; do
    offset=$((gzip_size - 8))
    if [[ $field == isize ]]; then offset=$((gzip_size - 4)); fi
    read -r byte < <(od -An -tu1 -j"$offset" -N1 "$work/original.crate")
    cp "$work/original.crate" "$work/bad-$field.crate"
    emit_byte "$((byte ^ 1))" | dd of="$work/bad-$field.crate" bs=1 seek="$offset" conv=notrunc status=none
    archive_case refuse "gzip-bad-$field" "$work/bad-$field.crate"
done
cp "$work/stored.crate" "$work/bad-len.crate"
printf '\000' | dd of="$work/bad-len.crate" bs=1 seek=13 conv=notrunc status=none
archive_case refuse gzip-bad-stored-len "$work/bad-len.crate"
cp "$work/original.crate" "$work/reserved-block.crate"
printf '\007' | dd of="$work/reserved-block.crate" bs=1 seek=10 conv=notrunc status=none
archive_case refuse gzip-reserved-block "$work/reserved-block.crate"
cp "$work/original.crate" "$work/truncated-gzip.crate"
truncate -s -1 "$work/truncated-gzip.crate"
archive_case refuse gzip-truncated-trailer "$work/truncated-gzip.crate"
printf -v wrong '%064d' 0
produce refuse checksum --archive "$work/original.crate" --checksum "$wrong"
mkfifo "$work/archive-fifo"
produce refuse fifo-archive --archive "$work/archive-fifo" --checksum "$wrong"
grep -Fq 'not a regular file' "$work/stderr"
ln -s "$work/original.crate" "$work/archive-link"
produce refuse symlink-archive --archive "$work/archive-link" --checksum "$wrong"
cat "$work/original.crate" "$work/original.crate" > "$work/multi.crate"
archive_case refuse multi-stream "$work/multi.crate"
{ cat "$work/original.crate"; printf 'trailing'; } > "$work/trailing.crate"
archive_case refuse trailing-compressed "$work/trailing.crate"
{ cat "$work/original.crate"; printf '\0'; } > "$work/trailing-zero.crate"
archive_case refuse trailing-zero "$work/trailing-zero.crate"
cat "$work/original.tar" "$work/original.tar" > "$work/multi.tar"
gzip_tar "$work/multi.tar" "$work/multi-tar.crate"
archive_case refuse trailing-tar "$work/multi-tar.crate"
cp "$work/original.tar" "$work/short.tar"
truncate -s -1 "$work/short.tar"
gzip_tar "$work/short.tar" "$work/short.crate"
archive_case refuse bad-block-alignment "$work/short.crate"
cp "$work/original.tar" "$work/bad-header.tar"
printf X | dd of="$work/bad-header.tar" bs=1 conv=notrunc status=none
gzip_tar "$work/bad-header.tar" "$work/bad-header.crate"
archive_case refuse header-checksum "$work/bad-header.crate"
mutate_header() {
    local label=$1 offset=$2 value=$3 sum
    cp "$work/original.tar" "$work/$label.tar"
    printf '%s' "$value" | dd of="$work/$label.tar" bs=1 seek="$offset" conv=notrunc status=none
    sum=$(od -An -v -tu1 -N512 "$work/$label.tar" | awk \
        '{for(i=1;i<=NF;i++){s+=(at>=148&&at<156?32:$i);at++}} END{print s}')
    printf '%06o\0 ' "$sum" | dd of="$work/$label.tar" bs=1 seek=148 conv=notrunc status=none
    gzip_tar "$work/$label.tar" "$work/$label.crate"
    archive_case refuse "$label" "$work/$label.crate"
}
mutate_header tar-symlink 156 2
mutate_header tar-hardlink 156 1
mutate_header tar-extension 156 x
mutate_header tar-directory-payload 156 5
mutate_header tar-nonoctal 124 9
mutate_header tar-wrong-prefix 345 X
# Non-POSIX prefix bytes are not path components. Empty member names are
# invalid even when a POSIX prefix would synthesize a plausible directory.
# Archive::Tar may repair these fields, so the tightened policy is checked
# independently rather than requiring legacy malformed-input parity.
strict_tar_case() {
    local label=$1 raw=$2 sum
    sum=$(od -An -v -tu1 -N512 "$raw" | awk \
        '{for(i=1;i<=NF;i++){s+=(at>=148&&at<156?32:$i);at++}} END{print s}')
    printf '%06o\0 ' "$sum" | dd of="$raw" bs=1 seek=148 conv=notrunc status=none
    gzip_tar "$raw" "$work/$label.crate"
    LKJSCRIPT_MANIFEST_ORACLE= archive_case refuse "$label" "$work/$label.crate"
}
cp "$work/dirs.tar" "$work/gnu-prefix.tar"
printf 'ustar ' | dd of="$work/gnu-prefix.tar" bs=1 seek=257 conv=notrunc status=none
printf 'fixture-1.0' | dd of="$work/gnu-prefix.tar" bs=1 seek=345 conv=notrunc status=none
strict_tar_case tar-gnu-metadata "$work/gnu-prefix.tar"
grep -Fq 'non-POSIX tar prefix metadata' "$work/stderr"
cp "$work/dirs.tar" "$work/empty-name.tar"
dd if=/dev/zero of="$work/empty-name.tar" bs=100 count=1 conv=notrunc status=none
printf 'fixture-1.0' | dd of="$work/empty-name.tar" bs=1 seek=345 conv=notrunc status=none
strict_tar_case tar-empty-name "$work/empty-name.tar"
grep -Fq 'empty tar member name' "$work/stderr"
printf 'different bytes\n' > "$src/src/lib.rs"
archive_case refuse extracted-mismatch "$work/original.crate"
git_at add src/lib.rs
git_at commit -qm substitute
substitute=$(git_at rev-parse HEAD)
git_at reset --soft "$revision"
git_at replace "$revision" "$substitute"
git_case refuse git-replace
src="$work/sha256"
mkdir "$src"
git_at init -q --object-format=sha256
printf 'permission\n' > "$src/LICENSE"
printf '\000\377binary\n' > "$src/"$'caf\303\251 "quote"'
chmod +x "$src/"$'caf\303\251 "quote"'
git_at add .
git_at commit -qm sha256
revision=$(git_at rev-parse HEAD)
git_case ok git-sha256-unicode
git_case ok utf8-notice --notice $'caf\303\251 "quote"'
git_case ok utf8-identity --origin $'https://example.invalid/caf\303\251'
printf 'ok checked package manifest CLI: %s controls (%s successes, %s refusals)\n' "$cases" "$successes" "$refusals"
