#!/usr/bin/env bash
# Kernel feasibility witness; never a child-launch helper used by lkjscript.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p target
fixture="$(mktemp -d "$PWD/target/foreign-child-proof.XXXXXX")"
trap 'rm -rf -- "$fixture"' EXIT
root="$fixture/root"
before="$(cut -d' ' -f1-3 /proc/loadavg)"
start="$(date +%s%N)"
mkdir -p "$root/.runtime"
cc -std=c11 -O2 -Wall -Wextra -Werror tools/foreign-child-proof.c -o "$fixture/proof"
cp -L /lib64/ld-linux-x86-64.so.2 "$root/.runtime/ld.so"
cp -L /lib/x86_64-linux-gnu/libc.so.6 "$root/.runtime/libc.so.6"
cp -L /lib/x86_64-linux-gnu/libpcre2-8.so.0 "$root/.runtime/libpcre2-8.so.0"
cp -L /lib/x86_64-linux-gnu/libz.so.1 "$root/.runtime/libz.so.1"
env -i PATH=/usr/bin:/bin GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null /usr/bin/git init -q "$root"
printf 'inside\n' > "$root/inside.txt"
printf 'outside secret\n' > "$fixture/outside.txt"
ln -s "$fixture/outside.txt" "$root/escape"
"$fixture/proof" "$root" "$fixture/outside.txt" "$fixture/proof"
end="$(date +%s%N)"
after="$(cut -d' ' -f1-3 /proc/loadavg)"
printf 'foreign-child proof: %d.%03d s, load %s -> %s\n' "$(((end-start)/1000000000))" "$((((end-start)/1000000)%1000))" "$before" "$after"
