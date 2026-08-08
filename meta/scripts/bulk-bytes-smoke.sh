#!/usr/bin/env sh
set -eu

binary=${LKJSCRIPT_BIN:-target/release/lkjscript}
directory=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-bulk-bytes.XXXXXX")
path=$directory/payload.txt
trap 'rm -rf -- "$directory"' EXIT HUP INT TERM

output=$("$binary" run src/examples/bulk-bytes/main.lkjscript -- "$path")
[ "$output" = 'exact bytes: é' ]
[ -f "$path" ]
[ "$(cat "$path")" = 'exact bytes: é' ]
printf '%s\n' 'bulk-bytes-smoke ok'
