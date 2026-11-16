#!/usr/bin/env bash
# Capability and syscall contracts for absent-only publication and pinned regular reads.
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
work=$(mktemp -d "$root/target/directory-rename-new.XXXXXX")
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
fail() { printf 'directory rename_new: %s\n' "$*" >&2; exit 1; }
cat >"$work/specimen.lkj" <<'SOURCE'
fn data(root: &mut Directory, name: &str) -> Vec<u8> {
    let mut file = root.open_regular(name).unwrap();
    file.read_to_end().unwrap()
}
fn invalid_raw(root: &mut Directory, name: &[u8]) {
    assert_eq!(root.__lkjscript_rename_new(name, b"unused\0"), -22i64);
    assert_eq!(root.__lkjscript_rename_new(b"collision\0", name), -22i64);
    assert!(matches!(root.__lkjscript_open_regular(name), Err(22)));
}
fn occupied(root: &mut Directory, name: &str) {
    assert_eq!(root.rename_new("collision", name).err().unwrap(), 17i64);
    assert_eq!(data(root, "collision"), b"collision\n"[..].to_vec());
}
fn special(root: &mut Directory, name: &str) {
    assert!(matches!(root.open_regular(name), Err(22)));
}
fn invalid_public(root: &mut Directory, name: &str) {
    assert_eq!(root.rename_new(name, "unused").err().unwrap(), 22i64);
    assert_eq!(root.rename_new("collision", name).err().unwrap(), 22i64);
    assert!(matches!(root.open_regular(name), Err(22)));
}
pub fn main(mut root: Directory, mut devices: Directory, mut input: Stdin) {
    let mut mode = [0u8; 1];
    assert_eq!(input.read(&mut mode).unwrap(), 1usize);
    if mode[0] == b'b' {
        root.rename_new("source", "published").unwrap();
        assert_eq!(data(&mut root, "published"), b"original\n"[..].to_vec());
        assert!(matches!(root.open_regular("source"), Err(2)));
        assert!(matches!(root.open_regular("missing"), Err(2)));
        assert_eq!(root.rename_new("missing", "absent").err().unwrap(), 2i64);
        occupied(&mut root, "occupied");
        occupied(&mut root, "dangling");
        occupied(&mut root, "folder");
        occupied(&mut root, "fifo");
        occupied(&mut root, "linked");
        occupied(&mut root, "hardlink");
        occupied(&mut root, "collision");
        special(&mut root, "dangling");
        special(&mut root, "folder");
        special(&mut root, "fifo");
        special(&mut root, "linked");
        assert!(matches!(devices.open_regular("null"), Err(22)));
        invalid_public(&mut root, "");
        invalid_public(&mut root, ".");
        invalid_public(&mut root, "..");
        invalid_public(&mut root, "a/b");
        invalid_public(&mut root, "nul\0tail");
        invalid_raw(&mut root, b"");
        invalid_raw(&mut root, b"\0");
        invalid_raw(&mut root, b".\0");
        invalid_raw(&mut root, b"..\0");
        invalid_raw(&mut root, b"a/b\0");
        invalid_raw(&mut root, b"a\0b\0");
        invalid_raw(&mut root, b"unterminated");
        root.rename_new("κ", "λ").unwrap();
        assert_eq!(data(&mut root, "λ"), b"unicode\n"[..].to_vec());
        assert_eq!(root.__lkjscript_rename_new(b"\xff\0", b"\xfe\0"), 0i64);
        let mut raw = root.__lkjscript_open_regular(b"\xfe\0").unwrap();
        assert_eq!(raw.read_to_end().unwrap(), b"raw\n"[..].to_vec());
        let mut readonly = root.open_regular("published").unwrap();
        assert_eq!(readonly.write(b"bad").err().unwrap(), 9i64);
        assert_eq!(readonly.read_to_end().unwrap(), b"original\n"[..].to_vec());
    } else if mode[0] == b'r' {
        assert!(matches!(root.open_regular("winner"), Err(2)));
        assert_eq!(root.rename_new("source", "winner").err().unwrap(), 17i64);
        assert_eq!(data(&mut root, "source"), b"original\n"[..].to_vec());
        assert_eq!(data(&mut root, "winner"), b"competing\n"[..].to_vec());
    } else if mode[0] == b'p' {
        assert_eq!(data(&mut root, "observed"), b"original\n"[..].to_vec());
    } else if mode[0] == b'e' || mode[0] == b'u' {
        let expected = if mode[0] == b'e' { 38i64 } else { 95i64 };
        assert_eq!(root.rename_new("source", "denied").err().unwrap(), expected);
        assert_eq!(data(&mut root, "source"), b"original\n"[..].to_vec());
        assert!(matches!(root.open_regular("denied"), Err(2)));
    } else {
        panic!("unknown fixture mode");
    }
}
SOURCE
bounded cc -std=c11 -O2 -Wall -Wextra -Werror tools/directory-rename-new-witness.c -o "$work/boundary"
args=("$work/specimen.lkj" --library-dir "$root/library" --no-store
    --grant root=env:LKJSCRIPT_RENAME_NEW_ROOT --grant devices=/dev --grant input)
