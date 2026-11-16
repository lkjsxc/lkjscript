#!/usr/bin/env bash
# Reads of disposable entries must neither create a miss nor follow a leaf.
set -euo pipefail
cd "$(dirname "$0")/.."
(($# == 1)) || { echo 'usage: cache-read-test.sh COMPILER' >&2; exit 2; }
compiler=$(realpath "$1")
fixture=$(mktemp -d target/compiler-cache-read-test.XXXXXX)
fixture=$(realpath "$fixture")
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n ${LKJSCRIPT_CACHE_READ_EVIDENCE:-} ]]; then
        mkdir -p "$LKJSCRIPT_CACHE_READ_EVIDENCE"
        while IFS= read -r -d '' record; do
            case $(basename "$record") in key|probe|historical-probe) continue ;; esac
            if [[ ! -s $record ]] || LC_ALL=C grep -Iq '' "$record"; then
                cp "$record" "$LKJSCRIPT_CACHE_READ_EVIDENCE/"
            fi
        done < <(find "$fixture" -maxdepth 1 -type f -print0)
    fi
    if ((status != 0)); then
        printf 'cache-read contract failed; retained diagnostics follow\n' >&2
        for log in "$fixture"/*.stderr; do
            [[ -f $log ]] && { printf '%s\n' "$log" >&2; tail -8 "$log" >&2; }
        done
    fi
    rm -rf -- "$fixture"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
umask 077
mkdir "$fixture/source" "$fixture/seed" "$fixture/empty-ordinary" "$fixture/empty-locator"
head -c 32 /dev/urandom > "$fixture/key"
cp src/diagnostic.lkj src/ir.lkj src/ir_binary.lkj src/reuse.lkj src/store.lkj "$fixture/source/"
# An explicit saved source is used only for the retained pre-fix refusal control.
if [[ -n ${LKJSCRIPT_CACHE_READ_SOURCE:-} ]]; then cp "$LKJSCRIPT_CACHE_READ_SOURCE" "$fixture/source/store.lkj"; fi
cat > "$fixture/source/main.lkj" <<'SOURCE'
mod diagnostic;
mod ir;
mod ir_binary;
mod reuse;
mod store;
pub use std::sha256;
use store::QueryAccess;
fn main(mut world: World) {
    let args = world.args();
    assert!(args.len() == 5usize);
    let mode = String::from_utf8(args[0].clone()).unwrap();
    let directory = Some(String::from_utf8(args[1].clone()).unwrap());
    let key_file = Some(String::from_utf8(args[2].clone()).unwrap());
    let ordinary = sha256::hex_digest(b"cache read ordinary");
    let identity = String::from_utf8(args[3].clone()).unwrap();
    let page = String::from_utf8(args[4].clone()).unwrap();
    let selector = sha256::hex_digest(b"cache read selector");
    if mode == "init" {
        let mut cache = store::Store::open_bounded(&mut world, &directory, &key_file, 65536usize).unwrap();
        cache.put("probe", &ordinary, b"ordinary payload");
        let mut node = Vec::new();
        for word in vec![3usize, 0usize, 0usize, 0usize] { store::push_word(&mut node, word); }
        let mut writer = cache.begin_query_container().unwrap();
        let root = writer.append_node(&node).unwrap();
        let source = sha256::hex_digest(b"snapshot source");
        let checked = sha256::hex_digest(b"snapshot checked IR");
        let manifest = writer.finish(&source, &checked, &selector, 0usize, &root).unwrap();
        assert!(cache.publish_query_locator(&manifest.identity, &selector, &root.digest));
        world.stdout().write(format!("id={} root={}\n", manifest.identity, root.digest).as_bytes());
        cache.finish();
    } else if mode == "ordinary-hit" || mode == "ordinary-miss" {
        let mut cache = store::Store::open_bounded(&mut world, &directory, &key_file, 65536usize).unwrap();
        let value = cache.get("probe", &ordinary);
        if mode == "ordinary-hit" { assert!(value.unwrap() == b"ordinary payload"[..].to_vec()); }
        else { assert!(value.is_none()); }
    } else if mode == "locator-miss" {
        assert!(store::QueryReader::open(&mut world, &directory, &key_file, 65536usize, &identity, &selector).is_err());
    } else {
        let mut reader = store::QueryReader::open(&mut world, &directory, &key_file, 65536usize, &identity, &selector).unwrap();
        assert!(reader.query_bound_root() == Some(page.clone()));
        if mode == "root-hit" {
            let bytes = reader.query_root(&identity).unwrap();
            let manifest = store::QueryManifest::decode(&bytes).unwrap();
            assert_eq!(manifest.identity, identity); assert_eq!(manifest.root, page);
        } else if mode == "root-miss" { assert!(reader.query_root(&identity).is_none()); }
        else {
            let node = match reader.query_root(&identity) {
                Some(bytes) => {
                    let manifest = store::QueryManifest::decode(&bytes).unwrap();
                    reader.query_node(&identity, &manifest.root_extent())
                }
                None => None,
            };
            if mode == "page-hit" { assert_eq!(sha256::hex_digest(&node.unwrap()), page); }
            else { assert!(mode == "page-miss" && node.is_none()); }
        }
    }
}
SOURCE
cp "$fixture/source/main.lkj" "$fixture/probe-source.lkj"
sha256sum "$fixture/source/store.lkj" > "$fixture/store-source.sha256"
run() {
    local label=$1; shift
    timeout --kill-after=2s 30s "$@" > "$fixture/$label.stdout" 2> "$fixture/$label.stderr"
}
manifest() {
    local directory=$1 output=$2
    (cd "$directory"; find . -mindepth 1 -maxdepth 1 -printf '%f\t%y\t%m\t%s\t%l\n' | LC_ALL=C sort
        find . -maxdepth 1 -type f -print0 | LC_ALL=C sort -z | xargs -0 -r sha256sum) > "$output"
}
unchanged() {
    local label=$1 executable=$2 mode=$3 directory=$4
    manifest "$directory" "$fixture/$label.before"
    run "$label" "$executable" "$mode" "$directory" "$fixture/key" "$identity" "$node_digest"
    manifest "$directory" "$fixture/$label.after"
    cmp "$fixture/$label.before" "$fixture/$label.after"
    printf '%s\tunchanged\n' "$label" >> "$fixture/contracts.tsv"
}
# Probe construction is not subject to the short hostile-leaf watchdog.
"$compiler" build main.lkj --source-dir "$fixture/source" --no-store --inline-limit 0 -o "$fixture/probe" \
    > "$fixture/build.stdout" 2> "$fixture/build.stderr"
sha256sum "$fixture/probe" > "$fixture/probe.sha256"
identity=$(printf "absent snapshot" | sha256sum | cut -d" " -f1)
node_digest=$(printf "absent node" | sha256sum | cut -d" " -f1)
unchanged ordinary-absent "$fixture/probe" ordinary-miss "$fixture/empty-ordinary"
unchanged locator-absent "$fixture/probe" locator-miss "$fixture/empty-locator"
run init "$fixture/probe" init "$fixture/seed" "$fixture/key" "$identity" "$node_digest"
identity=$(sed -n 's/^id=\([0-9a-f]*\) root=.*/\1/p' "$fixture/init.stdout")
node_digest=$(sed -n 's/.* root=\([0-9a-f]*\)$/\1/p' "$fixture/init.stdout")
[[ ${#identity} == 64 && ${#node_digest} == 64 ]]
cp "$fixture/probe" "$fixture/historical-probe"
printf 'different read-only query executable\n' >> "$fixture/historical-probe"
sha256sum "$fixture/historical-probe" > "$fixture/historical-probe.sha256"
[[ $(sha256sum "$fixture/probe" | cut -d' ' -f1) != $(sha256sum "$fixture/historical-probe" | cut -d' ' -f1) ]]
ordinary=$(find "$fixture/seed" -maxdepth 1 -type f -name 'probe-*' -printf '%f')
root=$(find "$fixture/seed" -maxdepth 1 -type f -name 'semantic-container-v3-*' -printf '%f')
page=$(find "$fixture/seed" -maxdepth 1 -type f -name 'semantic-container-v3-*' -printf '%f')
locator=$(find "$fixture/seed" -maxdepth 1 -type f -name 'semantic-locator-v3-*' -printf '%f')
[[ -n $ordinary && -n $root && -n $page && -n $locator ]]
printf 'ordinary\t%s\nroot\t%s\npage\t%s\nlocator\t%s\n' "$ordinary" "$root" "$page" "$locator" > "$fixture/entries.tsv"
cp -a "$fixture/seed" "$fixture/readonly"
find "$fixture/readonly" -maxdepth 1 -type f -exec chmod 0444 {} +
unchanged ordinary-readonly "$fixture/probe" ordinary-hit "$fixture/readonly"
unchanged historical-root-readonly "$fixture/historical-probe" root-hit "$fixture/readonly"
unchanged historical-page-readonly "$fixture/historical-probe" page-hit "$fixture/readonly"
# Root and node APIs share one container path but exercise separate lookup paths.
for kind in root page; do
    name=${!kind}
    directory="$fixture/missing-$kind"
    cp -a "$fixture/seed" "$directory"
    rm -- "$directory/$name"
    # Exact pinned root ID, or valid page digest, after authenticating the locator.
    unchanged "historical-$kind-absent" "$fixture/historical-probe" "$kind-miss" "$directory"
done
for kind in ordinary root page locator; do
    name=${!kind}
    executable="$fixture/historical-probe"
    [[ $kind == ordinary ]] && executable="$fixture/probe"
    for leaf in symlink dangling fifo; do
        label="$kind-$leaf"
        directory="$fixture/$label"
        cp -a "$fixture/seed" "$directory"
        rm -- "$directory/$name"
        case $leaf in
            symlink)
                cp "$fixture/seed/$name" "$fixture/$label-target"
                sha256sum "$fixture/$label-target" > "$fixture/$label-target.before"
                ln -s "$fixture/$label-target" "$directory/$name"
                ;;
            dangling) ln -s "$fixture/$label-uncreated" "$directory/$name" ;;
            fifo) mkfifo "$directory/$name" ;;
        esac
        unchanged "$label" "$executable" "$kind-miss" "$directory"
        if [[ $leaf == symlink ]]; then sha256sum -c "$fixture/$label-target.before" > "$fixture/$label-target.after"; fi
        if [[ $leaf == dangling ]]; then [[ ! -e $fixture/$label-uncreated ]]; fi
    done
    directory="$fixture/$kind-forged"
    cp -a "$fixture/seed" "$directory"
    printf 'forged authenticated entry\n' > "$directory/$name"
    run "$kind-forged" "$executable" "$kind-miss" "$directory" "$fixture/key" "$identity" "$node_digest"
    printf '%s\trefused\n' "$kind-forged" >> "$fixture/contracts.tsv"
done
# An authentic envelope from another namespace is also a miss.
for kind in ordinary root; do
    directory="$fixture/$kind-substituted"
    cp -a "$fixture/seed" "$directory"
    name=${!kind}
    replacement=$page
    [[ $kind == root ]] && replacement=$ordinary
    cp "$fixture/seed/$replacement" "$directory/$name"
    executable="$fixture/historical-probe"
    [[ $kind == ordinary ]] && executable="$fixture/probe"
    run "$kind-substituted" "$executable" "$kind-miss" "$directory" "$fixture/key" "$identity" "$node_digest"
    printf '%s\trefused\n' "$kind-substituted" >> "$fixture/contracts.tsv"
done
printf 'ok cache-read contracts: %s cases\n' "$(wc -l < "$fixture/contracts.tsv")"
