#!/usr/bin/env bash
# One-shot HTTP hello smoke: serve one request, curl it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${LKJSCRIPT2026:-${LKJ:-$ROOT/target/debug/lkjscript2026}}"
PORT="${PORT:-18080}"
# patch: hello hardcodes 8080 — use env by running with default and map, or rewrite
# For smoke, use 8080 if free else skip. Prefer spawning with fixed 8080.
if ss -ltn 2>/dev/null | grep -q ':8080 '; then
  echo "http-smoke skip: :8080 busy"
  exit 0
fi
"$BIN" run "$ROOT/examples/http/hello.lkjsxc" &
PID=$!
trap 'kill $PID 2>/dev/null || true' EXIT
for i in 1 2 3 4 5 6 7 8 9 10; do
  if curl -fsS "http://127.0.0.1:8080/" -o /tmp/http-smoke.out 2>/dev/null; then
    break
  fi
  sleep 0.1
done
grep -q ok /tmp/http-smoke.out
wait "$PID" || true
printf 'http-smoke ok\n'
