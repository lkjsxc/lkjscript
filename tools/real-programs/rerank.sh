#!/usr/bin/env bash
# Build the checked tool in private scratch state; keep input paths relative to
# the invoking directory, not the compiler's source root.
set -euo pipefail
root=$(cd -- "$(dirname -- "$0")/../.." && pwd)
compiler=${LKJSCRIPT_COMPILER:-$root/target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
work=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-rerank.XXXXXX")
work=$(cd -- "$work" && pwd)
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
(cd "$root" && "$compiler" build tools/real-programs/rerank.lkj --source-dir . --library-dir library --no-store -o "$work/rerank")
"$work/rerank" "$@"
