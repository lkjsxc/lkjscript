#!/usr/bin/env bash
set -euo pipefail
# Prospective only: root must release this exact reviewed guard before invocation.
[[ $# -eq 1 && $1 == --remove ]] || { printf '%s\n' 'usage: bash launch.sh --remove' >&2; exit 64; }
repo=/home/coder/workspace/lkjscript
prep=$repo/target/compiler-scalar-cohort-cleanup-preparation-1
out=$repo/target/compiler-scalar-cohort-cleanup-execution-1
umask 077
cd "$repo"
mkdir -- "$out" # A refusal/partial execution is retained; there is no implicit retry.
printf '%s\n' "$repo" > "$out/cwd"
printf '%s\n' '4 GiB AS; 900 s TERM watchdog; 10 s KILL grace; one cleanup worker; no compiler/native/compression' > "$out/limits"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/start.utc"
date +%s%N > "$out/start.ns"
cat /proc/loadavg > "$out/load-before"
cat > "$out/argv.json" <<'JSON'
{
  "argv": ["/usr/bin/time","-o","/home/coder/workspace/lkjscript/target/compiler-scalar-cohort-cleanup-execution-1/time","-f","wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x","/usr/bin/timeout","--signal=TERM","--kill-after=10s","900s","/usr/bin/prlimit","--as=4294967296","--","perl","/home/coder/workspace/lkjscript/target/compiler-scalar-cohort-cleanup-preparation-1/guard.pl","--remove"],
  "cwd": "/home/coder/workspace/lkjscript",
  "guard_sha256": "486fcd8db56a2ccd1d36117df5621cb97fa5656320cfe15511ce509632f4bf27",
  "snapshot_sha256": "529395e9b74464531137d27bc05b72df3d368bbe0c77edfbea97f68149efe132",
  "recovery_commit": "0b0b27803d27b932a5b91cde2d766bd3c8bb82b9",
  "contract": "cooperative closure; non-atomic removal; preserve partial progress and stop on any refusal"
}
JSON
run_guard() {
    local actual guard_name
    sha256sum -- "$prep/guard.pl" > "$out/guard.sha256" || return 1
    IFS=' ' read -r actual guard_name < "$out/guard.sha256" || return 1
    [[ $actual == 486fcd8db56a2ccd1d36117df5621cb97fa5656320cfe15511ce509632f4bf27 ]] || {
        printf '%s\n' 'refusal: reviewed guard changed' >&2
        return 1
    }
    /usr/bin/time -o "$out/time" \
        -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' \
        /usr/bin/timeout --signal=TERM --kill-after=10s 900s \
        /usr/bin/prlimit --as=4294967296 -- perl "$prep/guard.pl" --remove
}
set +e
run_guard > "$out/stdout" 2> "$out/stderr"
status=$?
set -e
printf '%s\n' "$status" > "$out/status"
date +%s%N > "$out/end.ns"
date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$out/end.utc"
cat /proc/loadavg > "$out/load-after"
printf 'cleanup status=%s capture=%s\n' "$status" "$out"
exit "$status"
