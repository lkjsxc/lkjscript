#!/bin/bash
# Exercise installation transactions and the native artifact interface with stubs.
set -euo pipefail
repository=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
mkdir -p "$work/fixture/tools" "$work/inputs/library/nested" "$work/path"
export LKJSCRIPT_INSTALL_TEST_WAIT=$work/wait.sh
export LKJSCRIPT_INSTALL_TEST_FIFO=$work/wait.fifo
export LKJSCRIPT_INSTALL_TEST_PID=$work/worker.pid
mkfifo "$LKJSCRIPT_INSTALL_TEST_FIFO"
cat > "$LKJSCRIPT_INSTALL_TEST_WAIT" <<'WAIT'
installer_wait() {
    [[ ${LKJSCRIPT_INSTALL_TEST_LONG_STAGE:-} == "$1" ]] || return 0
    exec 9<>"$LKJSCRIPT_INSTALL_TEST_FIFO"
    trap 'exit 143' TERM
    trap 'exit 130' INT
    printf '%s\n' "$BASHPID" > "$LKJSCRIPT_INSTALL_TEST_PID"
    while :; do read -r -t 60 -u 9 || :; done
}
WAIT
cp "$repository/tools/install-native.sh" "$repository/tools/native-process.sh" "$work/fixture/tools/"
printf '#!/bin/bash\nexit 0\n' > "$work/fixture/tools/lkjscript"
printf '// declared runtime bytes\n' > "$work/inputs/runtime.rs"
printf '// declared library bytes\n' > "$work/inputs/library/lib.rs"
printf '// nested bytes\r\n' > "$work/inputs/library/nested/with spaces.rs"
# The tested process cannot find Cargo, rustc or Node through PATH.
for dependency in basename bash cat chmod cmp cp dirname find mkdir mktemp mv rm setsid sha256sum sort; do
    ln -s "$(command -v "$dependency")" "$work/path/$dependency"
done
export LKJSCRIPT_INSTALL_TEST_ADDRESS_SPACE
LKJSCRIPT_INSTALL_TEST_ADDRESS_SPACE=$(ulimit -v)

cat > "$work/inputs/frontend" <<'FRONTEND'
#!/bin/bash
set -euo pipefail
source "$LKJSCRIPT_INSTALL_TEST_WAIT"
installer_wait frontend
[[ $(ulimit -v) == "$LKJSCRIPT_INSTALL_TEST_ADDRESS_SPACE" ]]
[[ $# == 3 && $1 == --source && $3 == --no-library ]]
[[ ${LKJSCRIPT_INSTALL_TEST_FAILURE:-} != frontend ]] || exit 19
if [[ -n ${LKJSCRIPT_INSTALL_TEST_RACE_DEST:-} && ! -d $LKJSCRIPT_INSTALL_TEST_RACE_DEST ]]; then
    mkdir "$LKJSCRIPT_INSTALL_TEST_RACE_DEST"
    printf 'keep racing destination\n' > "$LKJSCRIPT_INSTALL_TEST_RACE_DEST/keep"
fi
case "$2" in
    '$runtime/runtime.rs')
        cmp -s '$runtime/runtime.rs' ../runtime.rs
        printf 'runtime IR\n'
        ;;
    probe.rs)
        [[ $(<probe.rs) == 'pub fn probe() -> i64 { 42 }' ]]
        printf 'probe IR\n'
        ;;
    *) exit 20 ;;
