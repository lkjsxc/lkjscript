#!/bin/bash
# Install an explicit native toolchain. Resource controls are inherited.
set -euo pipefail
export LC_ALL=C

usage() {
    printf '%s\n' \
        'usage: install-native.sh DEST --frontend PATH --backend PATH --result PATH' \
        '    --library DIR --runtime-source FILE [--source-commit ID]' \
        'DEST must not exist. All compiler and source inputs are explicit.' \
        'The supplied native tools regenerate and validate runtime.lkjir before publication.' \
        'Requires Bash, coreutils, findutils and util-linux; no Rust or Node tooling is used.'
}

fail() {
    printf 'install-native: %s\n' "$*" >&2
    exit 1
}

if [[ $# == 1 && $1 == --help ]]; then
    usage
    exit 0
fi
[[ $# -gt 0 ]] || { usage >&2; exit 1; }
destination=$1
shift
[[ -n $destination ]] || fail 'DEST must be nonempty'
frontend=
backend=
result=
library=
runtime_source=
source_commit=unspecified
seen=' '
while [[ $# -gt 0 ]]; do
    option=$1
    case "$option" in
        --frontend|--backend|--result|--library|--runtime-source|--source-commit) ;;
        *) fail "unknown option: $option" ;;
    esac
    [[ $seen != *" $option "* ]] || fail "repeated option: $option"
    [[ $# -ge 2 && -n $2 ]] || fail "$option needs a value"
    seen+="$option "
    case "$option" in
        --frontend) frontend=$2 ;;
        --backend) backend=$2 ;;
        --result) result=$2 ;;
        --library) library=$2 ;;
        --runtime-source) runtime_source=$2 ;;
        --source-commit) source_commit=$2 ;;
    esac
    shift 2
done
[[ -n $frontend ]] || fail '--frontend is required'
[[ -n $backend ]] || fail '--backend is required'
[[ -n $result ]] || fail '--result is required'
[[ -n $library ]] || fail '--library is required'
[[ -n $runtime_source ]] || fail '--runtime-source is required'
for dependency in basename chmod cmp cp dirname find mkdir mktemp mv rm sha256sum sort; do
    command -v "$dependency" >/dev/null || fail "required command is missing: $dependency"
done

