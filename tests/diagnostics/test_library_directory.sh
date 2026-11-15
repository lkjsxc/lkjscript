#!/bin/sh
# pending: test --library-dir must forward the explicit library to both child
# routes. If it cannot, name the embedded-library fallback instead of reporting
# a missing function from a file that the invocation explicitly replaced.
# Reproduction: sh tests/diagnostics/test_library_directory.sh COMPILER
set -eu
compiler=$1
test_root=$(mktemp -d target/programs-library-selection.XXXXXX)
trap 'rm -rf "$test_root"' EXIT HUP INT TERM
cp -R library "$test_root/library"
cat >>"$test_root/library/time.rs" <<'LIBRARY'
pub fn invocation_library_marker() -> i64 { 42i64 }
LIBRARY
cat >"$test_root/case.rs" <<'CASE'
// expect: value 42
use std::time;
pub fn probe() -> i64 { time::invocation_library_marker() }
CASE
"$compiler" run "$test_root/case.rs" --engine interpreter --library-dir "$test_root/library" --no-store >"$test_root/direct"
test "$(cat "$test_root/direct")" = 42
"$compiler" test "$test_root/case.rs" --library-dir "$test_root/library" --no-store --jobs 1
