#!/usr/bin/env bash
# Separate, tiny prelaunch validation; never a measured rung.
set -euo pipefail
cd "$(dirname "$0")/../.."
repository=$PWD
evidence="$repository/target/compiler-container-helper-preflight-1"
[[ ! -e $evidence ]] || { echo 'existing helper preflight evidence' >&2; exit 2; }
mkdir "$evidence"
load() { awk '{printf "%s,%s,%s",$1,$2,$3}' /proc/loadavg; }
trap 'status=$?; trap - EXIT; printf "supervisor_status=%s\n" "$status" >> "$evidence/completion"; exit "$status"' EXIT
trap 'exit 130' HUP INT TERM
run() {
    local label=$1 directory=$2 status=0; shift 2
    printf '%s\n' "$directory" > "$evidence/$label.cwd"
    printf '%q ' "$@" > "$evidence/$label.command"; printf '\n' >> "$evidence/$label.command"
    load > "$evidence/$label.load-before"
    (cd "$directory" && /usr/bin/time -q -f '%e\t%U\t%S\t%M\t%x' -o "$evidence/$label.time" \
        prlimit --as=4294967296 -- timeout --foreground --kill-after=10s 900 "$@") \
        > "$evidence/$label.stdout" 2> "$evidence/$label.stderr" || status=$?
    load > "$evidence/$label.load-after"
    printf '%s\n' "$status" > "$evidence/$label.status"
    printf '%s status=%s time=%s load=%s->%s\n' "$label" "$status" "$(cat "$evidence/$label.time")" \
        "$(cat "$evidence/$label.load-before")" "$(cat "$evidence/$label.load-after")"
    return "$status"
}
sha256sum bench/compiler-query-container.sh bench/compiler-query-container-compare.pl \
    bench/compiler-query-container-census.pl target/compiler-container-baseline target/compiler-container-inline64-g2 \
    > "$evidence/inputs.sha256"
run bash-syntax "$repository" bash -c 'for file in "$@"; do bash -n "$file" || exit; done' _ bench/compiler-query-container.sh \
    tools/query-store-test.sh tools/semantic-query-test.sh tools/cache-read-test.sh \
    tools/cache-io-test.sh tools/semantic-index-page-test.sh
run comparator-syntax "$repository" perl -c bench/compiler-query-container-compare.pl
run census-syntax "$repository" perl -c bench/compiler-query-container-census.pl
umask 077
fixture=$(mktemp -d "$repository/target/compiler-container-helper-run.XXXXXX")
key_directory=$(mktemp -d /tmp/compiler-container-helper-key.XXXXXX)
printf '%s\n' "$fixture" > "$evidence/fixture.path"
printf '%s\n' "$key_directory" > "$fixture/owner-key"
printf '%s\n' "$repository" > "$fixture/owner-repository"
printf '%s\n' "$evidence" > "$fixture/owner-evidence"
printf '%s\n' "$fixture" > "$key_directory/owner-run"
head -c 32 /dev/urandom > "$key_directory/key"
key="$key_directory/key"
mkdir "$fixture/source" "$fixture/store-baseline" "$fixture/store-candidate"
cat > "$fixture/source/main.lkj" <<'SOURCE'
mod edit {
    pub fn target(value: u64) -> u64 { value ^ 1u64 }
    pub fn other(value: u64) -> u64 { value + 2u64 }
}
fn main() -> u64 { edit::target(edit::other(40u64)) }
SOURCE
cp -a library "$fixture/library"
(cd "$fixture" && find source library -type f -print0 | sort -z | xargs -0 sha256sum) > "$evidence/source-inputs.sha256"
a=adc0b26f2c6db38710971b48ca88a6598078537959e04accbaae60a07a82ccc7
b=eed6ca32edcc182a92c4e697f4702ec685dff8e7778b0374a27552aa9c938947
for mode in baseline candidate; do
    compiler="$repository/target/compiler-container-baseline"; producer=$a
    if [[ $mode == candidate ]]; then compiler="$repository/target/compiler-container-inline64-g2"; producer=$b; fi
    [[ $(sha256sum "$compiler" | awk '{print $1}') == "$producer" ]]
    run "$mode-index" "$fixture/source" env LKJSCRIPT_STORE_IO=1 "$compiler" build main.lkj --source-dir "$fixture/source" \
        --no-library --runtime-source "$fixture/library/runtime.lkj" --store "$fixture/store-$mode" \
        --store-key "$key" --store-bytes 67108864 --inline-limit 0 --stats --index -o "$fixture/image-$mode"
    sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]\{64\}\) source=.*$/\1/p' "$evidence/$mode-index.stdout" > "$evidence/$mode.snapshot"
    [[ $(wc -c < "$evidence/$mode.snapshot") == 65 ]]
    run "$mode-census" "$fixture/source" perl "$repository/bench/compiler-query-container-census.pl" \
        "$fixture/store-$mode" "$key" 67108864
done
cmp "$fixture/image-baseline" "$fixture/image-candidate"
printf 'byte-identical native images\n' > "$evidence/image-parity"
sa=$(cat "$evidence/baseline.snapshot"); sb=$(cat "$evidence/candidate.snapshot")
run map-oracle "$fixture/source" perl "$repository/bench/compiler-query-container-compare.pl" maps \
    "$fixture/store-baseline" "$fixture/store-candidate" "$key" "$a" "$b" "$sa" "$sb"
for limit in 1 96; do
    for mode in baseline candidate; do
        compiler="$repository/target/compiler-container-baseline"; pin=$sa
        if [[ $mode == candidate ]]; then compiler="$repository/target/compiler-container-inline64-g2"; pin=$sb; fi
        run "$mode-query-$limit" "$fixture/source" "$compiler" query main.lkj --source-dir "$fixture/source" --no-library \
            --store "$fixture/store-$mode" --store-key "$key" --snapshot "$pin" --kind outline --id edit \
            --depth 1 --limit "$limit" --bytes 32768 --stats
        if [[ $limit == 1 ]]; then
            grep -Eq 'fact=continuation[[:space:]]cursor=[0-9a-f]+ facts=1 ' "$evidence/$mode-query-$limit.stdout"
        fi
    done
    run "query-parity-$limit" "$fixture/source" perl "$repository/bench/compiler-query-container-compare.pl" query \
        "$evidence/map-oracle.stdout" "$key" "$evidence/baseline-query-$limit.stdout" \
        "$evidence/candidate-query-$limit.stdout" outline edit 1
done
printf 'tiny-map-and-query-helper-validation=passed\nforced-continuation-limit=1\nnormal-bound-limit=96\nfixtures-retained=true\n' >> "$evidence/completion"