absolute_path() {
    if [[ $1 == /* ]]; then
        printf '%s\n' "$1"
    else
        printf '%s/%s\n' "$PWD" "$1"
    fi
}

regular_file() {
    [[ ! -L $2 ]] || fail "$1 must not be a symlink: $2"
    [[ -f $2 && -r $2 ]] || fail "$1 must be a readable regular file: $2"
}

# Copy this checkout's driver, never discover a compiler or source implicitly.
script_directory=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
driver=$script_directory/lkjscript
process_helper=$script_directory/native-process.sh
frontend=$(absolute_path "$frontend")
backend=$(absolute_path "$backend")
result=$(absolute_path "$result")
runtime_source=$(absolute_path "$runtime_source")
library=$(absolute_path "$library")
regular_file frontend "$frontend"
regular_file backend "$backend"
regular_file result "$result"
regular_file driver "$driver"
regular_file process-helper "$process_helper"
source "$process_helper"
regular_file runtime-source "$runtime_source"
[[ ! -L $library ]] || fail "library directory must not be a symlink: $library"
[[ -d $library ]] || fail "library must be a directory: $library"
library=$(cd -- "$library" && pwd -P)
bad=$(find "$library" ! -type d ! -type f -print -quit) ||
    fail "cannot inspect library directory: $library"
[[ -z $bad ]] || fail "library contains a symlink or special file: $bad"
regular_file library/lib.rs "$library/lib.rs"

[[ ! -e $destination && ! -L $destination ]] || fail "DEST already exists: $destination"
destination_name=$(basename -- "$destination")
[[ $destination_name != . && $destination_name != .. && $destination_name != / ]] ||
    fail "DEST must name a new directory: $destination"
destination_parent=$(dirname -- "$destination")
mkdir -p -- "$destination_parent"
destination_parent=$(cd -- "$destination_parent" && pwd -P)
destination=$destination_parent/$destination_name
[[ ! -e $destination && ! -L $destination ]] || fail "DEST already exists: $destination"
case "$destination_parent/" in
    "$library/"*) fail 'DEST parent must be outside the library input tree' ;;
esac

stage=
cleanup() {
    if [[ -n $stage && -d $stage ]]; then
        rm -rf -- "$stage"
    fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
stage=$(mktemp -d -- "$destination_parent/.lkjscript-install.XXXXXXXX")
lkjscript_process_init "$stage"
mkdir -- "$stage/bin" "$stage/libexec" "$stage/library" "$stage/.work"
cp --no-dereference -- "$driver" "$stage/bin/lkjscript"
cp --no-dereference -- "$frontend" "$stage/libexec/frontend"
cp --no-dereference -- "$backend" "$stage/libexec/backend"
cp --no-dereference -- "$result" "$stage/libexec/result"
cp --no-dereference -- "$process_helper" "$stage/libexec/native-process.sh"
chmod 644 -- "$stage/libexec/native-process.sh"
cp --no-dereference -- "$runtime_source" "$stage/runtime.rs"
cp -R --no-dereference -- "$library/." "$stage/library/"
# Audit copied nodes before chmod or executing anything, including input races.
bad=$(find "$stage" ! -type d ! -type f -print -quit) ||
    fail 'cannot inspect staged files'
[[ -z $bad ]] || fail "copied input contains a symlink or special file: $bad"
chmod 755 -- "$stage/bin/lkjscript" "$stage/libexec/frontend" "$stage/libexec/backend" "$stage/libexec/result"
work=$stage/.work
mkdir -- "$work/"'$runtime'
cp -- "$stage/runtime.rs" "$work/"'$runtime/runtime.rs'
lkjscript_process_run "$work" 1 "$stage/libexec/frontend" --source '$runtime/runtime.rs' --no-library > "$work/runtime.lkjir" ||
    fail 'native frontend rejected the runtime source'
[[ -s $work/runtime.lkjir ]] || fail 'native frontend emitted an empty runtime'
printf 'pub fn probe() -> i64 { 42 }\n' > "$work/probe.rs"
lkjscript_process_run "$work" 1 "$stage/libexec/frontend" --source probe.rs --no-library > "$work/probe.lkjir" ||
    fail 'native frontend rejected the installation probe'
lkjscript_process_run "$work" 1 "$stage/libexec/backend" probe.lkjir --runtime runtime.lkjir --entry probe > "$work/probe.elf" ||
    fail 'native backend rejected the regenerated runtime or probe'
chmod 755 "$work/probe.elf"
lkjscript_process_run "$work" 1 "$work/probe.elf" >/dev/null ||
    fail 'native installation probe failed'
lkjscript_process_run "$work" 1 "$stage/libexec/backend" probe.lkjir --runtime runtime.lkjir --entry probe \
    --result-descriptor probe.result > "$work/packet.elf" ||
    fail 'native backend could not emit an evaluation artifact'
chmod 755 "$work/packet.elf"
status=0
lkjscript_process_run "$work" 1 "$work/packet.elf" 3>"$work/packet.raw" >/dev/null || status=$?
printf '\n%s\n' "$status" >> "$work/packet.raw"
lkjscript_process_run "$work" 1 "$stage/libexec/result" probe.result installation-probe \
    <"$work/packet.raw" >"$work/result.stdout" ||
    fail 'native evaluation artifact and result reader disagree'
[[ $status == 0 ]] || fail 'native evaluation artifact failed'
printf '42\n' > "$work/expected.stdout"
cmp -s "$work/expected.stdout" "$work/result.stdout" ||
    fail 'native result reader did not return exactly 42 and a newline'
cp -- "$work/runtime.lkjir" "$stage/runtime.lkjir"
rm -rf -- "$work"

hash_file() {
    local digest
    digest=$(sha256sum < "$stage/$1") || return 1
    printf '%s' "${digest%% *}"
}

write_hashes() {
    # GNU sha256sum escapes unusual filenames; its checker reads that framing.
    find "$1" -type f ! -path ./SHA256SUMS -print0 |
        sort -z |
        while IFS= read -r -d '' path; do
            sha256sum -- "$path" || exit 1
        done
}

record_input() {
    local digest
    digest=$(hash_file "$3") || return 1
    printf 'input %s %q %s\n' "$1" "$2" "$digest"
}

library_hash=$(cd -- "$stage" && write_hashes ./library | sha256sum)
runtime_hash=$(hash_file runtime.lkjir)
{
    printf 'format lkjscript-native-v1\nsource_commit %q\n' "$source_commit"
    record_input frontend "$frontend" libexec/frontend
    record_input backend "$backend" libexec/backend
    record_input result "$result" libexec/result
    record_input driver "$driver" bin/lkjscript
    record_input process_helper "$process_helper" libexec/native-process.sh
    record_input runtime_source "$runtime_source" runtime.rs
    printf 'input library %q\n' "$library"
    printf 'library_tree_sha256 %s\n' "${library_hash%% *}"
    printf 'generated runtime_ir %s\n' "$runtime_hash"
    printf 'validation runtime_seal plain_execution scalar_result passed\n'
} > "$stage/manifest"
(
    cd -- "$stage"
    write_hashes . > SHA256SUMS
    sha256sum --check --status SHA256SUMS ||
        fail 'staged file hashes failed verification'
)
chmod 755 -- "$stage"
# Sibling staging permits one rename; --no-clobber also handles a racing creator.
mv --no-clobber -T -- "$stage" "$destination"
[[ ! -d $stage ]] || fail "DEST appeared before publication: $destination"
stage=
printf 'Installed native toolchain: %s\n' "$destination"
