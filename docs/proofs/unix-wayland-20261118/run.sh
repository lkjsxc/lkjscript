#!/usr/bin/env bash
# Optional real compositor proof. Reuse existing binaries without installing dependencies.
set -euo pipefail
cd "$(dirname "$0")/../../.."
compiler=target/lkjscript
deps=/home/coder/workspace/lkjbrowser/.deps/root
output="$PWD/docs/proofs/unix-wayland-20261118/evidence"
while (($#)); do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        --deps) deps=$2; shift 2 ;;
        --output) output=$2; shift 2 ;;
        *) printf 'usage: %s [--compiler PATH] [--deps EXISTING_ROOT] [--output DIR]\n' "$0" >&2; exit 2 ;;
    esac
done
compiler=$(realpath "$compiler")
deps=$(realpath "$deps")
mkdir -p "$output"
output=$(realpath "$output")
[[ -x "$deps/usr/bin/weston" && -x "$deps/usr/bin/weston-screenshooter" ]]
work=$(mktemp -d /tmp/lkjscript-wayland.XXXXXX)
compositor= client=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    for pid in "$client" "$compositor"; do
        if [[ -n "$pid" ]]; then kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi
    done
    if ((code)); then
        for file in "$output/build.err" "$output/client.err" "$output/weston-stdio.log"; do [[ ! -f "$file" ]] || tail -30 "$file" >&2; done
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
mkdir "$work/runtime" "$work/frame" "$work/shot"
chmod 700 "$work/runtime"
{
    date -u '+UTC %Y-%m-%dT%H:%M:%SZ'
    printf 'compiler-source-commit '; git rev-parse HEAD
    printf 'compiler-source-diff-sha256 '; git diff --binary HEAD -- src library | sha256sum
    sha256sum "$compiler" docs/proofs/unix-wayland-20261118/main.lkj docs/proofs/unix-wayland-20261118/wayland.lkj docs/proofs/unix-wayland-20261118/verify-png.js
    printf 'original-browser-source-commit 2d37da55ffd5b881b2d441b049c72642f8ef4b52\noriginal-browser-wayland-sha256 026490c4ca6a0d81ed60922cb54d697dd93329c0561ee07e790311c175e09f2a\n'
    printf 'compositor-dependencies %s\n' "$deps"
} >"$output/provenance.txt"
"$compiler" build docs/proofs/unix-wayland-20261118/main.lkj --library-dir library --no-store --grant socket=unix:env:LKJSCRIPT_UNIX_FRAME_SOCKET --grant root=env:LKJSCRIPT_UNIX_FRAME_ROOT --grant ready --grant clock --grant output -o "$work/client" >"$output/build.out" 2>"$output/build.err"
"$compiler" authority docs/proofs/unix-wayland-20261118/main.lkj --library-dir library --no-store --grant socket=unix:env:LKJSCRIPT_UNIX_FRAME_SOCKET --grant root=env:LKJSCRIPT_UNIX_FRAME_ROOT --grant ready --grant clock --grant output >"$output/authority.txt" 2>"$output/authority.err"
sha256sum "$work/client" >>"$output/provenance.txt"
export XDG_RUNTIME_DIR="$work/runtime"
export WAYLAND_DISPLAY=unix-proof
export LD_LIBRARY_PATH="$deps/usr/lib/x86_64-linux-gnu:$deps/usr/lib/x86_64-linux-gnu/weston"
export WESTON_MODULE_MAP="headless-backend.so=$deps/usr/lib/x86_64-linux-gnu/libweston-13/headless-backend.so;kiosk-shell.so=$deps/usr/lib/x86_64-linux-gnu/weston/kiosk-shell.so"
"$deps/usr/bin/weston" --version >"$output/weston-version.txt"
printf '%q ' "$deps/usr/bin/weston" --backend=headless --renderer=pixman --width=320 --height=240 --shell="$deps/usr/lib/x86_64-linux-gnu/weston/kiosk-shell.so" --socket="$WAYLAND_DISPLAY" --idle-time=0 --no-config --debug --log="$output/weston.log" >"$output/launch-command.txt"
printf '\nXDG_RUNTIME_DIR=%q\nLD_LIBRARY_PATH=%q\nWESTON_MODULE_MAP=%q\n' "$XDG_RUNTIME_DIR" "$LD_LIBRARY_PATH" "$WESTON_MODULE_MAP" >>"$output/launch-command.txt"
"$deps/usr/bin/weston" --backend=headless --renderer=pixman --width=320 --height=240 --shell="$deps/usr/lib/x86_64-linux-gnu/weston/kiosk-shell.so" --socket="$WAYLAND_DISPLAY" --idle-time=0 --no-config --debug --log="$output/weston.log" >"$output/weston-stdio.log" 2>&1 &
compositor=$!
for _ in {1..200}; do [[ ! -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]] || break; kill -0 "$compositor"; sleep 0.02; done
[[ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]]
LKJSCRIPT_UNIX_FRAME_SOCKET="$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" LKJSCRIPT_UNIX_FRAME_ROOT="$work/frame" "$work/client" >"$output/client.log" 2>"$output/client.err" &
client=$!
for _ in {1..300}; do [[ ! -s "$work/frame/ready" ]] || break; kill -0 "$client"; sleep 0.02; done
[[ -s "$work/frame/ready" ]]
sleep 0.1
(cd "$work/shot"; "$deps/usr/bin/weston-screenshooter") >"$output/capture.log" 2>&1
shots=("$work/shot"/wayland-screenshot-*.png)
[[ ${#shots[@]} == 1 && -s ${shots[0]} ]]
cp "${shots[0]}" "$output/frame.png"
node docs/proofs/unix-wayland-20261118/verify-png.js "$output/frame.png" >"$output/verification.json"
sha256sum "$work/frame/frame.bgra" >>"$output/provenance.txt"
printf 'captured' >"$work/frame/capture-done"
wait "$client"
client=
grep -q 'committed=1 frame_bytes=616704' "$output/client.log"
kill "$compositor"
wait "$compositor" || true
compositor=
printf 'ok real Weston frame: 320x240, 76800 exact compositor pixels, direct owned Unix File transport; evidence %s\n' "$output"
