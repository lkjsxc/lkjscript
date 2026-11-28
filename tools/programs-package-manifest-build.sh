#!/usr/bin/env bash
# Source this to build the checked CLI and narrow Linux/zlib adapter.
build_package_manifest() (
    set -euo pipefail
    local compiler=$1 repo=$2 work=$3
    "$compiler" build "$repo/tools/programs_package_manifest.lkj" --library-dir "$repo/library" --no-store \
        --grant arguments --grant input --grant output --grant errors --grant status -o "$work/manifest"
    cc -std=c11 -O2 -Wall -Wextra -Werror "$repo/tools/programs-package-manifest-host.c" -lz -o "$work/host"
)
