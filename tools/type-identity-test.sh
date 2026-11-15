#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler";; esac
fixture=$(mktemp -d target/compiler-type-identity.XXXXXX)
trap 'rm -rf "$fixture"' EXIT HUP INT TERM
mkdir "$fixture/old" "$fixture/new" "$fixture/store"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/old/main.lkj" <<'SOURCE'
struct Message { tag: u8, value: i32 }
pub fn probe() -> i32 { let value = Message { tag: 1u8, value: 41 }; value.value + 1 }
SOURCE
{ printf '%s\n' 'fn unrelated(flag: bool) -> u8 { if flag { 3u8 } else { 4u8 } }'; cat "$fixture/old/main.lkj"; } > "$fixture/new/main.lkj"
identity() {
    local side=$1 snapshot
    local options=(main.lkj --source-dir "$fixture/$side" --no-library --store "$fixture/store" --store-key "$fixture/key")
    "$compiler" build "${options[@]}" --index -o "$fixture/$side/image" > "$fixture/build"
    snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/build")
    "$compiler" query "${options[@]}" --snapshot "$snapshot" --kind type --id Message > "$fixture/$side/type"
    "$compiler" query "${options[@]}" --snapshot "$snapshot" --kind layout --id Message > "$fixture/$side/layout"
    grep -Eq 'level=checked-ir.*representation-id=[0-9a-f]{64}.*representation-domain=lkjscript-ir-type-identity-linux-x86_64-1' "$fixture/$side/type"
    sed -n 's/.*representation-id=\([0-9a-f]*\).*/\1/p' "$fixture/$side/type" > "$fixture/$side/id"
    sed -n 's/.*representation-id=\([0-9a-f]*\).*/\1/p' "$fixture/$side/layout" > "$fixture/$side/layout-id"
    test "$(wc -l < "$fixture/$side/id")" -eq 1
    cmp "$fixture/$side/id" "$fixture/$side/layout-id"
}
identity old
identity new
cmp "$fixture/old/id" "$fixture/new/id"
sed -i 's/i32/i64/g' "$fixture/new/main.lkj"
identity new
if cmp -s "$fixture/old/id" "$fixture/new/id"; then printf 'changed message representation retained its identity\n' >&2; exit 1; fi
printf 'ok checked type identity: independent images, unrelated declarations and incompatible representation\n'
