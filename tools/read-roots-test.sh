#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
confinement=0
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --confinement) confinement=1; shift ;;
        *) printf 'usage: %s [--compiler PATH] [--confinement]\n' "$0" >&2; exit 2 ;;
    esac
done
work=$(mktemp -d "$PWD/target/programs-read-roots.XXXXXX")
child=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$child" ]]; then
        # Each recorded child is our non-foreground timeout process-group leader.
        kill -KILL -- "-$child" 2>/dev/null || kill -KILL "$child" 2>/dev/null || true
        wait "$child" 2>/dev/null || true
    fi
    if ((code)); then
        for log in "$work/build.err" "$work/output" "$work/error"; do [[ ! -f "$log" ]] || cat "$log" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
started=$(date +%s%N)
before=$(cat /proc/loadavg)
export RO_ROOT="$work/root"
read_grants=(--grant root=env:RO_ROOT --grant input --grant output)
write_grants=(--grant root=env:RO_ROOT --grant output)
"$compiler" build tests/fixtures/authority/read_root.lkj --library-dir library --no-store "${read_grants[@]}" -o "$work/read" 2>"$work/build.err"
"$compiler" build tests/fixtures/authority/file_seek.lkj --library-dir library --no-store "${write_grants[@]}" -o "$work/seek" 2>>"$work/build.err"
"$compiler" authority tests/fixtures/authority/read_root.lkj --library-dir library >"$work/authority"
grep -q 'root: ReadDirectory' "$work/authority"
if grep -Eq '(^| )file_(write|set_private|executable|truncate|lock)' "$work/authority"; then
    printf 'read-root authority contains mutation\n' >&2; exit 1
fi
for engine in standalone native interpreter; do
    mkdir -p "$work/root/sub"
    printf 'outside\n' >"$work/outside.txt"
    printf 'before\n' >"$work/root/snapshot"
    touch -d '@1' "$work/root/snapshot"
    printf 'after\n' >"$work/root/next"
    touch -d '@2' "$work/root/next"
    printf 'inside\n' >"$work/root/sub/nested"
    printf 'old\n' >"$work/root/old-time"
    touch -d '@-1' "$work/root/old-time"
    ln -s ../outside.txt "$work/root/escape"
    ln -s sub/nested "$work/root/inside-link"
    mkfifo "$work/root/pipe" "$work/control"
    exec 9<>"$work/control"
    if [[ "$engine" = standalone ]]; then
        timeout --kill-after=1s 20s "$work/read" <"$work/control" 9>&- >"$work/output" 2>"$work/error" &
    else
        timeout --kill-after=1s 20s "$compiler" run tests/fixtures/authority/read_root.lkj --library-dir library --no-store "${read_grants[@]}" --engine "$engine" <"$work/control" 9>&- >"$work/output" 2>"$work/error" &
    fi
    child=$!
    ready=no
    for ((attempt=0;attempt<1000;attempt++)); do
        if grep -q '^opened$' "$work/output"; then ready=yes; break; fi
        kill -0 "$child" 2>/dev/null || break
        sleep 0.01
    done
    [[ "$ready" = yes ]]
    mv "$work/root/next" "$work/root/snapshot"
    printf 'x' >&9
    wait "$child"
    child=
    exec 9>&-
    printf 'opened\nok read roots\n' >"$work/expected"
    cmp "$work/expected" "$work/output"
    test ! -s "$work/error"
    rm -f "$work/control"
    printf 'protected\n' >"$work/root/protected"
    ln -s protected "$work/root/link"
    if [[ "$engine" = standalone ]]; then
        "$work/seek" >"$work/output"
    else
        "$compiler" run tests/fixtures/authority/file_seek.lkj --library-dir library --no-store "${write_grants[@]}" --engine "$engine" >"$work/output"
    fi
    printf 'ok file seek\n' >"$work/expected"
    cmp "$work/expected" "$work/output"
    printf 'protected\n' >"$work/expected"
    cmp "$work/expected" "$work/root/protected"
    rm -rf -- "$work/root"
