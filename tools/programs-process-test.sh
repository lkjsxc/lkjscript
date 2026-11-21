#!/usr/bin/env bash
# Default gate: authenticated processes, ordered/actor protocols, sealed regions,
# deadlines, transfer/lifetime mutations. --bench is the explicit four-CPU run.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "${LKJSCRIPT_PROGRAMS_TEST_REAPER:-0}" != 1 ]]; then
    exec perl -e '
        use strict;use warnings;
        syscall(157,36,1,0,0,0)==0 or die "subreaper: $!";
        $ENV{LKJSCRIPT_PROGRAMS_TEST_REAPER}=1;
        my $child=fork();defined($child) or die "fork: $!";
        if(!$child){exec "bash",@ARGV;die "exec: $!";}
        $SIG{TERM}=sub{kill "TERM",$child;};$SIG{INT}=sub{kill "INT",$child;};$SIG{HUP}=sub{kill "HUP",$child;};
        my $result;do {$result=waitpid($child,0);}while($result<0 && $!{EINTR});
        my $status=$?;
        while(waitpid(-1,1)>0){}
        open my $kids,"<","/proc/$$/task/$$/children" or die $!;
        my $line=<$kids>//"";close $kids;
        my @left=grep {length} split " ",$line;
        if(@left){kill "KILL",@left;while(waitpid(-1,0)>0){};warn "test leaked owned children, killed/reaped\n";$status=256;}
        exit(($status&127)?128+($status&127):$status>>8);
    ' "$0" "$@"
fi
compiler=target/lkjscript
part=contracts
while (($#)); do
    case "$1" in
        --compiler) compiler=$2;shift 2;;
        --bench) part=bench;shift;;
        *) printf 'unknown option: %s\n' "$1" >&2;exit 2;;
    esac
done
work=$(mktemp -d target/programs-process.XXXXXX)
server_pid=
before=$(cut -d' ' -f1 /proc/loadavg)
started=$(date +%s%N)
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$server_pid" ]]; then kill "$server_pid" 2>/dev/null || true;wait "$server_pid" 2>/dev/null || true;fi
    if ((code)); then
        for log in "$work/error" "$work/server.err" "$work/output";do test ! -f "$log" || tail -30 "$log" >&2;done
    fi
    local after elapsed
    after=$(cut -d' ' -f1 /proc/loadavg);elapsed=$(($(date +%s%N)-started))
    awk -v ns="$elapsed" -v before="$before" -v after="$after" -v code="$code" 'BEGIN {printf "process gate %.3f s load %s -> %s status=%s\n",ns/1000000000,before,after,code}'
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM
umask 077
mkdir "$work/private" "$work/state"
head -c 32 /dev/urandom > "$work/private/key"
printf 'file payload' > "$work/state/payload"
build_child() {
    local name=$1
    "$compiler" build-child "tests/fixtures/process/$name.lkj" --library-dir library --no-store --image-key "$work/private/key" -o "$work/$name" > "$work/$name.authority" 2>"$work/error"
}
run_case() {
    local fixture=$1 image=$2 expected=$3
    shift 3
    local engine grant
    local args=("tests/fixtures/process/$fixture.lkj" --library-dir library --no-store)
    if [[ "$image" != none ]]; then args+=(--image-key "$work/private/key" --grant "image=$work/$image");fi
    for grant in "$@";do args+=(--grant "$grant");done
    for engine in native interpreter;do
        timeout --kill-after=3s 60s "$compiler" run "${args[@]}" --engine "$engine" > "$work/output" 2>"$work/error"
        grep -E -qx "$expected" "$work/output"
    done
    "$compiler" build "${args[@]}" -o "$work/$fixture-parent" > "$work/build.out" 2>"$work/error"
    timeout --kill-after=3s 30s "$work/$fixture-parent" > "$work/output" 2>"$work/error"
    printf 'ok process %s native/interpreter/standalone\n' "$fixture"
}
measure() {
    local name=$1
    shift
    local first last
    first=$(cut -d' ' -f1 /proc/loadavg)
    "$@"
    last=$(cut -d' ' -f1 /proc/loadavg)
    printf '%s load %s -> %s\n' "$name" "$first" "$last"
}
if [[ "$part" == bench ]]; then
    build_child pool_worker
    build_child payload_worker
    for fixture in pool payload;do
        "$compiler" build "tests/fixtures/process/${fixture}_bench.lkj" --library-dir library --no-store --image-key "$work/private/key" --grant "image=$work/${fixture}_worker" --grant clock --grant output -o "$work/$fixture-bench" > "$work/build.out" 2>"$work/error"
    done
    for run in 1 2 3;do measure "pool pair $run" timeout --kill-after=3s 90s "$work/pool-bench";done
    measure '1MiB logical payload, page sample, full-byte control first' timeout --kill-after=3s 30s "$work/payload-bench"
    exit 0
