#!/usr/bin/env bash
# Offline projection controls. The host shell makes disposable Git/filesystem
# fixtures; checked code supplies policy and independently validates JSON.
sanitize_project_git() {
    local key
    while IFS= read -r key; do
        case "$key" in
            GIT_DIR|GIT_WORK_TREE|GIT_INDEX_FILE|GIT_COMMON_DIR|GIT_OBJECT_DIRECTORY|GIT_ALTERNATE_OBJECT_DIRECTORIES|GIT_CONFIG_PARAMETERS|GIT_CONFIG_COUNT|GIT_CONFIG_KEY_*|GIT_CONFIG_VALUE_*) unset "$key" ;;
        esac
    done < <(compgen -e)
    export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_OPTIONAL_LOCKS=0
}
test_project() (
    set -euo pipefail
    local repo=$1 compiler=$2 project_source=$3 built=$4 mode=$5
    sanitize_project_git
    umask 022
    local tmp source checks=0 rc
    tmp=$(mktemp -d "${TMPDIR:-/tmp}/lkj-project-test.XXXXXX")
    source=$tmp/source
    trap 'rc=$?; if ((rc)); then cat "$tmp/stderr" 2>/dev/null || :; fi; rm -rf -- "$tmp"; exit "$rc"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' HUP TERM
    "$compiler" build "$project_source" --entry self_checks --library-dir "$repo/library" --no-store \
        --grant output --grant errors --grant status -o "$tmp/self-checks"
    "$compiler" build "$repo/tools/real-programs/project-test.lkj" --library-dir "$repo/library" --no-store \
        --grant arguments --grant fixture=env:LKJSCRIPT_PROJECT_TEST_ROOT --grant output --grant errors --grant status -o "$tmp/validate"
    validate() { LKJSCRIPT_PROJECT_TEST_ROOT=$tmp "$tmp/validate" "$@"; }
    invoke() { "$built/host" "$built/project" "$@"; }
    pass() {
        invoke "$@" > "$tmp/stdout" 2> "$tmp/stderr" || { cat "$tmp/stderr" >&2; return 1; }
        test ! -s "$tmp/stderr"
    }
    check() {
        local label=$1; shift
        checks=$((checks+1))
        if ! "$@"; then printf 'project self-test %s failed: %s\n' "$checks" "$label" >&2; exit 1; fi
    }
    refused_status() { [[ $rc == 255 && ! -s $tmp/stdout ]]; }
    refuse() {
        local label=$1 reason=$2; shift 2
        rc=0; invoke "$@" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
        check "$label" refused_status
        if [[ -n $reason ]]; then check "$label: reason" grep -Fq -- "$reason" "$tmp/stderr"; fi
    }
    absent() { [[ ! -e $1 && ! -L $1 ]]; }
    field() { validate get stdout "$1"; }
    gitp() { git --no-replace-objects -c core.hooksPath=/dev/null -c core.fsmonitor=false -C "$source" "$@"; }
    commit() { gitp add -- .; gitp -c commit.gpgsign=false commit --quiet -m "$1"; }
    identity() {
        while IFS= read -r -d '' path; do
            printf '%s\0' "$path"; stat --printf='%d %i %f\n' -- "$source/$path"
            if [[ -L $source/$path ]]; then readlink -n -- "$source/$path" | sha256sum
            else sha256sum < "$source/$path"; fi
        done < <(gitp ls-files -z)
    }
    report() {
        local dest=$1 operation=$2 files=$3 aliases=$4 symlinks=$5
        validate report stdout "$source" "$dest" "$(gitp rev-parse HEAD)" "$operation" "$files" "$aliases" "$symlinks" "${dest#"$tmp/"}/projection.json"
    }
    # Same three pure assertions as the original: legacy bytes and safe paths.
    [[ $("$tmp/self-checks") == 3 ]]; checks=3
    mkdir -m 700 -p "$source/src"
    gitp -c init.defaultBranch=main init --quiet
    gitp config user.name 'Projection fixture'; gitp config user.email fixture@example.invalid
    gitp config commit.gpgsign false; gitp config core.autocrlf false
    gitp config remote.origin.url https://example.invalid/specimen.git
    local rust=$'#[test]\nfn original_test() { assert_eq!(1, 1); }\n' lib="$source/src/lib.rs"
    printf '%s' "$rust" > "$lib"; printf '\0\377asset\n' > "$source/asset.bin"
    cp "$source/asset.bin" "$tmp/asset"; commit fixture
    identity > "$tmp/before"
    pass "$source" "$tmp/first"
    check 'complete tracked-file projection' report "$tmp/first" create 2 1 0
    local first_hash
    first_hash=$(field manifest_sha256)
    pass --verify "$source" "$tmp/first"
    check 'read-only verification' report "$tmp/first" verify 2 1 0
    identity > "$tmp/after"
    check 'original bytes preserved' cmp "$tmp/before" "$tmp/after"
    check 'original mode preserved' test "$(stat -c %a "$lib")" = 644
    pass "$source" "$tmp/second"
    check 'deterministic manifest' test "$(field manifest_sha256)" = "$first_hash"
    refuse 'existing DEST rejected' '' "$source" "$tmp/first"
    pass --verify "$source" "$tmp/first"
    check 'existing DEST not cleaned' test "$(field aliases)" = 1
    rm "$tmp/first/src/lib.lkj"; printf 'changed alias\n' > "$tmp/first/src/lib.lkj"
    refuse 'changed alias rejected' '' --verify "$source" "$tmp/first"
    check 'failed verify leaves destination' test -f "$tmp/first/src/lib.lkj"
    printf '%s' "$rust" > "$tmp/original"
    check 'alias replacement preserves source' cmp "$tmp/original" "$lib"
    printf '%s' "$rust" > "$tmp/first/src/lib.lkj"
    refuse 'identical copy rejected' '' --verify "$source" "$tmp/first"
    rm -rf "$tmp/first" "$tmp/second"
    chmod 655 "$lib"; pass "$source" "$tmp/nonowner-added"
    check 'non-owner execute addition accepted' test "$(field aliases)" = 1
    check 'non-owner bits preserved' test "$(stat -c %a "$lib")" = 655
    rm -rf "$tmp/nonowner-added"
    chmod 744 "$lib"
    refuse 'owner execute addition rejected' 'dirty tracked executable mode' "$source" "$tmp/owner-added"
    check 'mode refusal before creation' absent "$tmp/owner-added"
    chmod 755 "$lib"; commit 'owner-execute fixture'; chmod 655 "$lib"
    refuse 'owner execute removal rejected' 'dirty tracked executable mode' "$source" "$tmp/owner-removed"
    check 'mode removal refusal before creation' absent "$tmp/owner-removed"
    chmod 744 "$lib"; pass "$source" "$tmp/nonowner-removed"
    check 'non-owner execute removal accepted' test "$(field aliases)" = 1
    check 'owner-only bits preserved' test "$(stat -c %a "$lib")" = 744
    rm -rf "$tmp/nonowner-removed"; chmod 644 "$lib"; commit 'restore execute mode'
    printf '%s// dirty\n' "$rust" > "$lib"
    refuse 'dirty bytes rejected' '' "$source" "$tmp/dirty"
    check 'dirty refusal before creation' absent "$tmp/dirty"; printf '%s' "$rust" > "$lib"
    gitp update-index --assume-unchanged -- src/lib.rs; printf '%s// hidden dirty\n' "$rust" > "$lib"
    refuse 'assume unchanged checked' '' "$source" "$tmp/hidden-dirty"
    printf '%s' "$rust" > "$lib"; gitp update-index --no-assume-unchanged -- src/lib.rs
    printf 'staged asset\n' > "$source/asset.bin"; gitp add -- asset.bin; cp "$tmp/asset" "$source/asset.bin"
    refuse 'dirty index rejected' '' "$source" "$tmp/dirty-index"; gitp reset --quiet HEAD -- asset.bin
    printf 'collision\n' > "$source/src/lib.lkj"; commit 'collision fixture'
    refuse 'suffix collision rejected' '' "$source" "$tmp/collision"
    check 'collision before creation' absent "$tmp/collision"
    gitp rm --quiet -- src/lib.lkj; commit 'remove collision'
    ln -s lib.rs "$source/src/link.rs"; commit 'symlink fixture'; pass "$source" "$tmp/symlink"
    check 'safe file symlink preserved' report "$tmp/symlink" create 3 2 1
    link_alias() { [[ -L $tmp/symlink/src/link.lkj && $(stat -c '%d:%i' "$source/src/link.rs") == "$(stat -c '%d:%i' "$tmp/symlink/src/link.lkj")" && $(readlink "$tmp/symlink/src/link.lkj") == lib.rs ]]; }
    check 'symlink alias has original inode' link_alias
    rm -rf "$tmp/symlink"; mkdir "$source/bench" "$source/test-pages"
    local html=$'<p>fixture</p>\n'
    printf '%s' "$html" > "$source/test-pages/one.html"
    ln -s ../test-pages "$source/bench/test-pages"; ln -s test-pages/one.html "$source/bench/page"
    commit 'internal directory links'; pass "$source" "$tmp/internal"
    check 'internal directory links preserved' report "$tmp/internal" create 6 2 3
    check 'manifest binds link bytes and graph' validate directory-link internal/projection.json
    printf '%schanged' "$html" > "$source/test-pages/one.html"
    refuse 'target payload mutation rejected' 'dirty tracked bytes' --verify "$source" "$tmp/internal"
    printf '%s' "$html" > "$source/test-pages/one.html"
    local target
    for target in ../missing ../test-pages; do
        rm "$tmp/internal/bench/test-pages"; ln -s "$target" "$tmp/internal/bench/test-pages"
        refuse 'copied symlink rejected' 'not an original-source hardlink' --verify "$source" "$tmp/internal"
    done
    rm "$tmp/internal/bench/test-pages"; ln -P "$source/bench/test-pages" "$tmp/internal/bench/test-pages"
    rm "$source/bench/test-pages"; ln -s ../src "$source/bench/test-pages"
    refuse 'source target text mutation rejected' 'dirty tracked bytes' --verify "$source" "$tmp/internal"
    rm "$source/bench/test-pages"; ln -P "$tmp/internal/bench/test-pages" "$source/bench/test-pages"
    pass --verify "$source" "$tmp/internal"
    check 'restored original links verify' test "$(field tracked_symlinks)" = 3
    rm -rf "$tmp/internal"
    local name reason
    while IFS='|' read -r name target reason; do
        ln -s "$target" "$source/bench/bad"; commit "$name link"
        refuse "$name rejected" "$reason" "$source" "$tmp/$name"
        check "$name before creation" absent "$tmp/$name"
        gitp rm --quiet -- bench/bad; commit "remove $name"
    done <<'LINKS'
absolute|/outside-fixture|absolute
escape|../../outside-fixture|escapes SOURCE
dangling|missing|dangling or untracked
new-alias|../src/lib.lkj|dangling or untracked
new-manifest|../projection.json|dangling or untracked
directory-cycle|..|cyclic directory symlink
LINKS
    ln -s b "$source/bench/a"; ln -s a "$source/bench/b"; commit 'chain cycle'
    refuse 'chain cycle rejected' 'cyclic symlink chain' "$source" "$tmp/chain-cycle"
    check 'chain refusal before creation' absent "$tmp/chain-cycle"
    gitp rm --quiet -- bench/a bench/b; commit 'remove chain cycle'
    local old_head new_head
    old_head=$(gitp rev-parse HEAD); printf '%s// replacement tree\n' "$rust" > "$lib"; commit replacement; new_head=$(gitp rev-parse HEAD)
    gitp replace "$old_head" "$new_head"; gitp update-ref HEAD "$old_head"
    refuse 'replacement refs cannot relabel tree' '' "$source" "$tmp/replaced-head"
    check 'replace refusal before creation' absent "$tmp/replaced-head"
    gitp replace --delete "$old_head" >/dev/null; gitp update-ref HEAD "$new_head"; printf '%s' "$rust" > "$lib"; commit 'remove replacement'
    # Fault injected only into a disposable bridge. Its successful physical
    # link precedes EIO, forcing checked cleanup after a symlink inode was linked.
    cat > "$tmp/fault.c" <<FAULT
#define main project_host_main
#define link fixture_link
#include "$repo/tools/real-programs/project-host.c"
#undef main
#undef link
int fixture_link(const char *from,const char *to) {
    int rc=linkat(AT_FDCWD,from,AT_FDCWD,to,0); struct stat s;
    if(!rc && lstat(from,&s)==0 && S_ISLNK(s.st_mode)){errno=EIO;return -1;}return rc;
}
int main(int argc,char **argv){return project_host_main(argc,argv);}
FAULT
    cc -std=c11 -O2 -Wall -Wextra -Werror "$tmp/fault.c" -o "$tmp/fault"
    rc=0; "$tmp/fault" "$built/project" "$source" "$tmp/partial" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
    check 'mid-creation failure propagated' refused_status
    partial_cleaned() { absent "$tmp/partial" && grep -Fq 'i/o error' "$tmp/stderr"; }
    check 'own partial tree cleaned without traversal' partial_cleaned
    check 'source survives cleanup' cmp "$tmp/original" "$lib"
    ln -s "$source" "$tmp/source-link"
    refuse 'symlink component before dot-dot rejected' 'symlink directory component' "$tmp/source-link/../source" "$tmp/component"
    [[ $checks == 71 ]] || { printf 'projection assertion inventory: expected 71, got %s\n' "$checks" >&2; exit 1; }
    # Signal fixture uses the public coordinator contract after an owned symlink.
    local signal_source=$tmp/signals
    mkdir "$signal_source"
    git -C "$signal_source" -c init.defaultBranch=main init --quiet
    git -C "$signal_source" config user.name 'Projection signal fixture'
    git -C "$signal_source" config user.email fixture@example.invalid
    git -C "$signal_source" config remote.origin.url https://example.invalid/signals.git
    local i signal pid watcher delivered
    for ((i=0;i<512;i++)); do printf '// %s\n' "$i" > "$signal_source/file-$(printf '%04d' "$i").rs"; done
    ln -s file-0000.rs "$signal_source/000-link.rs"
    git -C "$signal_source" add .
    git -C "$signal_source" -c core.hooksPath=/dev/null -c commit.gpgsign=false commit --quiet -m 'signal fixture'
    source=$signal_source; identity > "$tmp/signal-before"
    for signal in INT TERM; do
        rm -f "$tmp/sent"
        "$built/host" "$built/project" "$source" "$tmp/SIG$signal" > "$tmp/stdout" 2> "$tmp/stderr" & pid=$!
        (
            end=$((SECONDS+20))
            while kill -0 "$pid" 2>/dev/null; do
                if [[ -L $tmp/SIG$signal/000-link.lkj ]]; then kill -s "$signal" "$pid"; : > "$tmp/sent"; exit 0; fi
                if ((SECONDS>=end)); then kill -KILL "$pid" 2>/dev/null || :; exit 1; fi
                sleep 0.001
            done
            exit 1
        ) & watcher=$!
        rc=0; wait "$pid" || rc=$?
        delivered=0; wait "$watcher" || delivered=$?
        signal_delivered() { [[ $delivered == 0 && -f $tmp/sent ]]; }
        check "$signal delivered after a symlink object was linked" signal_delivered
        check "$signal preserves failure exit255" test "$rc" = 255
        reason=interrupted; [[ $signal != TERM ]] || reason=terminated
        signal_report() { [[ ! -s $tmp/stdout ]] && grep -Fxq "$reason" "$tmp/stderr"; }
        check "$signal reports interruption without success" signal_report
        check "$signal cleans only owned destination" absent "$tmp/SIG$signal"
        identity > "$tmp/signal-after"
        check "$signal preserves source bytes/mode/inode" cmp "$tmp/signal-before" "$tmp/signal-after"
    done
    [[ $checks == 81 ]]
    if [[ $mode == self ]]; then
        rm -rf -- "$tmp"; absent "$tmp"
        printf '{\n   "self_test" : {\n      "checks" : %s,\n      "fixture_cleaned" : true,\n      "passed" : true,\n      "signal_controls" : [\n         "SIGINT",\n         "SIGTERM"\n      ]\n   }\n}\n' "$checks"
    else
        source "$repo/tools/real-programs/project-extra-test.sh"
        project_extra_controls "$repo" "$compiler" "$project_source" "$built" "$tmp"
        rm -rf -- "$tmp"; absent "$tmp"
        printf 'Git projection: %s original assertions and extended controls passed\n' "$checks"
    fi
)
if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
    set -euo pipefail
    repo=$(cd -- "$(dirname -- "$0")/../.." && pwd)
    compiler=${1:-"$repo/target/lkjscript"}; (($#==0)) || shift
    case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
    project_source=$repo/tools/real-programs/project.lkj
    if (($#)); then [[ $# == 2 && $1 == --project ]]; project_source=$2; fi
    work=$(mktemp -d)
    trap 'rm -rf -- "$work"' EXIT
    source "$repo/tools/real-programs/project-build.sh"
    build_project "$compiler" "$project_source" "$repo/library" "$work"
    test_project "$repo" "$compiler" "$project_source" "$work" extended
fi