done
# A program directory resolves a relative read-root grant from its own directory.
mkdir -p "$work/relative/data"
printf 'relative\n' >"$work/relative/data/body"
cat >"$work/relative/main.lkj" <<'SOURCE'
pub fn main(mut root: ReadDirectory) {
    assert!(root.open("body").unwrap().read_to_end().unwrap().as_slice() == b"relative\n");
}
SOURCE
cat >"$work/relative/program.lkj" <<'MANIFEST'
pub const SOURCE: &str = "main.lkj";
pub const ENTRY: &str = "main";
pub const GRANTS: &[&str] = &["root=data"];
pub const CASES: &[&str] = &[];
MANIFEST
"$compiler" build "$work/relative" --library-dir library --no-store -o "$work/relative-run" 2>"$work/build.err"
"$work/relative-run"
if ((confinement)); then
cc -std=c11 -O2 -Wall -Wextra -Werror tools/openat2-unavailable.c -o "$work/no-openat2"
export RO_OUTSIDE="$work/outside/value"
fallback_grants=(--grant root=env:RO_ROOT --grant read_root=env:RO_ROOT --grant input --grant output --grant absolute_path=RO_OUTSIDE)
"$compiler" build tests/fixtures/authority/read_root_fallback.lkj --library-dir library --no-store "${fallback_grants[@]}" -o "$work/fallback" 2>"$work/build.err"
for engine in standalone native interpreter; do
    mkdir -p "$work/root/sub" "$work/root/anchor" "$work/outside"
    printf 'protected\n' >"$work/root/protected"
    printf 'inside\n' >"$work/root/anchor/value"
    printf 'outside\n' >"$work/outside/value"
    printf 'outside\n' >"$work/outside.txt"
    ln -s protected "$work/root/inside-link"
    ln -s ../outside.txt "$work/root/escape"
    ln -s sub "$work/root/link-sub"
    mkfifo "$work/root/pipe" "$work/control"
    exec 9<>"$work/control"
    if [[ "$engine" = standalone ]]; then
        timeout --kill-after=1s 20s "$work/no-openat2" "$work/fallback" <"$work/control" 9>&- >"$work/output" 2>"$work/error" &
    else
        timeout --kill-after=1s 20s "$work/no-openat2" "$compiler" run tests/fixtures/authority/read_root_fallback.lkj --library-dir library --no-store "${fallback_grants[@]}" --engine "$engine" <"$work/control" 9>&- >"$work/output" 2>"$work/error" &
    fi
    child=$!
    ready=no
    for ((attempt=0;attempt<1000;attempt++)); do
        if grep -q '^anchored$' "$work/output"; then ready=yes; break; fi
        kill -0 "$child" 2>/dev/null || break
        sleep 0.01
    done
    [[ "$ready" = yes ]]
    mv "$work/root/anchor" "$work/root/renamed"
    ln -s ../outside "$work/root/anchor"
    printf 'x' >&9
    wait "$child"
    child=
    exec 9>&-
    printf 'anchored\nok fallback\n' >"$work/expected"
    cmp "$work/expected" "$work/output"
    test ! -s "$work/error"
    printf 'protected\n' >"$work/expected"
    cmp "$work/expected" "$work/root/protected"
    printf 'outside\n' >"$work/expected"
    cmp "$work/expected" "$work/outside/value"
    test "$(stat -c %a "$work/root/sub/private")" = 600
    rm -rf -- "$work/root"
    rm -f "$work/control"
done
fi
elapsed=$(($(date +%s%N)-started))
printf 'ok read-root regular files, containment, inode replacement, mtime, O_APPEND seek/no-follow'
if ((confinement)); then printf ', ENOSYS anchored fallback'; fi
printf '; seconds='
awk -v ns="$elapsed" 'BEGIN {printf "%.3f", ns/1000000000}'
printf '; load %s -> %s\n' "$before" "$(cat /proc/loadavg)"
