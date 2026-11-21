#!/usr/bin/env bash
# Independent byte oracle for the fixed mixed create/replace specimen.
set -euo pipefail

fail() {
    printf 'create-kill fixture: %s\n' "$1" >&2
    exit 1
}
[[ $# == 1 ]] || fail 'expected fixture root'
root=$1

compare() {
    local path=$1
    [[ -f "$root/$path" ]] && cmp -s -- "$root/$path" "$2" || fail "$path differs"
}

printf -v padding '%16384s' ''
old_padding=${padding// /x}
new_padding=${padding// /y}

main_source() {
    printf 'mod old%d;\n' {0..15}
    printf 'pub fn main() -> i64 { old0::value() }\n'
}

request() {
    local index
    printf '{"operations":['
    for index in {0..15}; do
        printf '{"id":"old%d::value","op":"replace-body","source":"{ %di64 }"},' "$index" "$((1000+index))"
        printf '{"id":"crate","op":"add-item","projection":{"path":"fresh%d.lkj","source":"// %s\\npub fn value() -> i64 { %di64 }\\n"},"source":"mod fresh%d;"},' \
            "$index" "$new_padding" "$((2000+index))" "$index"
    done
    printf '{"id":"main","op":"replace-body","source":"{ old0::value() + fresh0::value() }"}],"version":1}\n'
}

for index in {0..15}; do
    compare "old/old$index.lkj" <(printf '// %s\npub fn value() -> i64 { %di64 }\n' "$old_padding" "$index")
done
compare old/main.lkj <(main_source)
compare request.json <(request)

# Every expected name has passed its byte check. Count all entries to detect extras.
shopt -s dotglob nullglob
sources=("$root"/old/*)
[[ ${#sources[@]} == 17 ]] || fail 'source inventory differs'
