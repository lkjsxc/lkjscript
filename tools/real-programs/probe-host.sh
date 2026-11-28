#!/usr/bin/env bash
# Linux host boundary: path resolution for grants, the adjacent checked Git verifier,
# the explicitly chosen GNU-time launcher, and hardlink publication. The checked
# program drives the transaction through a NUL-framed, fixed-verb protocol.
# Source this file; the public launcher never accepts a substitute verifier.

build_probe() (
    set -euo pipefail
    local compiler=$1 source=$2 library=$3 work=$4
    "$compiler" build "$source" --entry select --library-dir "$library" --no-store \
        --grant arguments --grant output --grant errors --grant status -o "$work/select"
    "$compiler" build "$source" --library-dir "$library" --no-store \
        --grant arguments --grant input --grant output --grant errors --grant status \
        --grant scratch=env:LKJSCRIPT_PROBE_SCRATCH --grant clock \
        --grant compiler_dir=env:LKJSCRIPT_PROBE_COMPILER_DIR \
        --grant library_dir=env:LKJSCRIPT_PROBE_LIBRARY_DIR \
        --grant projection_dir=env:LKJSCRIPT_PROBE_PROJECTION_DIR \
        --grant time_dir=env:LKJSCRIPT_PROBE_TIME_DIR -o "$work/probe"
)

