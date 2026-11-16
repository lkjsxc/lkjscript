#!/usr/bin/env bash
# A final memo write must not advertise a snapshot it evicted.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
fixture=$(mktemp -d target/query-publication.XXXXXX)
finish() {
    local result=$?
    trap - EXIT HUP INT TERM
    if [[ -n "${LKJSCRIPT_QUERY_PUBLICATION_EVIDENCE:-}" ]]; then
        mkdir -p "$LKJSCRIPT_QUERY_PUBLICATION_EVIDENCE"
        for evidence in "$fixture"/*.out "$fixture"/*.err "$fixture"/*.tsv; do
            [[ ! -f "$evidence" ]] || cp "$evidence" "$LKJSCRIPT_QUERY_PUBLICATION_EVIDENCE/"
        done
    fi
    rm -rf "$fixture"
    exit "$result"
}
trap finish EXIT HUP INT TERM
umask 077
mkdir "$fixture/source" "$fixture/roomy" "$fixture/tight"
head -c 32 /dev/urandom > "$fixture/key"
cat > "$fixture/source/main.lkj" <<'SOURCE'
pub fn main() -> u64 { 42u64 }
SOURCE
fail() { printf 'query-publication: %s\n' "$*" >&2; exit 1; }
timed() {
    local label=$1
    shift
    local status
    printf '%s\tload-before\t%s\n' "$label" "$(cat /proc/loadavg)" >> "$fixture/timing.tsv"
    # Minimal CI images need no separate time package. The explicit switch
    # exercises exactly the same fallback without changing the host.
    if [[ -x /usr/bin/time && ${LKJSCRIPT_QUERY_PUBLICATION_NO_TIME:-0} != 1 ]]; then
        if /usr/bin/time -f 'wall=%e user=%U sys=%S peak-kib=%M' -o "$fixture/$label.time" \
            "$@" > "$fixture/$label.out" 2> "$fixture/$label.err"; then
            status=0
        else
            status=$?
        fi
    else
        local started elapsed
        started=$(date +%s%N)
        if "$@" > "$fixture/$label.out" 2> "$fixture/$label.err"; then
            status=0
        else
            status=$?
        fi
        elapsed=$(($(date +%s%N) - started))
        printf 'wall=%d.%09d user=NA sys=NA peak-kib=NA\n' \
            "$((elapsed / 1000000000))" "$((elapsed % 1000000000))" > "$fixture/$label.time"
    fi
    printf '%s\tresult\t%s\t%s\n' "$label" "$status" "$(cat "$fixture/$label.time")" >> "$fixture/timing.tsv"
    printf '%s\tload-after\t%s\n' "$label" "$(cat /proc/loadavg)" >> "$fixture/timing.tsv"
    return "$status"
}
inventory() {
    find "$1" -maxdepth 1 -type f ! -name '.budget-lock' -printf '%f\t%s\n' | LC_ALL=C sort
}
common=(main.lkj --source-dir "$fixture/source" --no-library --store-key "$fixture/key")
printf 'compiler-sha256\t%s\n' "$(sha256sum "$compiler" | cut -d' ' -f1)" > "$fixture/boundary.tsv"
timed roomy-build "$compiler" build "${common[@]}" --store "$fixture/roomy" \
    --store-bytes 67108864 --index -o "$fixture/program" || fail 'roomy indexed build failed'
snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/roomy-build.out")
[[ ${#snapshot} == 64 ]] || fail 'roomy build omitted its snapshot'
timed roomy-query "$compiler" query "${common[@]}" --store "$fixture/roomy" \
    --snapshot "$snapshot" --kind meta --id crate || fail 'roomy snapshot was unusable'
inventory "$fixture/roomy" > "$fixture/roomy-inventory.tsv"

# Budget v2 charges each entry's encoded name, explicit 64-byte owner,
# three length/size words, and its complete envelope. With an empty store,
# checkpoint first writes .budget-index after these two semantic memo puts.
# Removing the final inputs-semantic entry gives the exact preceding boundary;
# this is one derived cap, not a search for a favorable failing capacity.
mapfile -t plans < <(find "$fixture/roomy" -maxdepth 1 -type f -name 'inputs-semantic-*' -printf '%f\n')
[[ ${#plans[@]} == 1 ]] || fail 'expected one final semantic input plan'
plan=${plans[0]}
plan_bytes=$(stat -c '%s' "$fixture/roomy/$plan")
plan_charge=$((plan_bytes + 24 + ${#plan} + 64))
total=$(awk '{ sum += $2 } END { printf "%.0f\n", sum }' "$fixture/roomy-inventory.tsv")
capacity=$((total - plan_charge))
[[ $capacity -gt $plan_charge ]] || fail 'fixture does not isolate the last memo write'
printf 'roomy-bytes\t%s\nlast-plan-bytes\t%s\nlast-plan-charge\t%s\nexact-capacity\t%s\n' \
    "$total" "$plan_bytes" "$plan_charge" "$capacity" >> "$fixture/boundary.tsv"

if timed tight-build "$compiler" build "${common[@]}" --store "$fixture/tight" \
    --store-bytes "$capacity" --index -o "$fixture/program"; then
    status=0
else
    status=$?
fi
inventory "$fixture/tight" > "$fixture/tight-inventory.tsv"
# This record proves that publication reached the final memo put. A refusal
# during the earlier page/locator publication does not satisfy this regression.
[[ -f "$fixture/tight/$plan" ]] || fail 'tight build stopped before the final semantic memo write'
[[ $(stat -c '%s' "$fixture/tight/$plan") == "$plan_bytes" ]] || fail 'final semantic memo shape changed'
if [[ $status == 0 ]]; then
    advertised=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/tight-build.out")
    [[ ${#advertised} == 64 ]] || fail 'tight build succeeded without a snapshot or refusal'
    if timed lost-query "$compiler" query "${common[@]}" --store "$fixture/tight" \
        --snapshot "$advertised" --kind meta --id crate; then
        fail 'tight cap no longer reaches the intended eviction boundary'
    fi
    [[ ! -s "$fixture/lost-query.out" ]] || fail 'missing snapshot query leaked output'
    grep -q 'E_QUERY_SNAPSHOT' "$fixture/lost-query.err" || fail 'lost snapshot failed for another reason'
    fail 'build advertised a snapshot evicted by its final semantic memo write'
fi
[[ ! -s "$fixture/tight-build.out" ]] || fail 'capacity refusal printed a snapshot or partial answer'
grep -q 'E_QUERY_STORE' "$fixture/tight-build.err" || {
    cat "$fixture/tight-build.err" >&2
    fail 'tight build did not give the store publication refusal'
}
printf 'query-publication: passed at exact %s-byte boundary\n' "$capacity"
cat "$fixture/timing.tsv"
