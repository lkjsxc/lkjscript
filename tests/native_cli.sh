#!/usr/bin/env bash
# Contract tests for the native shell workflow, using executable compiler stubs.
# Usage: bash tests/native_cli.sh [PATH_TO_LKJ]
# No source compilation or Rust/Node command is needed by this test.
set -Eeuo pipefail
repository=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
wrapper=$(realpath -e -- "${1:-$repository/tools/lkj}")
[[ $# -le 1 ]] || { printf 'usage: native_cli.sh [PATH_TO_LKJ]\n' >&2; exit 1; }
work=$(mktemp -d)
case_name=setup
cleanup() { rm -rf -- "$work"; }
trap cleanup EXIT
failure() {
    printf 'native_cli: %s: %s\n' "$case_name" "$*" >&2
    if [[ -s $work/stderr ]]; then cat -- "$work/stderr" >&2; fi
    exit 1
}
trap 'failure "unexpected failure at line $LINENO: $BASH_COMMAND"' ERR
caller=$work/caller\ with\ spaces
toolchain=$work/toolchain\ with\ spaces
trace=$work/trace
path=$work/path
mkdir -p -- "$caller/project" "$caller/outputs" "$toolchain/bin" \
    "$toolchain/libexec" "$toolchain/library" "$trace" "$path" "$work/tmp"
cp -- "$wrapper" "$toolchain/bin/lkj"
cp -- "$(dirname -- "$wrapper")/native-process.sh" "$toolchain/libexec/native-process.sh"
printf '// source fixture\n' > "$caller/project/input.rs"
printf 'fixture K\n' > "$caller/direct.k"
printf 'fixture bundle\n' > "$caller/input.lkjsb"
printf '// selected library\n' > "$toolchain/library/lib.rs"
printf '// selected runtime source\n' > "$toolchain/runtime.rs"
printf 'default runtime K\n' > "$toolchain/runtime.k"
printf 'format lkjscript-native-v1\nsource_commit cli-fixture\n' > "$toolchain/manifest"
printf 'stdin with spaces\nsecond line\n' > "$work/stdin"
# This PATH deliberately lacks Rust, Cargo and Node. Keep the host tools needed
# by the shell workflow and these fixtures, rather than an alternate compiler.
for dependency in bash basename cat chmod cmp cp dirname env find ln mkdir mktemp mv prlimit realpath rm setsid sha256sum sort; do
    ln -s -- "$(command -v "$dependency")" "$path/$dependency"
done
export LKJ_CLI_TEST_TRACE=$trace
export LKJ_CLI_TEST_TARGET_BODY=$work/target-body
export LKJ_CLI_TEST_CONTROL=$work/expected-control
export LKJ_CLI_TEST_WAIT=$work/wait.sh
export LKJ_CLI_TEST_FIFO=$work/wait.fifo
mkfifo -- "$LKJ_CLI_TEST_FIFO"
export TMPDIR=$work/tmp
unset LKJ_TOOLCHAIN LKJ_RUN_MEMORY_BYTES LKJ_RUN_STACK_BYTES

cat > "$work/wait.sh" <<'WAIT'
lkj_fixture_wait() {
    [[ ${LKJ_CLI_TEST_LONG_STAGE:-} == "$1" ]] || return 0
    local stage=$1
    exec 9<>"$LKJ_CLI_TEST_FIFO"
    trap 'exit 143' TERM
    trap 'exit 130' INT
    printf '%s\n' "$BASHPID" > "$LKJ_CLI_TEST_TRACE/$stage.pid"
    while :; do read -r -t 60 -u 9 || :; done
}
WAIT
# This shim also exposes cancellation before env restores async INT handling.
rm -- "$path/env"
cat > "$path/env" <<'ENV'
#!/bin/bash
source "$LKJ_CLI_TEST_WAIT"
lkj_fixture_wait launcher
exec /usr/bin/env "$@"
ENV
chmod 755 "$path/env"

cat > "$toolchain/libexec/frontend" <<'FRONTEND'
#!/usr/bin/env bash
set -euo pipefail
source "$LKJ_CLI_TEST_WAIT"
lkj_fixture_wait frontend
printf '%s\0' "$@" > "$LKJ_CLI_TEST_TRACE/frontend.args"
printf '%s %s\n' "$(ulimit -v)" "$(ulimit -s)" > "$LKJ_CLI_TEST_TRACE/frontend.limits"
root=
directory=.
library=
mode=k
bundle=
while (($#)); do
    case $1 in
        --source) root=$2; shift 2 ;;
        --source-dir) directory=$2; shift 2 ;;
        --library-dir) library=$2; shift 2 ;;
        --no-library) library=none; shift ;;
        --check) mode=check; shift ;;
        --pack) mode=pack; shift ;;
        -*) printf 'fixture frontend: unexpected argument %s\n' "$1" >&2; exit 90 ;;
        *) [[ -z $bundle ]]; bundle=$1; shift ;;
    esac
