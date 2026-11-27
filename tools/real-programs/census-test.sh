#!/usr/bin/env bash
# Golden reports were independently rendered by the predecessor census.
# The gate needs only the checked compiler, Bash and standard coreutils.
set -euo pipefail

make_fixtures() {
    local work=$1
    mkdir -p "$work/package/src/nested" "$work/package/tests" "$work/package/examples" "$work/package/extra"
    cat > "$work/package/src/nested/é.rs" <<'RS'
// async unsafe #[test] fake!()
/* unsafe /* extern */ await */
#![cfg_attr(feature="fake", cfg_attr(x, test, derive(r#Debug, ::𐐀::É, 名)), tokio::test)]
#[r#test] #[::test] #[cfg_attr(a, cfg_attr(b, r#test), derive(Clone, invalid(x), r#σ::𐐀))]
r#async r#unsafe r#await r#dyn r#extern async await dyn unsafe extern
r#vec!(); ::r#std::r#vec!(); $r#vec!(); custom::vec!{}; 𐐀::名![];
core::assert!(); alloc::println!(); ::vec!(); $dynamic!(); r#macro_rules!();
macro_rules! $r#unsafe {} macro_rules! r#dyn {} macro_rules! {}
const RAW: &str = r###"#[test] "## fake!()
unsafe"###;
const BYTES: &[u8] = br##"async "#"##;
c"#[test] unsafe" cr##"await"## b"extern" "escaped \" unsafe"
('🦀', '\u{1f980}', '\'', '\\', b'\x2f', b'x', '\x', '\u{_}') 'r#unsafe 'async
1async 0x_unsafe 0b10extern 0o7dyn 1e-2dyn 1.0await 1.unsafe 1..async 1._unsafe
ά!(); 𐐀!(); _!(); 名::r#vec!(); vec![]
#[derive(A, B<C>, ::C, r#D)] #[cfg_attr(x, derive(), cfg_attr(y, r#test))]
RS
    # BOM, non-ASCII whitespace and XID boundary differences. White_Space
    # separates an identifier from !, whereas a punctuation scalar does not.
    printf '\357\273\277unsafe\r\n' > "$work/package/src/edges.rs"
    printf 'a\u0085!(); b\u00a0!(); c\u1680!(); d\u2000!(); e\u200a!(); f\u2028!(); g\u2029!(); h\u202f!(); i\u205f!(); j\u3000!(); k\ufeff!(); l\u200b!();\n' >> "$work/package/src/edges.rs"
    printf '\u037aunsafe \u309bawait \u2118async \u212edyn unsafe\u00b7 await\u0301 unsafe\u200c async\u200d \u1c89extern \U00016ea0unsafe \U00010400::\u03b1!();\n' >> "$work/package/src/edges.rs"
    printf '🦀 #[test] async\r\n' > "$work/package/tests/𐐀.rs"
    printf '#![test]\n#[cfg_attr(x, test)]\n' > "$work/package/tests/other.rs"
    printf 'unsafe' > "$work/package/extra/one.rs"
    : > "$work/package/src/empty.rs"
    printf 'await\n' > "$work/package/src/"$'\ue000.rs'
    printf 'dyn\n' > "$work/package/src/"$'\U00010000.rs'
    printf 'extern\n' > "$work/package/src/"$'del\x7f\nname.rs'
    printf 'async\n' > "$work/package/examples/skipped.rs"
    printf 'unsafe\n' > "$work/package/src/not-rust.txt"
    printf 'dyn\n' > "$work/package/root.rs"
    mkfifo "$work/package/src/fifo.rs"
    mkdir -p "$work/fallback/inside"
    printf '#[test] fn f() {}' > "$work/fallback/inside/a.rs"
    mkdir -p "$work/prototype/src" "$work/dot-link"
    printf '#[cfg_attr(constructor, test)]\n#[derive(toString, Debug)]\n#[derive(r#constructor)]\n#[test]\n' > "$work/prototype/src/a.rs"
    printf 'unsafe\n' > "$work/dot-link/a.rs"
    ln -s a.rs "$work/dot-link/b.txt"
    # Invalid inputs must be isolated so that each reason is independent.
    local kind
    for kind in linked entry-link default-link nonutf8-name nonutf8-file comment raw string empty; do
        mkdir -p "$work/$kind/src"
    done
    printf 'unsafe\n' > "$work/linked/src/ok.rs"
    ln -s src "$work/linked/link"
    printf 'unsafe\n' > "$work/entry-link/src/ok.rs"
    ln -s ok.rs "$work/entry-link/src/not-rust.txt"
    mkdir "$work/default-link/actual"
    printf 'unsafe\n' > "$work/default-link/actual/ok.rs"
    rmdir "$work/default-link/src"
    ln -s actual "$work/default-link/src"
    printf x > "$work/nonutf8-name/src/"$'\xff.rs'
    printf '\xff' > "$work/nonutf8-file/src/bad.rs"
    printf '😀 /* nested /* */' > "$work/comment/src/bad.rs"
    printf '😀 r##"missing"#' > "$work/raw/src/bad.rs"
    printf '😀 b"missing' > "$work/string/src/bad.rs"
    printf x > "$work/not-dir"
}

