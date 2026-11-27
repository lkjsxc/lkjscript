#!/usr/bin/env bash
# Private, one-shot handoff between a seed-only bootstrap and its local consumer.
# Run from the repository root. This is not a persistent or cross-job cache.
set -euo pipefail
refuse() { printf 'FAIL bootstrap handoff: %s\n' "$*" >&2; exit 1; }

inputs() {
    local key=$1 unexpected
    # Include the actual tree (including untracked files), not just Git's index.
    # Refuse aliases/special files rather than hash a different input closure.
    local roots=(src library seed tools/bootstrap.sh tools/bootstrap-handoff.sh tools/store-key.sh)
    unexpected=$(find "${roots[@]}" ! -type d ! -type f -print -quit) || refuse 'cannot enumerate bootstrap inputs'
    if [[ -n $unexpected ]]; then
        refuse 'non-regular bootstrap input'
    fi
    {
        find "${roots[@]}" -print0 | LC_ALL=C sort -z &&
        find "${roots[@]}" -type f -print0 | LC_ALL=C sort -z | xargs -0 -r sha256sum -z -- &&
        printf 'store=%s\0key=%s\0' "$2" "$key" &&
        if [[ -n $key ]]; then sha256sum -z -- "$key"; fi
    } | sha256sum | cut -d' ' -f1
}

# Bind the receipt to a live ancestor, not merely a path left by an earlier run.
# /proc/stat field 22 is the process start tick, field 4 its parent.
proc_fields() {
    local record
    read -r record 2>/dev/null <"/proc/$1/stat" || return 1
    read -r -a fields <<<"${record##*) }"
    [[ ${#fields[@]} -ge 20 ]]
}
owner() {
    local -a fields
    proc_fields "$1" || refuse 'bootstrap process unavailable'
    printf '%s %s %s\n' "$1" "${fields[19]}" "$(cat /proc/sys/kernel/random/boot_id)"
}
live_owner() {
    local claim=$1 pid=$PPID boot_id
    local -a fields
    boot_id=$(cat /proc/sys/kernel/random/boot_id) || refuse "cannot read boot identity"
    while ((pid > 1)); do
        proc_fields "$pid" || break
        if [[ $claim == "$pid ${fields[19]} $boot_id" ]]; then
            return 0
        fi
        pid=${fields[1]}
    done
    refuse 'not a live same-run bootstrap ancestor'
}

case "${1:-}" in
    inputs)
        [[ $# == 3 ]] || refuse 'expected key and store'
        inputs "$2" "$3"
        ;;
    owner)
        [[ $# == 2 && $2 =~ ^[0-9]+$ ]] || refuse 'expected bootstrap process ID'
        owner "$2"
        ;;
    verify)
        [[ $# == 4 ]] || refuse 'expected private handoff, key and store'
        receipt=$2
        [[ $receipt =~ ^target/bootstrap\.[[:alnum:]]{6}/handoff$ &&
           -f $receipt && ! -L $receipt && ! -L ${receipt%/*} ]] || refuse 'missing private receipt'
        [[ -f target/lkjscript && ! -L target/lkjscript && -x target/lkjscript ]] || refuse 'missing or invalid compiler'
        claim=$(sed -n '2p' "$receipt") || refuse 'cannot read receipt'
        live_owner "$claim"
        actual_inputs=$(inputs "$3" "$4") || refuse 'cannot hash bootstrap inputs'
        actual_compiler=$(sha256sum target/lkjscript | cut -d' ' -f1) || refuse 'cannot hash compiler'
        if ! printf 'lkjscript-bootstrap-handoff-v2\n%s\n%s\n%s\n' "$claim" "$actual_inputs" "$actual_compiler" | cmp -s - "$receipt"; then
            refuse 'changed inputs or compiler identity'
        fi
        rm -- "$receipt" || refuse 'cannot consume receipt'
        printf 'ok bootstrap handoff (seed fixed point, compiler sha256=%s)\n' "$actual_compiler"
        ;;
    *) refuse 'expected inputs, owner or verify' ;;
esac
