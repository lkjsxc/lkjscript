#!/usr/bin/env sh
set -eu

binary=${LKJSCRIPT_BIN:-target/release/lkjscript}
path=/tmp/lkjscript-durable-files-smoke.txt
trap 'rm -f "$path"' EXIT HUP INT TERM

[ "$("$binary" run src/examples/durable-files/main.lkjscript)" = 6 ]
[ "$(cat "$path")" = record ]
[ "$("$binary" run src/examples/durable-files/main.lkjscript)" = 12 ]
[ "$(cat "$path")" = recordrecord ]
printf '%s\n' 'durable-files-smoke ok'
