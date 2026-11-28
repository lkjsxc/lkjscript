#!/usr/bin/env bash
# Supplemental controls and the real public probe/projector witness.
project_fixture() {
    local root=$1 format=$2
    mkdir -p "$root/src" "$root/docs"
    git -C "$root" -c init.defaultBranch=main init --quiet --object-format="$format"
    git -C "$root" config user.name 'Projection fixture'
    git -C "$root" config user.email fixture@example.invalid
    git -C "$root" config commit.gpgsign false
    git -C "$root" config core.autocrlf false
    git -C "$root" config remote.origin.url https://example.invalid/specimen.git
    printf 'fn main() -> i64 { 42 }\n' > "$root/src/lib.rs"
    printf '\0\377asset\n' > "$root/asset.bin"
    printf 'Unicode and byte-order fixture\n' > "$root/docs/"$'café-\177-😀-\t-"\n'
    printf '#!/bin/sh\nexit 0\n' > "$root/script"; chmod 751 "$root/script"
    ln -s lib.rs "$root/src/link.rs"; ln -s docs "$root/docs-copy"
    git -C "$root" add .
    git -C "$root" -c core.hooksPath=/dev/null commit --quiet -m fixture
}
project_extra_controls() {
    local repo=$1 compiler=$2 project_source=$3 built=$4 tmp=$5
    local format expected dest actual extra_checks=0
    # Pins are derived from the saved original producer; only root and commit
    # lines are normalized. Every other manifest byte stays in the assertion.
    for format in sha1 sha256; do
        source=$tmp/extra-$format; dest=$tmp/projected-$format
        project_fixture "$source" "$format"
        pass "$source" "$dest"; report "$dest" create 6 2 2
        expected=a0a388701b624bcca4a83757f93e8f1756ed65d6e9a3dd49ee9a6e1eb7b41457
        [[ $format != sha256 ]] || expected=525cec9fbc5d55e36417d2a98ca89626c028e63b377b5936f46b49fd9277117e
        validate golden "projected-$format/projection.json" "$expected"
        pass --verify "$source" "$dest"; report "$dest" verify 6 2 2
        [[ $(stat -c %a "$dest") == 700 && $(stat -c %a "$dest/projection.json") == 600 ]]
        [[ $(stat -c '%d:%i' "$source/src/link.rs") == "$(stat -c '%d:%i' "$dest/src/link.lkj")" ]]
        [[ -L $dest/src/link.lkj && -L $dest/docs-copy ]]
        extra_checks=$((extra_checks+1))
        # Exercise empty, multi-block and multi-host-frame Git blob hashing.
        rm -rf "$dest"
        dd if=/dev/zero of="$source/wide" bs=65536 count=2 status=none
        printf x >> "$source/wide"; : > "$source/empty"; commit 'streaming boundaries'
        pass "$source" "$dest"; report "$dest" create 8 2 2
        pass --verify "$source" "$dest"; report "$dest" verify 8 2 2
        extra_checks=$((extra_checks+1))
        rm -rf "$dest"; gitp rm --quiet wide empty; commit 'remove streaming boundaries'
        pass "$source" "$dest"; report "$dest" create 6 2 2
    done
    source=$tmp/extra-sha1; dest=$tmp/projected-sha1
    # Keep the original self-test assertion inventory separate.
    extra_refuse() {
        local label=$1 reason=$2; shift 2
        rc=0; invoke "$@" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
        if [[ $rc != 255 || -s $tmp/stdout ]] || ! grep -Fq -- "$reason" "$tmp/stderr"; then
            printf 'project extended refusal failed: %s (status %s)\n' "$label" "$rc" >&2
            cat "$tmp/stderr" >&2; return 1
        fi
        extra_checks=$((extra_checks+1))
    }
    extra_refuse unknown 'use --help for usage' --help --unknown
    extra_refuse missing 'SOURCE and DEST are required' "$source"
    extra_refuse invalid-argv 'non-UTF-8 command-line arguments' "$tmp/"$'bad\377' "$tmp/refused"
    extra_refuse empty-source 'empty or NUL-containing SOURCE' '' "$tmp/refused"
    extra_refuse missing-component 'missing directory component in SOURCE' "$tmp/missing" "$tmp/refused"
    extra_refuse non-directory 'non-directory component in SOURCE' "$source/asset.bin" "$tmp/refused"
    extra_refuse subdirectory 'SOURCE must be the Git worktree root' "$source/src" "$tmp/refused"
    extra_refuse empty-dest 'empty or NUL-containing DEST' "$source" ''
    extra_refuse dot-dest 'DEST must name a fresh directory' "$source" .
    extra_refuse inside-dest 'DEST must be outside SOURCE' "$source" "$source/inside"
    ln -s "$source" "$tmp/parent-link"
    extra_refuse symlink-parent 'symlink directory component in DEST parent' "$source" "$tmp/parent-link/../refused"
    ln -s missing "$tmp/dangling-dest"
    extra_refuse dangling-dest 'DEST already exists' "$source" "$tmp/dangling-dest"
    printf 'unexpected\n' > "$dest/unexpected"
    extra_refuse unexpected-file 'unexpected projection file: unexpected' --verify "$source" "$dest"
    rm "$dest/unexpected"; mkdir "$dest/unexpected"
    extra_refuse unexpected-directory 'unexpected projection directory: unexpected' --verify "$source" "$dest"
    rmdir "$dest/unexpected"; mkfifo "$dest/special"
    extra_refuse special-projection 'nonregular nonsymlink projection node: special' --verify "$source" "$dest"
    rm "$dest/special"
    cp "$dest/projection.json" "$tmp/manifest-original"; printf 'bad\n' > "$dest/projection.json"
    extra_refuse manifest-bytes 'manifest differs from current clean SOURCE identity' --verify "$source" "$dest"
    rm "$dest/projection.json"
    extra_refuse missing-manifest 'missing node:' --verify "$source" "$dest"
    ln -s "$tmp/manifest-original" "$dest/projection.json"
    extra_refuse manifest-symlink 'projection node type changed: projection.json' --verify "$source" "$dest"
    rm "$dest/projection.json"; cp "$tmp/manifest-original" "$dest/projection.json"
    rm "$dest/src/lib.rs"; cp "$source/src/lib.rs" "$dest/src/lib.rs"
    extra_refuse copied-original 'not an original-source hardlink: src/lib.rs' --verify "$source" "$dest"
    rm "$dest/src/lib.rs"; ln "$source/src/lib.rs" "$dest/src/lib.rs"
    cp "$source/src/lib.rs" "$tmp/lib-before"; rm "$source/src/lib.rs"; cp "$tmp/lib-before" "$source/src/lib.rs"
    extra_refuse replaced-source-inode 'not an original-source hardlink: src/lib.lkj' --verify "$source" "$dest"
    rm "$source/src/lib.rs"; ln "$dest/src/lib.rs" "$source/src/lib.rs"
    pass --verify "$source" "$dest"; report "$dest" verify 6 2 2
    rm -rf "$dest"
    local bad reason
    while IFS='|' read -r bad reason; do
        ln -s "$bad" "$source/bad"; commit 'bad link'
        extra_refuse "target $bad" "$reason" "$source" "$tmp/refused"
        absent "$tmp/refused"; gitp rm --quiet bad; commit 'remove bad'
    done <<'BAD'
src/lib.rs/|symlink target traverses a non-directory
src/lib.rs/.|symlink target traverses a non-directory
src/lib.rs/..|symlink target traverses a non-directory
C:somewhere|empty, absolute, or ambiguous symlink target
src\lib.rs|empty, absolute, or ambiguous symlink target
BAD
    printf 'untracked\n' > "$source/untracked"
    ln -s untracked "$source/bad"; gitp add bad; gitp -c commit.gpgsign=false commit --quiet -m 'untracked link'
    extra_refuse untracked-target 'dangling or untracked symlink target: untracked' "$source" "$tmp/refused"
    gitp rm --quiet bad; commit 'remove untracked link'; gitp rm --quiet untracked; commit 'remove test payload'
    ln -s $'bad\377' "$source/bad"; commit 'invalid target'
    extra_refuse invalid-target 'non-UTF-8 symlink target' "$source" "$tmp/refused"
    gitp rm --quiet bad; commit 'remove invalid target'
    printf 'invalid path\n' > "$source/"$'bad\377'; commit 'invalid path'
    extra_refuse invalid-path 'non-UTF-8 Git entries' "$source" "$tmp/refused"
    gitp rm --quiet -- $'bad\377'; commit 'remove invalid path'
    for bad in 'C:drive' $'back\\slash'; do
        printf 'ambiguous\n' > "$source/$bad"; commit 'ambiguous path'
        extra_refuse ambiguous-path 'absolute or ambiguous tracked path' "$source" "$tmp/refused"
        gitp rm --quiet -- "$bad"; commit 'remove ambiguous path'
    done
    local head
    head=$(gitp rev-parse HEAD); gitp update-index --add --cacheinfo "160000,$head,vendor"
    extra_refuse gitlink 'gitlink or unsupported tracked mode 160000: vendor' "$source" "$tmp/refused"
    gitp reset --quiet HEAD -- vendor
    gitp config --add remote.origin.url https://example.invalid/second.git
    extra_refuse duplicate-origin 'SOURCE needs exactly one nonempty origin URL' "$source" "$tmp/refused"
    gitp config --unset-all remote.origin.url
    extra_refuse missing-origin 'Git config failed (status 1)' "$source" "$tmp/refused"
    gitp config remote.origin.url ''
    extra_refuse empty-origin 'SOURCE needs exactly one nonempty origin URL' "$source" "$tmp/refused"
    gitp config remote.origin.url https://example.invalid/specimen.git
    # The original source's raw bytes, rather than Git filters or index flags,
    # decide cleanliness. A missing regular node and FIFO replacement cannot hang.
    mv "$source/src/lib.rs" "$tmp/moved"
    extra_refuse missing-node 'missing node:' "$source" "$tmp/refused"
    mkfifo "$source/src/lib.rs"
    extra_refuse source-fifo 'nonregular file:' "$source" "$tmp/refused"
    rm "$source/src/lib.rs"; mv "$tmp/moved" "$source/src/lib.rs"
    mv "$source/src" "$tmp/moved-src"; ln -s "$tmp/moved-src" "$source/src"
    extra_refuse source-parent-link 'symlink parent:' "$source" "$tmp/refused"
    rm "$source/src"; mv "$tmp/moved-src" "$source/src"
    printf 'reserved\n' > "$source/projection.json"; commit 'manifest collision'
    extra_refuse manifest-collision 'projection path collision: projection.json' "$source" "$tmp/refused"
    gitp rm --quiet projection.json; commit 'remove manifest collision'
    mkdir "$source/src/lib.lkj"; printf 'child\n' > "$source/src/lib.lkj/child"; commit 'directory collision'
    extra_refuse directory-collision 'projection file/directory collision: src/lib.lkj' "$source" "$tmp/refused"
    gitp rm -r --quiet src/lib.lkj; commit 'remove directory collision'
    # Index stages, empty roots and raw namespace names are independent refusal
    # classes, not variants of dirty payload bytes.
    local blob
    blob=$(gitp rev-parse HEAD:asset.bin)
    gitp update-index --force-remove -- asset.bin
    printf '100644 %s 1\tasset.bin\n' "$blob" | gitp update-index --index-info
    extra_refuse unmerged-index 'unmerged or duplicate tracked path: asset.bin' "$source" "$tmp/refused"
    gitp reset --quiet HEAD -- asset.bin
    pass "$source" "$tmp/invalid-projection-path"
    printf 'raw name\n' > "$tmp/invalid-projection-path/"$'bad\377'
    extra_refuse invalid-projection-path 'non-UTF-8 projection path' --verify "$source" "$tmp/invalid-projection-path"
    rm -rf "$tmp/invalid-projection-path"
    mkdir "$source/a" "$source/b"; printf a > "$source/a/file"; printf b > "$source/b/file"
    ln -s ../b "$source/a/link"; ln -s ../a "$source/b/link"; commit 'sibling directory cycle'
    extra_refuse sibling-directory-cycle 'cyclic directory symlink graph at:' "$source" "$tmp/refused"
    gitp rm -r --quiet a b; commit 'remove sibling cycle'
    gitp config remote.origin.url $'invalid\377'
    extra_refuse invalid-origin 'non-UTF-8 origin URL' "$source" "$tmp/refused"
    gitp config remote.origin.url https://example.invalid/specimen.git
    local saved_source=$source
    source=$tmp/empty-source; mkdir "$source"; gitp -c init.defaultBranch=main init --quiet
    gitp config user.name Fixture; gitp config user.email fixture@example.invalid
    gitp config remote.origin.url https://example.invalid/empty.git
    gitp -c commit.gpgsign=false commit --quiet --allow-empty -m empty
    extra_refuse empty-source-repo 'SOURCE has no tracked files' "$source" "$tmp/refused"
    source=$saved_source
    # EXDEV needs a second writable filesystem; do not assume TMPDIR is /tmp.
    local second_root='' candidate cross
    for candidate in /dev/shm "$repo/target" /tmp; do
        if [[ -d $candidate && -w $candidate && $(stat -c %d "$candidate") != "$(stat -c %d "$source")" ]]; then second_root=$candidate; break; fi
    done
    if [[ -n $second_root ]]; then
        cross=$(mktemp -d "$second_root/lkj-project-test.XXXXXX")
        rc=0; invoke "$source" "$cross/projection" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
        [[ $rc == 255 && ! -s $tmp/stdout ]] && grep -Fq 'EXDEV:' "$tmp/stderr" && grep -Fq 'copy fallback is forbidden' "$tmp/stderr"
        absent "$cross/projection"; rmdir "$cross"; extra_checks=$((extra_checks+1))
    else
        # EXDEV cannot be exercised without a second writable filesystem.
        printf 'Projection EXDEV control unavailable: only one writable filesystem\n' >&2
    fi
    project_host_controls "$repo" "$built" "$tmp" "$source"
    # Public launchers and a real compiler/time executable, no fixture verifier.
    local production=$tmp/production
    mkdir -p "$production/source/src"
    source=$production/source
    gitp -c init.defaultBranch=main init --quiet
    gitp config user.name 'Production probe fixture'; gitp config user.email fixture@example.invalid
    gitp config remote.origin.url https://example.invalid/production.git
    printf 'mod child;\nfn main() -> i64 { child::value() }\n' > "$source/src/lib.rs"
    printf 'pub fn value() -> i64 { 42 }\n' > "$source/src/child.rs"
    commit fixture
    LKJSCRIPT_COMPILER=$compiler "$repo/tools/real-programs/project.sh" "$source" "$production/projection" > "$tmp/stdout" 2> "$tmp/stderr"
    test ! -s "$tmp/stderr"; report "$production/projection" create 2 2 0
    local compiler_hash
    compiler_hash=$(sha256sum "$compiler"); compiler_hash=${compiler_hash%% *}
    LKJSCRIPT_COMPILER=$compiler "$repo/tools/real-programs/probe.sh" \
        --compiler "$compiler" --library "$repo/library" --time /usr/bin/time \
        --source "$source" --projection "$production/projection" --package fixture --version 1.0.0 \
        --entry src/lib.rs --output "$production/receipt.json" > "$tmp/stdout" 2> "$tmp/stderr"
    test ! -s "$tmp/stderr"
    validate probe production/receipt.json production/projection/projection.json "$(gitp rev-parse HEAD)" "$compiler_hash"
    [[ $(field observations) == 2 && $(field source_unchanged) == true ]]
    [[ $source/src/lib.rs -ef $production/projection/src/lib.lkj && $source/src/child.rs -ef $production/projection/src/child.lkj ]]
    # A real corrupt projection must block the public probe before publication.
    rm "$production/projection/src/lib.lkj"; cp "$source/src/lib.rs" "$production/projection/src/lib.lkj"
    rc=0; LKJSCRIPT_COMPILER=$compiler "$repo/tools/real-programs/probe.sh" \
        --compiler "$compiler" --library "$repo/library" --time /usr/bin/time \
        --source "$source" --projection "$production/projection" --package fixture --version 1.0.0 \
        --entry src/lib.rs --output "$production/refused.json" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
    [[ $rc == 255 && ! -s $tmp/stdout ]] && absent "$production/refused.json"
    grep -Fq 'not an original-source hardlink: src/lib.lkj' "$tmp/stderr"
    grep -Fq 'projection verification failed: 255' "$tmp/stderr"
    printf 'Projection extended controls: %s; production probe.sh + project.sh: PASS (2 real compiler routes, corrupt-projection refusal)\n' "$extra_checks"
}

