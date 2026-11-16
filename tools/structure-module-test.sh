#!/usr/bin/env bash
# New external-module projections stay absent until a complete checked transaction.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d "$PWD/target/compiler-module-projection.XXXXXX")
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
head -c 32 /dev/urandom > "$fixture/key"
cases=0
reset_source() {
    rm -rf "$fixture/source"
    mkdir "$fixture/source"
    printf '%s\n' 'mod keep;' 'fn main() -> i64 { keep::value() }' > "$fixture/source/main.lkj"
    printf '%s\n' '// untouched: π' 'pub fn value() -> i64 { 1i64 }' > "$fixture/source/keep.lkj"
    cp "$fixture/source/keep.lkj" "$fixture/keep-before"
    common=(main.lkj --source-dir "$fixture/source" --no-library --no-store)
}
snapshot() {
    (cd "$fixture/source"
     find . -name '*.lkj' -printf '%y %p %l\n' | LC_ALL=C sort
     find . -type f -name '*.lkj' -print0 | LC_ALL=C sort -z | xargs -0 sha256sum)
}
edit() {
    "$compiler" edit "${common[@]}" --request "$fixture/request" "$@" > "$fixture/plan" 2> "$fixture/error"
}
applied() {
    edit --store-key "$fixture/key" || { cat "$fixture/error" >&2; exit 1; }
    grep -q 'mode=apply' "$fixture/plan"
    test ! -e "$fixture/source/.lkjscript-edit.pending"
    test -z "$(find "$fixture/source" -name '*.lkjscript-next' -print -quit)"
    cmp "$fixture/source/keep.lkj" "$fixture/keep-before"
    cases=$((cases + 1))
}
refuse() {
    snapshot > "$fixture/before"
    if timeout --kill-after=2s 60s "$compiler" edit "${common[@]}" --request "$fixture/request" --store-key "$fixture/key" > "$fixture/plan" 2> "$fixture/error"; then
        printf 'unexpected accepted module projection\n' >&2; exit 1
    else
        code=$?
        test "$code" -ne 124
        test "$code" -ne 137
    fi
    snapshot > "$fixture/after"
    cmp "$fixture/before" "$fixture/after"
    grep -q 'E_EDIT_' "$fixture/error"
    test ! -e "$fixture/source/.lkjscript-edit.pending"
    test -z "$(find "$fixture/source" -name '*.lkjscript-next' -print -quit)"
    cases=$((cases + 1))
}
reset_source
cat > "$fixture/request" <<'JSON'
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod child;","projection":{"path":"child.lkj","source":"pub fn value() -> i64 { 42i64 }\n"}},{"op":"replace-body","id":"main","source":"{ child::value() }"}]}
JSON
snapshot > "$fixture/before"
edit --dry-run || { cat "$fixture/error" >&2; exit 1; }
snapshot > "$fixture/after"
cmp "$fixture/before" "$fixture/after"
grep -q 'mode=dry-run' "$fixture/plan"
grep -q 'file before=absent after=' "$fixture/plan"
test ! -e "$fixture/source/child.lkj"
test ! -e "$fixture/source/.lkjscript-edit.lock"
cases=$((cases + 1))
applied
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 42
grep -q 'file before=absent after=' "$fixture/plan"

# Empty external modules retain their real loaded file without child-item guesses.
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod empty;","projection":{"path":"empty.lkj","source":""}}]}' > "$fixture/request"
applied
test -f "$fixture/source/empty.lkj"
test ! -s "$fixture/source/empty.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"empty","source":"pub fn value() -> i64 { 7i64 }"},{"op":"replace-body","id":"main","source":"{ empty::value() }"}]}' > "$fixture/request"
applied
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 7

# The loader's inline, direct-file and mod.lkj directory rules remain distinct.
reset_source
printf '%s\n' 'mod keep;' 'mod inline {}' 'pub mod direct;' 'pub mod nested;' 'fn main() -> i64 { 0i64 }' > "$fixture/source/main.lkj"
: > "$fixture/source/direct.lkj"
mkdir "$fixture/source/inline" "$fixture/source/direct" "$fixture/source/nested"
: > "$fixture/source/nested/mod.lkj"
cat > "$fixture/request" <<'JSON'
{"version":1,"operations":[{"op":"add-item","id":"inline","source":"pub mod first;","projection":{"path":"inline/first.lkj","source":"pub fn value() -> i64 { 10i64 }"}},{"op":"add-item","id":"direct","source":"pub mod second;","projection":{"path":"direct/second.lkj","source":"pub fn value() -> i64 { 20i64 }"}},{"op":"add-item","id":"nested","source":"pub mod third;","projection":{"path":"nested/third.lkj","source":"pub fn value() -> i64 { 12i64 }"}},{"op":"replace-body","id":"main","source":"{ inline::first::value() + direct::second::value() + nested::third::value() }"}]}
JSON
applied
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 42

