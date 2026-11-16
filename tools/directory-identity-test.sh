#!/usr/bin/env bash
# Exact held-directory identity and unchanged-on-error buffers in all three engines.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
shift $(( $# > 0 ? 1 : 0 ))
witness=
if (($#)); then
    [[ $1 == --witness && $# == 2 ]] || { printf 'usage: %s [COMPILER] [--witness RUSTC]\n' "$0" >&2; exit 2; }
    witness=$2
fi
root=$PWD
[[ $compiler == /* ]] || compiler="$root/$compiler"
work=$(mktemp -d "$root/target/directory-identity.XXXXXX")
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then
        for file in "$work/build.err" "$work/run.err" "$work/witness.err"; do [[ ! -f $file ]] || cat "$file" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
started=$(date +%s%N)
load_before=$(cat /proc/loadavg)
bounded() { prlimit --as=4294967296 -- timeout --kill-after=5s 900s "$@"; }
fail() { printf 'directory identity: %s\n' "$*" >&2; exit 1; }
cat >"$work/specimen.lkj" <<'SOURCE'
fn malformed(root: &mut Directory) {
    let mut empty = [0u8; 0];
    assert_eq!(root.__lkjscript_identity(&mut empty), -22i64);
    let mut short = [0xa5u8; 15];
    assert_eq!(root.__lkjscript_identity(&mut short), -22i64);
    assert_eq!(short, [0xa5u8; 15]);
    let mut long = [0xa5u8; 17];
    assert_eq!(root.__lkjscript_identity(&mut long), -22i64);
    assert_eq!(long, [0xa5u8; 17]);
}
pub fn main(mut root: Directory, mut input: Stdin, mut out: Stdout) {
    let mut mode = [0u8; 1];
    assert_eq!(input.read(&mut mode).unwrap(), 1usize);
    malformed(&mut root);
    if mode[0] == b'n' {
        let before = std::io::directory_identity(&mut root).unwrap();
        let mut alias = root.open_directory(".").unwrap();
        assert_eq!(std::io::directory_identity(&mut alias).unwrap(), before);
        let mut bytes = [0xa5u8; 16];
        assert_eq!(root.__lkjscript_identity(&mut bytes), 0i64);
        assert_eq!(bytes, before);
        out.write(&before);
        assert_eq!(input.read(&mut mode).unwrap(), 1usize);
        assert_eq!(mode[0], b'x');
        let after = std::io::directory_identity(&mut root).unwrap();
        assert_eq!(after, before);
        out.write(&after);
    } else if mode[0] == b'e' {
        let mut bytes = [0xa5u8; 16];
        assert_eq!(root.__lkjscript_identity(&mut bytes), -5i64);
        assert_eq!(bytes, [0xa5u8; 16]);
        assert_eq!(std::io::directory_identity(&mut root).err().unwrap(), 5i64);
        out.write(b"ok\n");
    } else {
        panic!("unknown identity fixture mode");
    }
}
SOURCE
bounded cc -std=c11 -O2 -Wall -Wextra -Werror tools/directory-identity-witness.c -o "$work/boundary"
args=("$work/specimen.lkj" --library-dir "$root/library" --no-store
    --grant root=env:LKJSCRIPT_DIRECTORY_IDENTITY_ROOT --grant input --grant out)
bounded "$compiler" build "${args[@]}" -o "$work/native" >"$work/build.out" 2>"$work/build.err"
bounded "$compiler" authority "${args[@]}" >"$work/authority" 2>>"$work/build.err"
grep -Fq directory_identity "$work/authority" || fail 'authority omitted held-directory identity effect'
routes=(native interpreter)
if [[ -n $witness ]]; then
    # Only the explicit facade call in this fixed fixture is rebound; values and raw calls are identical.
    sed 's/std::io::directory_identity/host::directory_identity/g' "$work/specimen.lkj" >"$work/witness-source.rs"
    {
        printf '#![allow(dead_code, unused_imports)]\n#[path = "%s/witness/bridge/output.rs"] mod host;\n' "$root"
        printf 'mod specimen { use super::host; use super::host::{Directory, Stdin, Stdout}; include!("%s/witness-source.rs"); }\n' "$work"
        cat <<'RUST'
fn enter(mut world: host::World) {
    let root = world.directory(&std::env::var("LKJSCRIPT_DIRECTORY_IDENTITY_ROOT").unwrap()).unwrap();
    let input = world.stdin();
    let out = world.stdout();
    specimen::main(root, input, out);
}
fn main() { host::enter(enter); }
RUST
    } >"$work/witness.rs"
    bounded "$witness" --edition=2021 --crate-name directory_identity_boundary -C opt-level=0 "$work/witness.rs" -o "$work/rust" >"$work/witness.out" 2>"$work/witness.err"
    bounded "$witness" --edition=2021 --test witness/build.rs -o "$work/mirror-tests" >>"$work/witness.out" 2>>"$work/witness.err"
    bounded "$work/mirror-tests" --exact directory_binding_preserves_source_literals_and_other_names >>"$work/witness.out" 2>>"$work/witness.err"
    routes+=(witness)
fi
for route in "${routes[@]}"; do
    if [[ $route == native ]]; then command=("$work/native")
    elif [[ $route == interpreter ]]; then command=("$compiler" run "${args[@]}" --engine interpreter)
    else command=("$work/rust"); fi
    directory="$work/$route-race"
    mkdir "$directory"
    export LKJSCRIPT_DIRECTORY_IDENTITY_ROOT="$directory"
    bounded "$work/boundary" race "$directory" "${command[@]}" >"$work/run.out" 2>"$work/run.err"
    cat "$work/run.out"
    [[ ! -s "$work/run.err" ]] || fail "$route held identity wrote diagnostics"
    directory="$work/$route-error"
    mkdir "$directory"
    export LKJSCRIPT_DIRECTORY_IDENTITY_ROOT="$directory"
    printf e >"$work/input"
    bounded "$work/boundary" deny "$directory" "${command[@]}" <"$work/input" >"$work/run.out" 2>"$work/run.err"
    [[ $(cat "$work/run.out") == ok && ! -s "$work/run.err" ]] || fail "$route metadata error did not preserve buffers"
    printf 'ok directory identity %s: lengths 0/15/17 refuse without changes; raw/helper metadata EIO preserves caller bytes\n' "$route"
done
cat >"$work/ref-good.lkjir" <<'IR'
lkjscript-ir 1
type "()" unit
type "bool" bool
type "i32" int 32 signed
type "i64" int 64 signed
type "usize" int 64 unsigned
type "u8" int 8 unsigned
type "f64" f64
type "Directory" directory
type "&mut Directory" ref mut 7
type "[u8]" slice shared 5
type "&mut [u8]" ref mut 9
fn "probe" 2 3
local "root" 8 let
local "bytes" 10 let
local "result" 3 let
@1 intrinsic 2 "directory_identity" 0 1
@2 return 2
endfn
IR
bounded "$compiler" check "$work/ref-good.lkjir" --no-store >"$work/check.out" 2>"$work/run.err"
for bad in immutable read-authority result arity bytes; do
    case $bad in
        immutable) sed 's/ref mut 7/ref shared 7/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        read-authority) sed 's/type "Directory" directory/type "Directory" read_directory/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        result) sed -e 's/fn "probe" 2 3/fn "probe" 2 4/' -e 's/local "result" 3 let/local "result" 4 let/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        arity) sed 's/"directory_identity" 0 1/"directory_identity" 0/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        bytes) sed 's/ref mut 9/ref shared 9/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
    esac
    if bounded "$compiler" check "$work/bad.lkjir" --no-store >"$work/check.out" 2>"$work/run.err"; then fail "forged $bad signature accepted"; fi
    grep -Fq E_TYPE "$work/run.err" || fail "forged $bad did not fail type admission"
done
printf 'ok directory identity: checked-IR mutable Directory and mutable byte-view boundary, arity and result\n'
ended=$(date +%s%N)
load_after=$(cat /proc/loadavg)
awk -v elapsed="$((ended-started))" -v before="$load_before" -v after="$load_after" -v routes="${#routes[@]}" 'BEGIN { printf "ok directory identity suite: %d routes, %.3f s, load %s -> %s\n", routes, elapsed/1000000000, before, after }'
