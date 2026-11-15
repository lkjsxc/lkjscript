#!/usr/bin/env bash
# Exact repository service settings, fresh roots/ports, and a stand-in relay.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
work=$(mktemp -d "$PWD/target/programs-desk-systemd.XXXXXX")
unit=lkjscript-programs-desk-$(basename "$work" | tr . -)
wake_unit=$unit-wake
active=0
wake_active=0
port=
main_pid=
invocation=
live_before=$(systemctl show desk -p MainPID -p ActiveEnterTimestampMonotonic)
before=$(cat /proc/loadavg)
started=$(date +%s%N)
stop_owned() {
    local owned
    for owned in "$@"; do
        [[ "$(systemctl show "$owned" -p LoadState --value 2>/dev/null)" != not-found ]] || continue
        if ! timeout --kill-after=2s 20s sudo -n systemctl stop "$owned" 2>"$work/stop.err"; then
            [[ "$(systemctl show "$owned" -p LoadState --value 2>/dev/null)" == not-found ]] || {
                cat "$work/stop.err" >&2; return 1;
            }
        fi
    done
}
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((wake_active)); then
        stop_owned "$wake_unit.path" "$wake_unit.service" || code=1
    fi
    if ((active)); then timeout --kill-after=2s 20s sudo -n systemctl stop "$unit.service" || code=1; fi
    for owned in "$unit.service" "$wake_unit.service"; do
        if [[ "$(systemctl show "$owned" -p LoadState --value 2>/dev/null)" != not-found ]]; then
            timeout --kill-after=2s 5s sudo -n systemctl kill --signal=KILL "$owned" 2>/dev/null || true
        fi
    done
    if ((code)); then
        for log in "$work/build.err" "$work/journal" "$work/wake.journal"; do [[ ! -f "$log" ]] || cat "$log" >&2; done
    fi
    if [[ "$(systemctl show desk -p MainPID -p ActiveEnterTimestampMonotonic)" != "$live_before" ]]; then
        printf 'live desk changed during isolated proof\n' >&2
        code=1
    fi
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir "$work/native" "$work/board" "$work/handoff"
"$compiler" build examples/desk --library-dir library --no-store -o "$work/server" 2>"$work/build.err"
properties=()
unit_properties() {
    local file=$1 section= line key value
    properties=()
    while IFS= read -r line; do
        [[ -n "$line" && "$line" != \#* ]] || continue
        if [[ "$line" == \[*\] ]]; then section=$line; continue; fi
        [[ "$section" == '[Service]' && "$line" == *=* ]] || continue
        key=${line%%=*}
        value=${line#*=}
        [[ "$key" != ExecStart ]] || continue
        value=${value//\/home\/coder\/.local\/state\/desk/$work/native}
        value=${value//\/home\/coder\/.local\/state\/cxloop/$work/board}
        value=${value//\/home\/coder\/.claude\/projects\/-home-coder-workspace\/memory/$work/handoff}
        if [[ "$key" == Environment && "$value" == DESK_RELAY=* ]]; then value=DESK_RELAY=$work/relay; fi
        properties+=("--property=$key=$value")
    done <"$file"
}
capture() { sudo -n journalctl -q "_SYSTEMD_INVOCATION_ID=$invocation" -o cat --no-pager >"$work/journal"; }
start_service() {
    unit_properties examples/desk.service
    sudo -n systemd-run --quiet --collect --unit="$unit" "${properties[@]}" "$work/server"
    active=1
    invocation=
    for ((attempt=0;attempt<600;attempt++)); do
        invocation=$(systemctl show "$unit.service" -p InvocationID --value)
        [[ -z "$invocation" ]] && { sleep 0.05; continue; }
        capture
        port=$(awk '$1=="port" {port=$2} END {print port}' "$work/journal")
        [[ -z "$port" ]] || break
        [[ "$(systemctl show "$unit.service" -p NRestarts --value)" == 0 ]] || { cat "$work/journal"; return 1; }
        sleep 0.05
    done
    [[ -n "$port" ]]
    main_pid=$(systemctl show "$unit.service" -p MainPID --value)
    [[ "$main_pid" =~ ^[1-9][0-9]*$ ]]
    [[ "$(sudo -n readlink "/proc/$main_pid/fd/1")" == socket:* ]]
    [[ "$(systemctl show "$unit.service" -p ProtectSystem --value)" == strict ]]
    [[ "$(systemctl show "$unit.service" -p ProtectHome --value)" == read-only ]]
    [[ -z "$(systemctl show "$unit.service" -p CapabilityBoundingSet --value)" ]]
}
stop_service() {
    timeout --kill-after=2s 20s sudo -n systemctl stop "$unit.service"
    active=0
    capture
    grep -Fxq stopped "$work/journal"
    ! kill -0 "$main_pid" 2>/dev/null
    for ((attempt=0;attempt<100;attempt++)); do
        [[ "$(systemctl show "$unit.service" -p LoadState --value 2>/dev/null)" == not-found ]] && return 0
        sleep 0.05
    done
    return 1
}
start_service
fixed_port=$port
bash tools/http-desk-live-parity.sh "$port" "$work"
printf 'Desk effective unit properties (host-wide drop-ins are included):\n'
systemctl show "$unit.service" -p NoNewPrivileges -p ProtectSystem -p ProtectHome -p ReadWritePaths \
    -p PrivateTmp -p PrivateDevices -p ProtectKernelTunables -p ProtectKernelModules -p ProtectControlGroups \
    -p RestrictNamespaces -p RestrictSUIDSGID -p LockPersonality -p MemoryDenyWriteExecute \
    -p CapabilityBoundingSet -p AmbientCapabilities -p RestrictAddressFamilies -p MemoryCurrent -p MemoryPeak
stop_service
cp -a examples/desk "$work/program"
sed -i "s/127.0.0.1:0/127.0.0.1:$fixed_port/" "$work/program/program.lkj"
"$compiler" build "$work/program" --library-dir library --no-store -o "$work/rebound" 2>"$work/build.err"
mv "$work/rebound" "$work/server"
port=
start_service
[[ "$port" == "$fixed_port" ]]
bash tools/http-desk-live-parity.sh "$port" "$work"
stop_service
# Repeated invocation after a simulated crash applies one idempotent work item.
cat >"$work/relay" <<'RELAY'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == relay && -f "$2" && "$3" == --force ]]
printf '%s\n' "$CXLOOP_WAKE_ID" >>"$DESK_STATE/relay-invocations"
if [[ ! -e "$DESK_STATE/relay-applied" ]]; then
    printf '%s\n' "$CXLOOP_WAKE_ID" >"$DESK_STATE/relay-applied"
    exit 75
fi
grep -Fxq "$CXLOOP_WAKE_ID" "$DESK_STATE/relay-applied"
RELAY
chmod +x "$work/relay"
printf '0\n' >"$work/native/seen"
printf 'desk-wake 1 1 00000000000000000000000000000001\n' >"$work/native/wake"
printf 'stand-in handoff\n' >"$work/handoff/handoff-unit.md"
unit_properties examples/desk-wake.service
sudo -n systemd-run --quiet --collect --unit="$wake_unit" \
    --path-property="PathExists=$work/native/wake" "${properties[@]}" "$PWD/examples/desk-wake"
wake_active=1
for ((attempt=0;attempt<600;attempt++)); do
    [[ -e "$work/native/wake" ]] || break
    sleep 0.05
done
sudo -n journalctl -q -u "$wake_unit.service" -o cat --no-pager >"$work/wake.journal"
test ! -e "$work/native/wake"
test "$(cat "$work/native/wake-ack")" = 1
test "$(wc -l <"$work/native/relay-applied")" = 1
test "$(wc -l <"$work/native/relay-invocations")" -ge 2
stop_owned "$wake_unit.path" "$wake_unit.service"
wake_active=0
elapsed=$(($(date +%s%N)-started))
printf 'ok desk exact-unit/live-GET parity, same-port restart, path-unit idempotent retry (%s ms, load %s -> %s)\n' \
    "$((elapsed/1000000))" "$before" "$(cat /proc/loadavg)"
