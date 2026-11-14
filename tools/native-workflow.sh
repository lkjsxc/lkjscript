#!/bin/bash
# Bootstrap seeds for the process integration tests; their tested path is native.
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
bootstrap=$(realpath -e -- "${1:-target/release/lkjscript}")
bash tests/native_cli.sh tools/lkj
bash tests/native_install.sh
bash tests/native_rebuild.sh
work=$(mktemp -d "${TMPDIR:-/tmp}/lkj-native-gate.XXXXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
for component in frontend kc result; do
    "$bootstrap" build "src/${component}_main.rs" --entry main -o "$work/$component"
done
bash tools/install-native.sh "$work/toolchain" \
    --frontend "$work/frontend" --kc "$work/kc" --result "$work/result" \
    --library library --runtime-source library/runtime.rs --source-commit gate-working-tree
bash tests/native_workflow.sh "$work/toolchain" "$work/results"
