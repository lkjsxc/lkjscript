#!/bin/sh
# Run only a uniquely named candidate; never change the live origin or /opt.
set -eu
cd "$(dirname "$0")/.."
compiler=target/lkjscript
while [ "$#" -gt 0 ]; do
    case "$1" in
        --compiler) compiler=$2; shift 2 ;;
        *) printf 'usage: sh tools/http-site-systemd.sh [--compiler PATH]\n' >&2; exit 2 ;;
    esac
done
work=$(mktemp -d target/http-site-systemd.XXXXXX)
unit="lkjscript-programs-$(basename "$work" | tr . -)"
stage="/run/$unit"
stage_owned=0
active=0
runner=
started=
main_pid=
port=
live_pid=
cleanup_failed() {
    printf '%s\n' "$1" >&2
    result=1
    verified=no
}
candidate_collected() {
    # --collect can remove our failed startup unit before cleanup begins.
    # Absence is already clean; an inspection failure still fails verification
    # and leaves the bounded stop/kill fallback for this unique owned name.
    candidate_status=0
    candidate_state=$(timeout --kill-after=1s 2s systemctl show "$unit.service" -p LoadState --value) || candidate_status=$?
    if [ "$candidate_state" = not-found ] && [ "$candidate_status" -le 1 ]; then
        unit_state=collected
        active=0
        return 0
    fi
    if [ "$candidate_status" -ne 0 ]; then
        cleanup_failed "candidate unit cannot be inspected before cleanup: $unit"
    fi
    return 1
}
launcher_finished() {
    if ! kill -0 "$runner" 2>/dev/null; then return 0; fi
    # An exited child may remain a zombie until the shell's explicit wait.
    awk '$1 == "State:" && $2 == "Z" {done=1} END {exit !done}' "/proc/$runner/status" 2>/dev/null
}
reap_launcher() {
    # Never wait on a live launcher without a bound, even if systemctl failed.
    for launcher_signal in none TERM KILL; do
        if [ "$launcher_signal" != none ]; then kill -"$launcher_signal" "$runner" 2>/dev/null || true; fi
        launcher_attempts=0
        while ! launcher_finished; do
            launcher_attempts=$((launcher_attempts + 1))
            [ "$launcher_attempts" -lt 50 ] || break
            sleep 0.1
        done
        if launcher_finished; then
            launcher_status=0
            wait "$runner" || launcher_status=$?
            runner=
            return "$launcher_status"
        fi
    done
    return 124
}
cleanup() {
    result=$?
    verified=yes
    unit_state=not-started
    trap - EXIT HUP INT TERM
    set +e
    if [ "$active" = 1 ] && ! candidate_collected; then
        if timeout --kill-after=2s 15s sudo -n systemctl stop "$unit.service"; then
            active=0
        elif candidate_collected; then
            # It may also disappear between the inspection and stop call.
            :
        else
            # The latest inspection still found the unit (or failed), so retain
            # the stop failure and attempt bounded termination of our unit only.
            cleanup_failed "candidate stop failed or timed out: $unit"
            if ! timeout --kill-after=2s 5s sudo -n systemctl kill --signal=KILL "$unit.service"; then
                candidate_collected || cleanup_failed "candidate kill failed or timed out: $unit"
            fi
        fi
    fi
    if [ -n "$runner" ]; then
        reap_launcher || cleanup_failed "candidate launcher did not exit successfully within its reap bound: $started"
    fi
    if [ "$result" -ne 0 ]; then
        for log in "$work/run.out" "$work/run.err" "$work/parity.out"; do
            if [ -f "$log" ]; then cat "$log"; fi
        done
    fi
    if [ -n "$started" ] && kill -0 "$started" 2>/dev/null; then
        cleanup_failed "candidate launcher remains: $started"
    fi
    if [ -n "$main_pid" ] && kill -0 "$main_pid" 2>/dev/null; then
        cleanup_failed "candidate process remains: $main_pid"
    fi
    if [ -n "$port" ]; then
        case "$port" in
            *[!0-9]*) cleanup_failed "candidate listener cannot be identified: $port" ;;
            *)
                hex=$(printf '%04X' "$port")
                if ! awk -v wanted="$hex" 'NR > 1 {split($2, address, ":"); if(address[2] == wanted && $4 == "0A") found=1} END {exit found ? 1 : 0}' /proc/net/tcp; then
                    cleanup_failed "candidate listener remains or cannot be inspected: $port"
                fi
                ;;
        esac
    fi
    if [ -n "$started" ]; then
        attempts=0
        while :; do
            candidate_status=0
            candidate_state=$(timeout --kill-after=1s 2s systemctl show "$unit.service" -p LoadState --value) || candidate_status=$?
            if [ "$candidate_state" = not-found ] && [ "$candidate_status" -le 1 ]; then
                unit_state=collected
                active=0
                break
            fi
            if [ "$candidate_status" -ne 0 ]; then
                cleanup_failed "candidate unit cannot be inspected: $unit"
                break
            fi
            attempts=$((attempts + 1))
            if [ "$attempts" -ge 10 ]; then cleanup_failed "candidate unit was not collected: $unit"; break; fi
            sleep 0.1
        done
    fi
    if [ "$stage_owned" = 1 ]; then
        if ! timeout --kill-after=2s 10s sudo -n rm -rf -- "$stage" || [ -e "$stage" ]; then
            cleanup_failed "candidate stage was not removed: $stage"
        fi
    fi
    if [ -z "$live_pid" ]; then
        cleanup_failed "initial live origin PID was unavailable; preservation cannot be verified"
    elif ! current_live=$(timeout --kill-after=1s 5s systemctl show lkjscript-site.service -p MainPID --value); then
        cleanup_failed "live origin PID cannot be rechecked"
    elif [ "$current_live" != "$live_pid" ]; then
        cleanup_failed "live origin PID changed externally: $live_pid -> $current_live"
    fi
    if ! timeout --kill-after=2s 10s rm -rf -- "$work" || [ -e "$work" ]; then
        cleanup_failed "private work directory was not removed: $work"
    fi
    shown_launcher=$started
    shown_candidate=$main_pid
    shown_port=$port
    [ -n "$shown_launcher" ] || shown_launcher=none
    [ -n "$shown_candidate" ] || shown_candidate=none
    [ -n "$shown_port" ] || shown_port=none
    if [ "$verified" = yes ]; then
        printf 'Site systemd cleanup complete: launcher %s and candidate %s gone; port %s released; unit %s %s; owned stage and private work directory removed; live PID %s unchanged\n' "$shown_launcher" "$shown_candidate" "$shown_port" "$unit" "$unit_state" "$live_pid"
    else
        printf 'Site systemd cleanup incomplete: launcher %s; candidate %s; port %s; unit %s; stage %s; work %s; see verification errors above\n' "$shown_launcher" "$shown_candidate" "$shown_port" "$unit" "$stage" "$work" >&2
    fi
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
live_pid=$(timeout --kill-after=1s 5s systemctl show lkjscript-site.service -p MainPID --value)
test "$live_pid" -gt 0
test "$(systemctl show "$unit.service" -p LoadState --value)" = not-found
sudo -n test ! -e "$stage"
sudo -n install -d -m 0755 "$stage"
stage_owned=1
printf 'Candidate build load before: %s\n' "$(cut -d ' ' -f 1-3 /proc/loadavg)"
"$compiler" build examples/site.lkj --library-dir library --stats --grant output \
    --grant listener=127.0.0.1:0 \
    --grant clock --grant ready --grant stop -o "$work/server"
