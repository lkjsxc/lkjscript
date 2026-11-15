#!/usr/bin/env bash
# Filesystem include bytes cannot make an embedded source item editable.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
library=${2:-library}
fixture=$(mktemp -d target/compiler-embedded-source-test.XXXXXX)
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
mkdir -p "$fixture/source/library"
cp "$library/bytes.lkj" "$fixture/source/library/bytes.lkj"
cp "$fixture/source/library/bytes.lkj" "$fixture/original"
printf '%s\n' 'fn main() -> i64 { let _ = include_bytes!("library/bytes.lkj"); 42i64 }' > "$fixture/source/main.lkj"
cp "$fixture/source/main.lkj" "$fixture/main-before"
head -c 32 /dev/urandom > "$fixture/key"
# Keep every later AST span valid: otherwise a span failure could hide the
# origin confusion. This parses only the fixed, known test-library function.
perl -0777 -ne '
    my ($body) = /pub fn append_bytes[^\{]*(\{\n.*?\n\})/s;
    defined $body or die "append_bytes body absent\n";
    my $new = "{ missing() }";
    length($body) >= length($new) or die "body too short\n";
    $new .= " " x (length($body) - length($new));
    print qq({"version":1,"operations":[{"op":"replace-body","id":"std::bytes::append_bytes","source":"$new"}]}\n);
' "$fixture/original" > "$fixture/request.json"
fixture="$PWD/$fixture"
# Omit --source-dir deliberately: both the embedded diagnostic name and the
# included physical file are exactly library/bytes.lkj. No --library-dir means
# the std item still comes from the executable's embedded bytes.
for mode in dry apply; do
    options=(--dry-run)
    if test "$mode" = apply; then options=(--store-key "$fixture/key"); fi
    if (cd "$fixture/source"; "$compiler" edit main.lkj --no-store \
        --request "$fixture/request.json" "${options[@]}" > "$fixture/plan" 2> "$fixture/error"); then
        printf 'unexpected accepted embedded source transaction: %s\n' "$mode" >&2
        cat "$fixture/plan" >&2
        exit 1
    fi
    grep -q 'E_EDIT_EMBEDDED' "$fixture/error" || { cat "$fixture/error" >&2; exit 1; }
    cmp "$fixture/original" "$fixture/source/library/bytes.lkj"
    cmp "$fixture/main-before" "$fixture/source/main.lkj"
    test ! -e "$fixture/source/.lkjscript-edit.pending"
done
printf 'ok embedded source provenance: include collision refuses dry/apply without writes\n'
