#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
cargo run -q -p lkjscript-xtask -- quiet verify
