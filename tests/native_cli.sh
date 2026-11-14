#!/bin/bash
# Contract tests for the native shell workflow, using executable compiler stubs.
# Usage: bash tests/native_cli.sh [PATH_TO_LKJSCRIPT]
# No source compilation or Rust/Node command is needed by this test.
set -Eeuo pipefail
repository=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
wrapper=$(realpath -e -- "${1:-$repository/tools/lkjscript}")
[[ $# -le 1 ]] || { printf 'usage: native_cli.sh [PATH_TO_LKJSCRIPT]\n' >&2; exit 1; }
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
cp -- "$wrapper" "$toolchain/bin/lkjscript"
cp -- "$(dirname -- "$wrapper")/native-process.sh" "$toolchain/libexec/native-process.sh"
printf '// source fixture\n' > "$caller/project/input.rs"
printf 'fixture IR\n' > "$caller/direct.lkjir"
printf 'fixture bundle\n' > "$caller/input.lkjsb"
printf '// selected library\n' > "$toolchain/library/lib.rs"
printf '// selected runtime source\n' > "$toolchain/runtime.rs"
printf 'default runtime IR\n' > "$toolchain/runtime.lkjir"
printf 'format lkjscript-native-v1\nsource_commit cli-fixture\n' > "$toolchain/manifest"
printf 'stdin with spaces\nsecond line\n' > "$work/stdin"
# This PATH deliberately lacks Rust, Cargo and Node. Keep the host tools needed
# by the shell workflow and these fixtures, rather than an alternate compiler.
for dependency in bash basename cat chmod cmp cp dirname find ln mkdir mktemp mv realpath rm setsid sha256sum sort; do
    ln -s -- "$(command -v "$dependency")" "$path/$dependency"
done
export LKJSCRIPT_CLI_TEST_TRACE=$trace
export LKJSCRIPT_CLI_TEST_TARGET_BODY=$work/target-body
export LKJSCRIPT_CLI_TEST_CONTROL=$work/expected-control
export LKJSCRIPT_CLI_TEST_WAIT=$work/wait.sh
export LKJSCRIPT_CLI_TEST_FIFO=$work/wait.fifo
export LKJSCRIPT_CLI_TEST_SETSID
LKJSCRIPT_CLI_TEST_SETSID=$(command -v setsid)
mkfifo -- "$LKJSCRIPT_CLI_TEST_FIFO"
export TMPDIR=$work/tmp
unset LKJSCRIPT_TOOLCHAIN

cat > "$work/wait.sh" <<'WAIT'
lkjscript_fixture_wait() {
    [[ ${LKJSCRIPT_CLI_TEST_LONG_STAGE:-} == "$1" ]] || return 0
    local stage=$1
    exec 9<>"$LKJSCRIPT_CLI_TEST_FIFO"
    trap 'exit 143' TERM
    trap 'exit 130' INT
    printf '%s\n' "$BASHPID" > "$LKJSCRIPT_CLI_TEST_TRACE/$stage.pid"
    while :; do read -r -t 60 -u 9 || :; done
}
WAIT
# This shim exposes cancellation before group creation and the ready marker.
rm -- "$path/setsid"
cat > "$path/setsid" <<'SETSID'
#!/bin/bash
source "$LKJSCRIPT_CLI_TEST_WAIT"
lkjscript_fixture_wait launcher
exec "$LKJSCRIPT_CLI_TEST_SETSID" "$@"
SETSID
chmod 755 "$path/setsid"

cat > "$toolchain/libexec/frontend" <<'FRONTEND'
#!/bin/bash
set -euo pipefail
source "$LKJSCRIPT_CLI_TEST_WAIT"
lkjscript_fixture_wait frontend
printf '%s\0' "$@" > "$LKJSCRIPT_CLI_TEST_TRACE/frontend.args"
root=
directory=.
library=
mode=ir
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
    printf '%s\n' "$root" > "$LKJSCRIPT_CLI_TEST_TRACE/frontend.root"
    cp -- "$directory/$root" "$LKJSCRIPT_CLI_TEST_TRACE/frontend.input"
else
    [[ -f $bundle ]]
    cp -- "$bundle" "$LKJSCRIPT_CLI_TEST_TRACE/frontend.input"
fi
if [[ ${LKJSCRIPT_CLI_TEST_FAIL_STAGE:-} == frontend ]]; then
    printf 'partial rejected IR\n'
    printf 'original.rs:7:2: E_FIXTURE source rejection\n' >&2
    exit 31
fi
case $mode in
    check) ;;
    pack) printf 'fixture bundle\n' ;;
    ir)
        if [[ $root == '$runtime/runtime.rs' ]]; then
            printf 'runtime:'
            cat -- "$directory/$root"
        else
            printf 'fixture IR\n'
        fi
        ;;
