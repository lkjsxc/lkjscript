#!/usr/bin/env bash
# SQLite source smoke: owned connection/statement handles query a durable row.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${LKJSCRIPT_BIN:-$ROOT/target/debug/lkjscript}"
DIRECTORY=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-sqlite.XXXXXX")
DB=$DIRECTORY/example.db
trap 'rm -rf -- "$DIRECTORY"' EXIT
actual="$("$BIN" run "$ROOT/src/examples/sqlite/main.lkjscript" -- "$DB")"
[[ "$actual" == "42" ]]
printf 'sqlite-smoke ok\n'
