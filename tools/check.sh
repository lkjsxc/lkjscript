#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
started=$SECONDS
cargo fmt --check
cargo test --quiet
cargo clippy --quiet --all-targets -- -D warnings
cargo build --quiet
node tools/conformance.mjs
printf 'Full check: %s s\n' "$((SECONDS-started))"