bounded "$compiler" build "${args[@]}" -o "$work/native" >"$work/build.out" 2>"$work/build.err"
bounded "$compiler" authority "${args[@]}" >"$work/authority" 2>>"$work/build.err"
grep -Fq directory_rename_new "$work/authority" || fail 'authority omitted no-replace effect'
grep -Fq directory_open_regular "$work/authority" || fail 'authority omitted regular-read effect'
routes=(native interpreter)
if [[ -n $witness ]]; then
    {
        printf '#![allow(dead_code, unused_imports)]\n#[path = "%s/witness/bridge/output.rs"] mod host;\n' "$root"
        printf 'mod specimen { use super::host::{Directory, Stdin}; include!("%s/specimen.lkj"); }\n' "$work"
        cat <<'RUST'
fn enter(mut world: host::World) {
    let root = world.directory(&std::env::var("LKJSCRIPT_RENAME_NEW_ROOT").unwrap()).unwrap();
    let devices = world.directory("/dev").unwrap();
    let input = world.stdin();
    specimen::main(root, devices, input);
}
fn main() { host::enter(enter); }
RUST
    } >"$work/witness.rs"
    bounded "$witness" --edition=2021 --crate-name directory_boundary -C opt-level=0 "$work/witness.rs" -o "$work/rust" >"$work/witness.out" 2>"$work/witness.err"
    routes+=(witness)
fi
for route in "${routes[@]}"; do
    if [[ $route == native ]]; then command=("$work/native")
    elif [[ $route == interpreter ]]; then command=("$compiler" run "${args[@]}" --engine interpreter)
    else command=("$work/rust"); fi
    directory="$work/$route-basic"
    mkdir "$directory" "$directory/folder"
    printf 'original\n' >"$directory/source"
    printf 'collision\n' >"$directory/collision"
    printf 'occupied\n' >"$directory/occupied"
    printf 'unicode\n' >"$directory/κ"
    printf 'raw\n' >"$directory/"$'\xff'
    ln -s missing "$directory/dangling"
    ln -s collision "$directory/linked"
    ln "$directory/collision" "$directory/hardlink"
    mkfifo "$directory/fifo"
    export LKJSCRIPT_RENAME_NEW_ROOT="$directory"
    stat -c '%n %d %i %F' "$directory/occupied" "$directory/dangling" "$directory/folder" "$directory/fifo" "$directory/linked" "$directory/hardlink" >"$work/$route-before"
    printf b >"$work/input"
    bounded "${command[@]}" <"$work/input" >"$work/run.out" 2>"$work/run.err"
    [[ ! -s "$work/run.err" ]] || fail "$route wrote diagnostics"
    stat -c '%n %d %i %F' "$directory/occupied" "$directory/dangling" "$directory/folder" "$directory/fifo" "$directory/linked" "$directory/hardlink" >"$work/$route-after"
    cmp "$work/$route-before" "$work/$route-after"
    [[ $(cat "$directory/occupied") == occupied && $(readlink "$directory/dangling") == missing && $(readlink "$directory/linked") == collision ]]
    [[ ! -e "$directory/unused" && ! -e "$directory/absent" ]]
    printf 'ok directory rename_new %s: absent success, 7 occupied destinations, public/raw basenames, UTF-8/raw bytes, regular-only read and readonly File\n' "$route"
    for kind in rename-race pin-race; do
        directory="$work/$route-$kind"
        mkdir "$directory"
        if [[ $kind == rename-race ]]; then printf 'original\n' >"$directory/source"; printf r >"$work/input"
        else printf 'original\n' >"$directory/observed"; printf p >"$work/input"; fi
        export LKJSCRIPT_RENAME_NEW_ROOT="$directory"
        bounded "$work/boundary" "$kind" "$directory" "${command[@]}" <"$work/input" >"$work/run.out" 2>"$work/run.err"
        cat "$work/run.out"
        [[ ! -s "$work/run.err" ]] || fail "$route $kind wrote diagnostics"
    done
    for error in 38 95; do
        directory="$work/$route-denied-$error"
        mkdir "$directory"
        printf 'original\n' >"$directory/source"
        if ((error == 38)); then printf e >"$work/input"; else printf u >"$work/input"; fi
        export LKJSCRIPT_RENAME_NEW_ROOT="$directory"
        bounded "$work/boundary" deny "$error" "${command[@]}" <"$work/input" >"$work/run.out" 2>"$work/run.err"
        [[ ! -s "$work/run.err" && ! -e "$directory/denied" ]] || fail "$route failure did not stay closed"
    done
    printf 'ok directory rename_new %s: ENOSYS/EOPNOTSUPP propagate without a replacing fallback\n' "$route"
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
type "&[u8]" ref shared 9
fn "probe" 3 3
local "root" 8 let
local "old" 10 let
local "new" 10 let
local "result" 3 let
@1 intrinsic 3 "directory_rename_new" 0 1 2
@2 return 3
endfn
IR
bounded "$compiler" check "$work/ref-good.lkjir" --no-store >"$work/check.out" 2>"$work/run.err"
for bad in immutable read-authority result arity bytes read-result; do
    case $bad in
        read-authority) sed 's/type "Directory" directory/type "Directory" read_directory/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        immutable) sed 's/ref mut 7/ref shared 7/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        result) sed -e 's/fn "probe" 3 3/fn "probe" 3 4/' -e 's/local "result" 3 let/local "result" 4 let/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        arity) sed 's/"directory_rename_new" 0 1 2/"directory_rename_new" 0 1/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        bytes) sed 's/ref shared 9/ref mut 9/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
        read-result) sed 's/"directory_rename_new" 0 1 2/"directory_open_regular" 0 1/' "$work/ref-good.lkjir" >"$work/bad.lkjir";;
    esac
    if bounded "$compiler" check "$work/bad.lkjir" --no-store >"$work/check.out" 2>"$work/run.err"; then fail "forged $bad signature accepted"; fi
    grep -Fq E_TYPE "$work/run.err" || fail "forged $bad did not fail type admission"