project_host_controls() {
    local repo=$1 built=$2 tmp=$3 source=$4 mode expected rc pid signal end
    cc -std=c11 -O2 -Wall -Wextra -Werror "$repo/tools/real-programs/project-host-test.c" -o "$tmp/host-faults"
    identity > "$tmp/fault-before"
    for mode in opened-inode opened-type read-size read-mtime read-ctime link-missing link-inode link-mtime link-ctime link-type; do
        case "$mode" in
            opened-*) expected='file identity changed while opening:' ;;
            read-*) expected='file changed while reading:' ;;
            link-*) expected='symbolic link changed while reading:' ;;
        esac
        rc=0
        PROJECT_FAULT=$mode PROJECT_FAULT_ROOT=$tmp/fault-dest \
            "$tmp/host-faults" "$built/project" "$source" "$tmp/fault-dest" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
        [[ $rc == 255 && ! -s $tmp/stdout ]] && grep -Fq "$expected" "$tmp/stderr" && absent "$tmp/fault-dest"
        extra_checks=$((extra_checks+1))
    done
    for mode in cleanup-invalid cleanup-replaced mkdir-race; do
        rc=0
        PROJECT_FAULT=$mode PROJECT_FAULT_ROOT=$tmp/fault-dest \
            "$tmp/host-faults" "$built/project" "$source" "$tmp/fault-dest" > "$tmp/stdout" 2> "$tmp/stderr" || rc=$?
        [[ $rc == 255 && ! -s $tmp/stdout ]]
        case "$mode" in
            cleanup-invalid) absent "$tmp/fault-dest"; grep -Fq 'i/o error' "$tmp/stderr" ;;
            cleanup-replaced)
                [[ $(<"$tmp/fault-dest/sentinel") == keep && -d $tmp/fault-dest.owned ]]
                grep -Fq 'cleanup also failed: real-programs-project: refusing cleanup of replaced destination:' "$tmp/stderr"
                rm -rf "$tmp/fault-dest" "$tmp/fault-dest.owned" ;;
            mkdir-race)
                [[ $(<"$tmp/fault-dest/sentinel") == keep ]]
                grep -Fq "EEXIST: file already exists, mkdir '$tmp/fault-dest'" "$tmp/stderr"
                rm -rf "$tmp/fault-dest" ;;
        esac
        extra_checks=$((extra_checks+1))
    done
    identity > "$tmp/fault-after"; cmp "$tmp/fault-before" "$tmp/fault-after"
    pass "$source" "$tmp/signal-verify"
    # Verification keeps default INT/TERM behavior; only creation defers them
    # to owned mutation boundaries. The test bridge pauses at its first fstat.
    for signal in INT TERM; do
        rm -f "$tmp/ready"
        PROJECT_FAULT=verify-signal PROJECT_FAULT_ROOT=$tmp \
            "$tmp/host-faults" "$built/project" --verify "$source" "$tmp/signal-verify" > "$tmp/stdout" 2> "$tmp/stderr" & pid=$!
        end=$((SECONDS+20))
        while [[ ! -f $tmp/ready ]] && kill -0 "$pid" 2>/dev/null; do
            if ((SECONDS>=end)); then kill -KILL "$pid" 2>/dev/null || :; wait "$pid" || :; return 1; fi
            sleep 0.001
        done
        [[ -f $tmp/ready ]]; kill -s "$signal" "$pid"
        rc=0; wait "$pid" || rc=$?
        if [[ $signal == INT ]]; then [[ $rc == 130 ]]; else [[ $rc == 143 ]]; fi
        [[ ! -s $tmp/stdout && ! -s $tmp/stderr && -f $tmp/signal-verify/projection.json ]]
        extra_checks=$((extra_checks+1))
    done

    # The public launcher must also retain default termination while building.
    # This compiler fixture stops before policy execution or destination writes.
    mkdir "$tmp/launcher-tmp"
    cat > "$tmp/paused-compiler" <<'COMPILER'
