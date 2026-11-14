# Shared supervision for the native shell entry points.
# Long noninteractive stages have their own tracked process group. Application
# targets keep the caller's terminal group. Each child is waited for explicitly.

lkjscript_process_init() {
    local dependency
    for dependency in bash setsid; do
        command -v "$dependency" >/dev/null || {
            printf 'lkjscript: required command is missing: %s\n' "$dependency" >&2
            return 1
        }
    done
    set +m
    lkjscript_child_pid=
    lkjscript_child_group=0
    lkjscript_child_launching=0
    lkjscript_cancel_signal=
    lkjscript_cancel_status=0
    lkjscript_child_ready=$1/.lkjscript-child-ready
    trap 'lkjscript_process_cancel INT 130' INT
    trap 'lkjscript_process_cancel TERM 143' TERM
}

lkjscript_process_cancel() {
    if [[ -z $lkjscript_cancel_signal ]]; then
        lkjscript_cancel_signal=$1
        lkjscript_cancel_status=$2
    fi
    # A trap can run between starting the async command and recording $!.
    (( !lkjscript_child_launching )) || return 0
    trap '' INT TERM
    if [[ -n $lkjscript_child_pid ]]; then
        if [[ ! -e $lkjscript_child_ready ]]; then
            # Before the ready marker, Bash may still have INT ignored. TERM
            # stops that launcher; a final group signal covers a concurrent exec.
            kill -TERM "$lkjscript_child_pid" 2>/dev/null || :
        elif ((lkjscript_child_group)); then
            kill -s "$lkjscript_cancel_signal" -- "-$lkjscript_child_pid" 2>/dev/null || :
        else
            kill -s "$lkjscript_cancel_signal" "$lkjscript_child_pid" 2>/dev/null || :
        fi
        wait "$lkjscript_child_pid" 2>/dev/null || :
        if ((lkjscript_child_group)); then
            # GNU time can exit before the command it was measuring. The group
            # is ours even after its original leader has been reaped.
            kill -s "$lkjscript_cancel_signal" -- "-$lkjscript_child_pid" 2>/dev/null || :
        fi
        lkjscript_child_pid=
    fi
    exit "$lkjscript_cancel_status"
}

lkjscript_process_exec() {
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
    ' lkjscript-child "$ready" "$@"
}

# Arguments: working directory (empty = inherit), own group (0/1), command...
lkjscript_process_run() {
    local directory=$1 group=$2 input_fd= status=0
    shift 2
    rm -f -- "$lkjscript_child_ready"
    lkjscript_child_group=$group
    lkjscript_child_launching=1
    # Duplication onto another descriptor distinguishes open and closed stdin;
    # the child closes the saved descriptor after restoring fd0.
    if { exec {input_fd}<&0; } 2>/dev/null; then
        lkjscript_process_exec "$directory" "$group" "$input_fd" "$lkjscript_child_ready" "$@" <&"$input_fd" &
    else
        lkjscript_process_exec "$directory" "$group" "" "$lkjscript_child_ready" "$@" 0<&- &
    fi
    lkjscript_child_pid=$!
    [[ -z $input_fd ]] || exec {input_fd}<&-
    lkjscript_child_launching=0
    if [[ -n $lkjscript_cancel_signal ]]; then
        lkjscript_process_cancel "$lkjscript_cancel_signal" "$lkjscript_cancel_status"
    fi
    wait "$lkjscript_child_pid" || status=$?
    lkjscript_child_pid=
    rm -f -- "$lkjscript_child_ready"
    return "$status"
}
