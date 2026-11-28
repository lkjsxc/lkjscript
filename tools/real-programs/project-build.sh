#!/usr/bin/env bash
# Build the checked policy and its Linux syscall adapter. Source this file.
build_project() (
    set -euo pipefail
    local compiler=$1 source=$2 library=$3 work=$4
    "$compiler" build "$source" --entry select --library-dir "$library" --no-store \
        --grant arguments --grant output --grant errors --grant status -o "$work/select"
    "$compiler" build "$source" --library-dir "$library" --no-store \
        --grant arguments --grant input --grant output --grant errors --grant status -o "$work/project"
    cc -std=c11 -O2 -Wall -Wextra -Werror "${source%/*}/project-host.c" -o "$work/host"
)
