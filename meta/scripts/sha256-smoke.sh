#!/usr/bin/env sh
set -eu

binary=${LKJSCRIPT_BIN:-target/release/lkjscript}
output=$("$binary" run src/examples/sha256/main.lkjscript)
[ "$output" = '186' ]
printf '%s\n' 'sha256-smoke ok'
