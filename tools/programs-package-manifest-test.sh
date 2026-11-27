#!/usr/bin/env bash
# Exercise the independent fixture producer on live Git and archive mutations.
set -euo pipefail
export LC_ALL=C GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
work=$(mktemp -d "$1/package-inventory.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
source="$work/source"
mkdir -p "$source/src"
printf 'permission\n' > "$source/LICENSE"
printf 'pub fn answer() -> u8 { 7 }\n' > "$work/code"
cp "$work/code" "$source/src/lib.rs"
git_at() {
    git -c core.hooksPath=/dev/null -c core.fsmonitor=false -c commit.gpgsign=false \
        -c user.name=Fixture -c user.email=fixture@example.invalid -C "$source" "$@"
}
git_at init -q
git_at add LICENSE src/lib.rs
git_at commit -qm fixture
revision=$(git_at rev-parse HEAD)
cases=0
produce() {
    local want=$1 route=$2 input=$3 reason=$4 checksum=${5:-} status=0
    ((cases+=1))
    local prefix="$work/package-$cases"
    local -a command=(bash tools/programs-package-fixture.sh "$route" "$source" "$input")
    if [[ $route == archive ]]; then command+=("$checksum"); fi
    command+=(fixture 1.0 "$prefix")
    if [[ -p $input ]]; then command=(timeout --kill-after=1s 5s "${command[@]}"); fi
    "${command[@]}" > "$work/tree" 2> "$work/stderr" || status=$?
    if [[ $status != "$want" ]]; then
        printf 'inventory control %s: expected status %s, got %s\n' "$cases" "$want" "$status" >&2
        cat "$work/stderr" >&2
        exit 1
    fi
    if [[ $want == 0 ]]; then
        test ! -s "$work/stderr"
        test -s "$prefix.pin.json"
        test -s "$prefix.manifest.json"
    else
        test ! -e "$prefix.pin.json"
        test ! -e "$prefix.manifest.json"
        test ! -s "$work/tree"
        grep -F -- "$reason" "$work/stderr" > /dev/null
    fi
}
produce 0 git "$revision" ''
cp "$work/tree" "$work/git-tree"
printf 'pub fn answer() -> u8 { 8 }\n' > "$source/src/lib.rs"
produce 1 git "$revision" 'tracked source differs from Git object'
git_at add src/lib.rs
produce 1 git "$revision" 'Git index differs from HEAD'
git_at reset -q HEAD -- src/lib.rs
cp "$work/code" "$source/src/lib.rs"
rm "$source/src/lib.rs"
ln -s ../LICENSE "$source/src/lib.rs"
produce 1 git "$revision" 'symlink source node'
rm "$source/src/lib.rs"
cp "$work/code" "$source/src/lib.rs"

# Independently authored regular-file ustar specimen (Archive::Tar); replay its
# exact tar bytes through the host gzip -n fixture profile for portability.
gzip -dc tools/fixtures/programs-package.tar.gz > "$work/original.tar"
gzip -n -c "$work/original.tar" > "$work/original.crate"
archive_case() {
    local want=$1 bytes=$2 reason=$3 checksum=${4:-}
    if [[ -z $checksum ]]; then read -r checksum _ < <(sha256sum "$bytes"); fi
    produce "$want" archive "$bytes" "$reason" "$checksum"
}
archive_case 0 "$work/original.crate" ''
cmp "$work/git-tree" "$work/tree"
# The independent reference producer's logical digest of the two-file tree.
printf '923c6e0caca7fed9ed39c2f2bf1646b1d1bc91c85c9d076ca048cc86c76aa007\n' > "$work/expected-tree"
cmp "$work/expected-tree" "$work/tree"
printf -v wrong_checksum '%064d' 0
archive_case 1 "$work/original.crate" 'archive checksum mismatch' "$wrong_checksum"
mkfifo "$work/archive-fifo"
archive_case 1 "$work/archive-fifo" 'not a regular file' "$wrong_checksum"
cat "$work/original.crate" "$work/original.crate" > "$work/concat.crate"
archive_case 1 "$work/concat.crate" 'noncanonical compressed payload'
{ cat "$work/original.crate"; printf 'trailing compressed payload'; } > "$work/trailing.crate"
archive_case 1 "$work/trailing.crate" 'invalid compressed payload'
cat "$work/original.tar" "$work/original.tar" | gzip -n -c > "$work/concat-tar.crate"
archive_case 1 "$work/concat-tar.crate" 'trailing tar payload'
cp "$work/original.tar" "$work/short.tar"
truncate -s -1 "$work/short.tar"
gzip -n -c "$work/short.tar" > "$work/short.crate"
archive_case 1 "$work/short.crate" 'tar length is not block-aligned'
cp "$work/original.tar" "$work/bad-header.tar"
printf X | dd of="$work/bad-header.tar" bs=1 conv=notrunc status=none
gzip -n -c "$work/bad-header.tar" > "$work/bad-header.crate"
archive_case 1 "$work/bad-header.crate" 'tar header checksum mismatch'
printf 'different bytes\n' > "$source/src/lib.rs"
archive_case 1 "$work/original.crate" 'extracted source differs from archive member'
git_at add src/lib.rs
git_at commit -qm substitute
substitute=$(git_at rev-parse HEAD)
git_at reset --soft "$revision"
git_at replace "$revision" "$substitute"
produce 1 git "$revision" 'Git index differs from HEAD'
printf 'ok package inventory: %s controls, exact Git/archive trees and compressed/tar framing refusals\n' "$cases"
