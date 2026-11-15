#!/usr/bin/env bash
# Binding wrapper only: copied image path, fresh state and foreground stand-in.
set -euo pipefail
cd "$(dirname "$0")/.."
work=$(mktemp -d "$PWD/target/programs-desk-wrapper.XXXXXX")
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then
        for log in "$work/observed" "$work/output"; do [[ ! -f "$log" ]] || cat "$log" >&2; done
    fi
    rm -rf "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
load_before=$(cat /proc/loadavg)
started=$(date +%s%N)
mkdir "$work/state"
sed "s|^image=/home/coder/.local/libexec/lkjscript/desk$|image=$work/image|" examples/desk-command > "$work/desk"
chmod 700 "$work/desk"
cat > "$work/image" <<'IMAGE'
#!/bin/sh
set -eu
printf '%s\n' "$DESK_STATE" "$TZ" "$#" "$@" > "$WRAPPER_TEST_DIR/observed"
if [ "${WRAPPER_CAPTURE_STDIN:-0}" = 1 ]; then cat > "$WRAPPER_TEST_DIR/stdin"; fi
printf '%s' "${WRAPPER_REPLY:-}"
exit "${WRAPPER_EXIT:-0}"
IMAGE
chmod 700 "$work/image"
export WRAPPER_TEST_DIR="$work"
env -u DESK_STATE -u TZ "$work/desk" path
printf '%s\n' /home/coder/.local/state/desk UTC0 1 path > "$work/expected"
cmp "$work/expected" "$work/observed"
DESK_STATE= TZ= "$work/desk" path
printf '%s\n' '' '' 1 path > "$work/expected"
cmp "$work/expected" "$work/observed"
status=0
DESK_STATE="$work/state" TZ=UTC-9 WRAPPER_EXIT=37 "$work/desk" note 'two words' --from fixture || status=$?
test "$status" = 37
printf '%s\n' "$work/state" UTC-9 4 note 'two words' --from fixture > "$work/expected"
cmp "$work/expected" "$work/observed"
export DESK_STATE="$work/state" TZ=UTC0
printf 'inherited input\n日本語\n' > "$work/input"
WRAPPER_CAPTURE_STDIN=1 WRAPPER_REPLY='unchanged stdout' "$work/desk" note --stdin < "$work/input" > "$work/output"
cmp "$work/input" "$work/stdin"
printf '%s' 'unchanged stdout' > "$work/expected"
cmp "$work/expected" "$work/output"
printf '%s\n' "$work/state" UTC0 2 note --stdin > "$work/expected"
cmp "$work/expected" "$work/observed"
for marker in listen-begin listen-end; do
    "$work/desk" "$marker"
    printf '%s\n' "$work/state" UTC0 1 "$marker" > "$work/expected"
    cmp "$work/expected" "$work/observed"
done
printf 'ok desk binding wrapper: literal defaults, explicit overrides, exact argv, stdin/stdout, exit status and marker forwarding; %s ms; load %s -> %s\n' \
    "$((($(date +%s%N)-started)/1000000))" "$load_before" "$(cat /proc/loadavg)"
