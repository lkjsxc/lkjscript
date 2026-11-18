#!/usr/bin/env bash
set -euo pipefail
cd /home/coder/workspace/lkjscript-programs-anonymous-20261118
proof=$PWD/bench/anonymous-files-data
lock=/home/coder/workspace/prompts/talk/lkjscript-gate.lock
mkdir "$lock"
trap 'status=$?; printf "%s\n" "$status" > "$proof/full-gate.exit"; rmdir "$lock"; exit "$status"' EXIT
[[ $(git rev-parse origin/main) == 6f6236afe* ]]
sha256sum --check "$proof/producer.sha256"
sha256sum --check "$proof/seed.sha256"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$proof/full-gate.started"
LKJSCRIPT_GATE_JOBS=4 LKJSCRIPT_STORE="$PWD/target/store-anonymous-full" bash tools/check.sh > "$proof/full-gate.log" 2>&1
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$proof/full-gate.finished"
bash tools/real-programs-test.sh target/lkjscript > "$proof/standalone-real-programs.log" 2>&1
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$proof/standalone-real-programs.finished"
sha256sum --check "$proof/producer.sha256"
sha256sum --check "$proof/seed.sha256"
printf 'anonymous-file full gate and standalone real-program proof passed\n'