done
if [[ -n $root ]]; then
    [[ $root != /* && -f $directory/$root ]]
    [[ $library == none || -f $library/lib.rs ]]
    printf '%s\n' "$root" > "$LKJ_CLI_TEST_TRACE/frontend.root"
    cp -- "$directory/$root" "$LKJ_CLI_TEST_TRACE/frontend.input"
else
    [[ -f $bundle ]]
    cp -- "$bundle" "$LKJ_CLI_TEST_TRACE/frontend.input"
fi
if [[ ${LKJ_CLI_TEST_FAIL_STAGE:-} == frontend ]]; then
    printf 'partial rejected K\n'
    printf 'original.rs:7:2: E_FIXTURE source rejection\n' >&2
    exit 31
fi
case $mode in
    check) ;;
    pack) printf 'fixture bundle\n' ;;
    k)
        if [[ $root == '$runtime/runtime.rs' ]]; then
            printf 'runtime:'
            cat -- "$directory/$root"
        else
            printf 'fixture K\n'
        fi
        ;;
esac
FRONTEND
cat > "$toolchain/libexec/kc" <<'KC'
#!/usr/bin/env bash
set -euo pipefail
source "$LKJ_CLI_TEST_WAIT"
lkj_fixture_wait kc
printf '%s\0' "$@" > "$LKJ_CLI_TEST_TRACE/kc.args"
printf '%s %s\n' "$(ulimit -v)" "$(ulimit -s)" > "$LKJ_CLI_TEST_TRACE/kc.limits"
program=$1
shift
runtime=
descriptor=
entry=default
check=0
while (($#)); do
    case $1 in
        --runtime) runtime=$2; shift 2 ;;
        --entry) entry=$2; shift 2 ;;
        --result-descriptor) descriptor=$2; shift 2 ;;
        --check) check=1; shift ;;
        *) printf 'fixture kc: unexpected argument %s\n' "$1" >&2; exit 90 ;;
    esac
done
[[ -f $program ]]
cp -- "$program" "$LKJ_CLI_TEST_TRACE/kc.input"
if [[ ${LKJ_CLI_TEST_FAIL_STAGE:-} == kc ]]; then
    printf 'partial rejected ELF\n'
    printf 'program.k:9:1: E_FIXTURE K rejection\n' >&2
    exit 32
fi
if ((check)); then exit 0; fi
[[ -f $runtime ]]
cp -- "$runtime" "$LKJ_CLI_TEST_TRACE/kc.runtime"
packet=0
if [[ -n $descriptor ]]; then
    printf 'descriptor:%s\n' "$entry" > "$descriptor"
    packet=1
fi
printf '#!/usr/bin/env bash\nset -euo pipefail\nlkj_fixture_packet=%s\n' "$packet"
cat -- "$LKJ_CLI_TEST_TARGET_BODY"
KC
cat > "$toolchain/libexec/result" <<'RESULT'
#!/usr/bin/env bash
set -euo pipefail
source "$LKJ_CLI_TEST_WAIT"
lkj_fixture_wait result
[[ $# == 2 && -f $1 ]]
[[ $(<"$1") == descriptor:* ]]
printf '%s\n' "$2" > "$LKJ_CLI_TEST_TRACE/result.label"
printf '%s %s\n' "$(ulimit -v)" "$(ulimit -s)" > "$LKJ_CLI_TEST_TRACE/result.limits"
cat > "$LKJ_CLI_TEST_TRACE/result.control"
cmp -s -- "$LKJ_CLI_TEST_CONTROL" "$LKJ_CLI_TEST_TRACE/result.control" || {
    printf 'fixture result: malformed packet or wrong parent status\n' >&2
    exit 91
}
printf '42\n'
exit "${LKJ_CLI_TEST_TARGET_STATUS:-0}"
RESULT
cat > "$work/target-body" <<'TARGET'
source "$LKJ_CLI_TEST_WAIT"
lkj_fixture_wait target
case ${LKJ_CLI_TEST_TARGET_MODE:-io} in
    io)
        printf 'cwd=%s\n' "$PWD"
        for argument in "$@"; do printf 'arg=<%s>\n' "$argument"; done
        printf 'stdin='
        cat
        printf 'target stderr\n' >&2
        ;;
    limits)
        printf '%s %s\n' "$(ulimit -v)" "$(ulimit -s)"
        ;;
    hard-limits)
        printf '%s %s\n' "$(ulimit -Hv)" "$(ulimit -Hs)"
        ;;
    closed-stdin)
        if { exec {input_copy}<&0; } 2>/dev/null; then exit 92; fi
        printf 'closed stdin\n'
        ;;
esac
if ((lkj_fixture_packet)); then
    printf '\000\000\000\000\000\000\000\000\052\000\000\000\000\000\000\000' >&3
    printf -v status_octal '%03o' "${LKJ_CLI_TEST_TARGET_STATUS:-0}"
    printf '%b' "\\$status_octal\000\000\000\000\000\000\000" >&3
fi
exit "${LKJ_CLI_TEST_TARGET_STATUS:-0}"
TARGET
chmod 755 -- "$toolchain/bin/lkj" "$toolchain/libexec/"*
(
    cd -- "$toolchain"
    find . -type f ! -name SHA256SUMS -print0 |
        sort -z |
        while IFS= read -r -d '' file; do sha256sum -- "$file"; done > SHA256SUMS
)

launcher=()
status=0
invoke() {
    rm -f -- "$trace/"*
    status=0
    (
        cd -- "$caller"
        PATH=$path "${launcher[@]}" bash "$wrapper" --toolchain "$toolchain" "$@"
    ) < "$work/stdin" > "$work/stdout" 2> "$work/stderr" || status=$?
}
expect_status() { [[ $status == "$1" ]] || failure "expected status $1, got $status"; }
expect_text() {
    cmp -s -- "$1" <(printf '%s' "$2") ||
        failure "unexpected bytes in $1: $(printf '%q' "$(<"$1")")"
}
expect_empty() { [[ ! -s $1 ]] || failure "expected empty $1"; }
expect_token() {
    local token
    while IFS= read -r -d '' token; do
        if [[ $token == "$2" ]]; then return 0; fi
    done < "$1"
    failure "argument $2 is absent from $1"
}
expect_no_stage() { [[ ! -e $trace/$1.args ]] || failure "unexpected $1 invocation"; }
expect_no_work() {
    [[ -z $(find "$work/tmp" -mindepth 1 -print -quit) ]] || failure 'temporary work survived'
    [[ -z $(find "$caller/outputs" -name '.lkj.*' -print -quit) ]] || failure 'publication temporary survived'
}

case_name='source, K and bundle routes'
invoke check project/input.rs
expect_status 0
expect_empty "$work/stdout"
expect_empty "$work/stderr"
expect_token "$trace/frontend.args" --check
expect_token "$trace/frontend.args" "$toolchain/library"
expect_no_stage kc
invoke check direct.k
expect_status 0
expect_no_stage frontend
expect_token "$trace/kc.args" --check
cmp -s -- "$caller/direct.k" "$trace/kc.input"
invoke k input.lkjsb
expect_status 0
expect_text "$work/stdout" $'fixture K\n'
cmp -s -- "$caller/input.lkjsb" "$trace/frontend.input"
expect_no_stage kc
invoke pack input.rs --source-dir "$caller/project" --no-library
expect_status 0
expect_text "$work/stdout" $'fixture bundle\n'
expect_token "$trace/frontend.args" --pack
expect_token "$trace/frontend.args" --no-library
expect_text "$trace/frontend.root" $'input.rs\n'
invoke k "$caller/project/input.rs" --no-library -o outputs/checked.k
expect_status 0
expect_empty "$work/stdout"
expect_text "$caller/outputs/checked.k" $'fixture K\n'

case_name='explicit runtime, entry and output publication'
printf 'selected runtime K\n' > "$caller/runtime override.k"
invoke build direct.k --runtime 'runtime override.k' --entry chosen -o 'outputs/built image'
expect_status 0
expect_no_stage frontend
expect_token "$trace/kc.args" chosen
expect_text "$trace/kc.runtime" $'selected runtime K\n'
expect_empty "$work/stdout"
[[ -x $caller/outputs/built\ image ]] || failure 'built artifact is not executable'
expect_no_work

case_name='failed compilation preserves published files and diagnostics'
printf 'old K\n' > "$caller/outputs/existing.k"
export LKJ_CLI_TEST_FAIL_STAGE=frontend
invoke k project/input.rs -o outputs/existing.k
unset LKJ_CLI_TEST_FAIL_STAGE
expect_status 31
expect_text "$caller/outputs/existing.k" $'old K\n'
expect_empty "$work/stdout"
expect_text "$work/stderr" $'original.rs:7:2: E_FIXTURE source rejection\n'
expect_no_stage kc
printf 'old ELF\n' > "$caller/outputs/existing.elf"
export LKJ_CLI_TEST_FAIL_STAGE=kc
invoke build direct.k -o outputs/existing.elf
unset LKJ_CLI_TEST_FAIL_STAGE
expect_status 32
expect_text "$caller/outputs/existing.elf" $'old ELF\n'
expect_empty "$work/stdout"
expect_text "$work/stderr" $'program.k:9:1: E_FIXTURE K rejection\n'
expect_no_work

case_name='output aliases cannot replace source inputs'
ln -- "$caller/project/input.rs" "$caller/source-hardlink"
ln -s -- "$caller/project/input.rs" "$caller/source-symlink"
for output in project/input.rs source-hardlink source-symlink; do
    invoke k project/input.rs -o "$output"
    [[ $status != 0 ]] || failure "accepted input alias $output"
    expect_text "$caller/project/input.rs" $'// source fixture\n'
    expect_no_stage frontend
done
invoke build direct.k -o direct.k
[[ $status != 0 ]] || failure 'accepted direct K output alias'
expect_text "$caller/direct.k" $'fixture K\n'
ln -- "$caller/runtime override.k" "$caller/runtime-hardlink.k"
for output in 'runtime override.k' runtime-hardlink.k; do
    invoke build direct.k --runtime 'runtime override.k' -o "$output"
    [[ $status != 0 ]] || failure "accepted runtime input alias $output"
    expect_text "$caller/runtime override.k" $'selected runtime K\n'
    expect_no_stage frontend
    expect_no_stage kc
done

case_name='run preserves caller streams, CWD, arguments and status'
export LKJ_CLI_TEST_TARGET_STATUS=37
invoke run project/input.rs -- 'with spaces' '' $'two\nlines' '--entry' '-n'
expect_status 37
{
    printf 'cwd=%s\n' "$caller"
    printf 'arg=<%s>\n' 'with spaces' '' $'two\nlines' '--entry' '-n'
    printf 'stdin='
    cat -- "$work/stdin"
} > "$work/expected-stdout"
cmp -s -- "$work/stdout" "$work/expected-stdout"
expect_text "$work/stderr" $'target stderr\n'
expect_no_work

case_name='eval isolates fd3 and appends the actual target status'
{
    printf '\000\000\000\000\000\000\000\000\052\000\000\000\000\000\000\000\045\000\000\000\000\000\000\000'
    printf '\n37\n'
} > "$work/expected-control"
invoke eval project/input.rs -- 'kept argument'
expect_status 37
{
    printf 'cwd=%s\narg=<kept argument>\nstdin=' "$caller"
    cat -- "$work/stdin"
    printf '42\n'
} > "$work/expected-stdout"
cmp -s -- "$work/stdout" "$work/expected-stdout"
expect_text "$work/stderr" $'target stderr\n'
expect_text "$trace/result.label" $'project/input.rs\n'
expect_token "$trace/kc.args" --result-descriptor
expect_no_work
unset LKJ_CLI_TEST_TARGET_STATUS

case_name='target budgets inherit and cannot relax parent limits'
export LKJ_CLI_TEST_TARGET_MODE=limits
launcher=(prlimit --as=536870912 --stack=8388608 --)
invoke run project/input.rs
expect_status 0
expect_text "$work/stdout" $'524288 8192\n'
export LKJ_RUN_MEMORY_BYTES=268435456 LKJ_RUN_STACK_BYTES=1048576
invoke run project/input.rs
expect_status 0
expect_text "$work/stdout" $'262144 1024\n'
expect_text "$trace/frontend.limits" $'524288 8192\n'
expect_text "$trace/kc.limits" $'524288 8192\n'
export LKJ_CLI_TEST_TARGET_MODE=hard-limits
invoke run project/input.rs
expect_status 0
expect_text "$work/stdout" $'524288 8192\n'
export LKJ_CLI_TEST_TARGET_MODE=limits
{
    printf '\000\000\000\000\000\000\000\000\052\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000'
    printf '\n0\n'
} > "$work/expected-control"
invoke eval project/input.rs
expect_status 0
expect_text "$work/stdout" $'262144 1024\n42\n'
expect_text "$trace/frontend.limits" $'524288 8192\n'
expect_text "$trace/kc.limits" $'524288 8192\n'
expect_text "$trace/result.limits" $'524288 8192\n'
export LKJ_RUN_MEMORY_BYTES=1073741824 LKJ_RUN_STACK_BYTES=16777216
invoke run project/input.rs
expect_status 0
expect_text "$work/stdout" $'524288 8192\n'
export LKJ_RUN_MEMORY_BYTES=unlimited LKJ_RUN_STACK_BYTES=unlimited
invoke run project/input.rs
expect_status 0
expect_text "$work/stdout" $'524288 8192\n'
export LKJ_RUN_MEMORY_BYTES=0 LKJ_RUN_STACK_BYTES=0
invoke run project/input.rs
[[ $status != 0 ]] || failure 'zero target budget was accepted'
expect_no_stage frontend
# These controls are irrelevant to compilation, including invalid values.
invoke build project/input.rs -o outputs/unlimited-compiler
expect_status 0
expect_text "$trace/frontend.limits" $'524288 8192\n'
expect_text "$trace/kc.limits" $'524288 8192\n'
unset LKJ_RUN_MEMORY_BYTES LKJ_RUN_STACK_BYTES LKJ_CLI_TEST_TARGET_MODE
launcher=()

case_name='runtime preparation has explicit source and stable logical identity'
printf 'replacement runtime source\n' > "$caller/runtime source.rs"
invoke runtime-k --runtime-source 'runtime source.rs' -o outputs/runtime.k
expect_status 0
expect_text "$caller/outputs/runtime.k" $'runtime:replacement runtime source\n'
expect_text "$trace/frontend.root" $'$runtime/runtime.rs\n'
expect_token "$trace/frontend.args" --no-library
expect_no_stage kc
invoke runtime-k --runtime-source 'runtime source.rs' -o 'runtime source.rs'
[[ $status != 0 ]] || failure 'runtime preparation overwrote its source'
expect_text "$caller/runtime source.rs" $'replacement runtime source\n'

case_name='paths retain kernel symlink semantics and final output entries'
mkdir -p -- "$caller/physical/deep" "$caller/logical" "$caller/relative-tmp"
ln -s -- ../physical/deep "$caller/logical/link"
printf 'kernel-selected K\n' > "$caller/physical/input.k"
printf 'wrong lexical K\n' > "$caller/logical/input.k"
invoke k logical/link/../input.k
expect_status 0
expect_text "$work/stdout" $'kernel-selected K\n'
printf '// kernel-selected source\n' > "$caller/physical/input.rs"
printf '// wrong lexical source\n' > "$caller/logical/input.rs"
invoke check input.rs --source-dir logical/link/.. --no-library
expect_status 0
expect_text "$trace/frontend.input" $'// kernel-selected source\n'
printf 'kernel-selected runtime\n' > "$caller/physical/runtime.k"
printf 'wrong lexical runtime\n' > "$caller/logical/runtime.k"
invoke build direct.k --runtime logical/link/../runtime.k -o outputs/path-test
expect_status 0
expect_text "$trace/kc.runtime" $'kernel-selected runtime\n'
printf 'keep symlink referent\n' > "$caller/outputs/referent"
ln -s -- referent "$caller/outputs/final-link"
invoke k direct.k -o outputs/final-link
expect_status 0
[[ ! -L $caller/outputs/final-link ]] || failure 'final output symlink was followed'
expect_text "$caller/outputs/final-link" $'fixture K\n'
expect_text "$caller/outputs/referent" $'keep symlink referent\n'
TMPDIR=relative-tmp invoke runtime-k -o outputs/relative-runtime.k
expect_status 0
expect_text "$caller/outputs/relative-runtime.k" $'runtime:// selected runtime source\n'
[[ -z $(find "$caller/relative-tmp" -mindepth 1 -print -quit) ]] || failure 'relative temporary directory leaked'

case_name='async launch preserves a closed stdin'
export LKJ_CLI_TEST_TARGET_MODE=closed-stdin
for mode in run eval; do
    status=0
    (
        cd -- "$caller"
        PATH=$path bash "$wrapper" --toolchain "$toolchain" "$mode" project/input.rs 0<&-
    ) >"$work/stdout" 2>"$work/stderr" || status=$?
    expect_status 0
    if [[ $mode == run ]]; then expect_text "$work/stdout" $'closed stdin\n'
    else expect_text "$work/stdout" $'closed stdin\n42\n'; fi
done
unset LKJ_CLI_TEST_TARGET_MODE

case_name='TERM and INT cancel only the owned child and clean staging'
cancel_case() {
    local stage=$1 signal=$2 expected=$3 wrapper_pid worker_pid timer_pid finished attempt
    local arguments=(run project/input.rs)
    if [[ $stage == result ]]; then arguments=(eval project/input.rs); fi
    rm -f -- "$trace/"*
    (
        cd -- "$caller"
        exec /usr/bin/env --default-signal=INT,QUIT PATH="$path" LKJ_CLI_TEST_LONG_STAGE="$stage" \
            bash "$wrapper" --toolchain "$toolchain" "${arguments[@]}"
    ) <"$work/stdin" >"$work/stdout" 2>"$work/stderr" &
    wrapper_pid=$!
    for ((attempt=0; attempt<500; attempt++)); do
        [[ ! -f $trace/$stage.pid ]] || break
        sleep 0.01
    done
    if [[ ! -f $trace/$stage.pid ]]; then
        kill -KILL "$wrapper_pid" 2>/dev/null || :
        wait "$wrapper_pid" 2>/dev/null || :
        failure "$stage never reached its wait point"
    fi
    worker_pid=$(<"$trace/$stage.pid")
    kill -s "$signal" "$wrapper_pid"
    sleep 5 &
    timer_pid=$!
    status=0
    finished=
    wait -n -p finished "$wrapper_pid" "$timer_pid" || status=$?
    if [[ $finished == "$timer_pid" ]]; then
        kill -KILL "$wrapper_pid" "$worker_pid" 2>/dev/null || :
        wait "$wrapper_pid" 2>/dev/null || :
        failure "$signal did not cancel $stage"
    fi
    kill "$timer_pid" 2>/dev/null || :
    wait "$timer_pid" 2>/dev/null || :
    expect_status "$expected"
    [[ ! -e /proc/$worker_pid/status ]] || failure "$stage remained after $signal"
    expect_no_work
}
for stage in frontend kc target result launcher; do
    cancel_case "$stage" TERM 143
    cancel_case "$stage" INT 130
done

case_name='installed selection and explicit verification'
invoke toolchain
expect_status 0
[[ $(<"$work/stdout") == *'source_commit cli-fixture'* ]] || failure 'manifest is not visible'
invoke verify
expect_status 0
printf '// altered library\n' >> "$toolchain/library/lib.rs"
invoke verify
[[ $status != 0 ]] || failure 'verification accepted altered library bytes'
printf '// selected library\n' > "$toolchain/library/lib.rs"
(
    cd -- "$caller"
    PATH=$path bash "$toolchain/bin/lkj" toolchain
) > "$work/stdout" 2> "$work/stderr"
[[ $(<"$work/stdout") == *'source_commit cli-fixture'* ]] || failure 'installed command did not select its own toolchain'
expect_no_work
printf 'native_cli: all workflow contract tests passed\n'
