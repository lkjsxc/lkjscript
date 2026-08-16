#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT"

TARGET_DIR=${CARGO_TARGET_DIR:-"$ROOT/target"}
case "$TARGET_DIR" in
  /*) ;;
  *) TARGET_DIR="$ROOT/$TARGET_DIR" ;;
esac

cargo build --workspace --release --locked
exec python3 "$ROOT/examples/release-channel/driver.py" \
  "$TARGET_DIR/release/lkjscript" "$TARGET_DIR/release/lkjscriptd"
