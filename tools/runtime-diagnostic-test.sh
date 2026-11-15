#!/usr/bin/env bash
# Isolated journal proof. Only this script's transient units and temporary tree
# are created/stopped; the production documentation origin is never involved.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(readlink -f "${1:-target/lkjscript}")
work=$(mktemp -d "$PWD/target/runtime-diagnostics.XXXXXX")
unit_prefix="compiler-diagnostics-$$"
units=()
cleanup() {
    local unit
    for unit in "${units[@]}"; do sudo -n systemctl stop "$unit" >/dev/null 2>&1 || true; done
    rm -rf "$work"
}
trap cleanup EXIT HUP INT TERM
journal() { sudo -n journalctl --no-pager --quiet -o cat "_SYSTEMD_UNIT=$1" _TRANSPORT=stdout; }
run_unit() {
    local name=$1 expected=$2 status=0 unit
    shift 2
    unit="$unit_prefix-$name.service"
    units+=("$unit")
    sudo -n systemd-run --quiet --wait --collect --unit "$unit" \
        -p Type=exec -p "User=$(id -u)" -p "WorkingDirectory=$PWD" \
        -p StandardOutput=journal -p StandardError=journal -p RuntimeMaxSec=15s \
        "$@" >"$work/launch.out" 2>"$work/launch.err" || status=$?
    [[ $status == 1 ]] || { cat "$work/launch.err" >&2; printf 'unexpected unit status %s: %s\n' "$status" "$name" >&2; return 1; }
    journal "$unit" > "$work/journal"
    if [[ -z $expected ]]; then [[ ! -s "$work/journal" ]];
    else
        [[ $(wc -l < "$work/journal") == 1 ]] || { cat "$work/journal" >&2; printf 'journal line count differs: %s\n' "$name" >&2; return 1; }
        grep -Fxq -- "$expected" "$work/journal" || { cat "$work/journal" >&2; printf 'expected %s\n' "$expected" >&2; return 1; }
        [[ $(wc -c < "$work/journal") -le 2048 ]]
    fi
}
cat > "$work/directory.lkj" <<'SOURCE'
pub fn main(_root: Directory, _errors: Stderr) {}
SOURCE
cat > "$work/read-directory.lkj" <<'SOURCE'
pub fn main(_root: ReadDirectory, _errors: Stderr) {}
SOURCE
cat > "$work/foreign-acquire.lkj" <<'SOURCE'
pub fn main(_tool: ForeignCommand, _errors: Stderr) {}
SOURCE
cat > "$work/null-device.lkj" <<'SOURCE'
pub fn main(_sink: NullDevice, _errors: Stderr) {}
SOURCE
printf 'invalid executable\n' > "$work/invalid-image"
printf 'ordinary file, not a null device\n' > "$work/fake-null"
cat > "$work/listener.lkj" <<'SOURCE'
pub fn main(_incoming: Listener, _errors: Stderr) {}
SOURCE
cat > "$work/readiness.lkj" <<'SOURCE'
pub fn main(_ready: Readiness, _errors: Stderr) {}
SOURCE
cat > "$work/stop.lkj" <<'SOURCE'
pub fn main(_stop: Stop, _errors: Stderr) {}
SOURCE
cat > "$work/stdout.lkj" <<'SOURCE'
pub fn main(mut output: Stdout, _errors: Stderr) {
    output.write(b"unwritten");
}
SOURCE
cat > "$work/stderr.lkj" <<'SOURCE'
pub fn main(mut errors: Stderr, _output: Stdout) {
    errors.write(b"unwritten");
}
SOURCE
cat > "$work/silent.lkj" <<'SOURCE'
pub fn main() { panic!("no output grant"); }
SOURCE
cat > "$work/holder.lkj" <<'SOURCE'
pub fn main(mut incoming: Listener, mut output: Stdout) {
    output.write(format!("PORT {}\n", incoming.port().unwrap()).as_bytes());
    let _connection = incoming.accept();
}
SOURCE
"$compiler" build "$work/directory.lkj" --grant "_root=$work/missing" --grant _errors --no-store -o "$work/directory"
"$compiler" build "$work/directory.lkj" --grant "_root=env:LKJSCRIPT_DIAGNOSTIC_ABSENT_$$" --grant _errors --no-store -o "$work/environment"
"$compiler" build "$work/read-directory.lkj" --grant "_root=$work/missing" --grant _errors --no-store -o "$work/read-directory"
"$compiler" build "$work/read-directory.lkj" --grant "_root=env:LKJSCRIPT_DIAGNOSTIC_ABSENT_$" --grant _errors --no-store -o "$work/read-environment"
"$compiler" build "$work/foreign-acquire.lkj" --grant "_tool=$work/missing-image" --grant _errors --no-store -o "$work/foreign-missing"
"$compiler" build "$work/foreign-acquire.lkj" --grant "_tool=$work/invalid-image" --grant _errors --no-store -o "$work/foreign-invalid"
"$compiler" build "$work/null-device.lkj" --grant _sink --grant _errors --no-store -o "$work/null-device"
"$compiler" build "$work/readiness.lkj" --grant _ready --grant _errors --no-store -o "$work/readiness"
"$compiler" build "$work/stop.lkj" --grant _stop --grant _errors --no-store -o "$work/stop"
"$compiler" build "$work/stdout.lkj" --grant output --grant _errors --no-store -o "$work/stdout"
"$compiler" build "$work/stderr.lkj" --grant errors --grant _output --no-store -o "$work/stderr"
"$compiler" build "$work/silent.lkj" --no-store -o "$work/silent"
"$compiler" build "$work/holder.lkj" --grant incoming=127.0.0.1:0 --grant output --no-store -o "$work/holder"
holder="$unit_prefix-holder.service"
units+=("$holder")
sudo -n systemd-run --quiet --collect --unit "$holder" -p Type=exec -p "User=$(id -u)" \
    -p StandardOutput=journal -p StandardError=journal -p RuntimeMaxSec=60s -- "$work/holder"