esac
FRONTEND
cat > "$toolchain/libexec/backend" <<'BACKEND'
#!/bin/bash
set -euo pipefail
source "$LKJSCRIPT_CLI_TEST_WAIT"
lkjscript_fixture_wait backend
printf '%s\0' "$@" > "$LKJSCRIPT_CLI_TEST_TRACE/backend.args"
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
        *) printf 'fixture backend: unexpected argument %s\n' "$1" >&2; exit 90 ;;
    esac
done
[[ -f $program ]]
cp -- "$program" "$LKJSCRIPT_CLI_TEST_TRACE/backend.input"
if [[ ${LKJSCRIPT_CLI_TEST_FAIL_STAGE:-} == backend ]]; then
    printf 'partial rejected ELF\n'
    printf 'program.lkjir:9:1: E_FIXTURE IR rejection\n' >&2
    exit 32
fi
if ((check)); then exit 0; fi
[[ -f $runtime ]]
cp -- "$runtime" "$LKJSCRIPT_CLI_TEST_TRACE/backend.runtime"
packet=0
if [[ -n $descriptor ]]; then
    printf 'descriptor:%s\n' "$entry" > "$descriptor"
    packet=1
fi
printf '#!/bin/bash\nset -euo pipefail\nlkjscript_fixture_packet=%s\n' "$packet"
cat -- "$LKJSCRIPT_CLI_TEST_TARGET_BODY"
BACKEND
cat > "$toolchain/libexec/result" <<'RESULT'
#!/bin/bash
set -euo pipefail
source "$LKJSCRIPT_CLI_TEST_WAIT"
lkjscript_fixture_wait result
[[ $# == 2 && -f $1 ]]
[[ $(<"$1") == descriptor:* ]]
printf '%s\n' "$2" > "$LKJSCRIPT_CLI_TEST_TRACE/result.label"
cat > "$LKJSCRIPT_CLI_TEST_TRACE/result.control"
cmp -s -- "$LKJSCRIPT_CLI_TEST_CONTROL" "$LKJSCRIPT_CLI_TEST_TRACE/result.control" || {
    printf 'fixture result: malformed packet or wrong parent status\n' >&2
    exit 91
}
printf '42\n'
exit "${LKJSCRIPT_CLI_TEST_TARGET_STATUS:-0}"
RESULT
cat > "$work/target-body" <<'TARGET'
source "$LKJSCRIPT_CLI_TEST_WAIT"
lkjscript_fixture_wait target
case ${LKJSCRIPT_CLI_TEST_TARGET_MODE:-io} in
    io)
        printf 'cwd=%s\n' "$PWD"
        for argument in "$@"; do printf 'arg=<%s>\n' "$argument"; done
        printf 'stdin='
        cat
        printf 'target stderr\n' >&2
        ;;
    closed-stdin)
        if { exec {input_copy}<&0; } 2>/dev/null; then exit 92; fi
        printf 'closed stdin\n'
        ;;
esac
if ((lkjscript_fixture_packet)); then
    printf '\000\000\000\000\000\000\000\000\052\000\000\000\000\000\000\000' >&3
    printf -v status_octal '%03o' "${LKJSCRIPT_CLI_TEST_TARGET_STATUS:-0}"
    printf '%b' "\\$status_octal\000\000\000\000\000\000\000" >&3
fi
exit "${LKJSCRIPT_CLI_TEST_TARGET_STATUS:-0}"
TARGET
chmod 755 -- "$toolchain/bin/lkjscript" "$toolchain/libexec/"*
(
    cd -- "$toolchain"
    find . -type f ! -name SHA256SUMS -print0 |
        sort -z |
        while IFS= read -r -d '' file; do sha256sum -- "$file"; done > SHA256SUMS
)

