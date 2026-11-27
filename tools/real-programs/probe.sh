#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
source "$repo/tools/real-programs/probe-host.sh"
compiler=${LKJSCRIPT_COMPILER:-"$repo/target/lkjscript"}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
build_probe "$compiler" "$repo/tools/real-programs/probe.lkj" "$repo/library" "$work"
run_probe "$work/select" "$work/probe" node "$repo/tools/real-programs/project.mjs" "$@"