printf 'Candidate build load after: %s\n' "$(cut -d ' ' -f 1-3 /proc/loadavg)"
sudo -n install -m 0755 "$work/server" "$stage/lkjscript-site"
before=$(cut -d ' ' -f 1-3 /proc/loadavg)
begin=$(date +%s%N)
invocation=
capture_journal() {
    if [ -n "$invocation" ]; then
        sudo -n journalctl -b "_SYSTEMD_INVOCATION_ID=$invocation" -o cat --no-pager >"$work/run.out"
    fi
}
settings() {
    systemctl show "$1.service" -p DynamicUser -p WorkingDirectory -p Restart -p RestartUSec \
        -p StandardInput -p StandardOutput -p StandardError -p UMask -p TimeoutStopUSec \
        -p ProtectSystem -p ProtectHome -p NoNewPrivileges -p MemoryDenyWriteExecute \
        -p MemoryMax -p TasksMax -p LimitNOFILE -p PrivateTmp -p PrivateDevices \
        -p ProtectKernelTunables -p ProtectKernelModules -p ProtectControlGroups \
        -p RestrictSUIDSGID -p RestrictNamespaces -p LockPersonality \
        -p CapabilityBoundingSet -p AmbientCapabilities -p RestrictAddressFamilies \
        -p ProcSubset -p ProtectProc -p PrivateNetwork
}
start_candidate() {
    invocation=
    port=
    : >"$work/run.out"
    sudo -n systemd-run --collect --wait --unit="$unit" --service-type=simple \
        --property='Description=lkjscript documentation origin' --property=After=network.target \
        --property=DynamicUser=yes --property=WorkingDirectory=/ \
        --property=Restart=on-failure --property=RestartSec=3 \
        --property=StandardInput=null --property=StandardOutput=journal --property=StandardError=inherit \
        --property=TimeoutStopSec=10 --property=UMask=0077 --property=NoNewPrivileges=yes \
        --property=PrivateTmp=yes --property=PrivateDevices=yes --property=ProtectSystem=strict \
        --property=ProtectHome=yes --property=ProtectKernelTunables=yes --property=ProtectKernelModules=yes \
        --property=ProtectControlGroups=yes --property=RestrictSUIDSGID=yes --property=RestrictNamespaces=yes \
        --property=LockPersonality=yes --property=MemoryDenyWriteExecute=yes \
        --property=CapabilityBoundingSet= --property=AmbientCapabilities= \
        --property='RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX' \
        --property=MemoryMax=128M --property=TasksMax=32 --property=LimitNOFILE=512 \
        "$stage/lkjscript-site" >"$work/launcher.out" 2>"$work/run.err" &
    runner=$!
    started=$runner
    active=1
    attempts=0
    while [ "$attempts" -lt 600 ]; do
        invocation=$(systemctl show "$unit.service" -p InvocationID --value)
        capture_journal
        port=$(awk '$1 == "port" {print $2}' "$work/run.out")
        if [ -n "$port" ]; then break; fi
        if ! kill -0 "$runner" 2>/dev/null; then wait "$runner" || true; runner=; return 1; fi
        restarts=$(systemctl show "$unit.service" -p NRestarts --value)
        if [ -n "$restarts" ] && [ "$restarts" -gt 0 ]; then
            printf 'Candidate failed its first startup; a restart cannot hide it\n' >&2
            return 1
        fi
        attempts=$((attempts + 1))
        sleep 0.1
    done
    test -n "$port"
    main_pid=$(systemctl show "$unit.service" -p MainPID --value)
    test "$main_pid" -gt 0
    test "$(awk '$1 == "stop-target" {print $2}' "$work/run.out")" = "$main_pid"
    case "$(sudo -n readlink "/proc/$main_pid/fd/1")" in
        socket:*) : ;;
        *) printf 'Candidate stdout is not the journal stream socket\n' >&2; return 1 ;;
    esac
    settings "$unit" >"$work/candidate.settings"
    diff -u "$work/live.settings" "$work/candidate.settings"
}
stop_candidate() {
    timeout --kill-after=2s 15s sudo -n systemctl stop "$unit.service"
    active=0
    reap_launcher
    capture_journal
    grep -Fx stopped "$work/run.out"
    ! kill -0 "$main_pid" 2>/dev/null
    attempts=0
    until candidate_collected; do
        attempts=$((attempts + 1))
        [ "$attempts" -lt 100 ] || return 1
        sleep 0.05
    done
}
settings lkjscript-site >"$work/live.settings"
: >"$work/run.out"
start_candidate
fixed_port=$port
printf 'Candidate effective settings match the live unit; stdout is a journal stream socket\n'
cat "$work/candidate.settings"
"$compiler" build tools/http-site-parity.lkj --library-dir library --grant output \
    --grant origin=127.0.0.1:8798 --grant "candidate=127.0.0.1:$port" \
    --grant root=examples/site-content -o "$work/parity"