port=
for ((attempt=0; attempt<100; attempt++)); do
    port=$(journal "$holder" | sed -n 's/^PORT //p')
    [[ -n $port ]] && break
    sleep 0.05
done
[[ $port =~ ^[0-9]+$ ]]
"$compiler" build "$work/listener.lkj" --grant "_incoming=127.0.0.1:$port" --grant _errors --no-store -o "$work/listener"
for route in artifact native interpreter; do
    if [[ $route == artifact ]]; then
        directory=("$work/directory"); environment=("$work/environment"); listener=("$work/listener")
        read_directory=("$work/read-directory"); read_environment=("$work/read-environment")
        foreign_missing=("$work/foreign-missing"); foreign_invalid=("$work/foreign-invalid"); null_device=("$work/null-device")
        readiness=("$work/readiness"); stop=("$work/stop"); output=("$work/stdout"); errors=("$work/stderr")
    else
        directory=("$compiler" run "$work/directory.lkj" --engine "$route" --grant "_root=$work/missing" --grant _errors --no-store)
        environment=("$compiler" run "$work/directory.lkj" --engine "$route" --grant "_root=env:LKJSCRIPT_DIAGNOSTIC_ABSENT_$$" --grant _errors --no-store)
        listener=("$compiler" run "$work/listener.lkj" --engine "$route" --grant "_incoming=127.0.0.1:$port" --grant _errors --no-store)
        read_directory=("$compiler" run "$work/read-directory.lkj" --engine "$route" --grant "_root=$work/missing" --grant _errors --no-store)
        read_environment=("$compiler" run "$work/read-directory.lkj" --engine "$route" --grant "_root=env:LKJSCRIPT_DIAGNOSTIC_ABSENT_$" --grant _errors --no-store)
        foreign_missing=("$compiler" run "$work/foreign-acquire.lkj" --engine "$route" --grant "_tool=$work/missing-image" --grant _errors --no-store)
        foreign_invalid=("$compiler" run "$work/foreign-acquire.lkj" --engine "$route" --grant "_tool=$work/invalid-image" --grant _errors --no-store)
        null_device=("$compiler" run "$work/null-device.lkj" --engine "$route" --grant _sink --grant _errors --no-store)
        readiness=("$compiler" run "$work/readiness.lkj" --engine "$route" --grant _ready --grant _errors --no-store)
        stop=("$compiler" run "$work/stop.lkj" --engine "$route" --grant _stop --grant _errors --no-store)
        output=("$compiler" run "$work/stdout.lkj" --engine "$route" --grant output --grant _errors --no-store)
        errors=("$compiler" run "$work/stderr.lkj" --engine "$route" --grant errors --grant _output --no-store)
    fi
    run_unit "directory-$route" "$work/directory.lkj:1: R0007 Directory $work/missing openat: ENOENT (2)" -- "${directory[@]}"
    run_unit "environment-$route" "$work/directory.lkj:1: R0007 Directory env:LKJSCRIPT_DIAGNOSTIC_ABSENT_$$ lookup: ENOENT (2)" -- "${environment[@]}"
    run_unit "read-directory-$route" "$work/read-directory.lkj:1: R0007 ReadDirectory $work/missing openat: ENOENT (2)" -- "${read_directory[@]}"
    run_unit "read-environment-$route" "$work/read-directory.lkj:1: R0007 ReadDirectory env:LKJSCRIPT_DIAGNOSTIC_ABSENT_$ lookup: ENOENT (2)" -- "${read_environment[@]}"
    run_unit "foreign-missing-$route" "$work/foreign-acquire.lkj:1: R0007 ForeignCommand $work/missing-image openat: ENOENT (2)" -- "${foreign_missing[@]}"
    run_unit "foreign-invalid-$route" "$work/foreign-acquire.lkj:1: R0007 ForeignCommand $work/invalid-image validate-image: ENOEXEC (8)" -- "${foreign_invalid[@]}"
    run_unit "null-invalid-$route" "$work/null-device.lkj:1: R0007 NullDevice /dev/null validate-device: ENODEV (19)" \
        -p "BindReadOnlyPaths=$work/fake-null:/dev/null" -- "${null_device[@]}"
    run_unit "listener-$route" "$work/listener.lkj:1: R0007 Listener 127.0.0.1:$port bind: EADDRINUSE (98)" -- "${listener[@]}"
    run_unit "readiness-$route" "$work/readiness.lkj:1: R0007 Readiness epoll epoll_create1: EPERM (1)" \
        -p 'SystemCallFilter=~epoll_create1' -p SystemCallErrorNumber=EPERM -- "${readiness[@]}"
    run_unit "stop-$route" "$work/stop.lkj:1: R0007 Stop SIGTERM signalfd4: EPERM (1)" \
        -p 'SystemCallFilter=~signalfd4' -p SystemCallErrorNumber=EPERM -- "${stop[@]}"
    run_unit "stdout-$route" "$work/stdout.lkj:2: R0007 Stdout 1 write: EBADF (9)" \
        -- /bin/bash -c 'exec 1>&-; exec "$@"' diagnostic "${output[@]}"
    run_unit "stderr-$route" "$work/stderr.lkj:2: R0007 Stderr 2 write: EBADF (9)" \
        -- /bin/bash -c 'exec 2>&-; exec "$@"' diagnostic "${errors[@]}"