done
cat >"$work/read-good.lkjir" <<'IR'
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
type "&[u8]" ref shared 9
type "File" file
type "Opened" enum move
variant "Ok" "value" 11
variant "Err" "error" 3
endtype
fn "probe" 2 12
local "root" 8 let
local "name" 10 let
local "result" 12 let
@1 intrinsic 2 "directory_open_regular" 0 1
@2 return 2
endfn
IR
bounded "$compiler" check "$work/read-good.lkjir" --no-store >"$work/check.out" 2>"$work/run.err"
for bad in immutable read-authority arity bytes result; do
    case $bad in
        immutable) sed 's/ref mut 7/ref shared 7/' "$work/read-good.lkjir" >"$work/bad.lkjir";;
        read-authority) sed 's/type "Directory" directory/type "Directory" read_directory/' "$work/read-good.lkjir" >"$work/bad.lkjir";;
        arity) sed 's/"directory_open_regular" 0 1/"directory_open_regular" 0/' "$work/read-good.lkjir" >"$work/bad.lkjir";;
        bytes) sed 's/ref shared 9/ref mut 9/' "$work/read-good.lkjir" >"$work/bad.lkjir";;
        result) sed 's/variant "Ok" "value" 11/variant "Ok" "value" 0/' "$work/read-good.lkjir" >"$work/bad.lkjir";;
    esac
    if bounded "$compiler" check "$work/bad.lkjir" --no-store >"$work/check.out" 2>"$work/run.err"; then fail "forged regular-read $bad accepted"; fi
    grep -Fq E_TYPE "$work/run.err" || fail "forged regular-read $bad did not fail type admission"
done
printf 'ok directory rename_new: independent checked-IR mutable authority, ReadDirectory refusal, arity, byte-view and result signatures\n'
ended=$(date +%s%N)
load_after=$(cat /proc/loadavg)
awk -v elapsed="$((ended-started))" -v before="$load_before" -v after="$load_after" -v routes="${#routes[@]}" 'BEGIN { printf "ok directory rename_new suite: %d routes, %.3f s, load %s -> %s\n", routes, elapsed/1000000000, before, after }'
