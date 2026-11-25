#!/usr/bin/env bash
# Prepare standalone store source only. This helper never builds or runs it.
set -euo pipefail
(($# == 1)) || { echo 'usage: store-fixture-source.sh DIRECTORY' >&2; exit 2; }
project=$(cd "$(dirname "$0")/.." && pwd)
destination=$1
mkdir -p "$destination/store" "$destination/native/records"
cp "$project/src/diagnostic.lkj" "$project/src/ir.lkj" "$project/src/ir_binary.lkj" \
    "$project/src/reuse.lkj" "$project/src/store.lkj" "$destination/"
cp -a "$project/src/store/." "$destination/store/"
cp "$project/src/native/records.lkj" "$destination/native/records.lkj"
cp "$project/src/native/records/tests.lkj" "$destination/native/records/tests.lkj"
