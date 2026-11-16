#!/usr/bin/env bash
set -u
set -o pipefail
export LC_ALL=C
cd /home/coder/workspace/lkjscript || exit 1
audit_root=/home/coder/workspace/lkjscript/target/compiler-reference-capture-setup-acceptance-1
run_phase() {
  phase_name=$1
  shift
  phase_root="$audit_root/captures/$phase_name"
  mkdir -m 700 "$phase_root" || return 1
  printf '%s\n' "$PWD" > "$phase_root/cwd"
  printf '%s\0' /usr/bin/time -o "$phase_root/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --signal=TERM --kill-after=10s 300s /usr/bin/prlimit --as=4294967296 -- "$@" > "$phase_root/argv"
  printf '%s\n' 'LC_ALL=C; offline saved-setup-failure/artifact/Git audit only; 4 GiB AS; 300 s TERM; 10 s grace; no compiler/native execution, key/store access, setup rerun or decompression/compression' > "$phase_root/environment"
  sha256sum "$audit_root/audit.pl" > "$phase_root/helper-before.sha256"
  date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$phase_root/start.utc"
  date +%s%N > "$phase_root/start.ns"
  read -r load_one load_five load_fifteen rest < /proc/loadavg
  printf '%s %s %s\n' "$load_one" "$load_five" "$load_fifteen" > "$phase_root/load-before"
  /usr/bin/time -o "$phase_root/time" -f 'wall_s=%e user_s=%U system_s=%S maxrss_kib=%M exit_status=%x' /usr/bin/timeout --signal=TERM --kill-after=10s 300s /usr/bin/prlimit --as=4294967296 -- "$@" > "$phase_root/stdout" 2> "$phase_root/stderr"
  phase_status=$?
  printf '%s\n' "$phase_status" > "$phase_root/status"
  read -r load_one load_five load_fifteen rest < /proc/loadavg
  printf '%s %s %s\n' "$load_one" "$load_five" "$load_fifteen" > "$phase_root/load-after"
  date +%s%N > "$phase_root/end.ns"
  date -u +%Y-%m-%dT%H:%M:%S.%NZ > "$phase_root/end.utc"
  sha256sum "$audit_root/audit.pl" > "$phase_root/helper-after.sha256"
  cmp "$phase_root/helper-before.sha256" "$phase_root/helper-after.sha256" || return 1
  printf '%s status=%s\n' "$phase_name" "$phase_status"
  cat "$phase_root/time" "$phase_root/stdout" "$phase_root/stderr"
  return "$phase_status"
}
run_phase syntax-1 /usr/bin/perl -c "$audit_root/audit.pl" || exit $?
run_phase audit-1 /usr/bin/perl "$audit_root/audit.pl"
