#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
compiler_unit_binary=${LKJSCRIPT_BINARY:-target/lkjscript}
mkdir -p target
"$compiler_unit_binary" build src/unit_check_main.lkj --source-dir . --library-dir library --runtime-source library/runtime.lkj -o target/unit-check --stats
target/unit-check
