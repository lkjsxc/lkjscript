#!/usr/bin/env sh
set -eu

binary=${LKJSCRIPT_BIN:-target/release/lkjscript}
path=/tmp/lkjscript-bulk-bytes-smoke.txt
trap 'rm -f "$path"' EXIT HUP INT TERM

output=$("$binary" run src/examples/bulk-bytes/main.lkjscript)
[ "$output" = 'exact bytes: é' ]
[ -f "$path" ]
[ "$(cat "$path")" = 'exact bytes: é' ]
printf '%s\n' 'bulk-bytes-smoke ok'
