#!/usr/bin/env bash
# Optional real-compositor proof: one entry, six grants, no named frame storage.
set -euo pipefail
cd "$(dirname "$0")/../../.."
root=$PWD
proof=$root/docs/proofs/anonymous-wayland-20261118
compiler=$root/target/lkjscript
deps=/home/coder/workspace/lkjbrowser/.deps/root
output=$proof/evidence
engine=native
while (($#)); do
    case $1 in
        --compiler) compiler=$2; shift 2 ;;
        --deps) deps=$2; shift 2 ;;
        --output) output=$2; shift 2 ;;
        --engine) engine=$2; shift 2 ;;
        *) printf 'usage: %s [--compiler PATH] [--deps EXISTING_ROOT] [--output DIR] [--engine native|interpreter]\n' "$0" >&2; exit 2 ;;
    esac
done
[[ $engine == native || $engine == interpreter ]]
compiler=$(realpath "$compiler")
deps=$(realpath "$deps")
mkdir -p "$output"
output=$(realpath "$output")
[[ -x $compiler && -x $deps/usr/bin/weston && -x $deps/usr/bin/weston-screenshooter ]]
work=$(mktemp -d /tmp/lkjscript-anonymous-frame.XXXXXX)
compositor= client= peer=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    for pid in "$client" "$compositor" "$peer"; do
        if [[ -n $pid ]]; then kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi
    done
    if ((code)); then
        for file in "$output/build.err" "$output/client.err" "$output/client.log" "$output/weston-stdio.log" "$output/peer.err"; do
            [[ ! -f $file ]] || tail -30 "$file" >&2
        done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