success() {
    local label=$1 expected=$2
    shift 2
    if ! "$runner" "$@" > "$work/$label.out" 2>"$work/$label.err"; then
        printf "%s: successful census refused\n" "$label" >&2
        cat "$work/$label.err" >&2
        return 1
    fi
    test ! -s "$work/$label.err"
    if ! printf '%s  %s\n' "$expected" "$work/$label.out" | sha256sum --check --strict --quiet; then
        printf "%s: canonical report hash mismatch\n" "$label" >&2
        sha256sum "$work/$label.out" >&2
        return 1
    fi
}
refusal() {
    local label=$1 reason=$2 status=0
    shift 2
    "$runner" "$@" > "$work/$label.out" 2>"$work/$label.err" || status=$?
    if [[ $status != 255 || -s $work/$label.out ]]; then
        printf '%s: expected status 255 and empty stdout, got %s\n' "$label" "$status" >&2
        return 1
    fi
    printf 'real-programs-census: %s\n' "$reason" | cmp - "$work/$label.err"
}
main() {
    cd "$(dirname "$0")/../.."
    local compiler=${1:-target/lkjscript} entry=${2:-tools/real-programs/census.lkj}
    case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
    mkdir -p target
    work=$(mktemp -d "$PWD/target/census-test.XXXXXX")
    trap 'rm -rf -- "$work"' EXIT
    export LC_ALL=C.UTF-8
    "$compiler" build "$entry" --library-dir library --no-store \
        --grant arguments --grant root=env:LKJSCRIPT_CENSUS_ROOT --grant package=LKJSCRIPT_CENSUS_ROOT \
        --grant defaults=LKJSCRIPT_CENSUS_DEFAULTS --grant output --grant errors --grant status \
        -o "$work/census"
    export LKJSCRIPT_CENSUS_BINARY="$work/census"
    runner="$PWD/tools/real-programs/census.sh"
    make_fixtures "$work"
    success self b52d7670c99756ab5d32c9a591f1490119b4856fd3a5c2b66b15b0f8863bc1c1 --self-test
    success defaults 9ccd14b2245d0514bcbec8c65dc9dcabb704a9e63b1fcb6bc0cc2f6d7fb328e1 "$work/package"
    success overlap f5fa9799cf1dd9e60912abf5657913f65836bda0a17bdf43f04c09c58acc3195 --source-root=./src// --source-root . --source-root src --test-root tests --test-root=src/nested "$work/package"
    success explicit b3a383a1b250002eda15e9de52ecf016a16f00b5e6b496129a671f40991d9e72 --source-root extra/one.rs --test-root=src/edges.rs --self-test "$work/package"
    success fallback 8cd7741859582a94fcf3d02b5eb29353604c62322c98506af49c4c45878d84c7 "$work/fallback"
    success dot be6d654f55ca351456dfb41f982e650f5426dd73126926adcf9bc2f0aa25751f --source-root= --test-root=./tests// "$work/package"
    success prototype 41f56f6985988c41e01b91eadc5deb02d8eeb7b8a72b8b5e824824eff64c5089 "$work/prototype"
    "$runner" --help > "$work/help.out" 2>"$work/help.err"
    test ! -s "$work/help.err"
    printf 'Usage: census.sh [--source-root REL] [--test-root REL] [--self-test] PACKAGE_ROOT\nOnly .rs files are inventoried. Test roots override source roots. Symlinks fail.\nCounts are lexical inventories, not compiler refusals or macro expansions.\n' | cmp - "$work/help.out"
    refusal root-link 'symlink root component: link' --source-root=link "$work/linked"
    refusal root-intermediate-link 'symlink root component: link/ok.rs' --source-root link/ok.rs "$work/linked"
    refusal entry-link "symlink under selected root: $work/entry-link/src/not-rust.txt" "$work/entry-link"
    refusal dot-link "symlink under selected root: $work/dot-link/./b.txt" "$work/dot-link"
    refusal default-link 'symlink root component: src' "$work/default-link"
    refusal traversal 'parent traversal in root: src/../tests' --source-root src/../tests "$work/package"
    refusal absolute 'root must be package-relative: /tmp' --test-root=/tmp "$work/package"
    refusal filename 'non-UTF-8 file name under selected roots' "$work/nonutf8-name"
    refusal content 'src/bad.rs is not UTF-8' "$work/nonutf8-file"
    refusal comment 'unterminated block comment at character 2' "$work/comment"
    refusal raw 'unterminated raw string at character 2' "$work/raw"
    refusal string 'unterminated string at character 2' "$work/string"
    refusal empty 'no .rs files under selected roots' "$work/empty"
    refusal not-dir "not a directory: $work/not-dir" "$work/not-dir"
    refusal missing-value 'missing root argument' --source-root
    refusal bad-flag 'use --help for usage' --wat "$work/package"
    refusal no-positional 'exactly one package root is required; use --help'
    refusal extra-positional 'exactly one package root is required; use --help' "$work/package" "$work/fallback"
    refusal missing-root 'no such root component: missing' --source-root=missing "$work/package"
    refusal missing-package "cannot resolve package root: $work/missing" "$work/missing"
    refusal invalid-arg 'non-UTF-8 argument' --source-root=$'\xff' "$work/package"
    printf 'ok source census: 19 self-tests, 6 exact package reports, help and 21 refusals\n'
}
# Source only to reconstruct the same disposable fixtures for an external
# differential comparison; the normal gate invokes main.
if [[ ${BASH_SOURCE[0]} == "$0" ]]; then main "$@"; fi
