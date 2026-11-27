#!/usr/bin/env bash
# Grant only the selected specimen directory. The checked program validates
# the arguments and reads each pinned basename via ReadDirectory.open.
set -euo pipefail
if (($# != 1)) || [[ $1 == --* ]]; then
    printf 'usage: verify-levenshtein.sh SPECIMEN_DIRECTORY\n' >&2
    exit 255
fi
# Node's path.join lexically normalizes before opening. The selected root
# itself may be a symlink; the three input basenames may not.
if ! IFS= read -r -d '' logical < <(realpath -msz -- "${1:-.}") ||
    ! IFS= read -r -d '' directory < <(realpath -ez -- "$logical") ||
    [[ ! -d $directory ]]; then
    printf 'provenance refusal: cannot resolve specimen directory\n' >&2
    exit 255
fi
export LKJSCRIPT_VERIFY_LEVENSHTEIN_SOURCE="$directory"
if [[ -n ${LKJSCRIPT_VERIFY_LEVENSHTEIN_BINARY:-} ]]; then
    exec "$LKJSCRIPT_VERIFY_LEVENSHTEIN_BINARY" "$@"
fi
repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
compiler=${LKJSCRIPT_COMPILER:-"$repo/target/lkjscript"}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
work=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-verify-levenshtein.XXXXXX")
work=$(cd -- "$work" && pwd)
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
(cd "$repo"; "$compiler" build tools/real-programs/verify-levenshtein.lkj \
    --library-dir "$repo/library" --no-store \
    --grant arguments --grant source=env:LKJSCRIPT_VERIFY_LEVENSHTEIN_SOURCE \
    --grant output --grant errors --grant status -o "$work/verify")
"$work/verify" "$@"
