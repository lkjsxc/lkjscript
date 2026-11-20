#!/usr/bin/env bash
# Complete UTF-8 signatures must paginate through every stored fact page.
set -euo pipefail
export LC_ALL=C
cd "$(dirname "$0")/.."
compiler=$(realpath "${1:-target/lkjscript}")
fixture=$(mktemp -d target/compiler-query-signature.XXXXXX)
finish() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n ${LKJSCRIPT_SIGNATURE_EVIDENCE:-} ]]; then
        mkdir -p "$LKJSCRIPT_SIGNATURE_EVIDENCE"
        for file in "$fixture"/*.out "$fixture"/*.err "$fixture"/*.log "$fixture"/*.expected "$fixture"/*.joined "$fixture"/*.tsv; do
            [[ -f $file ]] && cp -- "$file" "$LKJSCRIPT_SIGNATURE_EVIDENCE/"
        done
    fi
    rm -rf -- "$fixture"
    exit "$status"
}
trap finish EXIT
trap 'exit 130' HUP INT TERM
mkdir "$fixture/source" "$fixture/store"
umask 077
head -c 32 /dev/urandom > "$fixture/key"
# The fixture has ASCII and raw Greek UTF-8, but no JSON metacharacters or
# embedded newlines in either header. Extracting quoted text below is deliberately
# limited to that alphabet; this is not a general JSON decoder or a CI dependency.
{
    printf 'pub fn long_signature(/*'
    for ((part=0; part<2048; part++)); do printf 'xαβγ%04d ' "$part"; done
    printf '*/) -> i64'
} > "$fixture/long.expected"
short='pub fn short_signature() -> i64'
printf '%s' "$short" > "$fixture/short.expected"
{
    cat "$fixture/long.expected"
    printf ' { 7i64 }\n%s { 3i64 }\n' "$short"
} > "$fixture/source/signatures.lkj"
printf '%s\n' 'mod signatures; fn main() -> i64 { signatures::long_signature() + signatures::short_signature() }' > "$fixture/source/main.lkj"
expected_bytes=$(wc -c < "$fixture/long.expected")
common=(main.lkj --source-dir "$fixture/source" --no-library --store "$fixture/store" --store-key "$fixture/key")
"$compiler" build "${common[@]}" --index -o "$fixture/program" > "$fixture/build.log" 2>&1 || { cat "$fixture/build.log" >&2; exit 1; }
snapshot=$(sed -n 's/^LKJSCRIPT_SNAPSHOT \([0-9a-f]*\) .*/\1/p' "$fixture/build.log")
[[ ${#snapshot} == 64 ]] || { echo 'query-signature: missing snapshot' >&2; exit 1; }
query() {
    "$compiler" query "${common[@]}" --snapshot "$snapshot" --kind "$1" --id "$2" --depth 1 --limit 2 --bytes 4096 "${@:3}"
}
printf '"value@signatures::short_signature"\tlevel=source-claim\tfact=signature\tkind=function text="%s" bytes=%s complete=true\n' "$short" "${#short}" > "$fixture/short-line.expected"
query signature signatures::short_signature > "$fixture/short.out" 2> "$fixture/short.err"
awk -F '\t' '$3 == "fact=signature"' "$fixture/short.out" > "$fixture/short-line.joined"
cmp "$fixture/short-line.expected" "$fixture/short-line.joined"
fail() { printf 'query-signature: %s\n' "$*" >&2; }
walk() {
    local kind=$1 identity=$2 cursor= page=0 seen=0 total=0 adjusted=0 short_count=0 complete_count=0
    local out line continuation declared facts fragment chunk chunks bytes complete lead length
    local pattern='^"value@signatures::long_signature"'$'\t''level=source-claim'$'\t''fact=signature'$'\t''kind=function chunk=([0-9a-f]{16}) chunks=([0-9]+) text="([^"\\]*)" bytes=([0-9]+) complete=(true|false)$'
    local short_line
    short_line=$(cat "$fixture/short-line.expected") || return 1
    : > "$fixture/$kind.joined"
    printf 'response\tbytes\tfacts\tlong_chunks_seen\tcursor\n' > "$fixture/$kind.tsv"
    while ((page < 128)); do
        out="$fixture/$kind.$page.out"
        local extra=()
        if [[ -n $cursor ]]; then extra=(--cursor "$cursor"); fi
        query "$kind" "$identity" "${extra[@]}" > "$out" 2> "$fixture/$kind.$page.err" || { cat "$fixture/$kind.$page.err" >&2; return 1; }
        length=$(wc -c < "$out") || return 1
        ((length <= 4096)) || { fail "$kind exceeded the response byte bound"; return 1; }
        facts=$(awk -F '\t' '$3 == "fact=signature" { n++ } END { print n+0 }' "$out") || return 1
        declared=$(sed -n 's/.*cursor=[^ ]* facts=\([0-9]*\) .*/\1/p' "$out") || return 1
        [[ $declared == "$facts" ]] && ((facts <= 2)) || { fail "$kind fact bound/count"; return 1; }
        while IFS= read -r line; do
            [[ $line == *$'\tfact=signature\t'* ]] || continue
            if [[ $line == "$short_line" && $kind == outline ]]; then
                short_count=$((short_count + 1))
                continue
            fi
            [[ $line =~ $pattern ]] || { fail "$kind missing complete chunk metadata"; return 1; }
            chunk=${BASH_REMATCH[1]}
            chunks=${BASH_REMATCH[2]}
            fragment=${BASH_REMATCH[3]}
            bytes=${BASH_REMATCH[4]}
            complete=${BASH_REMATCH[5]}
            ((16#$chunk == seen)) || { fail "$kind chunk order or duplication"; return 1; }
            if ((seen == 0)); then total=$chunks; fi
            ((total > 16 && chunks == total && bytes == expected_bytes)) || { fail "$kind total metadata"; return 1; }
            length=${#fragment}
            ((length > 0 && length <= 1024)) || { fail "$kind fragment byte bound"; return 1; }
            printf -v lead '%d' "'${fragment:0:1}"
            ((lead < 128 || lead > 191)) || { fail "$kind split a UTF-8 sequence"; return 1; }
            if ((seen + 1 == total)); then
                [[ $complete == true ]] || { fail "$kind final chunk is not complete"; return 1; }
                complete_count=$((complete_count + 1))
            else
                [[ $complete == false ]] || { fail "$kind premature completeness"; return 1; }
                if ((length < 1024)); then adjusted=$((adjusted + 1)); fi
            fi
            printf '%s' "$fragment" >> "$fixture/$kind.joined"
            seen=$((seen + 1))
        done < "$out"
        continuation=$(tail -n 1 "$out")
        [[ $continuation == *$'\tfact=continuation\tcursor='* ]] || { fail "$kind missing final continuation"; return 1; }
        cursor=${continuation#*cursor=}
        cursor=${cursor%% *}
        printf '%s\t%s\t%s\t%s\t%s\n' "$page" "$(wc -c < "$out")" "$facts" "$seen" "$cursor" >> "$fixture/$kind.tsv"
        page=$((page + 1))
        if [[ $cursor == none ]]; then
            ((seen == total && total > 16 && complete_count == 1 && adjusted > 0 && page > 1)) || { fail "$kind ended before all UTF-8 chunks"; return 1; }
            if [[ $kind == outline ]]; then
                ((short_count == 1)) || { fail 'outline changed the short signature'; return 1; }
            fi
            cmp "$fixture/long.expected" "$fixture/$kind.joined" || { fail "$kind dropped or changed signature bytes"; return 1; }
            printf 'ok query-signature: %s chunks=%s responses=%s bytes=%s UTF8-adjusted=%s\n' "$kind" "$seen" "$page" "$expected_bytes" "$adjusted"
            return 0
        fi
        [[ $cursor =~ ^[0-9a-f]+$ ]] || { fail "$kind invalid continuation"; return 1; }
    done
    fail "$kind continuation did not finish"
    return 1
}
status=0
walk signature signatures::long_signature || status=1
walk outline signatures || status=1
exit "$status"
