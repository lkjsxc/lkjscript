#!/usr/bin/env bash
# Exact published producer control: Image.buffer creates a sealed anonymous File,
# but real Weston refuses it as wl_shm backing. No compiler/library edits occur.
set -euo pipefail
cd "$(dirname "$0")/../../.."
root=$PWD
proof=$root/bench/anonymous-files-data/image-buffer
compiler=
deps=/home/coder/workspace/lkjbrowser/.deps/root
output=$proof/reproduced
while (($#)); do
    case $1 in
        --compiler) compiler=$2; shift 2 ;;
        --deps) deps=$2; shift 2 ;;
        --output) output=$2; shift 2 ;;
        *) printf 'usage: %s [--compiler EXACT_CA749BD44_BINARY] [--deps EXISTING_ROOT] [--output DIR]\n' "$0" >&2; exit 2 ;;
    esac
done
mkdir -p "$output"
output=$(realpath "$output")
deps=$(realpath "$deps")
work=$(mktemp -d /tmp/lkjscript-sealed-frame.XXXXXX)
compositor=
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if [[ -n $compositor ]]; then kill "$compositor" 2>/dev/null || true; wait "$compositor" 2>/dev/null || true; fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
if [[ -z $compiler ]]; then
    git show ca749bd44:seed/lkjscript.gz | gzip -dc > "$work/lkjscript"
    chmod +x "$work/lkjscript"
    compiler=$work/lkjscript
fi
compiler=$(realpath "$compiler")
[[ $(sha256sum "$compiler" | cut -d' ' -f1) == 3d2fc84dfc880cdd52aee6ca9a16dbef6025500e8453cd7ab75f45f336fda353 ]]
mkdir "$work/source" "$work/runtime"
chmod 700 "$work/runtime"
cp "$proof/main.lkj" "$proof/child.lkj" "$work/source/"
cp "$root/docs/proofs/unix-wayland-20261118/wayland.lkj" "$work/source/"
[[ $(sha256sum "$work/source/wayland.lkj" | cut -d' ' -f1) == 2e88507db513d28d4dd6b73550f908e50d973e90589ab28cd8b0649d14ad55bf ]]
key=$(XDG_STATE_HOME="$work/private" sh "$root/tools/store-key.sh")
{
    date -u '+UTC %Y-%m-%dT%H:%M:%SZ'
    printf 'published-compiler-source '; git rev-parse ca749bd44
    printf 'published-seed-sha256 '; git show ca749bd44:seed/lkjscript.gz | sha256sum
    sha256sum "$compiler" "$proof/main.lkj" "$proof/child.lkj" "$work/source/wayland.lkj"
} >"$output/provenance.txt"
"$compiler" build-child child.lkj --source-dir "$work/source" --no-store --entry child --image-key "$key" -o "$work/child" >"$output/child.authority" 2>"$output/child.err"
"$compiler" build main.lkj --source-dir "$work/source" --no-store --grant socket=unix:env:LKJSCRIPT_UNIX_FRAME_SOCKET --grant "image=$work/child" --image-key "$key" --grant ready --grant clock --grant output -o "$work/client" >"$output/build.out" 2>"$output/build.err"
"$compiler" authority main.lkj --source-dir "$work/source" --no-store >"$output/authority.txt" 2>"$output/authority.err"
export XDG_RUNTIME_DIR="$work/runtime" WAYLAND_DISPLAY=sealed-frame
export LD_LIBRARY_PATH="$deps/usr/lib/x86_64-linux-gnu:$deps/usr/lib/x86_64-linux-gnu/weston"
export WESTON_MODULE_MAP="headless-backend.so=$deps/usr/lib/x86_64-linux-gnu/libweston-13/headless-backend.so;kiosk-shell.so=$deps/usr/lib/x86_64-linux-gnu/weston/kiosk-shell.so"
WAYLAND_DEBUG=server "$deps/usr/bin/weston" --backend=headless --renderer=pixman --width=320 --height=240 --shell="$deps/usr/lib/x86_64-linux-gnu/weston/kiosk-shell.so" --socket="$WAYLAND_DISPLAY" --idle-time=0 --no-config --debug --log="$output/weston.log" >"$output/weston-stdio.log" 2>&1 &
compositor=$!
for ((attempt=0;attempt<200;attempt++)); do [[ ! -S $XDG_RUNTIME_DIR/$WAYLAND_DISPLAY ]] || break; kill -0 "$compositor"; sleep 0.02; done
[[ -S $XDG_RUNTIME_DIR/$WAYLAND_DISPLAY ]]
set +e
LKJSCRIPT_UNIX_FRAME_SOCKET="$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" timeout --kill-after=5s 30s "$work/client" >"$output/client.log" 2>"$output/client.err"
status=$?
set -e
printf 'client_status=%s\n' "$status" >"$output/result.txt"
[[ $status == 1 ]]
grep -q 'sealed Image.buffer frame created; writable alias refused' "$output/client.log"
grep -q 'failed mmap fd .*: Operation not permitted' "$output/weston-stdio.log"
cat "$output/result.txt" "$output/client.log" "$output/client.err"
grep 'failed mmap' "$output/weston-stdio.log"
printf 'ok published Image.buffer control: anonymous sealed File exists, writes refuse, Weston refuses wl_shm mmap\n'
