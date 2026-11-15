#!/usr/bin/env bash
# cause: rule with no Rust counterpart.
# pending: Admission-key exclusion from program Directory/ReadDirectory grants
# is a provisioning precondition, not currently an enforced compiler rule.
# These accepted cases deliberately violate it and expose only the count read.
# The disjoint hardlink case rules out claiming lexical ancestry is sufficient.
# A future identity-aware refusal must update this expected current behavior.
set -euo pipefail
cd "$(dirname "$0")/../.."
compiler=${1:-target/lkjscript}
if (($# > 1)); then exit 2; fi
work=$(mktemp -d target/programs-image-key-overlap.XXXXXX)
before=$(cut -d' ' -f1 /proc/loadavg)
started=$(date +%s%N)
cleanup() {
    local code=$?
    trap - EXIT HUP INT TERM
    if ((code)); then test ! -f "$work/error" || tail -30 "$work/error" >&2; fi
    local after elapsed
    after=$(cut -d' ' -f1 /proc/loadavg)
    elapsed=$(($(date +%s%N)-started))
    awk -v ns="$elapsed" -v before="$before" -v after="$after" -v code="$code" 'BEGIN { printf "image-key provisioning pending %.3f s load %s -> %s status=%s\n", ns/1000000000, before, after, code }'
    rm -rf -- "$work"
    exit "$code"
}
trap cleanup EXIT HUP INT TERM
umask 077
mkdir "$work/source" "$work/private" "$work/alias"
head -c 32 /dev/urandom > "$work/private/key"
ln "$work/private/key" "$work/alias/key"
cat > "$work/source/worker.lkj" <<'LKJ'
pub fn main(channel: Channel) -> usize { drop(channel); 0usize }
LKJ
"$compiler" build-child worker.lkj --source-dir "$work/source" --library-dir library --no-store --image-key "$work/private/key" -o "$work/worker" > "$work/child.authority" 2>"$work/error"
for kind in Directory ReadDirectory; do
    cat > "$work/source/parent.lkj" <<LKJ
pub fn main(image: Image, mut root: $kind) -> usize {
    drop(image);
    let mut key = root.open("key").unwrap();
    let bytes = key.read_to_end().unwrap();
    assert_eq!(bytes.len(),32usize);
    bytes.len()
}
LKJ
    for exposure in private alias; do
        for engine in native interpreter; do
            "$compiler" run parent.lkj --source-dir "$work/source" --library-dir library --no-store --engine "$engine" --image-key "$work/private/key" --grant "image=$work/worker" --grant "root=$work/$exposure" > "$work/output" 2>"$work/error"
            grep -qx 32 "$work/output"
        done
        "$compiler" build parent.lkj --source-dir "$work/source" --library-dir library --no-store --image-key "$work/private/key" --grant "image=$work/worker" --grant "root=$work/$exposure" -o "$work/parent" > "$work/authority" 2>"$work/error"
        "$work/parent" > "$work/output" 2>"$work/error"
        printf 'pending admission-key provisioning: %s %s read 32 private bytes, three engines\n' "$kind" "$exposure"
    done
done
printf 'ok recorded gap: runtime key exclusion is not enforced; no runtime File-to-Image admission is offered\n'
