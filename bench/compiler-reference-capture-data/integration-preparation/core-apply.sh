#!/usr/bin/env bash
# Prepared apply command; not executed by the author. Root owns release/captures.
set -euo pipefail
cd /home/coder/workspace/lkjscript
exec /usr/bin/time -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
  /usr/bin/timeout --signal=TERM --kill-after=10s 900s \
  /usr/bin/prlimit --as=4294967296 -- \
  /home/coder/workspace/lkjscript/target/lkjscript edit src/main.lkj \
  --source-dir /home/coder/workspace/lkjscript \
  --library-dir /home/coder/workspace/lkjscript/library \
  --against src/main.lkj \
  --against-source-dir /home/coder/workspace/lkjscript/target/compiler-reference-capture-desired-1 \
  --against-library-dir /home/coder/workspace/lkjscript/target/compiler-reference-capture-desired-1/library \
  --store-key /home/coder/.local/state/lkjscript/store-key