done
# Genuine syscall refusals retain their cause through acquisition cleanup.
run_unit foreign-fstat "$work/foreign-acquire.lkj:1: R0007 ForeignCommand $work/invalid-image fstat: EPERM (1)" \
    -p 'SystemCallFilter=~fstat' -p SystemCallErrorNumber=EPERM -- "$work/foreign-invalid"
run_unit null-fstat "$work/null-device.lkj:1: R0007 NullDevice /dev/null fstat: EPERM (1)" \
    -p 'SystemCallFilter=~fstat' -p SystemCallErrorNumber=EPERM -- "$work/null-device"
run_unit silent '' -- "$work/silent"
for authority in stdout stderr both; do
    grants=()
    if [[ $authority == stdout ]]; then parameters='_output: Stdout'; grants+=(--grant _output);
    elif [[ $authority == stderr ]]; then parameters='_errors: Stderr'; grants+=(--grant _errors);
    else parameters='_output: Stdout, _errors: Stderr'; grants+=(--grant _output --grant _errors); fi
    printf 'pub fn main(%s) { panic!("authorized failure"); }\n' "$parameters" > "$work/$authority-trap.lkj"
    "$compiler" build "$work/$authority-trap.lkj" --no-store "${grants[@]}" -o "$work/$authority-trap"
    run_unit "$authority-trap" "$work/$authority-trap.lkj:1: R0016 authorized failure" \
        -- /bin/bash -c 'if [[ $1 == stderr ]]; then exec 1>&-; else exec 2>&-; fi; exec "$2"' diagnostic "$authority" "$work/$authority-trap"
done
sudo -n systemctl stop "$holder"

