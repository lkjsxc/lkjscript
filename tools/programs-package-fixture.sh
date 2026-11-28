#!/usr/bin/env bash
# Independent, test-only inventory producer. This is NOT the general-purpose
# acquisition CLI in programs-package-manifest.sh: fixture identities and paths
# are ASCII, archive members are regular ustar files, gzip is made with gzip -n,
# and a dependency (if present) is a leaf pin. Never use it for external sources.
set -euo pipefail
export LC_ALL=C
fail() { printf '%s\n' "$*" >&2; exit 1; }
digest() { local hash; read -r hash _ < <(sha256sum "$1"); printf '%s' "$hash"; }
route=$1 root=$2 input=$3
shift 3
if [[ $route == archive ]]; then checksum=$1; shift; fi
name=$1 version=$2 output=$3 child=${4:-}
[[ $name =~ ^[A-Za-z0-9_-]+$ && $version =~ ^[0-9.]+$ ]] || fail 'invalid fixture identity'
scratch=$(mktemp -d "$(dirname -- "$output")/inventory.XXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
origin=https://example.invalid/fixture
notice=no
declare -a files=()
declare -A seen=()
source_file() {
    local path=$1 part node=$root
    [[ $path =~ ^[A-Za-z0-9_.-]+(/[A-Za-z0-9_.-]+)*$ ]] || fail 'invalid fixture path'
    local IFS=/
    for part in $path; do
        [[ $part != . && $part != .. ]] || fail 'invalid fixture path'
        node+=/"$part"
        [[ ! -L $node ]] || fail 'symlink source node'
    done
    [[ -f $node ]] || fail 'not a regular source file'
}
record() {
    local path=$1 size hash
    [[ ! ${seen[$path]+present} ]] || fail 'duplicate source path'
    seen[$path]=1
    read -r size < <(wc -c < "$root/$path")
    hash=$(digest "$root/$path")
    files+=("[\"$path\",$size,\"$hash\"]")
    if [[ $path == LICENSE ]]; then notice=yes; fi
}
if [[ $route == git ]]; then
    unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_COMMON_DIR GIT_NAMESPACE GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT
    export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_OPTIONAL_LOCKS=0
    git_at() { git --no-replace-objects -c core.fsmonitor=false -c core.hooksPath=/dev/null -c core.quotePath=true -C "$root" "$@"; }
    revision=$input
    [[ $revision =~ ^[0-9a-f]{40}([0-9a-f]{24})?$ && $(git_at rev-parse --verify 'HEAD^{commit}') == "$revision" ]] || fail 'source HEAD differs'
    git_at ls-tree -r --full-tree HEAD > "$scratch/tree"
    git_at ls-files --stage > "$scratch/index"
    sed -e 's/ blob / /' -e $'s/\t/ 0\t/' "$scratch/tree" > "$scratch/expected-index"
    cmp -s "$scratch/index" "$scratch/expected-index" || fail 'Git index differs from HEAD'
    sort -t $'\t' -k2,2 "$scratch/tree" > "$scratch/sorted"
    while IFS=$'\t' read -r meta path; do
        read -r mode kind oid <<< "$meta"
        [[ $kind == blob && ( $mode == 100644 || $mode == 100755 ) ]] || fail 'unsupported Git node'
        source_file "$path"
        git_at cat-file blob "$oid" | cmp -s "$root/$path" - || fail 'tracked source differs from Git object'
        record "$path"
    done < "$scratch/sorted"
elif [[ $route == archive ]]; then
    [[ -f $input && ! -L $input ]] || fail 'not a regular file'
    [[ $checksum =~ ^[0-9a-f]{64}$ && $(digest "$input") == "$checksum" ]] || fail 'archive checksum mismatch'
    gzip -dc "$input" > "$scratch/tar" || fail 'invalid compressed payload'
    # The test profile owns its encoding. Re-encoding with the same gzip
    # rejects concatenated members (even empty ones) and ignored trailing bytes.
    # Do not mistake this for accepting every valid general-purpose gzip stream.
    gzip -n -c "$scratch/tar" > "$scratch/reencoded"
    cmp -s "$input" "$scratch/reencoded" || fail 'noncanonical compressed payload'
    # Verify framing directly, not via tar extraction (which may ignore tails).
    # The small fixture has no directory or extension headers.
    od -An -v -tu1 "$scratch/tar" | awk -v prefix="$name-$version/" '
        function refuse(reason) { print reason > "/dev/stderr"; exit 1 }
        function octal(at, len,    i,x,n,digits,tail) {
            for (i=at; i<at+len; i++) {
                x=b[i]
                if (x>=48 && x<=55 && !tail) { n=n*8+x-48; digits++ }
                else if (x==0 || x==32) { if (digits || x==0) tail=1 }
                else refuse("non-octal tar framing")
            }
            if (!digits) refuse("empty tar numeric field")
            return n
        }
        function field(at,len,    i,s) {
            for (i=at; i<at+len && b[i]; i++) s=s sprintf("%c",b[i])
            for (; i<at+len; i++) if (b[i]) refuse("invalid tar text field")
            return s
        }
        { for (i=1;i<=NF;i++) b[n++]=$i }
        END {
            if (n%512) refuse("tar length is not block-aligned")
            for (at=0; at+512<=n;) {
                sum=0
                for (i=at;i<at+512;i++) sum+=b[i]
                if (sum==0) {
                    if (at+1024>n) refuse("tar lacks two final zero blocks")
                    for (i=at;i<n;i++) if (b[i]) refuse("trailing tar payload")
                    exit 0
                }
                size=octal(at+124,12); check=octal(at+148,8)
                for (i=at+148;i<at+156;i++) sum+=32-b[i]
                if (sum!=check) refuse("tar header checksum mismatch")
                if (b[at+156]!=0 && b[at+156]!=48) refuse("unsupported tar node")
                # Exact ASCII ustar NUL magic and 00 version.
                if (b[at+257]!=117 || b[at+258]!=115 || b[at+259]!=116 ||
                    b[at+260]!=97 || b[at+261]!=114 || b[at+262]!=0) refuse("invalid ustar magic")
                if (b[at+263]!=48 || b[at+264]!=48) refuse("invalid ustar version")
                for (i=at+157;i<at+257;i++) if (b[i]) refuse("non-empty tar linkname")
                for (i=at+500;i<at+512;i++) if (b[i]) refuse("non-zero tar header padding")
                path=field(at,100); dir=field(at+345,155)
                if (dir!="") path=dir "/" path
                if (index(path,prefix)!=1) refuse("wrong archive prefix")
                path=substr(path,length(prefix)+1)
                if (path !~ /^[A-Za-z0-9_.\/-]+$/) refuse("invalid archive path")
                next_at=at+512+int((size+511)/512)*512
                if (next_at>n) refuse("truncated tar payload")
                for (i=at+512+size;i<next_at;i++) if (b[i]) refuse("non-zero tar data padding")
                printf "%s\t%d\t%d\n",path,at+512,size
                at=next_at
            }
            refuse("tar has no terminal framing")
        }' > "$scratch/members"
    sort "$scratch/members" > "$scratch/sorted"
    while IFS=$'\t' read -r path offset size; do
        source_file "$path"
        dd if="$scratch/tar" bs=1 skip="$offset" count="$size" status=none |
            cmp -s "$root/$path" - || fail 'extracted source differs from archive member'
        record "$path"
    done < "$scratch/sorted"
    revision="sha256:$checksum"
else
    fail 'unknown fixture route'
fi
[[ $notice == yes ]] || fail 'unlisted notice'
file_json=$(IFS=,; printf '[%s]' "${files[*]}")
read -r tree _ < <(printf 'lkjscript-source-tree-1\0%s' "$file_json" | sha256sum)
identity="[\"$name\",\"$version\",\"$origin\",\"$revision\",\"$tree\"]"
edges='[]'
if [[ -n $child ]]; then
    [[ -f $child && ! -L $child ]] || fail 'not a regular fixture dependency pin'
    pin=$(cat "$child")
    # Only the leaf child produced by this fixture is supported. Check the
    # literal byte encoding, including absence of trailing newlines.
    pattern='^\["lkjscript-package-pin-1",(\["[A-Za-z0-9_-]+","[0-9.]+","https://example\.invalid/fixture","[0-9a-f]{40}([0-9a-f]{24})?","[0-9a-f]{64}"\]),\[\]\]$'
    [[ $pin =~ $pattern ]] || fail 'invalid fixture dependency pin'
    edge=${BASH_REMATCH[1]}
    printf '%s' "$pin" | cmp -s "$child" - || fail 'noncanonical fixture dependency pin'
    edges="[$edge]"
fi
[[ ! -e $output.manifest.json && ! -L $output.manifest.json && ! -e $output.pin.json && ! -L $output.pin.json ]] || fail 'fixture output exists'
set -C
printf '["lkjscript-package-source-1-regular",%s,%s,%s,["LICENSE"]]' "$identity" "$edges" "$file_json" > "$output.manifest.json"
printf '["lkjscript-package-pin-1",%s,%s]' "$identity" "$edges" > "$output.pin.json"
printf '%s\n' "$tree"