"$compiler" build examples/site.lkj --library-dir library --grant output \
    --grant "listener=127.0.0.1:$port" --grant clock --grant ready --grant stop -o "$work/rebound"
for round in first restarted; do
    timeout --kill-after=5s 120s "$work/parity" >"$work/parity.out"
    tail -1 "$work/parity.out"
    printf 'Candidate memory after %s parity:\n' "$round"
    systemctl show "$unit.service" -p MemoryCurrent -p MemoryPeak
    awk '/^VmRSS:|^VmHWM:/ {print}' "/proc/$main_pid/status"
    stop_candidate
    if [ "$round" = first ]; then
        hex=$(printf '%04X' "$fixed_port")
        awk -v wanted="$hex" '$4 == "06" {split($2, address, ":"); if(address[2] == wanted) found++}
            END {printf "Closed candidate TIME_WAIT sockets before restart: %d\n", found; exit found ? 0 : 1}' /proc/net/tcp
        sudo -n install -m 0755 "$work/rebound" "$stage/lkjscript-site"
        start_candidate
        test "$port" = "$fixed_port"
    fi
done
elapsed=$(($(date +%s%N) - begin))
after=$(cut -d ' ' -f 1-3 /proc/loadavg)
awk -v elapsed="$elapsed" -v before="$before" -v after="$after" 'BEGIN {printf "ok site systemd parity and same-port restart (%.3f s, load %s -> %s)\n", elapsed / 1000000000, before, after}'