# Result-returning host calls deliberately remain Results. These tiny consumers
# report the already-known grant/operation and io::errno_name through Stderr.
make_call() {
    local name=$1 parameters=$2 operation=$3 expression=$4
    shift 4
    cat > "$work/$name.lkj" <<SOURCE
use std::io::errno_name;
pub fn main($parameters, mut errors: Stderr, mut status: ExitStatus) {
    let result = $expression;
    if let Err(errno) = result {
        errors.write(format!("$work/$name.lkj:3: $operation: {} ({errno})\\n", errno_name(errno)).as_bytes());
        status.set_status(1u8);
    }
}
SOURCE
    "$compiler" build "$work/$name.lkj" --no-store --grant errors --grant status "$@" -o "$work/$name"
}
printf 'read only\n' > "$work/plain"
printf 'invalid executable\n' > "$work/invalid-image"
chmod 0755 "$work/invalid-image"
make_call directory-call 'mut root: Directory' "Directory $work openat2" 'root.open("missing")' --grant "root=$work"
make_call file-call 'mut root: Directory' "File $work/plain write" '{ let mut file = root.open("plain").unwrap(); file.write_all(b"x") }' --grant "root=$work"
make_call read-directory-call 'mut root: ReadDirectory' "ReadDirectory $work openat2" 'root.open("missing")' --grant "root=$work"
make_call read-file-call 'mut root: ReadDirectory' "ReadFile $work/plain read" '{ let mut file = root.open("plain").unwrap(); let mut bytes = [0u8; 1]; file.read(&mut bytes) }' --grant "root=$work"
# NullDevice has no independent call: its exclusive consumer is ForeignCommand.run.
make_call foreign-call 'mut tool: ForeignCommand, mut root: ReadDirectory, mut sink: NullDevice' 'ForeignCommand /usr/bin/true timerfd_create' '{ let arguments: Vec<Vec<u8>> = Vec::new(); tool.run(&mut root, &mut sink, &arguments, 1_000_000_000usize) }' --grant tool=/usr/bin/true --grant "root=$work" --grant sink
make_call connector-call 'mut target: Connector' "Connector 127.0.0.1:$port connect" 'target.connect()' --grant "target=127.0.0.1:$port"
make_call listener-call 'mut incoming: Listener' 'Listener 127.0.0.1:0 accept4' 'incoming.accept()' --grant incoming=127.0.0.1:0
make_call readiness-call 'mut ready: Readiness, mut clock: Clock' 'Readiness epoll epoll_pwait2' 'ready.wait(&mut clock, 0usize)' --grant ready --grant clock
make_call stop-call 'mut stop: Stop' 'Stop SIGTERM read' 'stop.requested()' --grant stop
make_call stdin-call 'mut input: Stdin' 'Stdin 0 read' '{ let mut bytes = [0u8; 1]; input.read(&mut bytes) }' --grant input
make_call clock-call 'mut clock: Clock' 'Clock kernel clock_gettime' 'clock.nanoseconds()' --grant clock
make_call entropy-call 'mut random: Entropy' 'Entropy getrandom getrandom' '{ let mut bytes = [0u8; 1]; random.fill(&mut bytes) }' --grant random
make_call process-call 'mut process: Process, mut root: Directory' 'Process children execveat' '{ let image = root.open("invalid-image").unwrap(); let input = root.open("plain").unwrap(); let output = root.create("child-output").unwrap(); let errors = root.create("child-errors").unwrap(); let result = root.create("child-result").unwrap(); let args: Vec<Vec<u8>> = Vec::new(); process.spawn_file(&image, &args, &input, &output, &errors, &result) }' --grant process --grant "root=$work"
run_unit directory-call "$work/directory-call.lkj:3: Directory $work openat2: ENOENT (2)" -- "$work/directory-call"
run_unit file-call "$work/file-call.lkj:3: File $work/plain write: EBADF (9)" -- "$work/file-call"
run_unit read-directory-call "$work/read-directory-call.lkj:3: ReadDirectory $work openat2: ENOENT (2)" -- "$work/read-directory-call"
run_unit read-file-call "$work/read-file-call.lkj:3: ReadFile $work/plain read: EPERM (1)" -p 'SystemCallFilter=~read' -p SystemCallErrorNumber=EPERM -- "$work/read-file-call"
run_unit foreign-call "$work/foreign-call.lkj:3: ForeignCommand /usr/bin/true timerfd_create: EPERM (1)" -p 'SystemCallFilter=~timerfd_create' -p SystemCallErrorNumber=EPERM -- "$work/foreign-call"
run_unit connector-call "$work/connector-call.lkj:3: Connector 127.0.0.1:$port connect: ECONNREFUSED (111)" -- "$work/connector-call"
run_unit listener-call "$work/listener-call.lkj:3: Listener 127.0.0.1:0 accept4: EPERM (1)" -p 'SystemCallFilter=~accept4' -p SystemCallErrorNumber=EPERM -- "$work/listener-call"
run_unit readiness-call "$work/readiness-call.lkj:3: Readiness epoll epoll_pwait2: EPERM (1)" -p 'SystemCallFilter=~epoll_pwait2' -p SystemCallErrorNumber=EPERM -- "$work/readiness-call"
run_unit stop-call "$work/stop-call.lkj:3: Stop SIGTERM read: EPERM (1)" -p 'SystemCallFilter=~read' -p SystemCallErrorNumber=EPERM -- "$work/stop-call"
run_unit stdin-call "$work/stdin-call.lkj:3: Stdin 0 read: EBADF (9)" -- /bin/bash -c 'exec 0<&-; exec "$1"' diagnostic "$work/stdin-call"
run_unit clock-call "$work/clock-call.lkj:3: Clock kernel clock_gettime: EPERM (1)" -p 'SystemCallFilter=~clock_gettime' -p SystemCallErrorNumber=EPERM -- "$work/clock-call"
run_unit entropy-call "$work/entropy-call.lkj:3: Entropy getrandom getrandom: EPERM (1)" -p 'SystemCallFilter=~getrandom' -p SystemCallErrorNumber=EPERM -- "$work/entropy-call"
run_unit process-call "$work/process-call.lkj:3: Process children execveat: ENOEXEC (8)" -- "$work/process-call"
# Source identities bind the caller's actual source root, program directory,
# alternate library and runtime before any source/body proof can be reused.
cat > "$work/module-root.lkj" <<'SOURCE'
mod runtime_child;
pub fn probe() { runtime_child::fail(); }
SOURCE
cat > "$work/runtime_child.lkj" <<'SOURCE'
pub fn fail() {
    let zero = 0i64;
    let value = 1i64 / zero;
}
SOURCE
cat > "$work/program.lkj" <<'SOURCE'
const SOURCE: &str = "module-root.lkj";
const ENTRY: &str = "probe";
const GRANTS: &[&str] = &[];
const CASES: &[&str] = &[];
SOURCE
mkdir "$work/custom-library"
cat > "$work/custom-library/lib.lkj" <<'SOURCE'
pub mod diagnostic_lib_child;
SOURCE
cat > "$work/custom-library/diagnostic_lib_child.lkj" <<'SOURCE'
pub fn fail() { panic!("custom library"); }
SOURCE
cat > "$work/library-root.lkj" <<'SOURCE'
pub fn probe() { std::diagnostic_lib_child::fail(); }
SOURCE
for route in native interpreter; do
    run_unit "source-root-$route" "$work/runtime_child.lkj:3: R0002 division by zero" \
        -- "$compiler" run module-root.lkj --source-dir "$work" --engine "$route" --no-store
    run_unit "program-root-$route" "$work/runtime_child.lkj:3: R0002 division by zero" \
        -- "$compiler" run "$work" --engine "$route" --no-store
    run_unit "library-root-$route" "$work/custom-library/diagnostic_lib_child.lkj:1: R0016 custom library" \
        -- "$compiler" run "$work/library-root.lkj" --library-dir "$work/custom-library" --engine "$route" --no-store