fi
build_child worker
build_child pool_worker
run_case parent worker 42 clock "root=$work/state"
run_case region worker 42 clock "root=$work/state"
run_case region_unsealed_errno none 22
run_case pool pool_worker 42 clock
run_case lifecycle pool_worker 42 clock
"$compiler" authority tests/fixtures/process/parent.lkj --library-dir library --no-store --image-key "$work/private/key" --grant "image=$work/worker" --grant clock --grant "root=$work/state" > "$work/parent.authority" 2>"$work/error"
grep -E -q 'Channel\(slot0,owned\)' "$work/parent.authority"
head -c 32 /dev/urandom > "$work/private/wrong"
if "$compiler" build tests/fixtures/process/parent.lkj --library-dir library --no-store --image-key "$work/private/wrong" --grant "image=$work/worker" --grant clock --grant "root=$work/state" -o "$work/refused" > "$work/output" 2>"$work/error";then exit 1;fi
build_child unsupported_worker
if "$compiler" build tests/fixtures/process/parent.lkj --library-dir library --no-store --image-key "$work/private/key" --grant "image=$work/unsupported_worker" --grant clock --grant "root=$work/state" -o "$work/refused" > "$work/output" 2>"$work/error";then exit 1;fi
cp "$work/worker" "$work/altered"
cp "$work/worker.admission" "$work/altered.admission"
printf X >> "$work/altered"
if "$compiler" build tests/fixtures/process/parent.lkj --library-dir library --no-store --image-key "$work/private/key" --grant "image=$work/altered" --grant clock --grant "root=$work/state" -o "$work/refused" > "$work/output" 2>"$work/error";then exit 1;fi
printf 'ok image wrong key, changed bytes, unsupported File slot refused\n'
build_child parser_worker
run_case supervision_bound parser_worker 42 clock "root=$work/state"
"$compiler" build tests/fixtures/process/supervisor.lkj --library-dir library --no-store --image-key "$work/private/key" --grant "image=$work/parser_worker" --grant listener=127.0.0.1:0 --grant "root=$work/state" --grant clock -o "$work/supervisor" > "$work/build.out" 2>"$work/error"
timeout --kill-after=3s 30s "$work/supervisor" > "$work/server.out" 2>"$work/server.err" &
server_pid=$!
for _ in {1..200};do test ! -s "$work/state/port" || break;kill -0 "$server_pid";sleep 0.01;done
port=$(cat "$work/state/port")
# Observe the socket with Bash, independently of the compiled worker and supervisor.
for path in ok panic ok ok spin ok bad ok;do
    if ! timeout --kill-after=1s 2s bash -euc '
        exec 3<>"/dev/tcp/127.0.0.1/$1"
        printf "GET /%s HTTP/1.1\r\nHost: 127.0.0.1:%s\r\nAccept: */*\r\n\r\n" "$2" "$1" >&3
        exec cat <&3
    ' process-http "$port" "$path" > "$work/response"; then
        printf 'supervisor request /%s failed before a complete response\n' "$path" >&2
        exit 1
    fi
    case "$path" in
        ok) expected=$'HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n42';;
        bad) expected=$'HTTP/1.1 400 Bad Request\r\nContent-Length: 3\r\nConnection: close\r\n\r\nbad';;
        panic|spin) expected=;;
    esac
    if ! printf '%s' "$expected" | cmp - "$work/response"; then
        printf 'unexpected supervisor response to /%s\n' "$path" >&2
        exit 1
    fi
done
wait "$server_pid";server_pid=
grep -E -qx 'accepted=5 rejected=1 failed=2 restarts=2' "$work/state/supervisor-result"
test -s "$work/state/failure-1"
printf 'ok isolated parser supervisor: eight requests, trap/deadline restart, parent root/listener retained\n'
bash tools/programs-process-mutate.sh --compiler "$compiler"
printf 'ok process transfer and isolation contracts\n'
