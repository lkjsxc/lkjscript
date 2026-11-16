#!/usr/bin/env bash
# Required offline gate: original assertions, verified provenance and native corpus.
# The expensive full interpreted corpus is an explicit milestone rerun only.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=target/lkjscript
full_interpreter=no
if (($#)) && [[ $1 != --* ]]; then compiler=$1; shift; fi
while (($#)); do
    case "$1" in
        --full-interpreter) full_interpreter=yes; shift ;;
        *) printf 'usage: %s [COMPILER] [--full-interpreter]\n' "$0" >&2; exit 2 ;;
    esac
done
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
[[ -x "$compiler" ]] || { printf 'compiler is not executable: %s\n' "$compiler" >&2; exit 1; }
source=examples/real_programs/levenshtein
entry="$source/main.lkj"
golden=tests/fixtures/real_programs_levenshtein.output.gz
[[ -f "$golden" ]] || { printf 'missing checked corpus: %s\n' "$golden" >&2; exit 1; }
mkdir -p target
work=$(mktemp -d "$PWD/target/real-programs.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT
    if ((status)); then
        for log in "$work"/*.err "$work"/*-tests.out "$work/authority.out"; do
            [[ ! -f "$log" ]] || { printf '%s\n' "$log" >&2; cat "$log" >&2; }
        done
    fi
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
perl tools/real-programs/verify-levenshtein.pl --self-test "$source" > "$work/provenance.json"
perl -MJSON::PP -0777 -e '
    my $report=decode_json(<>);
    die "unexpected upstream inventory\n" unless $report->{test_count} == 8
        && $report->{assertion_count} == 26 && $report->{mutants_rejected} == 18
        && $report->{exact_upstream_bytes};
' "$work/provenance.json"
gzip -dc "$golden" > "$work/expected.out"
awk -F: 'NF != 2 || $1 != NR-1 || $2 !~ /^[0-9]+$/ { exit 1 } END { if (NR != 7239) exit 1 }' "$work/expected.out"
printf 'ok 8 tests\n' > "$work/expected-tests.out"
for engine in native interpreter; do
    "$compiler" test "$entry" --source-dir . --library-dir library --no-store --jobs 1 --engine "$engine" > "$work/$engine-tests.out" 2> "$work/$engine-tests.err"
    cmp "$work/expected-tests.out" "$work/$engine-tests.out"
    test ! -s "$work/$engine-tests.err"
done
# This upstream test reaches the algorithm. Its checked closure must have no
# grants and no observable operations, including any hidden file or oracle read.
"$compiler" authority "$entry" --source-dir . --library-dir library --no-store --entry upstream::should_work > "$work/authority.out" 2> "$work/authority.err"
cat > "$work/expected-authority.out" <<'AUTHORITY'
entry upstream::should_work
declared authority:
  none
reachable operations (conservative checked-IR branches and owned cleanup):
  none
AUTHORITY
sed -n '1,5p' "$work/authority.out" > "$work/authority-prefix.out"
cmp "$work/expected-authority.out" "$work/authority-prefix.out"
grep -Fxq '  upstream::levenshtein' "$work/authority.out"
test ! -s "$work/authority.err"
"$compiler" build "$entry" --source-dir . --library-dir library --no-store --grant output -o "$work/specimen" > "$work/build.out" 2> "$work/build.err"
"$work/specimen" > "$work/native.out" 2> "$work/native.err"
cmp "$work/expected.out" "$work/native.out"
test ! -s "$work/native.err"
if [[ "$full_interpreter" == yes ]]; then
    "$compiler" run "$entry" --source-dir . --library-dir library --no-store --grant output --engine interpreter > "$work/interpreter.out" 2> "$work/interpreter.err"
    cmp "$work/expected.out" "$work/interpreter.out"
    test ! -s "$work/interpreter.err"
    printf 'ok real programs: full interpreted 7,239-row corpus\n'
fi
printf 'ok real programs: exact upstream bytes, 18 provenance controls, 8 tests on each engine, pure algorithm authority and 7,239 native corpus rows\n'
