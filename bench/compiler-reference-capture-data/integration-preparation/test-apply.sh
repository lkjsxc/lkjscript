#!/usr/bin/env bash
# Prepared apply command; run only after root accepts the core transaction.
set -euo pipefail
cd /home/coder/workspace/lkjscript
exec /usr/bin/time -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
  /usr/bin/timeout --signal=TERM --kill-after=10s 900s \
  /usr/bin/prlimit --as=4294967296 -- \
  /home/coder/workspace/lkjscript/target/lkjscript edit src/semantic_main.lkj \
  --source-dir /home/coder/workspace/lkjscript \
  --library-dir /home/coder/workspace/lkjscript/library \
  --request target/compiler-reference-capture-integration-preparation-1/tests-request.json \
  --store-key /home/coder/.local/state/lkjscript/store-key
