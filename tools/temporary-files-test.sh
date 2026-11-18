#!/usr/bin/env bash
# Narrow anonymous storage, checked authority, entry controls and unit interfaces.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
shift $(( $# > 0 ? 1 : 0 ))
witness= evidence=
while (($#)); do
    case "$1" in
        --witness) witness=$2; shift 2 ;;
        --evidence) evidence=$2; shift 2 ;;
        *) printf 'usage: %s [COMPILER] [--witness RUSTC] [--evidence DIR]\n' "$0" >&2; exit 2 ;;
    esac
done
root=$PWD
compiler=$(realpath "$compiler")
mkdir -p target
work=$(mktemp -d "$root/target/temporary-files.XXXXXX")
if [[ -n $evidence ]]; then
    mkdir -p "$evidence"
    evidence=$(realpath "$evidence")
else
    evidence=$work/evidence
    mkdir "$evidence"
fi
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then
        printf 'temporary-file proof failed; evidence %s\n' "$evidence" >&2
        for log in "$evidence"/*.stderr; do [[ ! -s $log ]] || tail -12 "$log" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
record() {
    local label=$1 code
    shift
    printf '%q ' "$@" > "$evidence/$label.command"
    printf '\n' >> "$evidence/$label.command"
    if "$@" > "$evidence/$label.stdout" 2> "$evidence/$label.stderr"; then code=0; else code=$?; fi
    printf '%s\n' "$code" > "$evidence/$label.exit"
    return "$code"
}
refuse() {
    local label=$1 expected=$2
    shift 2
    if record "$label" "$@"; then printf 'accepted refusal control %s\n' "$label" >&2; exit 1; fi
    grep -Fq "$expected" "$evidence/$label.stderr"
    [[ ! -s $evidence/$label.stdout ]]
}
source=$root/tests/fixtures/authority/temporary_files_boundary.lkj
grants=(--grant storage --grant output)
record native-build "$compiler" build "$source" --library-dir "$root/library" --no-store "${grants[@]}" -o "$work/native"
record authority "$compiler" authority "$source" --library-dir "$root/library" --no-store
grep -Fq 'storage: TemporaryFiles (--grant storage)' "$evidence/authority.stdout"
grep -Fq 'output: Stdout (--grant output)' "$evidence/authority.stdout"
grep -q world_file "$evidence/authority.stdout"
if grep -Eq ': (World|Directory|ReadDirectory|Image|Process)|directory_|world_(directory|stdout|stdin|stderr|process|listen|connector|connect|tcp_port)' "$evidence/authority.stdout"; then
    cat "$evidence/authority.stdout" >&2
    exit 1
fi
record native timeout --kill-after=5s 90s "$work/native"
record interpreter timeout --kill-after=5s 90s "$compiler" run "$source" --library-dir "$root/library" --no-store --engine interpreter "${grants[@]}"
for route in native interpreter; do
    [[ $(cat "$evidence/$route.stdout") == 'temporary files boundary ok' && ! -s $evidence/$route.stderr ]]
    printf 'ok temporary files %s: independent writable Files, grant can end first, ordinary sealing\n' "$route"
done
if [[ -n $witness ]]; then
    cp "$root/witness/bridge/output.rs" "$work/host.rs"
    {
        printf '#![allow(dead_code, unused_imports)]\n#[path = "%s/host.rs"] mod host;\n' "$work"
        printf 'mod specimen { use super::host::{File, TemporaryFiles, Stdout}; include!("%s"); }\n' "$source"
        printf 'fn main() { host::enter_temporary_files(specimen::main); }\n'
    } > "$work/witness.rs"
    record witness-build "$witness" --edition=2024 --crate-name temporary_files_boundary -C opt-level=0 "$work/witness.rs" -o "$work/rust"
    record witness timeout --kill-after=5s 90s "$work/rust"
    [[ $(cat "$evidence/witness.stdout") == 'temporary files boundary ok' && ! -s $evidence/witness.stderr ]]
    printf 'ok temporary files Rust witness: same writable-file contract and output\n'
fi
for control in missing duplicate unknown payload-empty payload-path payload-port payload-environment payload-socket payload-byte-bound payload-file-bound; do
    case "$control" in
        missing) bad=(--grant output) ;;
        duplicate) bad=(--grant storage --grant storage --grant output) ;;
        unknown) bad=(--grant storage --grant output --grant unexpected) ;;
        payload-empty) bad=(--grant storage= --grant output) ;;
        payload-path) bad=(--grant storage=/tmp --grant output) ;;
        payload-port) bad=(--grant storage=80 --grant output) ;;
        payload-environment) bad=(--grant storage=env:STORAGE --grant output) ;;
        payload-socket) bad=(--grant storage=unix:/tmp/never-connect --grant output) ;;
        payload-byte-bound) bad=(--grant storage=bytes:4096 --grant output) ;;
        payload-file-bound) bad=(--grant storage=files:1 --grant output) ;;
    esac
    refuse "grant-$control-build" E_ENTRY "$compiler" build "$source" --library-dir "$root/library" --no-store "${bad[@]}" -o "$work/refused"
    [[ ! -e $work/refused ]]
    refuse "grant-$control-interpreter" E_ENTRY "$compiler" run "$source" --library-dir "$root/library" --no-store --engine interpreter "${bad[@]}"
done
printf 'preserve this destination\n' > "$work/sentinel"
cp "$work/sentinel" "$work/preserved"
refuse grant-invalid-preserves-output E_ENTRY "$compiler" build "$source" --library-dir "$root/library" --no-store --grant storage=unused --grant output -o "$work/preserved"
cmp "$work/sentinel" "$work/preserved"
printf 'ok temporary files: missing, duplicate, unknown and every defined payload control refuse before execution or publication\n'
mkdir "$work/unit-source" "$work/unit-store"
unit_key=$(XDG_STATE_HOME="$work/unit-key" sh tools/store-key.sh)
cat > "$work/unit-source/units.lkj" <<'SOURCE'
const UNITS: &[&str] = &["buffers=buffers.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["app=buffers"];
const ENTRY: &str = "app::main";
SOURCE
cat > "$work/unit-source/buffers.lkj" <<'SOURCE'
pub fn fresh(storage: &mut TemporaryFiles) -> Result<File, i64> { storage.temporary_file() }
SOURCE
sed 's/storage.temporary_file()/crate::buffers::fresh(storage)/' "$source" > "$work/unit-source/app.lkj"
unit_options=(units.lkj --source-dir "$work/unit-source" --library-dir "$root/library"
    --store "$work/unit-store" --store-key "$unit_key" "${grants[@]}")
record unit-cold-build "$compiler" build-units "${unit_options[@]}" -o "$work/unit-cold"
record unit-cold "$work/unit-cold"
record unit-warm-build "$compiler" build-units "${unit_options[@]}" -o "$work/unit-warm"
record unit-warm "$work/unit-warm"
cmp "$work/unit-cold" "$work/unit-warm"
for route in unit-cold unit-warm; do
    [[ $(cat "$evidence/$route.stdout") == 'temporary files boundary ok' && ! -s $evidence/$route.stderr ]]
done
printf 'ok temporary files: cold/warm authenticated units preserve the narrow grant and returned File\n'
mkdir "$work/child-source"
printf 'pub fn child(storage: TemporaryFiles) {}\n' > "$work/child-source/main.lkj"
refuse child-transfer E_CHILD_IMAGE "$compiler" build-child main.lkj --entry child --source-dir "$work/child-source" --library-dir "$root/library" --no-store --image-key "$unit_key" -o "$work/child"
[[ ! -e $work/child && ! -e $work/child.admission ]]
printf 'ok temporary files: unsupported child transfer refuses without image or admission publication\n'
cat > "$work/world-control.lkj" <<'SOURCE'
pub fn main(mut world: World) {
    let mut input = world.stdin_file().unwrap();
    assert_eq!(input.read_to_end().unwrap(), b"input\n"[..].to_vec());
    let mut image = world.executable().unwrap();
    let mut magic = [0u8; 4];
    assert_eq!(image.read(&mut magic).unwrap(), 4usize);
    assert_eq!(magic[..], [127u8, b'E', b'L', b'F'][..]);
    let mut file = world.temporary_file().unwrap();
    file.write_all(b"old World temporary file").unwrap();
    file.rewind().unwrap();
    assert_eq!(file.read_to_end().unwrap(), b"old World temporary file"[..].to_vec());
    world.stdout_file().unwrap().write_all(b"old World modes ok\n").unwrap();
    world.stderr_file().unwrap().write_all(b"old World stderr ok\n").unwrap();
}
SOURCE
printf 'input\n' > "$work/input"
record world-build "$compiler" build "$work/world-control.lkj" --library-dir "$root/library" --no-store -o "$work/world-control"
record world-native "$work/world-control" < "$work/input"
record world-interpreter "$compiler" run "$work/world-control.lkj" --library-dir "$root/library" --no-store --engine interpreter < "$work/input"
for route in world-native world-interpreter; do
    [[ $(cat "$evidence/$route.stdout") == 'old World modes ok' ]]
    [[ $(cat "$evidence/$route.stderr") == 'old World stderr ok' ]]
done
printf 'ok temporary files: existing World anonymous/stdin/stdout/stderr/executable selectors remain unchanged\n'