reset_source
mkdir "$fixture/source/created"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod created;","projection":{"path":"created/mod.lkj","source":"pub fn value() -> i64 { 42i64 }"}},{"op":"replace-body","id":"main","source":"{ created::value() }"}]}' > "$fixture/request"
applied
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 42

# Each refusal preserves all source bytes and every special leaf.
reset_source
while IFS= read -r request; do
    printf '%s\n' "$request" > "$fixture/request"
    refuse
done <<'JSON'
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod child;","projection":{"path":"child.lkj","source":"pub fn value() -> i64 { false }"}}]}
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod child;","projection":{"path":"wrong.lkj","source":""}}]}
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod child;","projection":{"path":"../child.lkj","source":""}}]}
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod child;","projection":{"path":"/child.lkj","source":""}}]}
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod child;","projection":{"path":"child/mod.lkj","source":""}}]}
{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub fn ordinary() {}","projection":{"path":"child.lkj","source":""}}]}
JSON
test ! -e "$fixture/source/child"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"crate","source":"pub mod occupied;","projection":{"path":"occupied.lkj","source":""}}]}' > "$fixture/request"
for kind in empty text dangling fifo directory; do
    case "$kind" in
        empty) : > "$fixture/source/occupied.lkj";;
        text) printf '%s\n' 'pub fn retained() -> i64 { 91i64 }' > "$fixture/source/occupied.lkj";;
        dangling) ln -s absent-target "$fixture/source/occupied.lkj";;
        fifo) mkfifo "$fixture/source/occupied.lkj";;
        directory) mkdir "$fixture/source/occupied.lkj";;
    esac
    refuse
    if test "$kind" = directory; then rmdir "$fixture/source/occupied.lkj"; else rm "$fixture/source/occupied.lkj"; fi
done
mkdir "$fixture/source/occupied"
printf '%s\n' '// existing alternative' > "$fixture/source/occupied/mod.lkj"
refuse
test ! -e "$fixture/source/occupied.lkj"

# Equal basenames in distinct retained parent inodes are ordinary valid edits.
reset_source
printf '%s\n' 'mod keep;' 'mod left {}' 'mod right {}' 'fn main() -> i64 { 0i64 }' > "$fixture/source/main.lkj"
mkdir "$fixture/source/left" "$fixture/source/right"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"left","source":"pub mod child;","projection":{"path":"left/child.lkj","source":""}},{"op":"add-item","id":"right","source":"pub mod child;","projection":{"path":"right/child.lkj","source":""}}]}' > "$fixture/request"
applied
test -f "$fixture/source/left/child.lkj"
test -f "$fixture/source/right/child.lkj"

# Parent aliases must refuse for new and existing slots before a durable intent.
reset_source
printf '%s\n' 'mod keep;' 'mod left {}' 'mod right {}' 'fn main() -> i64 { 0i64 }' > "$fixture/source/main.lkj"
mkdir "$fixture/source/left"
ln -s left "$fixture/source/right"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"left","source":"pub mod child;","projection":{"path":"left/child.lkj","source":""}},{"op":"add-item","id":"right","source":"pub mod child;","projection":{"path":"right/child.lkj","source":""}}]}' > "$fixture/request"
refuse
grep -q 'E_EDIT_PARENT_ALIAS' "$fixture/error"
printf '%s\n' 'mod keep;' 'mod left;' 'mod right;' 'fn main() -> i64 { left::value() + right::value() }' > "$fixture/source/main.lkj"
printf '%s\n' 'pub fn value() -> i64 { 1i64 }' > "$fixture/source/left/mod.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"left::value","source":"{ 21i64 }"},{"op":"replace-body","id":"right::value","source":"{ 22i64 }"}]}' > "$fixture/request"
refuse
grep -q 'E_EDIT_PARENT_ALIAS' "$fixture/error"
# A captured alias not named by any operation also invalidates the byte overlay.
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"left::value","source":"{ 21i64 }"}]}' > "$fixture/request"
refuse
grep -q 'E_EDIT_PARENT_ALIAS' "$fixture/error"
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 2