esac
FRONTEND
cat > "$work/inputs/backend" <<'BACKEND'
#!/bin/bash
set -euo pipefail
[[ ${LKJSCRIPT_INSTALL_TEST_FAILURE:-} != backend ]] || exit 21
[[ $# == 5 || $# == 7 ]]
[[ $1 == probe.lkjir && $2 == --runtime && $3 == runtime.lkjir && $4 == --entry && $5 == probe ]]
[[ $(<runtime.lkjir) == 'runtime IR' && $(<probe.lkjir) == 'probe IR' ]]
if [[ $# == 5 ]]; then
    printf '#!/bin/bash\nexit 0\n'
else
    [[ $6 == --result-descriptor && $7 == probe.result ]]
    printf 'descriptor\n' > probe.result
    cat <<'ELF'
#!/bin/bash
printf '\000\000\000\000\000\000\000\000\052\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000' >&3
[[ ${LKJSCRIPT_INSTALL_TEST_FAILURE:-} != packet ]] || exit 23
ELF
fi
BACKEND
cat > "$work/inputs/result" <<'RESULT'
#!/bin/bash
set -euo pipefail
source "$LKJSCRIPT_INSTALL_TEST_WAIT"
installer_wait result
[[ $# == 2 && $1 == probe.result && $2 == installation-probe ]]
[[ $(<probe.result) == descriptor ]]
cmp -s - <(printf '\000\000\000\000\000\000\000\000\052\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000\n0\n')
case "${LKJSCRIPT_INSTALL_TEST_FAILURE:-}" in
    result) exit 22 ;;
    wrong-result) printf '41\n' ;;
    missing-newline) printf '42' ;;
    *) printf '42\n' ;;
esac
RESULT
chmod 755 "$work/inputs/frontend" "$work/inputs/backend" "$work/inputs/result"
installer=$work/fixture/tools/install-native.sh
invoke() {
    PATH=$work/path bash "$installer" "$1" \
        --frontend "$work/inputs/frontend" --backend "$work/inputs/backend" \
        --result "$work/inputs/result" --library "$work/inputs/library" \
        --runtime-source "$work/inputs/runtime.rs" --source-commit fixture-commit
}
reject() {
    if invoke "$1" > "$work/output" 2> "$work/error"; then
        printf 'installation unexpectedly succeeded: %s\n' "$1" >&2
        exit 1
    fi
    [[ ! -e $1 ]]
    [[ -z $(find "$work" -maxdepth 1 -name '.lkjscript-install.*' -print -quit) ]]
}
destination=$work/toolchain\ with\ spaces
invoke "$destination" > "$work/output"
[[ -d $destination && ! -e $destination/.work ]]
[[ -x $destination/bin/lkjscript && -x $destination/libexec/frontend ]]
[[ -x $destination/libexec/backend && -x $destination/libexec/result ]]
[[ -z $(find "$destination" -type l -print -quit) ]]
[[ ! $destination/libexec/frontend -ef $work/inputs/frontend ]]
[[ ! $destination/library/lib.rs -ef $work/inputs/library/lib.rs ]]
[[ $(<"$destination/runtime.lkjir") == 'runtime IR' ]]
[[ $(<"$destination/manifest") == *'source_commit fixture-commit'* ]]
(cd "$destination" && sha256sum --check --status SHA256SUMS)
# Installed bytes remain valid after every declared input tree disappears.
mv "$work/inputs" "$work/inputs.saved"
(cd "$destination" && sha256sum --check --status SHA256SUMS)
mv "$work/inputs.saved" "$work/inputs"
if invoke "$destination" > "$work/output" 2> "$work/error"; then
    printf 'existing destination was accepted\n' >&2
    exit 1
fi
[[ $(<"$work/error") == *'DEST already exists'* ]]
(cd "$destination" && sha256sum --check --status SHA256SUMS)
printf 'corrupt\n' >> "$destination/library/lib.rs"
if (cd "$destination" && sha256sum --check --status SHA256SUMS); then
    printf 'corrupt installed source passed verification\n' >&2
    exit 1
fi

ln -s "$work/inputs/runtime.rs" "$work/inputs/library/external.rs"
reject "$work/symlink-rejected"
[[ $(<"$work/error") == *'symlink or special file'* ]]
rm "$work/inputs/library/external.rs"
mkfifo "$work/inputs/library/fifo"
reject "$work/fifo-rejected"
rm "$work/inputs/library/fifo"
for failure in frontend backend packet result wrong-result missing-newline; do
    export LKJSCRIPT_INSTALL_TEST_FAILURE=$failure
    reject "$work/failed-$failure"
done
unset LKJSCRIPT_INSTALL_TEST_FAILURE
export LKJSCRIPT_INSTALL_TEST_RACE_DEST=$work/racing-destination
if invoke "$LKJSCRIPT_INSTALL_TEST_RACE_DEST" > "$work/output" 2> "$work/error"; then
    printf 'racing destination was overwritten\n' >&2
    exit 1
fi
[[ $(<"$LKJSCRIPT_INSTALL_TEST_RACE_DEST/keep") == 'keep racing destination' ]]
[[ ! -e $LKJSCRIPT_INSTALL_TEST_RACE_DEST/bin ]]
[[ -z $(find "$work" -maxdepth 1 -name '.lkjscript-install.*' -print -quit) ]]
unset LKJSCRIPT_INSTALL_TEST_RACE_DEST
# A cancelled validation stage must not publish or retain staging/children.
for stage in frontend result; do
    for signal in TERM INT; do
        rm -f -- "$LKJSCRIPT_INSTALL_TEST_PID"
        (
            exec /usr/bin/env --default-signal=INT,QUIT PATH="$work/path" LKJSCRIPT_INSTALL_TEST_LONG_STAGE="$stage" \
                bash "$installer" "$work/cancelled-$stage-$signal" \
                --frontend "$work/inputs/frontend" --backend "$work/inputs/backend" \
                --result "$work/inputs/result" --library "$work/inputs/library" \
                --runtime-source "$work/inputs/runtime.rs"
        ) >"$work/output" 2>"$work/error" &
        installer_pid=$!
        for ((attempt=0; attempt<500; attempt++)); do
            [[ ! -f $LKJSCRIPT_INSTALL_TEST_PID ]] || break
            sleep 0.01
        done
        if [[ ! -f $LKJSCRIPT_INSTALL_TEST_PID ]]; then
            kill -KILL "$installer_pid" 2>/dev/null || :
            wait "$installer_pid" 2>/dev/null || :
            printf 'installer cancellation stage was not reached: %s\n' "$stage" >&2
            exit 1
        fi
        worker_pid=$(<"$LKJSCRIPT_INSTALL_TEST_PID")
        kill -s "$signal" "$installer_pid"
        sleep 5 &
        timer_pid=$!
        status=0
        finished=
        wait -n -p finished "$installer_pid" "$timer_pid" || status=$?
        if [[ $finished == "$timer_pid" ]]; then
            kill -KILL "$installer_pid" "$worker_pid" 2>/dev/null || :
            wait "$installer_pid" 2>/dev/null || :
            printf 'installer cancellation did not finish: %s %s\n' "$stage" "$signal" >&2
            exit 1
        fi
        kill "$timer_pid" 2>/dev/null || :
        wait "$timer_pid" 2>/dev/null || :
        expected=143
        if [[ $signal == INT ]]; then expected=130; fi
        [[ $status == "$expected" ]]
        [[ ! -e /proc/$worker_pid/status ]]
        [[ ! -e $work/cancelled-$stage-$signal ]]
        [[ -z $(find "$work" -maxdepth 1 -name '.lkjscript-install.*' -print -quit) ]]
    done
done
rm "$work/path/sha256sum"
reject "$work/missing-dependency"
[[ $(<"$work/error") == *'required command is missing: sha256sum'* ]]
printf 'Native installer: isolated tools; exact copies/hashes; runtime/result interface; symlink/special-file rejection; failure cleanup; no-clobber publication\n'
