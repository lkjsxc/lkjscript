#!/usr/bin/env bash
# Offline byte-path and observation controls. Git projection is still a separate
# hosted job; this suite supplies a pinned, independently verified fixture
# manifest with the same verifier response contract, not a fake probe receipt.
set -euo pipefail

sanitize_git() {
    local k
    while IFS= read -r k; do
        case "$k" in
            GIT_DIR|GIT_WORK_TREE|GIT_INDEX_FILE|GIT_COMMON_DIR|GIT_OBJECT_DIRECTORY|GIT_ALTERNATE_OBJECT_DIRECTORIES|GIT_CONFIG_PARAMETERS|GIT_CONFIG_COUNT|GIT_CONFIG_KEY_*|GIT_CONFIG_VALUE_*|PERL_UNICODE) unset "$k" ;;
        esac
    done < <(compgen -e)
    export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
}

fixture_tools() {
    local tmp=$1
    cat > "$tmp/compiler" <<'COMPILER'
#!/bin/bash
set -euo pipefail
[[ $# == 6 && $1 == check && $3 == --source-dir && $5 == --library-dir && -d $6 ]]
printf 'entry\0%s\0root\0%s\0payload\0' "$2" "$4"
cat -- "$4/$2"
printf '\377'
printf 'diagnostic ☃\n' >&2
COMPILER
    cat > "$tmp/time" <<'TIME'
#!/bin/bash
set -euo pipefail
[[ $# == 11 && $1 == -f && $2 == %M && $3 == -o ]]
report=$4; shift 4
status=0; "$@" || status=$?
printf '4321\n' > "$report"
exit "$status"
TIME
    cp -- "$tmp/compiler" "$tmp/compiler-源"
    cp -- "$tmp/time" "$tmp/time-源"
    chmod +x "$tmp"/compiler{,-源} "$tmp"/time{,-源}
    cat > "$tmp/verifier" <<'VERIFIER'
#!/bin/bash
# Independent oracle for this one-file fixture. No Node or probe code.
set -euo pipefail
[[ $# == 3 && $1 == --verify ]]
source=$2 projection=$3
dir=${source%/*}
entry=$(<"$dir/entry")
alias=${entry%.rs}.lkj
commit=$(git -C "$source" rev-parse HEAD)
[[ $commit == "$(<"$dir/commit")" ]]
[[ -z $(git -C "$source" status --porcelain --untracked-files=no) ]]
[[ $source/$entry -ef $projection/$entry && $source/$entry -ef $projection/$alias ]]
digest=$(sha256sum "$projection/projection.json")
digest=${digest%% *}
[[ $digest == "$(<"$dir/manifest.sha")" ]]
printf '{"operation":"verify","read_only":true,"manifest":"%s/projection.json","manifest_sha256":"%s","commit":"%s","tracked_files":1,"aliases":1}\n' "$projection" "$digest" "$commit"
VERIFIER
    chmod +x "$tmp/verifier"
}
fixture() {
    local tmp=$1 label=$2 entry=$3 unicode=$4
    local dir="$tmp/$label" suffix='' lib_relative=std/café.lkj
    [[ $unicode == no ]] || suffix=-源
    [[ $label != ascii ]] || lib_relative=std/plain.lkj
    local source="$dir/source$suffix" projection="$dir/projection$suffix" library="$dir/library$suffix" commit digest size
    mkdir -p "$source/src" "$projection/src" "$library/std"
    git -c core.hooksPath=/dev/null -C "$source" -c init.defaultBranch=main init --quiet --object-format=sha1
    git -C "$source" config user.name 'Local review fixture'
    git -C "$source" config user.email fixture@example.invalid
    git -C "$source" config commit.gpgsign false
    git -C "$source" config core.autocrlf false
    git -C "$source" config remote.origin.url https://example.invalid/fixture.git
    printf 'fn original() {}\r\n// café\r\n' > "$source/$entry"
    printf 'fixture library\n' > "$library/$lib_relative"
    git -C "$source" add -- .
    git -c core.hooksPath=/dev/null -C "$source" commit --quiet -m fixture
    commit=$(git -C "$source" rev-parse HEAD)
    ln -- "$source/$entry" "$projection/$entry"
    ln -- "$source/$entry" "$projection/${entry%.rs}.lkj"
    digest=$(sha256sum "$source/$entry"); digest=${digest%% *}
    size=$(wc -c < "$source/$entry")
    # Only fields consumed by the probe. The independent verifier checks the
    # actual Git commit, hardlinked bytes and exact frozen manifest digest.
    printf '{"schema":1,"kind":"git_tracked_suffix_projection","source":{"root":"%s","commit":"%s","origin":"https://example.invalid/fixture.git","object_format":"sha1"},"files":[{"path":"%s","bytes":%s,"sha256":"%s"}],"aliases":[{"from":"%s","to":"%s"}]}\n' \
        "$source" "$commit" "$entry" "$size" "$digest" "$entry" "${entry%.rs}.lkj" > "$projection/projection.json"
    digest=$(sha256sum "$projection/projection.json"); printf '%s\n' "${digest%% *}" > "$dir/manifest.sha"
    printf '%s\n' "$commit" > "$dir/commit"
    printf '%s\n' "$entry" > "$dir/entry"
}
fixture_args() {
    local tmp=$1 label=$2 entry=$3 unicode=$4 output=$5 suffix=''
    [[ $unicode == no ]] || suffix=-源
    args=(--compiler "$tmp/compiler$suffix" --time "$tmp/time$suffix" --library "$tmp/$label/library$suffix" \
        --source "$tmp/$label/source$suffix" --projection "$tmp/$label/projection$suffix" \
        --package fixture --version 1.0.0 --entry "$entry" --output "$output")
}

test_probe() (
    local repo=$1 compiler=$2 probe_source=$3
    source "$repo/tools/real-programs/probe-host.sh"
    sanitize_git
    local tmp work
    tmp=$(mktemp -d /tmp/lkj-probe-unicode.XXXXXX)
    work=$tmp/build; mkdir "$work"
    trap 'rm -rf -- "$tmp"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' HUP TERM
    build_probe "$compiler" "$probe_source" "$repo/library" "$work"
    "$compiler" build "$repo/tools/real-programs/probe-test.lkj" --library-dir "$repo/library" --no-store \
        --grant arguments --grant fixture=env:LKJSCRIPT_PROBE_TEST_ROOT --grant output --grant errors --grant status -o "$work/validate"
    fixture_tools "$tmp"
    fixture "$tmp" ascii src/lib.rs no
    fixture "$tmp" utf-entry src/café.rs no
    fixture "$tmp" utf-roots src/café.rs yes
    local -a args
    local checks=0 label entry unicode receipt rc output expected
    invoke() { run_probe "$work/select" "$work/probe" executable "$tmp/verifier" "$@"; }
    for label in ascii utf-entry utf-roots; do
        entry=src/café.rs; unicode=no; receipt=receipt-源.json
        [[ $label != ascii ]] || { entry=src/lib.rs; receipt=receipt.json; }
        [[ $label != utf-roots ]] || unicode=yes
        output="$tmp/$label/$receipt"
        fixture_args "$tmp" "$label" "$entry" "$unicode" "$output"
        invoke "${args[@]}" > "$tmp/stdout" 2> "$tmp/stderr" || { cat "$tmp/stderr" >&2; exit 1; }
        test ! -s "$tmp/stderr"
        LKJSCRIPT_PROBE_TEST_ROOT=$tmp "$work/validate" "$tmp" "$label" "$entry" "$unicode" "$receipt" "$(cat "$tmp/$label/commit")"
        [[ $(stat -c '%a' "$output") == 600 ]]
        checks=$((checks+1))
    done
    # Exact refusals, including raw argv bytes, no output/receipt on refusal.
    refuse() {
        local name=$1 reason=$2; shift 2
        rc=0; invoke "$@" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
        printf 'real-programs-probe: %s\n' "$reason" > "$tmp/expected"
        [[ $rc == 255 && ! -s $tmp/stdout ]] && cmp "$tmp/expected" "$tmp/stderr" || {
            printf 'probe refusal %s: expected status 255, got %s\n' "$name" "$rc" >&2
            cat "$tmp/stderr" >&2; exit 1
        }
        [[ ! -e $output && ! -L $output ]]
        checks=$((checks+1))
    }
    output="$tmp/refusal.json"
    fixture_args "$tmp" ascii src/lib.rs no "$output"
    local -a good=("${args[@]}")
    refuse invalid-entry 'non-UTF-8 entry path' "${good[@]:0:14}" --entry $'src/bad\377.rs' --output "$output"
    refuse invalid-source 'non-UTF-8 input path' "${good[@]}" --source "$tmp/"$'bad\377'
    refuse invalid-projection 'non-UTF-8 input path' "${good[@]}" --projection "$tmp/"$'bad\377'
    refuse invalid-output 'non-UTF-8 output path' "${good[@]}" --output "$tmp/"$'bad\377.json'
    refuse invalid-package 'non-UTF-8 package name' "${good[@]}" --package $'bad\377'
    refuse invalid-version 'non-UTF-8 package version' "${good[@]}" --version $'bad\377'
    refuse invalid-compiler 'non-UTF-8 input path' "${good[@]}" --compiler "$tmp/"$'bad\377'
    refuse invalid-library 'non-UTF-8 input path' "${good[@]}" --library "$tmp/"$'bad\377'
    refuse invalid-time 'non-UTF-8 input path' "${good[@]}" --time "$tmp/"$'bad\377'
    local bad="$tmp/ascii/library/"$'bad\377.lkj'
    printf 'invalid path fixture\n' > "$bad"
    refuse invalid-library-member 'non-UTF-8 library path' "${good[@]}"
    rm -- "$bad"
    refuse missing 'all named arguments and at least one --entry are required'
    refuse unknown 'use --help' "${good[@]}" --wrong value
    refuse missing-value 'use --help' "${good[@]}" --compiler
    refuse missing-input 'input path does not exist' "${good[@]}" --compiler "$tmp/missing"
    refuse bad-compiler 'compiler is not executable' "${good[@]}" --compiler "$tmp/ascii/library/std/plain.lkj"
    refuse bad-time 'GNU-time-compatible launcher is not executable' "${good[@]}" --time "$tmp/ascii/library/std/plain.lkj"
    refuse duplicate-entry 'entry must be a unique tracked Rust path: src/lib.rs' "${good[@]}" --entry src/lib.rs
    refuse unknown-entry 'entry must be a unique tracked Rust path: src/missing.rs' "${good[@]}" --entry src/missing.rs
    ln -s std/plain.lkj "$tmp/ascii/library/a-link"
    refuse symlink "symlink in library input: $tmp/ascii/library/a-link" "${good[@]}"
    rm -- "$tmp/ascii/library/a-link"
    mkfifo "$tmp/ascii/library/a-fifo"
    refuse nonregular "nonregular library input: $tmp/ascii/library/a-fifo" "${good[@]}"
    rm -- "$tmp/ascii/library/a-fifo"
    cp "$tmp/time" "$tmp/bad-time"
    sed -i "s/4321/missing/" "$tmp/bad-time"
    refuse missing-rss 'missing GNU-time peak RSS' "${good[@]}" --time "$tmp/bad-time"
    printf '#!/no/such/interpreter\n' > "$tmp/bad-exec"
    chmod +x "$tmp/bad-exec"
    refuse exec-failure "exec $tmp/bad-exec: spawnSync $tmp/bad-exec ENOENT" "${good[@]}" --time "$tmp/bad-exec"
    printf 'existing\n' > "$tmp/existing.json"
    rc=0; invoke "${good[@]}" --output "$tmp/existing.json" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
    [[ $rc == 255 && ! -s $tmp/stdout && $(<"$tmp/existing.json") == existing ]]
    grep -Fxq 'real-programs-probe: output already exists' "$tmp/stderr"; checks=$((checks+1))
    invoke --help > "$tmp/stdout" 2> "$tmp/stderr"
    grep -Fxq 'Usage: probe.sh --compiler COMPILER --library LIBRARY --source SOURCE' "$tmp/stdout"
    test ! -s "$tmp/stderr"
    # A valid UTF-8 spelling resolving to a non-UTF-8 OS path must refuse.
    local bad_resolved="$tmp/"$'real\377'
    cp "$tmp/compiler" "$bad_resolved"
    ln -s "$bad_resolved" "$tmp/resolve-link"
    refuse invalid-resolved 'input path does not exist' "${good[@]}" --compiler "$tmp/resolve-link"
    [[ ! -e "$tmp/"$'bad\377.json' ]]
    # Reverification and rehashing must happen after the two observations.
    cp "$tmp/compiler" "$tmp/compiler-before"
    cp "$tmp/ascii/library/std/plain.lkj" "$tmp/library-before"
    for target in compiler library; do
        cat > "$tmp/change-$target" <<'CHANGE'
#!/bin/bash
set -euo pipefail
report=$4; shift 4
status=0; "$@" || status=$?
printf '4321\n' > "$report"
CHANGE
        if [[ $target == compiler ]]; then printf '%s\n' 'printf "\\n# changed\\n" >> "$1"' >> "$tmp/change-$target"
        else printf '%s\n' 'printf "changed\\n" >> "$7/std/plain.lkj"' >> "$tmp/change-$target"; fi
        printf '%s\n' 'exit "$status"' >> "$tmp/change-$target"
        chmod +x "$tmp/change-$target"
        refuse "changed-$target" "$target bytes changed during observations" "${good[@]}" --time "$tmp/change-$target"
        cp "$tmp/compiler-before" "$tmp/compiler"
        cp "$tmp/library-before" "$tmp/ascii/library/std/plain.lkj"
    done
    cp "$tmp/verifier" "$tmp/verifier-before"
    sed -i '/^printf.*operation/i\digest=wrong' "$tmp/verifier"
    refuse mismatch 'projection manifest differs from the verified bytes' "${good[@]}"
    cp "$tmp/verifier-before" "$tmp/verifier"
    sed -i '/^printf.*operation/i\if [[ -e $dir/verified ]]; then digest=wrong; else touch "$dir/verified"; fi' "$tmp/verifier"
    refuse changed-manifest 'projection manifest changed during observations' "${good[@]}"
    rm -f "$tmp/ascii/verified"
    cp "$tmp/verifier-before" "$tmp/verifier"
    # Distinguish an actual launcher signal from a normal exit of 128+signal.
    # The full report remains retained even on either abnormal route.
    local kind
    for kind in exit signal; do
        cp "$tmp/time" "$tmp/time-$kind"
        if [[ $kind == exit ]]; then sed -i 's/exit "$status"/exit 143/' "$tmp/time-$kind"
        else sed -i 's/exit "$status"/kill -TERM "$$"/' "$tmp/time-$kind"; fi
        output="$tmp/$kind.json"
        fixture_args "$tmp" ascii src/lib.rs no "$output"
        invoke "${args[@]}" --time "$tmp/time-$kind" > "$tmp/stdout" 2> "$tmp/stderr"
        test ! -s "$tmp/stderr"
        if [[ $kind == exit ]]; then
            [[ $(grep -Fc '"exit_status": 143,' "$output") == 2 && $(grep -Fc '"launcher_signal": 0,' "$output") == 2 ]]
        else
            [[ $(grep -Fc '"exit_status": 0,' "$output") == 2 && $(grep -Fc '"launcher_signal": 15,' "$output") == 2 ]]
        fi
        [[ $(grep -Fc '"peak_rss_kib": 4321,' "$output") == 2 ]]
        checks=$((checks+1))
    done
    output="$tmp/refusal.json"
    # No output-parent temporary directory may survive success or refusal.
    [[ -z $(find "$tmp" -name 'root-probe-*' -print) ]]
    printf 'ok probe: %s CLI controls, pinned ASCII receipt fingerprint, Unicode and raw diagnostic bytes\n' "$checks"
)
if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
    compiler=${1:-"$repo/target/lkjscript"}
    if [[ $compiler == --help ]]; then
        printf 'Usage: probe-test.sh [COMPILER] [--probe SOURCE.lkj]\nNo real compiler observation or network is used.\n'
        exit 0
    fi
    if (($#)); then shift; fi
    case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
    probe_source="$repo/tools/real-programs/probe.lkj"
    if (($#)); then
        [[ $# == 2 && $1 == --probe ]] || { printf 'unexpected positional arguments\n' >&2; exit 255; }
        probe_source=$2
    fi
    test_probe "$repo" "$compiler" "$probe_source"
fi
