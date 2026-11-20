#!/usr/bin/env bash
# A stranger can discover the schema and use the printed request without source setup.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-structure-help.XXXXXX)
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
for topic in query edit compare; do
    "$compiler" "$topic" --help > "$fixture/$topic-help"
    "$compiler" "$topic" missing.lkj --source-dir "$fixture/absent" \
        --store "$fixture/absent-store" --store-key "$fixture/absent-key" -h > "$fixture/late-help"
    cmp "$fixture/$topic-help" "$fixture/late-help"
    grep -q "^lkjscript $topic SOURCE" "$fixture/$topic-help"
done
"$compiler" --help > "$fixture/global-help"
grep -q 'query --help' "$fixture/global-help"
grep -q 'edit --help' "$fixture/global-help"
grep -q 'compare --help' "$fixture/global-help"
# Use the first complete JSON request printed by the binary, without a test copy.
awk '/^{/ {copy=1} copy {print} copy && /^}/ {exit}' "$fixture/edit-help" > "$fixture/request"
test -s "$fixture/request"
mkdir "$fixture/source" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
chmod 600 "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
mod math { pub fn increment(value: i64) -> i64 { value + 1 } }
pub fn probe() -> i64 { math::increment(4) }
SOURCE
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
"$compiler" build "${common[@]}" --index -o "$fixture/image" > "$fixture/build"
"$compiler" query "${common[@]}" --kind outline --id math --depth 2 > "$fixture/outline"
grep -q '"value@math::increment"' "$fixture/outline"
"$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/edit"
for engine in native interpreter; do
    test "$("$compiler" run main.lkj --source-dir "$fixture/source" --no-library --no-store --engine "$engine")" = 6
done
# Schema and identity refusals point to the help that answers them.
printf '%s\n' '{"version":2,"operations":[]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/out" 2> "$fixture/error"; then exit 1; fi
grep -q 'E_EDIT_VERSION' "$fixture/error"
grep -q 'lkjscript edit --help' "$fixture/error"
printf '%s\n' '{' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/out" 2> "$fixture/error"; then exit 1; fi
grep -q 'E_EDIT_REQUEST' "$fixture/error"
grep -q 'lkjscript edit --help' "$fixture/error"
# The real stranger's guessed operation name must lead to the supported schema.
printf '%s\n' '{"version":1,"operations":[{"op":"replace","id":"math::increment"}]}' > "$fixture/request"
if "$compiler" edit "${common[@]}" --request "$fixture/request" > "$fixture/out" 2> "$fixture/error"; then exit 1; fi
grep -q 'E_EDIT_OPERATION' "$fixture/error"
grep -q 'lkjscript edit --help' "$fixture/error"
if "$compiler" query "${common[@]}" --kind missing > "$fixture/out" 2> "$fixture/error"; then exit 1; fi
grep -q 'E_QUERY_KIND' "$fixture/error"
grep -q 'lkjscript query --help' "$fixture/error"
if "$compiler" query "${common[@]}" --kind text --id missing > "$fixture/out" 2> "$fixture/error"; then exit 1; fi
grep -q 'E_QUERY_ID' "$fixture/error"
grep -q 'lkjscript query --help' "$fixture/error"
test ! -s "$fixture/out"
printf 'ok structural help: printed JSON edits a program; native/interpreter agree; refusals point to help\n'
