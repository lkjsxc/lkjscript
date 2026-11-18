#!/usr/bin/env bash
set -euo pipefail
cd /home/coder/workspace/lkjscript-semantic-operations-20261118
proof="$PWD/target/semantic-operations/full-gate"
store="$PWD/target/semantic-operations/full-gate-store"
lock=/home/coder/workspace/prompts/talk/lkjscript-gate.lock
expected_source=ecd0ff6d0e2210be5b7eaed9770b675b7c070554
expected_base=739c8cde24f8df4644274451d5f39c23302a8af6
expected_producer=103557eddf30d24e046cb09f6a881e549829283495c8648c0ac9dcd1d72a5d35
expected_seed=9327f64a3c9a5c37146ca6c91a6926a8707d9fef3d9fd67398125909f4b5f612
test "$(git rev-parse HEAD)" = "$expected_source"
git merge-base --is-ancestor "$expected_base" HEAD
git diff --exit-code "$expected_source" -- src library tools tests witness seed
printf '%s  %s\n' "$expected_producer" target/semantic-operations/raw-expression | sha256sum --check
printf '%s  %s\n' "$expected_seed" seed/lkjscript.gz | sha256sum --check
test ! -e "$store"
test ! -e "$proof/gate-started-at.txt"
mkdir -p "$proof"
mkdir "$lock"
trap 'gate_status=$?; date -u +%Y-%m-%dT%H:%M:%SZ > "$proof/gate-finished-at.txt"; printf "%s\n" "$gate_status" > "$proof/gate-exit-status.txt"; rmdir "$lock"; exit "$gate_status"' EXIT
printf 'source=%s\npublished_base=%s\nproducer=%s\nseed=%s\nworkers=4\nstore=%s\nstore_initial=absent\n' "$expected_source" "$expected_base" "$expected_producer" "$expected_seed" "$store" > "$proof/input.txt"
date -u +%Y-%m-%dT%H:%M:%SZ > "$proof/gate-started-at.txt"
cat /proc/loadavg > "$proof/load-before.txt"
/usr/bin/time -f 'wall_seconds=%e user_seconds=%U system_seconds=%S max_rss_kib=%M exit=%x' -o "$proof/time.txt" env LKJSCRIPT_GATE_JOBS=4 LKJSCRIPT_STORE="$store" bash tools/check.sh > "$proof/full-gate.log" 2>&1
cat /proc/loadavg > "$proof/load-after.txt"
cmp target/lkjscript target/semantic-operations/raw-expression
sha256sum target/lkjscript seed/lkjscript.gz > "$proof/output.sha256"
cat "$proof/time.txt"
tail -n 12 "$proof/full-gate.log"
