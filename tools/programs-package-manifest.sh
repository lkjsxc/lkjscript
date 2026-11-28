#!/usr/bin/env bash
# Public exact Git / checksum-pinned archive inventory CLI.
set -euo pipefail
repo=$(cd -- "$(dirname -- "$0")/.." && pwd)
source "$repo/tools/programs-package-manifest-build.sh"
compiler=${LKJSCRIPT_COMPILER:-"$repo/target/lkjscript"}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
build_package_manifest "$compiler" "$repo" "$work"
"$work/host" "$work/manifest" "$@"
