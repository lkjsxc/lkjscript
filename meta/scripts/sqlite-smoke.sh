#!/usr/bin/env bash
# SQLite source smoke: owned connection/statement handles query an in-memory row.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${LKJSCRIPT_BIN:-$ROOT/target/debug/lkjscript}"
actual="$("$BIN" run --engine vm "$ROOT/src/examples/sqlite/main.lkjscript")"
[[ "$actual" == "42" ]]
printf 'sqlite-smoke ok\n'
