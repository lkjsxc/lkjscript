#!/usr/bin/env bash
# Real pending connections, delayed peer bytes and descriptor cleanup, all on loopback.
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
mkdir -p target
work=$(mktemp -d "$root/target/connect-deadline.XXXXXX")
peer=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if [[ -n $peer ]]; then kill "$peer" 2>/dev/null || true; wait "$peer" 2>/dev/null || true; fi
    if ((code)); then
        for log in "$work/build.err" "$work/peer.err" "$work/run.err" "$work/witness.err"; do [[ ! -f $log ]] || cat "$log" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
cc -std=c11 -O2 -Wall -Wextra -Werror tools/connect-deadline-peer.c -o "$work/peer"
source=$root/tests/fixtures/authority/connect_deadline_boundary.lkj
routes=(native interpreter)
if [[ -n $witness ]]; then
    # Supply the explicit Arguments entry grant inside the optional witness host.
    {
        cat "$root/witness/bridge/output.rs"
        printf '\npub fn argument_grant(world: &mut World) -> Arguments { Arguments { arguments: world.args() } }\n'
    } > "$work/host.rs"
    {
        printf '#![allow(dead_code, unused_imports)]\n#[path = "%s/host.rs"] mod host;\n' "$work"
        printf 'mod specimen { use super::host::{Arguments, Clock, Connector, Stdout}; include!("%s"); }\n' "$source"
        cat <<'RUST'
fn enter(mut world: host::World) {
    let port = std::env::var("LKJSCRIPT_CONNECT_DEADLINE_PORT").unwrap().parse::<u16>().unwrap();
    let target = world.connector("127.0.0.1", port).unwrap();
    let clock = world.clock();
    let arguments = host::argument_grant(&mut world);
    let out = world.stdout();
    specimen::main(target, clock, arguments, out);
}
fn main() { host::enter(enter); }
RUST
    } > "$work/witness.rs"
    "$witness" --edition=2021 --crate-name connect_deadline_boundary -C opt-level=0 "$work/witness.rs" -o "$work/rust" >"$work/witness.out" 2>"$work/witness.err"
    routes+=(witness)
fi
for mode in timeout reply; do
    : > "$work/port"
    "$work/peer" "$mode" >"$work/port" 2>"$work/peer.err" &
    peer=$!
    for ((attempt=0; attempt<6000; attempt++)); do
        [[ ! -s $work/port ]] || break
        kill -0 "$peer" 2>/dev/null || { cat "$work/peer.err" >&2; exit 1; }
        sleep 0.01
    done
    read -r port < "$work/port"
    [[ $port =~ ^[0-9]+$ ]] || exit 1
    export LKJSCRIPT_CONNECT_DEADLINE_PORT=$port
    grants=(--grant "target=127.0.0.1:$port" --grant clock --grant arguments --grant out)
    "$compiler" build "$source" --library-dir "$root/library" --no-store "${grants[@]}" -o "$work/native" >"$work/build.out" 2>"$work/build.err"
    "$compiler" authority "$source" --library-dir "$root/library" >"$work/authority" 2>>"$work/build.err"
    grep -q connector_connect_deadline "$work/authority"
    for route in "${routes[@]}"; do
        if [[ $route == native ]]; then command=("$work/native" "$mode")
        elif [[ $route == interpreter ]]; then command=("$compiler" run "$source" --library-dir "$root/library" --no-store --engine interpreter "${grants[@]}" -- "$mode")
        else command=("$work/rust" "$mode"); fi
        # More failed connects than the process descriptor limit expose leaked attempts.
        (ulimit -n 64; timeout --kill-after=5s 90s "${command[@]}") >"$work/run.out" 2>"$work/run.err"
        [[ $(cat "$work/run.out") == 'deadline boundary ok' && ! -s $work/run.err ]] || exit 1
        printf 'ok connect deadline %s %s: bounded wait, exact errno, owned cleanup\n' "$route" "$mode"
    done
    if [[ $mode == reply ]]; then
        (ulimit -n 64; timeout --kill-after=5s 90s "$work/native" "$mode" 0<&- 2>&-) >"$work/run.out"
        [[ $(cat "$work/run.out") == 'deadline boundary ok' ]] || exit 1
        printf 'ok connect deadline native: preserves closed stdin/stderr while promoting its socket\n'
    fi
    kill "$peer"
    wait "$peer" 2>/dev/null || true
    peer=
done
# A separately checked object must retain the platform feature through cold/warm linkage.
mkdir "$work/unit-source" "$work/unit-store"
unit_key=$(XDG_STATE_HOME="$work/unit-key" sh tools/store-key.sh)
cat > "$work/unit-source/units.lkj" <<'SOURCE'
const UNITS: &[&str] = &["app=app.lkj"];
const DEPENDS: &[&str] = &[];
const ENTRY: &str = "app::main";
SOURCE
cat > "$work/unit-source/app.lkj" <<'SOURCE'
pub fn main(mut target: Connector, mut out: Stdout) {
    match target.connect_deadline(0usize) { Err(error) => assert_eq!(error, 110), Ok(_) => panic!("expired connection") }
    out.write(b"unit deadline ok\n");
}
SOURCE
unit_options=(units.lkj --source-dir "$work/unit-source" --library-dir "$root/library"
    --store "$work/unit-store" --store-key "$unit_key" --grant target=127.0.0.1:9 --grant out)
"$compiler" build-units "${unit_options[@]}" -o "$work/unit-cold" >"$work/build.out" 2>"$work/build.err"
[[ $("$work/unit-cold") == 'unit deadline ok' ]]
"$compiler" build-units "${unit_options[@]}" -o "$work/unit-warm" >"$work/build.out" 2>"$work/build.err"
cmp "$work/unit-cold" "$work/unit-warm"
[[ $("$work/unit-warm") == 'unit deadline ok' ]]
printf 'ok connect deadline: cold/warm source units retain the checked platform feature\n'
