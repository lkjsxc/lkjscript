#!/usr/bin/env bash
# One-shot HTTP hello smoke: serve one request, curl it, require our server to exit cleanly.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${LKJSCRIPT2026:-${LKJ:-$ROOT/target/debug/lkjscript2026}}"
TMP_DIR="$(mktemp -d)"
PID=""
cleanup() {
  if [[ -n "$PID" ]]; then
    kill "$PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

"$BIN" run "$ROOT/src/examples/http/hello.lkjml" \
  >"$TMP_DIR/server.out" 2>"$TMP_DIR/server.err" &
PID=$!
for _ in 1 2 3 4 5 6 7 8 9 10; do
  if curl -fsS "http://127.0.0.1:8080/" -o "$TMP_DIR/response" 2>/dev/null; then
    break
  fi
  if ! kill -0 "$PID" 2>/dev/null; then
    wait "$PID"
  fi
  sleep 0.1
done
grep -q ok "$TMP_DIR/response"
wait "$PID"
PID=""
printf 'http-smoke ok\n'
