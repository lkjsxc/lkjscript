#!/bin/sh
# Regression: test --library-dir forwards the explicit library to both child
# routes; the added marker is absent from the compiler's embedded library.
# Reproduction: sh tests/diagnostics/test_library_directory.sh COMPILER
set -eu
compiler=$1
test_root=$(mktemp -d target/programs-library-selection.XXXXXX)
trap 'rm -rf "$test_root"' EXIT HUP INT TERM
cp -R library "$test_root/library"
cat >>"$test_root/library/time.lkj" <<'LIBRARY'
pub fn invocation_library_marker() -> i64 { 42i64 }
LIBRARY
cat >"$test_root/case.lkj" <<'CASE'
// expect: value 42
use std::time;
pub fn probe() -> i64 { time::invocation_library_marker() }
CASE
"$compiler" run "$test_root/case.lkj" --engine interpreter --library-dir "$test_root/library" --no-store >"$test_root/direct"
test "$(cat "$test_root/direct")" = 42
"$compiler" test "$test_root/case.lkj" --library-dir "$test_root/library" --no-store --jobs 1 --engines native,interpreter --stats