status=0
invoke() {
    rm -f -- "$trace/"*
    status=0
    (
        cd -- "$caller"
        PATH=$path bash "$wrapper" --toolchain "$toolchain" "$@"
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
    [[ -z $(find "$caller/outputs" -name '.lkjscript.*' -print -quit) ]] || failure 'publication temporary survived'
}

case_name='source, IR and bundle routes'
invoke check project/input.rs
expect_status 0
expect_empty "$work/stdout"
expect_empty "$work/stderr"
expect_token "$trace/frontend.args" --check
expect_token "$trace/frontend.args" "$toolchain/library"
expect_no_stage backend
invoke check direct.lkjir
expect_status 0
expect_no_stage frontend
expect_token "$trace/backend.args" --check
cmp -s -- "$caller/direct.lkjir" "$trace/backend.input"
invoke ir input.lkjsb
expect_status 0
expect_text "$work/stdout" $'fixture IR\n'
cmp -s -- "$caller/input.lkjsb" "$trace/frontend.input"
expect_no_stage backend
invoke pack input.rs --source-dir "$caller/project" --no-library
expect_status 0
expect_text "$work/stdout" $'fixture bundle\n'
expect_token "$trace/frontend.args" --pack
expect_token "$trace/frontend.args" --no-library
expect_text "$trace/frontend.root" $'input.rs\n'
invoke ir "$caller/project/input.rs" --no-library -o outputs/checked.lkjir
expect_status 0
expect_empty "$work/stdout"
expect_text "$caller/outputs/checked.lkjir" $'fixture IR\n'

case_name='explicit runtime, entry and output publication'
printf 'selected runtime IR\n' > "$caller/runtime override.lkjir"
invoke build direct.lkjir --runtime 'runtime override.lkjir' --entry chosen -o 'outputs/built image'
expect_status 0
expect_no_stage frontend
expect_token "$trace/backend.args" chosen
expect_text "$trace/backend.runtime" $'selected runtime IR\n'
expect_empty "$work/stdout"
[[ -x $caller/outputs/built\ image ]] || failure 'built artifact is not executable'
expect_no_work

case_name='failed compilation preserves published files and diagnostics'
printf 'old IR\n' > "$caller/outputs/existing.lkjir"
export LKJSCRIPT_CLI_TEST_FAIL_STAGE=frontend
invoke ir project/input.rs -o outputs/existing.lkjir
unset LKJSCRIPT_CLI_TEST_FAIL_STAGE
expect_status 31
expect_text "$caller/outputs/existing.lkjir" $'old IR\n'
expect_empty "$work/stdout"
expect_text "$work/stderr" $'original.rs:7:2: E_FIXTURE source rejection\n'
expect_no_stage backend
printf 'old ELF\n' > "$caller/outputs/existing.elf"
export LKJSCRIPT_CLI_TEST_FAIL_STAGE=backend
invoke build direct.lkjir -o outputs/existing.elf
unset LKJSCRIPT_CLI_TEST_FAIL_STAGE
expect_status 32
expect_text "$caller/outputs/existing.elf" $'old ELF\n'
expect_empty "$work/stdout"
expect_text "$work/stderr" $'program.lkjir:9:1: E_FIXTURE IR rejection\n'
expect_no_work

case_name='output aliases cannot replace source inputs'
ln -- "$caller/project/input.rs" "$caller/source-hardlink"
ln -s -- "$caller/project/input.rs" "$caller/source-symlink"
for output in project/input.rs source-hardlink source-symlink; do
    invoke ir project/input.rs -o "$output"
    [[ $status != 0 ]] || failure "accepted input alias $output"
    expect_text "$caller/project/input.rs" $'// source fixture\n'
    expect_no_stage frontend
done
invoke build direct.lkjir -o direct.lkjir
[[ $status != 0 ]] || failure 'accepted direct IR output alias'
expect_text "$caller/direct.lkjir" $'fixture IR\n'
ln -- "$caller/runtime override.lkjir" "$caller/runtime-hardlink.lkjir"
for output in 'runtime override.lkjir' runtime-hardlink.lkjir; do
    invoke build direct.lkjir --runtime 'runtime override.lkjir' -o "$output"
    [[ $status != 0 ]] || failure "accepted runtime input alias $output"
    expect_text "$caller/runtime override.lkjir" $'selected runtime IR\n'
    expect_no_stage frontend
    expect_no_stage backend
done

case_name='run preserves caller streams, CWD, arguments and status'
export LKJSCRIPT_CLI_TEST_TARGET_STATUS=37
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
expect_token "$trace/backend.args" --result-descriptor
expect_no_work
unset LKJSCRIPT_CLI_TEST_TARGET_STATUS

# The remaining eval fixtures use a successful target status.
{
    printf '\000\000\000\000\000\000\000\000\052\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000'
    printf '\n0\n'
} > "$work/expected-control"

case_name='runtime preparation has explicit source and stable logical identity'
printf 'replacement runtime source\n' > "$caller/runtime source.rs"
invoke runtime-ir --runtime-source 'runtime source.rs' -o outputs/runtime.lkjir
expect_status 0
expect_text "$caller/outputs/runtime.lkjir" $'runtime:replacement runtime source\n'
expect_text "$trace/frontend.root" $'$runtime/runtime.rs\n'
expect_token "$trace/frontend.args" --no-library
expect_no_stage backend
invoke runtime-ir --runtime-source 'runtime source.rs' -o 'runtime source.rs'
[[ $status != 0 ]] || failure 'runtime preparation overwrote its source'
expect_text "$caller/runtime source.rs" $'replacement runtime source\n'

case_name='paths retain kernel symlink semantics and final output entries'
mkdir -p -- "$caller/physical/deep" "$caller/logical" "$caller/relative-tmp"
ln -s -- ../physical/deep "$caller/logical/link"
printf 'kernel-selected IR\n' > "$caller/physical/input.lkjir"
printf 'wrong lexical IR\n' > "$caller/logical/input.lkjir"
invoke ir logical/link/../input.lkjir
expect_status 0
expect_text "$work/stdout" $'kernel-selected IR\n'
printf '// kernel-selected source\n' > "$caller/physical/input.rs"
printf '// wrong lexical source\n' > "$caller/logical/input.rs"
invoke check input.rs --source-dir logical/link/.. --no-library
expect_status 0
expect_text "$trace/frontend.input" $'// kernel-selected source\n'
printf 'kernel-selected runtime\n' > "$caller/physical/runtime.lkjir"
printf 'wrong lexical runtime\n' > "$caller/logical/runtime.lkjir"
invoke build direct.lkjir --runtime logical/link/../runtime.lkjir -o outputs/path-test
expect_status 0
expect_text "$trace/backend.runtime" $'kernel-selected runtime\n'
printf 'keep symlink referent\n' > "$caller/outputs/referent"
ln -s -- referent "$caller/outputs/final-link"
invoke ir direct.lkjir -o outputs/final-link
expect_status 0
[[ ! -L $caller/outputs/final-link ]] || failure 'final output symlink was followed'
expect_text "$caller/outputs/final-link" $'fixture IR\n'
expect_text "$caller/outputs/referent" $'keep symlink referent\n'
TMPDIR=relative-tmp invoke runtime-ir -o outputs/relative-runtime.lkjir
expect_status 0
expect_text "$caller/outputs/relative-runtime.lkjir" $'runtime:// selected runtime source\n'
[[ -z $(find "$caller/relative-tmp" -mindepth 1 -print -quit) ]] || failure 'relative temporary directory leaked'

case_name='async launch preserves a closed stdin'
export LKJSCRIPT_CLI_TEST_TARGET_MODE=closed-stdin
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
unset LKJSCRIPT_CLI_TEST_TARGET_MODE

case_name='stdout publication cannot silently reopen a closed stream'
for command in ir pack runtime-ir; do
    arguments=("$command")
    if [[ $command != runtime-ir ]]; then arguments+=(project/input.rs); fi
    status=0
    (
        cd -- "$caller"
        PATH=$path bash "$wrapper" --toolchain "$toolchain" "${arguments[@]}" 1>&-
    ) >"$work/stdout" 2>"$work/stderr" || status=$?
    expect_status 1
    expect_empty "$work/stdout"
    expect_text "$work/stderr" $'lkjscript: standard output is closed\n'
done
for command in check build; do
    arguments=("$command" project/input.rs)
    if [[ $command == build ]]; then arguments+=(-o outputs/closed-stream-build); fi
    status=0
    (
        cd -- "$caller"
        PATH=$path bash "$wrapper" --toolchain "$toolchain" "${arguments[@]}" 0<&- 1>&- 2>&-
    ) >"$work/stdout" 2>"$work/stderr" || status=$?
    expect_status 0
done

case_name='TERM and INT cancel only the owned child and clean staging'
cancel_case() {
    local stage=$1 signal=$2 expected=$3 wrapper_pid worker_pid timer_pid finished attempt
    local arguments=(run project/input.rs)
    if [[ $stage == result ]]; then arguments=(eval project/input.rs); fi
    rm -f -- "$trace/"*
    (
        cd -- "$caller"
        exec /usr/bin/env --default-signal=INT,QUIT PATH="$path" LKJSCRIPT_CLI_TEST_LONG_STAGE="$stage" \
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
for stage in frontend backend target result launcher; do
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
    PATH=$path bash "$toolchain/bin/lkjscript" toolchain
) > "$work/stdout" 2> "$work/stderr"
[[ $(<"$work/stdout") == *'source_commit cli-fixture'* ]] || failure 'installed command did not select its own toolchain'
expect_no_work
printf 'native_cli: all workflow contract tests passed\n'
