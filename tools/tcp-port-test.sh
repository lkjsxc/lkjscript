#!/usr/bin/env bash
# Explicit outbound ports choose run-time IPv4 hosts without ambient World authority.
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
work=$(mktemp -d "$root/target/tcp-port.XXXXXX")
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
"$work/peer" reply >"$work/port" 2>"$work/peer.err" &
peer=$!
for ((attempt=0; attempt<6000; attempt++)); do
    [[ ! -s $work/port ]] || break
    kill -0 "$peer" 2>/dev/null || exit 1
    sleep 0.01
done
read -r port < "$work/port"
[[ $port =~ ^[0-9]+$ ]] || exit 1
export LKJSCRIPT_TCP_PORT=$port
source=$root/tests/fixtures/authority/tcp_port_boundary.lkj
grants=(--grant "tcp=$port" --grant clock --grant arguments --grant out)
"$compiler" build "$source" --library-dir "$root/library" --no-store "${grants[@]}" -o "$work/native" >"$work/build.out" 2>"$work/build.err"
"$compiler" authority "$source" --library-dir "$root/library" >"$work/authority" 2>>"$work/build.err"
grep -Fq 'tcp: TcpPort (--grant tcp=PORT)' "$work/authority"
grep -q tcp_port_connector "$work/authority"
# Arguments uses the existing world_arg_* operation names with an Arguments receiver.
if grep -Eq ': World|world_tcp_port|world_connector|world_connect|world_listen|listener_' "$work/authority"; then cat "$work/authority" >&2; exit 1; fi
routes=(native interpreter)
if [[ -n $witness ]]; then
    {
        cat "$root/witness/bridge/output.rs"
        printf '\npub fn argument_grant(world: &mut World) -> Arguments { Arguments { arguments: world.args() } }\n'
    } > "$work/host.rs"
    {
        printf '#![allow(dead_code, unused_imports)]\n#[path = "%s/host.rs"] mod host;\n' "$work"
        printf 'mod specimen { use super::host::{Arguments, Clock, TcpPort, Stdout}; include!("%s"); }\n' "$source"
        cat <<'RUST'
fn enter(mut world: host::World) {
    let port = std::env::var("LKJSCRIPT_TCP_PORT").unwrap().parse::<u16>().unwrap();
    let tcp = world.tcp_port(port).unwrap();
    let clock = world.clock();
    let arguments = host::argument_grant(&mut world);
    let out = world.stdout();
    specimen::main(tcp, clock, arguments, out);
}
fn main() { host::enter(enter); }
RUST
    } > "$work/witness.rs"
    "$witness" --edition=2021 --crate-name tcp_port_boundary -C opt-level=0 "$work/witness.rs" -o "$work/rust" >"$work/witness.out" 2>"$work/witness.err"
    routes+=(witness)
fi
for route in "${routes[@]}"; do
    if [[ $route == native ]]; then command=("$work/native" 127.0.0.1)
    elif [[ $route == interpreter ]]; then command=("$compiler" run "$source" --library-dir "$root/library" --no-store --engine interpreter "${grants[@]}" -- 127.0.0.1)
    else command=("$work/rust" 127.0.0.1); fi
    timeout --kill-after=5s 90s "${command[@]}" >"$work/run.out" 2>"$work/run.err"
    [[ $(cat "$work/run.out") == 'tcp port boundary ok' && ! -s $work/run.err ]] || exit 1
    printf 'ok tcp port %s: run-time numeric host, immutable grant port, no World\n' "$route"
done
for value in '' 0 65536 -1 80,443 env:PORT 127.0.0.1:80 184467440737095516160; do
    if "$compiler" build "$source" --library-dir "$root/library" --no-store --grant "tcp=$value" --grant clock --grant arguments --grant out -o "$work/refused" >"$work/run.out" 2>"$work/run.err"; then exit 1; fi
    grep -q E_ENTRY "$work/run.err"
    [[ ! -e $work/refused ]]
done
for missing in missing duplicate; do
    bad=(--grant clock --grant arguments --grant out)
    [[ $missing != duplicate ]] || bad+=(--grant "tcp=$port" --grant "tcp=$port")
    if "$compiler" run "$source" --library-dir "$root/library" --no-store --engine interpreter "${bad[@]}" -- 127.0.0.1 >"$work/run.out" 2>"$work/run.err"; then exit 1; fi
    grep -q E_ENTRY "$work/run.err"
done
printf 'ok tcp port: empty/zero/overflow/negative/list/environment/address/missing/duplicate grants refuse before publication or execution\n'
mkdir "$work/unit-source" "$work/unit-store"
unit_key=$(XDG_STATE_HOME="$work/unit-key" sh tools/store-key.sh)
cat > "$work/unit-source/units.lkj" <<'SOURCE'
const UNITS: &[&str] = &["net=net.lkj", "app=app.lkj"];
const DEPENDS: &[&str] = &["app=net"];
const ENTRY: &str = "app::main";
SOURCE
cat > "$work/unit-source/net.lkj" <<'SOURCE'
pub fn target(tcp: &mut TcpPort, address: &str) -> Result<Connector,i64> { tcp.connector(address) }
SOURCE
sed 's/tcp.connector(address.as_str())/crate::net::target(\&mut tcp, address.as_str())/' "$source" > "$work/unit-source/app.lkj"
unit_options=(units.lkj --source-dir "$work/unit-source" --library-dir "$root/library"
    --store "$work/unit-store" --store-key "$unit_key" "${grants[@]}")
"$compiler" build-units "${unit_options[@]}" -o "$work/unit-cold" >"$work/build.out" 2>"$work/build.err"
[[ $("$work/unit-cold" 127.0.0.1) == 'tcp port boundary ok' ]]
"$compiler" build-units "${unit_options[@]}" -o "$work/unit-warm" >"$work/build.out" 2>"$work/build.err"
cmp "$work/unit-cold" "$work/unit-warm"
[[ $("$work/unit-warm" 127.0.0.1) == 'tcp port boundary ok' ]]
printf 'ok tcp port: cold/warm authenticated units preserve port authority across interfaces\n'
# Child transfer remains explicit rather than creating an unreviewed packet kind.
mkdir "$work/child-source"
printf 'pub fn child(port: TcpPort) {}\n' > "$work/child-source/main.lkj"
if "$compiler" build-child main.lkj --entry child --source-dir "$work/child-source" --library-dir "$root/library" --no-store --image-key "$unit_key" -o "$work/child" >"$work/run.out" 2>"$work/run.err"; then exit 1; fi
grep -q E_CHILD_IMAGE "$work/run.err"
[[ ! -e $work/child && ! -e $work/child.admission ]]
printf 'ok tcp port: unsupported child transfer has a located refusal\n'
