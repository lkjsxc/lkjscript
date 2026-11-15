#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
if [[ ${1:-} == --compiler ]]; then compiler=$2; shift 2; fi
[[ $# == 0 ]] || exit 2
work=$(mktemp -d "$PWD/target/programs-foreign-session.XXXXXX")
cleanup() { local status=$?; trap - EXIT; if ((status)); then for log in "$work/build.err" "$work/output" "$work/error"; do [[ ! -f "$log" ]] || cat "$log" >&2; done; fi; rm -rf -- "$work"; exit "$status"; }
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
read -r load_before rest < /proc/loadavg
started=$(date +%s%N)
export LKJSCRIPT_FOREIGN_ROOT="$work/root"
mkdir -p "$LKJSCRIPT_FOREIGN_ROOT/.runtime"
for file in /lib64/ld-linux-x86-64.so.2 /lib/x86_64-linux-gnu/libc.so.6 /lib/x86_64-linux-gnu/libpcre2-8.so.0 /lib/x86_64-linux-gnu/libz.so.1; do
    destination=${file##*/}; [[ $destination != ld-linux-x86-64.so.2 ]] || destination=ld.so
    cp -L "$file" "$LKJSCRIPT_FOREIGN_ROOT/.runtime/$destination"
done
env -i PATH=/usr/bin:/bin GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null /usr/bin/git init -q "$LKJSCRIPT_FOREIGN_ROOT"
hash=$(printf 'hello\n' | env -i PATH=/usr/bin:/bin GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null /usr/bin/git -C "$LKJSCRIPT_FOREIGN_ROOT" hash-object -w --stdin)
"$compiler" build examples/foreign-session --library-dir library --no-store -o "$work/session" >"$work/build.out" 2>"$work/build.err"
"$compiler" authority examples/foreign-session --library-dir library >"$work/authority"
for text in ForeignCommand ReadDirectory NullDevice foreign_start foreign_exchange foreign_finish foreign_session_kill_reap_close 'long-lived sessions:'; do grep -Fq "$text" "$work/authority"; done
for engine in standalone native interpreter; do
    read -r engine_before rest < /proc/loadavg
    if [[ $engine == standalone ]]; then command=("$work/session" "$hash")
    else command=("$compiler" run examples/foreign-session --library-dir library --no-store --engine "$engine" -- "$hash")
    fi
    timeout --kill-after=2s 40s "${command[@]}" >"$work/output" 2>"$work/error"
    test ! -s "$work/error"
    grep -E -q '^ok foreign session(:| unavailable:)' "$work/output"
    read -r engine_after rest < /proc/loadavg
    printf '%s load %s -> %s ' "$engine" "$engine_before" "$engine_after"; cat "$work/output"
done
"$compiler" build src/programs_foreign_session_mutation.lkj --library-dir library --no-store --grant output -o "$work/mutation" 2>>"$work/build.err"
"$work/mutation"
read -r load_after rest < /proc/loadavg
elapsed=$(($(date +%s%N)-started))
awk -v elapsed="$elapsed" -v before="$load_before" -v after="$load_after" 'BEGIN{printf "ok foreign-session suite seconds=%.3f; load %s -> %s\n",elapsed/1000000000,before,after}'
