#!/usr/bin/env bash
# One build and one test execution. Slow controls are explicit #[ignore] tests.
set -euo pipefail
cd "$(dirname "$0")/.."
mode=${1:-routine}
if (($#)); then shift; fi
case "$mode" in
  routine|focused|slow) ;;
  *) printf 'usage: bash tools/check.sh [routine|focused FILTER...|slow FILTER...]\n' >&2; exit 2 ;;
esac
if [[ $mode != routine && $# == 0 ]]; then
  printf '%s checks require a test-name filter\n' "$mode" >&2
  exit 2
fi
if [[ $mode == routine && $# != 0 ]]; then
  printf 'routine checks run the complete routine suite; use focused for filters\n' >&2
  exit 2
fi
if [[ $mode != routine && ( -z $1 || $1 == -* ) ]]; then
  printf '%s checks require a test-name filter before harness options\n' "$mode" >&2
  exit 2
fi
profile=${LKJSCRIPT_CHECK_PROFILE:-test}
default_threads=16
if [[ $mode == slow ]]; then default_threads=1; fi
threads=${RUST_TEST_THREADS:-$default_threads}
if [[ ! $threads =~ ^[1-9][0-9]*$ ]]; then
  printf 'RUST_TEST_THREADS must be a positive integer\n' >&2
  exit 2
fi
if [[ $mode == routine ]]; then
  cargo fmt --all -- --check
  git diff --check
fi
mkdir -p target
scratch=$(mktemp -d "$PWD/target/check.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
started=$SECONDS
if ! cargo test --workspace --all-targets --locked --profile "$profile" --no-run --message-format=json > "$scratch/build.jsonl"; then
  node tools/check-tests.mjs diagnostics "$scratch/build.jsonl"
  exit 1
fi
printf 'build profile=%s elapsed_seconds=%s\n' "$profile" "$((SECONDS - started))"
node tools/check-tests.mjs "$mode" "$scratch/build.jsonl" "$threads" "$@"
