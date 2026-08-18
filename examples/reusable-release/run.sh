#!/bin/sh
set -eu

repository=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
binary=${1:-"$repository/target/release/lkjscript"}

if [ ! -x "$binary" ]; then
    cargo build --manifest-path "$repository/Cargo.toml" --release --locked
fi

exec python3 "$repository/examples/reusable-release/driver.py" "$binary"
