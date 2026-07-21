#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
cargo run -q -p lkjscript2026-xtask -- quiet verify