#!/usr/bin/env bash
: > "$PROJECT_LAUNCHER_ROOT/ready"
while [[ ! -f $PROJECT_LAUNCHER_ROOT/release ]]; do sleep 0.001; done
exit 23
COMPILER
    chmod +x "$tmp/paused-compiler"
    for signal in INT TERM; do
        rm -f "$tmp/ready" "$tmp/release"
        PROJECT_LAUNCHER_ROOT=$tmp LKJSCRIPT_COMPILER=$tmp/paused-compiler TMPDIR=$tmp/launcher-tmp \
            "$tmp/host-faults" --launcher "$repo/tools/real-programs/project.sh" --verify "$source" "$tmp/signal-verify" \
            > "$tmp/stdout" 2> "$tmp/stderr" & pid=$!
        end=$((SECONDS+20))
        while [[ ! -f $tmp/ready ]] && kill -0 "$pid" 2>/dev/null; do
            if ((SECONDS>=end)); then : > "$tmp/release"; kill -KILL "$pid" 2>/dev/null || :; wait "$pid" || :; return 1; fi
            sleep 0.001
        done
        [[ -f $tmp/ready ]]; kill -s "$signal" "$pid"; : > "$tmp/release"
        rc=0; wait "$pid" || rc=$?
        if [[ $signal == INT ]]; then [[ $rc == 130 ]]; else [[ $rc == 143 ]]; fi
        [[ ! -s $tmp/stdout && ! -s $tmp/stderr && -f $tmp/signal-verify/projection.json ]]
        [[ -z $(ls -A "$tmp/launcher-tmp") ]]
        extra_checks=$((extra_checks+1))
    done
    pass --verify "$source" "$tmp/signal-verify"
    rm -rf "$tmp/signal-verify"
}
