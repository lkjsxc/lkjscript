#!/usr/bin/env bash
# Gate: complete ordered result publication. Optional: killed parent at every phase.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
campaign=0
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --campaign) campaign=1; shift ;;
        *) printf 'usage: %s [--compiler PATH] [--campaign]\n' "$0" >&2; exit 2 ;;
    esac
done
work=$(mktemp -d "$PWD/target/programs-publication.XXXXXX")
parent=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$parent" ]]; then kill "$parent" 2>/dev/null || true; wait "$parent" 2>/dev/null || true; fi
    if ((code)); then
        for file in "$work/build.err" "$work/run.err"; do [[ ! -f "$file" ]] || cat "$file" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
read -r load_before _ </proc/loadavg
started=$(date +%s%N)
cp "$compiler" "$work/compiler"
compiler="$work/compiler"
mkdir "$work/normal"
head -c 32 /dev/urandom >"$work/key"
"$compiler" build-child tests/fixtures/authority/programs_publication_worker.lkj --library-dir library --no-store --image-key "$work/key" -o "$work/worker" >"$work/build.out" 2>"$work/build.err"
"$compiler" build tests/fixtures/authority/programs_pool_publication.lkj --library-dir library --no-store --image-key "$work/key" --grant "image=$work/worker" --grant root=env:PROGRAMS_PUBLICATION_ROOT --grant clock --grant input --grant output -o "$work/parent" >>"$work/build.out" 2>>"$work/build.err"
printf 'old-complete\n' >"$work/normal/published"
printf 'xxxxxx' >"$work/input"
PROGRAMS_PUBLICATION_ROOT="$work/normal" timeout --foreground --kill-after=5s 120s "$work/parent" <"$work/input" >"$work/run.out" 2>"$work/run.err" &
parent=$!
wait "$parent"
parent=
[[ ! -s "$work/run.err" ]]
printf 'pool\ncollected\npartial\nsynced\nrenamed\ncommitted\n' >"$work/phases"
cmp "$work/phases" "$work/run.out"
[[ $(wc -c <"$work/normal/published") == 128 ]]
[[ ! -e "$work/normal/next" ]]
if ((campaign)); then
    cc -std=c11 -O2 -Wall -Wextra -Werror tools/programs-publication-witness.c -o "$work/witness"
    timeout --foreground --kill-after=5s 300s "$work/witness" "$work/parent" "$work" "$work/normal/published" &
    parent=$!
    wait "$parent"
    parent=
fi
ended=$(date +%s%N)
read -r load_after _ </proc/loadavg
awk -v ns="$((ended-started))" -v before="$load_before" -v after="$load_after" -v campaign="$campaign" 'BEGIN { printf "ok pool publication: ordered complete128-byte result; campaign=%d (%.3f s, load %s -> %s)\n", campaign, ns/1000000000, before, after }'
