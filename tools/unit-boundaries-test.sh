#!/usr/bin/env bash
# Real source units exercise borrowed results, imported cleanup, and narrow effects.
set -euo pipefail
cd "$(dirname "$0")/.."
unit_compiler=$(realpath "${LKJSCRIPT_BINARY:-target/lkjscript}")
mkdir -p target
unit_fixture=$(mktemp -d target/unit-boundaries.XXXXXX)
unit_key_root=$(mktemp -d /tmp/lkjscript-unit-boundary-key.XXXXXX)
unit_cleanup() {
    local result=$?
    if (( result != 0 )); then
        for diagnostic in "$unit_fixture"/*.err; do
            [[ -f $diagnostic ]] || continue
            printf '%s\n' "$diagnostic" >&2
            cat "$diagnostic" >&2
        done
    fi
    rm -rf "$unit_fixture" "$unit_key_root"
}
trap unit_cleanup EXIT
trap 'exit 130' HUP INT TERM
mkdir "$unit_fixture/store"
unit_key=$(XDG_STATE_HOME="$unit_key_root" sh tools/store-key.sh)
unit_root=$(realpath "$unit_fixture")
cat > "$unit_root/units.lkj" <<'SOURCE'
const UNITS: &[&str] = &["provider=provider.lkj", "bridge=bridge.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["bridge=provider", "app=bridge"];
const ENTRY: &str = "app::main";
SOURCE
cat > "$unit_root/provider.lkj" <<'SOURCE'
pub struct Pair { pub left: i64, pub right: i64 }
pub fn first<'a>(value: &'a Pair) -> &'a i64 { &value.left }
pub struct Mark { out: Stdout, tag: u8 }
impl Drop for Mark { fn drop(&mut self) { self.out.write(b"A"); self.out.write(&[self.tag]); } }
pub fn marker(out: Stdout) -> Mark {
    Mark { out, tag: b'B' }
}
SOURCE
cat > "$unit_root/bridge.lkj" <<'SOURCE'
pub fn borrowed<'a>(value: &'a crate::provider::Pair) -> &'a i64 {
    crate::provider::first(value)
}
pub fn relay(out: Stdout) -> crate::provider::Mark {
    crate::provider::marker(out)
}
SOURCE
cat > "$unit_root/app.lkj" <<'SOURCE'
pub fn main(out: Stdout) {
    let pair = crate::provider::Pair { left: 17i64, right: 25i64 };
    let first = crate::bridge::borrowed(&pair);
    assert_eq!(*first, 17i64);
    let mark = crate::bridge::relay(out);
    drop(mark);
}
SOURCE
unit_options=(units.lkj --source-dir "$unit_root" --store "$unit_root/store" --store-key "$unit_key" --stats)
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/program" > "$unit_root/cold.out" 2> "$unit_root/cold.err"
[[ $("$unit_root/program") == AB ]]
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/warm" > "$unit_root/warm.out" 2> "$unit_root/warm.err"
grep -Fq -- 'checked=0 reused=3 emitted=0' "$unit_root/warm.out"
cmp "$unit_root/program" "$unit_root/warm"
# The same public interface must link the freshly checked implementation.
perl -0pi -e "s/tag: b'B'/tag: b'C'/" "$unit_root/provider.lkj"
"$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/edited" > "$unit_root/edit.out" 2> "$unit_root/edit.err"
grep -Fq -- 'checked=1 reused=2 emitted=1' "$unit_root/edit.out"
[[ $("$unit_root/edited") == AC ]]
# A narrow capability remains required when its only effect is an imported Drop.
if "$unit_compiler" build-units "${unit_options[@]}" -o "$unit_root/edited" > "$unit_root/grant.out" 2> "$unit_root/grant.err"; then
    echo 'ungranted imported capability was accepted' >&2
    exit 1
fi
[[ $("$unit_root/edited") == AC ]]
# A caller cannot mutate the owner while an imported returned field is live.
perl -0pi -e 's/let pair =/let mut pair =/; s/assert_eq!\(\*first, 17i64\);/pair.left = 19i64; assert_eq!(*first, 17i64);/' "$unit_root/app.lkj"
if "$unit_compiler" build-units "${unit_options[@]}" --grant out -o "$unit_root/edited" > "$unit_root/borrow.out" 2> "$unit_root/borrow.err"; then
    echo 'mutation through a live imported returned view was accepted' >&2
    exit 1
fi
[[ $("$unit_root/edited") == AC ]]
printf 'ok source unit boundaries: returned views, imported Drop, narrow effects, fresh implementations\n'
