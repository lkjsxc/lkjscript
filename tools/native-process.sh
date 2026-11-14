# Shared supervision for the native shell entry points.
# Long noninteractive stages have their own tracked process group. Application
# targets keep the caller's terminal group. Each child is waited for explicitly.

lkj_process_init() {
    local dependency
    for dependency in bash setsid; do
        command -v "$dependency" >/dev/null || {
            printf 'lkj: required command is missing: %s\n' "$dependency" >&2
            return 1
        }
    done
    set +m
    lkj_child_pid=
    lkj_child_group=0
    lkj_child_launching=0
    lkj_cancel_signal=
    lkj_cancel_status=0
    lkj_child_ready=$1/.lkj-child-ready
    trap 'lkj_process_cancel INT 130' INT
    trap 'lkj_process_cancel TERM 143' TERM
}

lkj_process_cancel() {
    if [[ -z $lkj_cancel_signal ]]; then
        lkj_cancel_signal=$1
        lkj_cancel_status=$2
    fi
    # A trap can run between starting the async command and recording $!.
    (( !lkj_child_launching )) || return 0
    trap '' INT TERM
    if [[ -n $lkj_child_pid ]]; then
        if [[ ! -e $lkj_child_ready ]]; then
            # Before the ready marker, Bash may still have INT ignored. TERM
            # stops that launcher; a final group signal covers a concurrent exec.
            kill -TERM "$lkj_child_pid" 2>/dev/null || :
        elif ((lkj_child_group)); then
            kill -s "$lkj_cancel_signal" -- "-$lkj_child_pid" 2>/dev/null || :
        else
            kill -s "$lkj_cancel_signal" "$lkj_child_pid" 2>/dev/null || :
        fi
        wait "$lkj_child_pid" 2>/dev/null || :
        if ((lkj_child_group)); then
            # GNU time can exit before the command it was measuring. The group
            # is ours even after its original leader has been reaped.
            kill -s "$lkj_cancel_signal" -- "-$lkj_child_pid" 2>/dev/null || :
        fi
        lkj_child_pid=
    fi
    exit "$lkj_cancel_status"
}

lkj_process_exec() {
    local directory=$1 group=$2 input_fd=$3 ready=$4
    shift 4
    # Reset Bash's async INT/QUIT disposition before starting another process.
    # An external env launcher can reopen closed standard descriptors (uutils).
    trap - INT QUIT
    [[ -z $input_fd ]] || exec {input_fd}<&-
    if [[ -n $directory ]]; then cd -- "$directory" || exit; fi
    local prefix=()
    if ((group)); then prefix=(setsid); fi
    # The marker follows group creation and signal restoration. The shell then
    # execs the actual command, retaining the PID tracked by its parent.
    exec "${prefix[@]}" /bin/bash -c '
        ready=$1
        shift
        : > "$ready" || exit
        exec "$@"
    ' lkj-child "$ready" "$@"
}

# Arguments: working directory (empty = inherit), own group (0/1), command...
lkj_process_run() {
    local directory=$1 group=$2 input_fd= status=0
    shift 2
    rm -f -- "$lkj_child_ready"
    lkj_child_group=$group
    lkj_child_launching=1
    # Duplication onto another descriptor distinguishes open and closed stdin;
    # the child closes the saved descriptor after restoring fd0.
    if { exec {input_fd}<&0; } 2>/dev/null; then
        lkj_process_exec "$directory" "$group" "$input_fd" "$lkj_child_ready" "$@" <&"$input_fd" &
    else
        lkj_process_exec "$directory" "$group" "" "$lkj_child_ready" "$@" 0<&- &
    fi
    lkj_child_pid=$!
    [[ -z $input_fd ]] || exec {input_fd}<&-
    lkj_child_launching=0
    if [[ -n $lkj_cancel_signal ]]; then
        lkj_process_cancel "$lkj_cancel_signal" "$lkj_cancel_status"
    fi
    wait "$lkj_child_pid" || status=$?
    lkj_child_pid=
    rm -f -- "$lkj_child_ready"
    return "$status"
}