done
cp library/runtime.lkj "$work/custom-runtime.lkj"
# A deterministic runtime refusal checks location mapping without resource stress.
perl -0pi -e 's/pub fn allocate\(bytes: u64\) -> u64 \{/pub fn allocate(bytes: u64) -> u64 { if bytes != 0u64 { return raw_fail(); }/' "$work/custom-runtime.lkj"
cat > "$work/runtime-root.lkj" <<'SOURCE'
pub fn probe(_errors: Stderr) { let value = Box::new(42i64); drop(value); }
SOURCE
"$compiler" build "$work/runtime-root.lkj" --grant _errors \
    --runtime-source "$work/custom-runtime.lkj" --no-store -o "$work/custom-runtime"
run_unit runtime-artifact "$work/custom-runtime.lkj:13: R0006 heap allocation failed" -- "$work/custom-runtime"
run_unit runtime-supervised "$work/custom-runtime.lkj:13: R0006 heap allocation failed" \
    -- "$compiler" run "$work/runtime-root.lkj" --grant _errors --engine native \
    --runtime-source "$work/custom-runtime.lkj" --no-store

printf 'ok 63 isolated journal failures: 38 automatic acquisition/write routes, 13 explicit Result calls, 4 output-authority cases, 8 source-root bindings\n'
