#!/usr/bin/env bash
set -euo pipefail
repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
source "$repo/tools/real-programs/project-build.sh"
compiler=${LKJSCRIPT_COMPILER:-"$repo/target/lkjscript"}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
# Before checked creation takes ownership, cancellation has default signal
# status. Clean the private build directory before re-raising that signal.
startup_signal() {
    local signal=$1
    trap - EXIT
    rm -rf -- "$work"
    trap - "$signal"
    kill -s "$signal" "$$"
}
trap 'startup_signal INT' INT
trap 'startup_signal HUP' HUP
trap 'startup_signal TERM' TERM
build_project "$compiler" "$repo/tools/real-programs/project.lkj" "$repo/library" "$work"
"$work/select" "$0" "$@" > "$work/options"
mapfile -d '' -t fields < "$work/options"
case "${fields[0]}" in
    help) printf '%s' "${fields[1]}" ;;
    self)
        source "$repo/tools/real-programs/project-test.sh"
        test_project "$repo" "$compiler" "$repo/tools/real-programs/project.lkj" "$work" self
        ;;
    project)
        # Open the checked image before unlinking build files, then exec it by
        # descriptor. The public PID becomes the coordinator for INT/TERM.
        trap - EXIT INT HUP TERM
        exec "$work/host" --ephemeral "$work" "$work/project" "$@"
        ;;
    *) exit 255 ;;
esac
