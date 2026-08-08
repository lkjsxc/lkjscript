#!/usr/bin/env sh
set -eu

binary=${LKJSCRIPT_BIN:-target/release/lkjscript}
directory=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-durable-files.XXXXXX")
path=$directory/records.txt
trap 'rm -rf -- "$directory"' EXIT HUP INT TERM

[ "$("$binary" run src/examples/durable-files/main.lkjscript -- "$path")" = 6 ]
[ "$(cat "$path")" = record ]
[ "$("$binary" run src/examples/durable-files/main.lkjscript -- "$path")" = 12 ]
[ "$(cat "$path")" = recordrecord ]
printf '%s\n' 'durable-files-smoke ok'