run_probe() (
    set -euo pipefail
    local select=$1 main=$2 verifier=$3
    shift 3
    local scratch temporary='' child='' request_fd reply_fd
    scratch=$(mktemp -d)
    cleanup() {
        local rc=$?
        trap - EXIT
        if [[ -n $child ]]; then kill "$child" 2>/dev/null || :; wait "$child" 2>/dev/null || :; fi
        [[ -z $temporary ]] || rm -rf -- "$temporary"
        rm -rf -- "$scratch"
        exit "$rc"
    }
    trap cleanup EXIT
    trap 'exit 130' INT
    trap 'exit 143' HUP TERM
    "$select" "$@" > "$scratch/options" || exit $?
    local -a fields paths
    mapfile -d '' -t fields < "$scratch/options"
    if [[ ${fields[0]} == help ]]; then printf '%s' "${fields[1]}"; exit 0; fi
    [[ ${fields[0]} == observe && ${#fields[@]} == 7 ]] || exit 1
    local i path
    for i in 1 2 3 4 5; do
        path=''
        IFS= read -r -d '' path < <(realpath -ez -- "${fields[i]}" 2>/dev/null) || path=''
        paths+=("$path")
    done
    local compiler=${paths[0]} library=${paths[1]} source=${paths[2]} projection=${paths[3]} time=${paths[4]} output=${fields[6]}
    local compiler_ok=no time_ok=no exists=no verifier_ok=no leaf=''
    [[ ! -f $compiler || ! -x $compiler ]] || compiler_ok=yes
    [[ ! -f $time || ! -x $time ]] || time_ok=yes
    [[ ! -e $output && ! -L $output ]] || exists=yes
    [[ ! -e $verifier ]] || verifier_ok=yes
    local compiler_dir=$scratch time_dir=$scratch library_dir=$scratch projection_dir=$scratch output_parent
    local p
    if [[ -n $compiler ]]; then p=${compiler%/*}; [[ ! -d ${p:-/} ]] || compiler_dir=${p:-/}; fi
    if [[ -n $time ]]; then p=${time%/*}; [[ ! -d ${p:-/} ]] || time_dir=${p:-/}; fi
    if [[ -d $library ]]; then library_dir=$library
    elif [[ -n $library ]]; then p=${library%/*}; [[ ! -d ${p:-/} ]] || { library_dir=${p:-/}; leaf=${library##*/}; }; fi
    [[ ! -d $projection ]] || projection_dir=$projection
    IFS= read -r -d '' output_parent < <(dirname -z -- "$output")
    mkfifo "$scratch/requests" "$scratch/replies"
    LKJSCRIPT_PROBE_SCRATCH=$scratch LKJSCRIPT_PROBE_COMPILER_DIR=$compiler_dir \
    LKJSCRIPT_PROBE_LIBRARY_DIR=$library_dir LKJSCRIPT_PROBE_PROJECTION_DIR=$projection_dir \
    LKJSCRIPT_PROBE_TIME_DIR=$time_dir \
        "$main" "${paths[@]}" "$compiler_ok" "$time_ok" "$exists" "$verifier_ok" "$leaf" "$@" \
        < "$scratch/replies" > "$scratch/requests" &
    child=$!
    exec {reply_fd}>"$scratch/replies"
    exec {request_fd}<"$scratch/requests"
    reply() { printf '%s\0' "$1" >&"$reply_fd"; }
    receive() { IFS= read -r -d '' "$1" <&"$request_fd"; }
    close_protocol() { exec {reply_fd}>&- {request_fd}<&-; }
    local verb rc index route selected root rss pending summary signal
    # The checked Child.wait contract conflates exit 128+N with signal N and
    # Process starts with an empty environment. Supervise the two explicitly
    # selected host commands here, preserving their ambient caller environment.
    supervise() {
        local stdout=$1 stderr=$2; shift 2
        local child_code='
            scratch=$1 had_lc=$2 old_lc=$3; shift 3
            if [[ $had_lc == x ]]; then export LC_ALL=$old_lc; else unset LC_ALL; fi
            shopt -s execfail
            exec "$@"
            printf "%s\n" "$?" > "$scratch/exec-failed"
            exit 127
        '
        rm -f -- "$scratch/exec-failed"
        (
            close_protocol
            # A fresh job table is necessary: nested Bash subshells can
            # suppress completion notifications even after set -m.
            exec bash -c '
                set -m
                stdout=$1 stderr=$2 scratch=$3 child_code=$4; shift 4
                old_lc=${LC_ALL-} had_lc=${LC_ALL+x}
                export LC_ALL=C
                bash -c "$child_code" probe-child "$scratch" "$had_lc" "$old_lc" "$@" </dev/null > "$stdout" 2> "$stderr" &
                wait "$!"
                printf "%s\n" "$?" > "$scratch/exit"
            ' probe-supervisor "$stdout" "$stderr" "$scratch" "$child_code" "$@"
        ) 2> "$scratch/wait"
        rc=$(<"$scratch/exit")
        signal=0
        if ((rc > 128 && rc <= 192)) && ! grep -Eq "Exit +$rc( |$)" "$scratch/wait"; then
            signal=$((rc-128)); rc=0
        fi
    }
    while receive verb; do
        case "$verb" in
            verify)
                rc=0
                supervise "$scratch/stdout" "$scratch/stderr" \
                    "$verifier" --verify "$source" "$projection"
                if ((signal)); then reply null; else reply "$rc"; fi
                ;;
            load)
                cat /proc/loadavg > "$scratch/loadavg"; reply ok ;;
            begin)
                if temporary=$(mktemp -d -- "$output_parent/root-probe-XXXXXX" 2>/dev/null); then
                    reply "${temporary##*/}"
                else
                    reply "error:ENOENT: no such file or directory, mkdtemp '$output_parent/root-probe-XXXXXX'"
                fi
                ;;
            observe)
                receive index; receive route; receive selected
                [[ -n $temporary && $index =~ ^[0-9]+$ ]] || exit 1
                case "$route" in
                    original_suffix) root=$source ;;
                    suffix_projection) root=$projection ;;
                    *) exit 1 ;;
                esac
                rss="$temporary/$index.rss"
                supervise "$temporary/$index.stdout" "$temporary/$index.stderr" \
                    "$time" -f %M -o "$rss" "$compiler" check "$selected" --source-dir "$root" --library-dir "$library"
                if [[ -f $scratch/exec-failed ]]; then
                    if [[ $(<"$scratch/exec-failed") == 127 ]]; then
                        reply "error:exec $time: spawnSync $time ENOENT"
                    else
                        reply "error:exec $time: spawnSync $time EACCES"
                    fi
                    continue
                fi
                if [[ ! -f $rss ]]; then
                    reply "error:ENOENT: no such file or directory, open '$rss'"
                    continue
                fi
                reply "$rc $signal"
                ;;
            collect)
                receive index
                [[ -n $temporary && $index =~ ^[0-9]+$ ]] || exit 1
                for stream in stdout stderr rss; do cp -- "$temporary/$index.$stream" "$scratch/$stream"; done
                reply ok ;;
            publish)
                [[ -n $temporary ]] || exit 1
                pending="$temporary/.root-probe-receipt"
                (umask 077; set -o noclobber; cat "$scratch/receipt" > "$pending")
                if ln -T -- "$pending" "$output" 2>/dev/null; then
                    rm -- "$pending"; reply ok
                elif [[ -e $output || -L $output ]]; then
                    reply "error:publish new receipt: EEXIST: file already exists, link '$pending' -> '$output'"
                else
                    reply "error:publish new receipt: unable to link '$pending' -> '$output'"
                fi
                ;;
            result)
                receive summary; printf '%s' "$summary"; reply ok ;;
            *) exit 1 ;;
        esac
    done
    close_protocol
    rc=0; wait "$child" || rc=$?; child=''
    exit "$rc"
)
