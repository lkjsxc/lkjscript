#!/bin/bash
# Bootstrap seeds for the process integration tests; their tested path is native.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
bootstrap=$(realpath -e -- "${1:-target/release/lkjscript}")
bash tests/native_cli.sh tools/lkjscript
bash tests/native_install.sh
bash tests/native_rebuild.sh
work=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-native-gate.XXXXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
"$bootstrap" runtime-ir >"$work/runtime.lkjir"
for component in frontend backend result; do
    "$bootstrap" ir "src/${component}_main.rs" >"$work/$component.lkjir"
    "$bootstrap" build "$work/$component.lkjir" --entry main -o "$work/$component"
done
"$work/frontend" --source library/runtime.rs --no-library >"$work/runtime-self.lkjir"
cmp "$work/runtime.lkjir" "$work/runtime-self.lkjir"
for component in frontend backend result; do
    "$work/frontend" --source "src/${component}_main.rs" --library-dir library >"$work/$component-self.lkjir"
    cmp "$work/$component.lkjir" "$work/$component-self.lkjir"
    (cd -- "$work"; ./backend "$component-self.lkjir" --runtime runtime-self.lkjir --entry main >"$component-self")
    cmp "$work/$component" "$work/$component-self"
done
printf 'ok native component fixed points, twin bytes and runtime IR\n'
bash tools/install-native.sh "$work/toolchain" \
    --frontend "$work/frontend" --backend "$work/backend" --result "$work/result" \
    --library library --runtime-source library/runtime.rs --source-commit gate-working-tree
bash tests/native_workflow.sh "$work/toolchain" "$work/results"
