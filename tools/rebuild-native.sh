#!/bin/bash
# Build one native toolchain generation with an explicitly installed seed.
set -euo pipefail
export LC_ALL=C

usage() {
    cat <<'HELP'
Usage: rebuild-native.sh DEST --seed TOOLCHAIN --source-dir CHECKOUT [--source-commit ID]

DEST must not exist. The installed seed prepares CHECKOUT/library/runtime.rs,
then builds the frontend, K compiler and result reader from CHECKOUT. The
checkout's installer validates and publishes their composition. One invocation
builds one generation; the caller keeps the source checkout consistent.
Commands, K SHA-256 hashes and optional GNU time/RSS measurements go to stderr.
All process controls are inherited. No Cargo, Node or Git command is invoked.
HELP
}
fail() { printf 'rebuild-native: %s\n' "$*" >&2; exit 1; }

if [[ $# == 1 && $1 == --help ]]; then usage; exit 0; fi
(($#)) || { usage >&2; exit 1; }
destination=$1
shift
[[ -n $destination ]] || fail 'DEST must be nonempty'
seed=
source_dir=
source_commit=unspecified
seen=' '
while (($#)); do
    option=$1
    case $option in
        --seed|--source-dir|--source-commit) ;;
        *) fail "unknown option: $option" ;;
    esac
    [[ $seen != *" $option "* ]] || fail "repeated option: $option"
    [[ $# -ge 2 && -n $2 ]] || fail "$option needs a value"
    seen+="$option "
    case $option in
        --seed) seed=$2 ;;
        --source-dir) source_dir=$2 ;;
        --source-commit) source_commit=$2 ;;
    esac
    shift 2
done
[[ -n $seed ]] || fail '--seed is required'
[[ -n $source_dir ]] || fail '--source-dir is required'
[[ ! -e $destination && ! -L $destination ]] || fail "DEST already exists: $destination"
[[ -d $seed ]] || fail "seed toolchain is not a directory: $seed"
[[ -d $source_dir ]] || fail "source checkout is not a directory: $source_dir"
seed=$(cd -- "$seed" && pwd -P)
source_dir=$(cd -- "$source_dir" && pwd -P)
[[ $destination == /* ]] || destination=$PWD/$destination
lkj=$seed/bin/lkj
installer=$source_dir/tools/install-native.sh
[[ -f $lkj && -x $lkj ]] || fail "seed command is not executable: $lkj"
[[ -f $installer && -r $installer ]] || fail "installer is not readable: $installer"
[[ -f $source_dir/library/runtime.rs && -r $source_dir/library/runtime.rs ]] ||
    fail "runtime source is not readable: $source_dir/library/runtime.rs"

work=
cleanup() {
    local status=$?
    trap - EXIT
    [[ -z $work ]] || rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
work=$(mktemp -d "${TMPDIR:-/tmp}/lkj-rebuild.XXXXXXXXXX")
work=$(cd -- "$work" && pwd -P)
chmod 700 -- "$work"
script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
source "$script_directory/native-process.sh"
lkj_process_init "$work"
run() {
    local status=0
    printf 'rebuild-native:' >&2
    printf ' %q' "$@" >&2
    printf '\n' >&2
    if [[ -x /usr/bin/time ]]; then
        lkj_process_run "" 1 /usr/bin/time -f 'rebuild-native: elapsed_seconds=%e max_rss_kib=%M' -- "$@" || status=$?
    else
        lkj_process_run "" 1 "$@" || status=$?
    fi
    printf 'rebuild-native: exit_status=%s\n' "$status" >&2
    return "$status"
}
log_k_hash() {
    local digest
    digest=$(sha256sum < "$2")
    printf 'rebuild-native: k_sha256 %s %s\n' "$1" "${digest%% *}" >&2
}
run "$lkj" --toolchain "$seed" runtime-k \
    --runtime-source "$source_dir/library/runtime.rs" -o "$work/runtime.k"
log_k_hash runtime "$work/runtime.k"
for component in frontend kc result; do
    run "$lkj" --toolchain "$seed" k "src/${component}_main.rs" \
        --source-dir "$source_dir" --library-dir "$source_dir/library" \
        -o "$work/$component.k"
    log_k_hash "$component" "$work/$component.k"
    run "$lkj" --toolchain "$seed" build "$work/$component.k" \
        --runtime "$work/runtime.k" --entry main -o "$work/$component"
done
run bash "$installer" "$destination" \
    --frontend "$work/frontend" --kc "$work/kc" --result "$work/result" \
    --library "$source_dir/library" --runtime-source "$source_dir/library/runtime.rs" \
    --source-commit "$source_commit"
