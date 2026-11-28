#!/usr/bin/env bash
# Configuration values and macro snapshots must agree with their exact inputs.
set -euo pipefail
cd "$(dirname "$0")/.."
mode=${2:-all}
case "$mode:$#" in all:[01]|--native:2|--interpreter:2) ;; *)
    printf 'usage: %s [compiler [--native | --interpreter]]\n' "$0" >&2
    exit 2
esac
compiler=$(realpath "${1:-target/lkjscript}")
library="$PWD/library"
mkdir -p target
fixture=$(mktemp -d "$PWD/target/configuration-origins.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for log in "$fixture"/*.error; do
            if [[ -s "$log" ]]; then tail -n 60 "$log" >&2; fi
        done
    fi
    rm -rf -- "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 129' HUP
trap 'exit 143' TERM
mkdir "$fixture/contracts"
for source in ast diagnostic names syntax resolve source; do
    cp "src/$source.lkj" "$fixture/contracts/$source.lkj"
done
cp -R src/ast src/syntax src/resolve "$fixture/contracts/"
cp tests/fixtures/configuration_origins.lkj "$fixture/contracts/main.lkj"
for engine in native interpreter; do
    if [[ $mode == --native && $engine != native || $mode == --interpreter && $engine != interpreter ]]; then continue; fi
    "$compiler" run main.lkj --source-dir "$fixture/contracts" --library-dir "$library" --no-store \
        --engine "$engine" > "$fixture/$engine.out" 2> "$fixture/$engine.error"
    if [[ $(<"$fixture/$engine.out") != 40 ]]; then
        printf 'configuration-origins: expected 40 controls on %s\n' "$engine" >&2
        exit 1
    fi
    printf 'configuration-origins engine=%s controls=40\n' "$engine"
done
