#!/usr/bin/env bash
# Only launch a checked executable with the selected read-only package root.
# The executable validates the original CLI, walks roots, lexes and renders.
set -euo pipefail

refuse() { printf 'real-programs-census: %s\n' "$1" >&2; exit 255; }

# Discover the single package argument for the startup directory grant. Leave
# usage errors and help/self-test precedence to the checked program.
root_arg=.
positionals=0
valid=yes
help=no
root_value=no
for word in "$@"; do
    if [[ $root_value == yes ]]; then root_value=no; continue; fi
    case "$word" in
        --help) help=yes ;;
        --self-test|--source-root=*|--test-root=*) ;;
        --source-root|--test-root) root_value=yes ;;
        -*) valid=no ;;
        *) root_arg=$word; positionals=$((positionals+1)) ;;
    esac
done

# realpath follows the package argument itself, just as the original CLI did.
# Explicit selected roots and every visited descendant are never followed.
if [[ $valid == yes && $help == no && $root_value == no && $positionals == 1 ]]; then
    # Node interprets the empty positional as the current directory.
    if ! IFS= read -r -d '' package < <(realpath -ez -- "${root_arg:-.}" 2>/dev/null); then
        refuse "cannot resolve package root: $root_arg"
    fi
    [[ -d $package ]] || refuse "not a directory: $root_arg"
else
    package=$PWD
fi
# Default selection originally followed stat(src/tests) only to decide whether
# to select them. Descendant access itself remains pinned and no-follow.
defaults=''
[[ ! -d $package/src ]] || defaults=s
[[ ! -d $package/tests ]] || defaults+=t
export LKJSCRIPT_CENSUS_ROOT="$package" LKJSCRIPT_CENSUS_DEFAULTS="$defaults"

# Tests may reuse a checked executable they just built in a private directory.
if [[ -n ${LKJSCRIPT_CENSUS_BINARY:-} ]]; then
    exec "$LKJSCRIPT_CENSUS_BINARY" "$@"
fi

repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
compiler=${LKJSCRIPT_COMPILER:-"$repo/target/lkjscript"}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
(cd "$repo"; "$compiler" build tools/real-programs/census.lkj \
    --library-dir "$repo/library" --no-store \
    --grant arguments --grant root=env:LKJSCRIPT_CENSUS_ROOT --grant package=LKJSCRIPT_CENSUS_ROOT \
    --grant defaults=LKJSCRIPT_CENSUS_DEFAULTS --grant output --grant errors --grant status \
    -o "$work/census")
"$work/census" "$@"