# Aliases discovered only by the candidate also invalidate the original overlay.
reset_source
printf '%s\n' 'mod keep;' 'mod left;' 'fn main() -> i64 { left::value() }' > "$fixture/source/main.lkj"
mkdir "$fixture/source/left"
ln -s left "$fixture/source/right"
printf '%s\n' 'pub fn value() -> i64 { 1i64 }' > "$fixture/source/left/mod.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"left::value","source":"{ 21i64 }"},{"op":"add-item","id":"crate","source":"mod right;"},{"op":"replace-body","id":"main","source":"{ left::value() + right::value() }"}]}' > "$fixture/request"
refuse
grep -q 'E_EDIT_PARENT_ALIAS' "$fixture/error"
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 1

# A captured negative library probe can alias a new source-module leaf.
reset_source
printf '%s\n' 'mod keep;' 'mod left {}' 'fn main() -> i64 { 0i64 }' > "$fixture/source/main.lkj"
mkdir "$fixture/source/left"
ln -s left "$fixture/source/library"
printf '%s\n' 'pub fn retained() -> i64 { 0i64 }' > "$fixture/source/left/lib.lkj"
common=(main.lkj --source-dir "$fixture/source" --library-dir "$fixture/source/library" --no-store)
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"left","source":"pub mod read_io;","projection":{"path":"left/read_io.lkj","source":"pub fn value() -> i64 { 42i64 }"}}]}' > "$fixture/request"
refuse
grep -q 'E_EDIT_PARENT_ALIAS' "$fixture/error"
test ! -e "$fixture/source/left/read_io.lkj"

# Final symlink dependencies cannot hide a differently named changed-file alias.
reset_source
mv "$fixture/source/keep.lkj" "$fixture/source/real.lkj"
ln -s real.lkj "$fixture/source/keep.lkj"
printf '%s\n' '{"version":1,"operations":[{"op":"replace-body","id":"main","source":"{ 2i64 }"}]}' > "$fixture/request"
refuse
grep -q 'open regular source dependency' "$fixture/error"

# Adding a library body promotes all definitions even after demanded-only reuse.
reset_source
mkdir "$fixture/source/library" "$fixture/store"
printf '%s\n' 'pub fn retained() -> i64 { 0i64 }' > "$fixture/source/library/lib.lkj"
common=(main.lkj --source-dir "$fixture/source" --library-dir "$fixture/source/library" --store "$fixture/store")
for pass in 1 2; do
    "$compiler" check "${common[@]}" --store-key "$fixture/key" --inline-limit 0 --stats > "$fixture/warm" 2> "$fixture/warm-stats"
done
grep -Eq 'front_reused[^0-9]*[1-9][0-9]*' "$fixture/warm-stats"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"std","source":"pub mod extra;","projection":{"path":"library/extra.lkj","source":"pub fn unused<T>(value: T) -> i64 { missing() }"}}]}' > "$fixture/request"
refuse
grep -q 'E_EDIT_CHECK' "$fixture/error"
grep -q 'missing' "$fixture/error"
printf '%s\n' '{"version":1,"operations":[{"op":"add-item","id":"std","source":"pub mod extra;","projection":{"path":"library/extra.lkj","source":"pub fn value() -> i64 { 42i64 }"}},{"op":"replace-body","id":"main","source":"{ std::extra::value() }"}]}' > "$fixture/request"
applied
grep -q 'library-validation=all' "$fixture/plan"
test "$("$compiler" run "$fixture/source/main.lkj" --library-dir "$fixture/source/library" --no-store)" = 42

# Replay can introduce one actual loaded module; its selected-root meaning must match.
reset_source
mkdir "$fixture/desired"
cp "$fixture/source/keep.lkj" "$fixture/desired/keep.lkj"
printf '%s\n' 'mod keep;' 'pub mod added;' 'fn main() -> i64 { added::value() }' > "$fixture/desired/main.lkj"
printf '%s\n' 'pub fn value() -> i64 { 42i64 }' > "$fixture/desired/added.lkj"
"$compiler" edit "${common[@]}" --against "$fixture/desired/main.lkj" --store-key "$fixture/key" > "$fixture/plan" 2> "$fixture/error" || { cat "$fixture/error" >&2; exit 1; }
grep -Eq '^replay desired=([0-9a-f]{64}) candidate=\1 verified=true operations=[1-9][0-9]*$' "$fixture/plan"
grep -q 'file before=absent after=' "$fixture/plan"
cmp "$fixture/source/added.lkj" "$fixture/desired/added.lkj"
cmp "$fixture/source/keep.lkj" "$fixture/keep-before"
test "$("$compiler" run "$fixture/source/main.lkj" --no-library --no-store)" = 42
test ! -e "$fixture/source/.lkjscript-edit.pending"
cases=$((cases + 1))
printf 'ok external module projections: %s dry/apply/refusal/replay observations\n' "$cases"
