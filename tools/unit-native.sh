#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${LKJSCRIPT_COMPILER:-target/lkjscript}
mkdir -p target
"$compiler" runtime-ir > target/unit-native-runtime.lkjir
"$compiler" build src/unit_native_main.lkj --library-dir library --no-store -o target/unit-native-contracts
target/unit-native-contracts