mkdir "$work/runtime" "$work/shot"
chmod 700 "$work/runtime"
cc -std=c11 -O2 -Wall -Wextra -Werror "$root/tools/connect-deadline-peer.c" -o "$work/peer"
cc -std=c11 -O2 -Wall -Wextra -Werror -fPIC -shared "$proof/weston-input.c" -o "$work/weston-input.so"
"$work/peer" reply >"$output/peer-port.txt" 2>"$output/peer.err" &
peer=$!
for ((attempt=0;attempt<6000;attempt++)); do [[ ! -s $output/peer-port.txt ]] || break; kill -0 "$peer"; sleep 0.01; done
read -r port < "$output/peer-port.txt"
[[ $port =~ ^[0-9]+$ ]]
grants=(--grant socket=unix:env:LKJSCRIPT_UNIX_FRAME_SOCKET --grant storage --grant "tcp=$port" --grant ready --grant clock --grant output)
{
    date -u '+UTC %Y-%m-%dT%H:%M:%SZ'
    printf 'compiler-source-commit '; git rev-parse HEAD
    printf 'compiler-source-diff-sha256 '; git diff --binary HEAD -- src library | sha256sum
    sha256sum "$compiler" "$proof/main.lkj" "$proof/wayland.lkj" "$proof/weston-input.c" "$root/tools/connect-deadline-peer.c" "$root/docs/proofs/unix-wayland-20261118/verify-png.js"
    printf 'engine %s\nruntime-address-input wl_output.geometry.model\ncompositor-dependencies %s\n' "$engine" "$deps"
    printf 'baseline-protocol-source-commit 375fcc629\n'
} >"$output/provenance.txt"
{
    printf '%q ' "$compiler" build "$proof/main.lkj" --library-dir "$root/library" --no-store "${grants[@]}" -o "$work/client"; printf '\n'
    printf '%q ' "$compiler" authority "$proof/main.lkj" --library-dir "$root/library" --no-store; printf '\n'
} >"$output/reproduction.txt"
"$compiler" build "$proof/main.lkj" --library-dir "$root/library" --no-store "${grants[@]}" -o "$work/client" >"$output/build.out" 2>"$output/build.err"
"$compiler" authority "$proof/main.lkj" --library-dir "$root/library" --no-store >"$output/authority.txt" 2>"$output/authority.err"
[[ $(sed -n '/^declared authority:/,/^reachable operations/p' "$output/authority.txt" | grep -c '^  ') == 6 ]]
for grant in 'socket: File Unix stream' 'storage: TemporaryFiles' 'tcp: TcpPort' 'ready: Readiness' 'clock: Clock' 'output: Stdout'; do grep -Fq "$grant" "$output/authority.txt"; done
if grep -Eq ': (World|Directory|ReadDirectory)|directory_create|directory_write|directory_rename|directory_remove|world_directory|world_read_directory|world_executable|image_spawn' "$output/authority.txt"; then cat "$output/authority.txt" >&2; exit 1; fi
for operation in world_file file_send_message tcp_port_connector connector_connect_deadline; do grep -wq "$operation" "$output/authority.txt"; done
sha256sum "$work/client" "$work/weston-input.so" "$work/peer" >>"$output/provenance.txt"
export XDG_RUNTIME_DIR="$work/runtime" WAYLAND_DISPLAY=anonymous-frame
export LD_LIBRARY_PATH="$deps/usr/lib/x86_64-linux-gnu:$deps/usr/lib/x86_64-linux-gnu/weston"
export WESTON_MODULE_MAP="headless-backend.so=$deps/usr/lib/x86_64-linux-gnu/libweston-13/headless-backend.so;kiosk-shell.so=$deps/usr/lib/x86_64-linux-gnu/weston/kiosk-shell.so"
"$deps/usr/bin/weston" --version >"$output/weston-version.txt"
weston=("$deps/usr/bin/weston" --backend=headless --renderer=pixman --width=320 --height=240 --shell="$deps/usr/lib/x86_64-linux-gnu/weston/kiosk-shell.so" --modules="$work/weston-input.so" --socket="$WAYLAND_DISPLAY" --idle-time=0 --no-config --debug --log="$output/weston.log")
{
    printf 'LKJSCRIPT_PROOF_TCP_ADDRESS=127.0.0.1 WAYLAND_DEBUG=server '; printf '%q ' "${weston[@]}"; printf '\n'
    printf 'XDG_RUNTIME_DIR=%q\nLD_LIBRARY_PATH=%q\nWESTON_MODULE_MAP=%q\n' "$XDG_RUNTIME_DIR" "$LD_LIBRARY_PATH" "$WESTON_MODULE_MAP"
} >"$output/launch-command.txt"
LKJSCRIPT_PROOF_TCP_ADDRESS=127.0.0.1 WAYLAND_DEBUG=server "${weston[@]}" >"$output/weston-stdio.log" 2>&1 &
compositor=$!
for ((attempt=0;attempt<200;attempt++)); do [[ ! -S $XDG_RUNTIME_DIR/$WAYLAND_DISPLAY ]] || break; kill -0 "$compositor"; sleep 0.02; done
[[ -S $XDG_RUNTIME_DIR/$WAYLAND_DISPLAY ]]
if [[ $engine == native ]]; then command=("$work/client");
else command=("$compiler" run "$proof/main.lkj" --library-dir "$root/library" --no-store --engine interpreter "${grants[@]}"); fi
printf 'LKJSCRIPT_UNIX_FRAME_SOCKET=%q ' "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" >>"$output/reproduction.txt"
printf '%q ' "${command[@]}" >>"$output/reproduction.txt"
printf '\n' >>"$output/reproduction.txt"
LKJSCRIPT_UNIX_FRAME_SOCKET="$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" timeout --kill-after=5s 90s "${command[@]}" >"$output/client.log" 2>"$output/client.err" &
client=$!
for ((attempt=0;attempt<6000;attempt++)); do
    if grep -q '^anonymous frame committed: runtime_address=127.0.0.1 tcp=RQ committed=1 frame_bytes=616704$' "$output/client.log"; then break; fi
    kill -0 "$client"; sleep 0.01
done
grep -q '^anonymous frame committed: runtime_address=127.0.0.1 tcp=RQ committed=1 frame_bytes=616704$' "$output/client.log"
(cd "$work/shot"; "$deps/usr/bin/weston-screenshooter") >"$output/capture.log" 2>&1
shots=("$work/shot"/wayland-screenshot-*.png)
[[ ${#shots[@]} == 1 && -s ${shots[0]} ]]
cp "${shots[0]}" "$output/frame.png"
node "$root/docs/proofs/unix-wayland-20261118/verify-png.js" "$output/frame.png" >"$output/verification.json"
# Socket EOF, after independent pixel verification, is the capture-complete signal.
kill "$compositor"
wait "$compositor"
compositor=
wait "$client"
client=
grep -q '^anonymous frame complete: .*committed=1$' "$output/client.log"
[[ ! -s $output/client.err && ! -s $output/peer.err ]]
kill "$peer"
wait "$peer" || true
peer=
printf 'ok anonymous frame %s: six grants, 320x240, 76800 exact Weston pixels, runtime IPv4 TcpPort R/Q exchange; evidence %s\n' "$engine" "$output" | tee "$output/result.txt"
