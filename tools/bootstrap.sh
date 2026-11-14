#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p target
bootstrap_directory=$(mktemp -d target/bootstrap.XXXXXX)
trap 'rm -rf "$bootstrap_directory"' EXIT HUP INT TERM
gzip -dc seed/lkjscript.gz > "$bootstrap_directory/lkjscript"
chmod +x "$bootstrap_directory/lkjscript"
"$bootstrap_directory/lkjscript" build src/main.rs --library-dir library --runtime-source library/runtime.rs -o target/lkjscript
target/lkjscript check-library --library-dir library
target/lkjscript build src/main.rs --library-dir library --runtime-source library/runtime.rs -o "$bootstrap_directory/reproduced"
cmp target/lkjscript "$bootstrap_directory/reproduced"
printf 'ok bootstrap\n'
