#!/bin/bash
# Exercise rebuild orchestration without executing a compiler workload.
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
rebuild=$script_dir/../tools/rebuild-native.sh
work=$(mktemp -d "${TMPDIR:-/tmp}/lkjscript-native-rebuild-test.XXXXXXXX")
test_pids=()
stub_pids=()
live_pid() {
    local state
    [[ -r /proc/$1/stat ]] || return 1
    IFS= read -r state < "/proc/$1/stat" || return 1
    state=${state##*) }
    [[ ${state%% *} != Z && ${state%% *} != X ]]
}
cleanup() {
    local pid
    trap - EXIT
    for pid in "${stub_pids[@]}" "${test_pids[@]}"; do
        if live_pid "$pid"; then kill -KILL "$pid" 2>/dev/null || :; fi
    done
    for pid in "${test_pids[@]}"; do wait "$pid" 2>/dev/null || :; done
    rm -rf -- "$work"
}
trap cleanup EXIT
fail() { printf 'native rebuild test: %s\n' "$*" >&2; exit 1; }
export REBUILD_TEST_SEED=$work/'seed toolchain'
export REBUILD_TEST_SOURCE=$work/'source checkout'
export REBUILD_TEST_CALLS=$work/calls
export REBUILD_TEST_COMMIT='explicit source identity'
export LKJSCRIPT_TOOLCHAIN=$work/'unselected toolchain'
export TMPDIR=$work/staging
mkdir -p -- "$REBUILD_TEST_SEED/bin" "$REBUILD_TEST_SOURCE/tools" \
    "$REBUILD_TEST_SOURCE/library" "$work/caller" "$work/forbidden" "$TMPDIR"
printf 'checkout runtime\n' > "$REBUILD_TEST_SOURCE/library/runtime.rs"
for name in cargo rustc node git; do
    printf '#!/bin/bash\nexit 99\n' > "$work/forbidden/$name"
    chmod 755 -- "$work/forbidden/$name"
done
export PATH=$work/forbidden:$PATH
cat > "$REBUILD_TEST_SEED/bin/lkjscript" <<'STUB'
#!/bin/bash
set -euo pipefail
[[ $1 == --toolchain && $2 == "$REBUILD_TEST_SEED" ]]
shift 2
command=$1
shift
case $command in
    runtime-ir)
        [[ $# == 4 && $1 == --runtime-source && $2 == "$REBUILD_TEST_SOURCE/library/runtime.rs" && $3 == -o ]]
        [[ $4 == /* ]]
        printf 'runtime-ir\n' >> "$REBUILD_TEST_CALLS"
        [[ $(stat -c %a -- "$(dirname -- "$4")") == 700 ]]
        cp -- "$2" "$4"
        ;;
    ir)
        input=$1
        shift
        [[ $# == 6 && $1 == --source-dir && $2 == "$REBUILD_TEST_SOURCE" ]]
        [[ $3 == --library-dir && $4 == "$REBUILD_TEST_SOURCE/library" && $5 == -o ]]
        printf 'ir %s\n' "$input" >> "$REBUILD_TEST_CALLS"
        [[ ${REBUILD_TEST_FAIL:-} != "ir $input" ]] || exit 24
        if [[ ${REBUILD_TEST_BLOCK:-} == 1 && $input == src/frontend_main.rs ]]; then
            REBUILD_TEST_TIME_PID=$PPID bash -c '
                trap "exit 130" INT
                trap "exit 143" TERM
                exec {blocked}<>"$REBUILD_TEST_FIFO"
                printf "%s %s %s\n" "$BASHPID" "$PPID" "$REBUILD_TEST_TIME_PID" > "$REBUILD_TEST_PIDS"
                IFS= read -r ignored <&"$blocked"
            '
        fi
        printf '%s\n' "$input" > "$6"
        ;;
    build)
        input=$1
        shift
        [[ $# == 6 && $1 == --runtime && $3 == --entry && $4 == main && $5 == -o ]]
        cmp -s -- "$2" "$REBUILD_TEST_SOURCE/library/runtime.rs"
        [[ $(dirname -- "$input") == "$(dirname -- "$2")" ]]
        [[ $(dirname -- "$input") == "$(dirname -- "$6")" ]]
        [[ $(basename -- "$input") == "$(basename -- "$6").lkjir" ]]
        printf 'build %s\n' "$(basename -- "$input")" >> "$REBUILD_TEST_CALLS"
        [[ ${REBUILD_TEST_FAIL:-} != "build $(basename -- "$input")" ]] || exit 24
        cp -- "$input" "$6"
        ;;
    *) exit 98 ;;
esac
STUB
chmod 755 -- "$REBUILD_TEST_SEED/bin/lkjscript"
cat > "$REBUILD_TEST_SOURCE/tools/install-native.sh" <<'STUB'
#!/bin/bash
set -euo pipefail
destination=$1
shift
[[ $# == 12 && $1 == --frontend && $3 == --backend && $5 == --result ]]
[[ $(cat -- "$2") == src/frontend_main.rs ]]
[[ $(cat -- "$4") == src/backend_main.rs ]]
[[ $(cat -- "$6") == src/result_main.rs ]]
[[ $7 == --library && $8 == "$REBUILD_TEST_SOURCE/library" ]]
[[ $9 == --runtime-source && ${10} == "$REBUILD_TEST_SOURCE/library/runtime.rs" ]]
[[ ${11} == --source-commit && ${12} == "$REBUILD_TEST_COMMIT" ]]
printf 'install\n' >> "$REBUILD_TEST_CALLS"
[[ ${REBUILD_TEST_FAIL:-} != install ]] || exit 25
[[ ! -e $destination && ! -L $destination ]]
mkdir -- "$destination"
printf 'installed\n' > "$destination/marker"
STUB
no_staging() {
    local remaining
    remaining=$(find "$TMPDIR" -mindepth 1 -print -quit)
    [[ -z $remaining ]] || fail "staging remains: $remaining"
}
cd -- "$work/caller"
bash "$rebuild" 'new toolchain' --seed "$REBUILD_TEST_SEED" \
    --source-dir "$REBUILD_TEST_SOURCE" --source-commit "$REBUILD_TEST_COMMIT" \
    > "$work/success.stdout" 2> "$work/success.stderr"
[[ $(cat 'new toolchain/marker') == installed ]] || fail 'destination was not installed'
mapfile -t calls < "$REBUILD_TEST_CALLS"
[[ ${#calls[@]} == 8 && ${calls[0]} == runtime-ir && ${calls[1]} == 'ir src/frontend_main.rs' \
    && ${calls[2]} == 'build frontend.lkjir' && ${calls[3]} == 'ir src/backend_main.rs' \
    && ${calls[4]} == 'build backend.lkjir' && ${calls[5]} == 'ir src/result_main.rs' \
    && ${calls[6]} == 'build result.lkjir' && ${calls[7]} == install ]] || fail 'generation command order differs'
hash_names=(runtime frontend backend result)
hash_count=0
while IFS= read -r line; do
    case $line in
        'rebuild-native: ir_sha256 '*)
            read -r prefix field component digest <<< "$line"
            ((hash_count < 4)) || fail 'too many IR hash records'
            [[ $component == "${hash_names[hash_count]}" ]] || fail 'IR hash record order differs'
            if [[ $component == runtime ]]; then
                expected=$(sha256sum < "$REBUILD_TEST_SOURCE/library/runtime.rs")
            else
                expected=$(printf '%s\n' "src/${component}_main.rs" | sha256sum)
            fi
            [[ $digest == "${expected%% *}" ]] || fail "wrong IR hash: $component"
            hash_count=$((hash_count + 1))
            ;;
    esac
done < "$work/success.stderr"
((hash_count == 4)) || fail 'missing IR hash records'
no_staging
cp -- "$REBUILD_TEST_CALLS" "$work/before-existing"
if bash "$rebuild" 'new toolchain' --seed "$REBUILD_TEST_SEED" --source-dir "$REBUILD_TEST_SOURCE" \
    > "$work/existing.stdout" 2> "$work/existing.stderr"; then
    fail 'existing destination was accepted'
fi
cmp -s -- "$REBUILD_TEST_CALLS" "$work/before-existing" || fail 'existing destination started compiler work'
[[ $(cat 'new toolchain/marker') == installed ]] || fail 'existing destination was modified'
for failure in 'ir src/backend_main.rs' 'build backend.lkjir' install; do
    : > "$REBUILD_TEST_CALLS"
    export REBUILD_TEST_FAIL=$failure
    if bash "$rebuild" 'failed toolchain' --seed "$REBUILD_TEST_SEED" \
        --source-dir "$REBUILD_TEST_SOURCE" --source-commit "$REBUILD_TEST_COMMIT" \
        > "$work/failure.stdout" 2> "$work/failure.stderr"; then
        fail "expected failure was lost: $failure"
    fi
    [[ ! -e 'failed toolchain' && ! -L 'failed toolchain' ]] || fail 'failed generation published a destination'
    no_staging
done
unset REBUILD_TEST_FAIL
[[ -x /usr/bin/time ]] || fail 'GNU time is required for the cancellation cases'
for signal in TERM INT; do
    expected_status=143
    [[ $signal != INT ]] || expected_status=130
    case_dir=$work/cancel-$signal
    mkdir -- "$case_dir"
    export REBUILD_TEST_BLOCK=1
    export REBUILD_TEST_FIFO=$case_dir/block.fifo
    export REBUILD_TEST_PIDS=$case_dir/pids
    mkfifo -- "$REBUILD_TEST_FIFO"
    exec {pause_fd}<>"$REBUILD_TEST_FIFO"
    destination=$work/caller/cancelled-$signal
    env --default-signal=INT,QUIT -- bash "$rebuild" "$destination" \
        --seed "$REBUILD_TEST_SEED" --source-dir "$REBUILD_TEST_SOURCE" \
        --source-commit "$REBUILD_TEST_COMMIT" \
        > "$case_dir/stdout" 2> "$case_dir/stderr" &
    wrapper=$!
    test_pids+=("$wrapper")
    sleep 8 &
    deadline=$!
    test_pids+=("$deadline")
    (
        until [[ -s $REBUILD_TEST_PIDS ]]; do
            IFS= read -r -t 0.01 ignored <&"$pause_fd" || :
        done
    ) &
    ready=$!
    test_pids+=("$ready")
    status=0
    wait -n -p completed "$ready" "$wrapper" "$deadline" || status=$?
    if [[ ${completed:-} != "$ready" || $status != 0 ]]; then
        cat -- "$case_dir/stderr" >&2
        fail "$signal case did not reach the blocked compiler"
    fi
    read -r compiler_pid seed_pid time_pid < "$REBUILD_TEST_PIDS"
    stub_pids+=("$compiler_pid" "$seed_pid" "$time_pid")
    [[ /proc/$time_pid/exe -ef /usr/bin/time ]] || fail 'long command is not wrapped by GNU time'
    for pid in "$compiler_pid" "$seed_pid" "$time_pid"; do
        live_pid "$pid" || fail "expected live descendant: $pid"
    done
    kill -s "$signal" "$wrapper"
    status=0
    wait -n -p completed "$wrapper" "$deadline" || status=$?
    [[ ${completed:-} == "$wrapper" && $status == "$expected_status" ]] ||
        fail "$signal cancellation did not promptly return $expected_status (got $status)"
    (
        while live_pid "$compiler_pid" || live_pid "$seed_pid" || live_pid "$time_pid"; do
            IFS= read -r -t 0.01 ignored <&"$pause_fd" || :
        done
    ) &
    drained=$!
    test_pids+=("$drained")
    status=0
    wait -n -p completed "$drained" "$deadline" || status=$?
    [[ ${completed:-} == "$drained" && $status == 0 ]] ||
        fail "$signal left a live descendant (zombies are not counted as live)"
    kill -TERM "$deadline" 2>/dev/null || :
    wait "$deadline" 2>/dev/null || :
    exec {pause_fd}<&-
    [[ ! -e $destination && ! -L $destination ]] || fail "$signal published a destination"
    no_staging
done
unset REBUILD_TEST_BLOCK
mkdir -- 'relative staging'
TMPDIR='relative staging' bash "$rebuild" 'relative toolchain' \
    --seed "$REBUILD_TEST_SEED" --source-dir "$REBUILD_TEST_SOURCE" \
    --source-commit "$REBUILD_TEST_COMMIT" \
    > "$work/relative.stdout" 2> "$work/relative.stderr"
[[ $(cat 'relative toolchain/marker') == installed ]] || fail 'relative TMPDIR rebuild failed'
[[ -z $(find 'relative staging' -mindepth 1 -print -quit) ]] || fail 'relative staging remains'
printf 'native rebuild orchestration and TERM/INT cancellation passed\n'
