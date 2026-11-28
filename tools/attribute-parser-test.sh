#!/usr/bin/env bash
# Exercise private parser cursor/rollback contracts using the production source.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# -le 2 ]] || { printf 'usage: %s [compiler [entry]]\n' "$0" >&2; exit 2; }
compiler=$(realpath "${1:-target/lkjscript}")
entry=${2:-main}
case "$entry" in
    main) expected=27 ;;
    probe_success) expected=5 ;;
    probe_failure) expected=4 ;;
    attribute_failure) expected=4 ;;
    disabled_items) expected=14 ;;
    *) printf 'attribute-parser: unknown entry %s\n' "$entry" >&2; exit 2 ;;
esac
library=$PWD/library
mkdir -p target
fixture=$(mktemp -d "$PWD/target/attribute-parser.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if ((status)); then
        for log in "$fixture"/*.error; do
            [[ ! -s $log ]] || tail -n 40 "$log" >&2
        done
    fi
    rm -rf -- "$fixture"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
mkdir "$fixture/contracts"
for source in ast diagnostic names syntax resolve source; do
    cp "src/$source.lkj" "$fixture/contracts/$source.lkj"
done
cp -R src/ast src/syntax src/resolve "$fixture/contracts/"
cp tests/fixtures/attribute_parser.lkj "$fixture/contracts/main.lkj"
cp tests/fixtures/attribute_parser_tests.lkj "$fixture/contracts/syntax/attribute_parser_tests.lkj"
printf '\npub(crate) mod attribute_parser_tests;\n' >> "$fixture/contracts/syntax.lkj"
for engine in native interpreter; do
    "$compiler" run main.lkj --source-dir "$fixture/contracts" --library-dir "$library" \
        --entry "$entry" --engine "$engine" --no-store > "$fixture/$engine.out" 2> "$fixture/$engine.error"
    [[ $(<"$fixture/$engine.out") == "$expected" ]] || {
        printf 'attribute-parser: expected %s controls for %s/%s\n' "$expected" "$entry" "$engine" >&2
        exit 1
    }
    printf 'attribute-parser entry=%s engine=%s controls=%s\n' "$entry" "$engine" "$expected"
done
