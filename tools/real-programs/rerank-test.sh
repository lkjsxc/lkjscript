#!/usr/bin/env bash
# Offline exact-site controls. Fixtures are authored here, while the complete
# stdout/report byte digests were obtained independently from the predecessor
# at dec3f7ba727fd4ef597acd4a067c6961b2b0fc56.
# Report digests include the census port's wording-only change, applied
# independently to the original verified Node oracle bytes.
set -euo pipefail
export LC_ALL=C
umask 077
root=$(cd -- "$(dirname -- "$0")/../.." && pwd)
compiler=${1:-"$root/target/lkjscript"}
case "$compiler" in /*) ;; *) compiler="$PWD/$compiler" ;; esac
mkdir -p "$root/target"
work=$(mktemp -d "$root/target/rerank-test.XXXXXX")
cleanup() {
    local status=$?
    trap - EXIT
    if ((status)); then
        for log in "$work/build.err" "$work/stdout" "$work/stderr"; do
            [[ ! -f "$log" ]] || { printf '%s\n' "$log" >&2; cat "$log" >&2; }
        done
    fi
    rm -rf -- "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
(cd "$root" && "$compiler" build tools/real-programs/rerank.lkj --source-dir . --library-dir library --no-store -o "$work/rerank") >"$work/build.out" 2>"$work/build.err"
cd "$work"
mkdir -p case/source/src
hash() { sha256sum "$1" | cut -d ' ' -f 1; }
hash_text() { printf '%s' "$1" | sha256sum | cut -d ' ' -f 1; }
length() { printf '%s' "$1" | wc -c; }
# All fixture text streams are ASCII; the non-ASCII stream uses base64.
quote() {
    local t=$1
    t=${t//\\/\\\\}
    t=${t//\"/\\\"}
    t=${t//$'\n'/\\n}
    t=${t//$'\r'/\\r}
    t=${t//$'\t'/\\t}
    printf '"%s"' "$t"
}
captured() {
    local t=$1
    if [[ ${2:-text} == base64 ]]; then
        printf '{"base64":"%s","bytes":%d,"sha256":"%s"}' "$(printf '%s' "$t" | base64 -w0)" "$(length "$t")" "$(hash_text "$t")"
    else
        printf '{"bytes":%d,"sha256":"%s","text":%s}' "$(length "$t")" "$(hash_text "$t")" "$(quote "$t")"
    fi
}
source=$'fn a() -> impl Value {}\nfn b(x: Qualified) {}\nfn c() {} // é 🦀\n'
printf '%s' "$source" > case/source/src/lib.rs
source_sha=$(hash case/source/src/lib.rs)
source_bytes=$(length "$source")
commit=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
compiler_sha=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
bad_sha=ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
file_entry() { printf '{"bytes":%d,"path":"src/%s.rs","sha256":"%s"}' "$source_bytes" "$1" "$source_sha"; }
projection() {
    local aliases='{"from":"src/lib.rs","to":"src/lib.lkj"}' files
    files=$(file_entry lib)
    if [[ ${1:-single} == multiple ]]; then
        aliases+=',{"from":"src/other.rs","to":"src/other.lkj"}'
        files+=",$(file_entry other)"
    fi
    printf '{"aliases":[%s],"files":[%s],"kind":"git_tracked_suffix_projection","schema":1,"source":{"commit":"%s","root":"case/source"}}' "$aliases" "$files" "$commit" > case/projection.json
    manifest_sha=$(hash case/projection.json)
}
projection
empty=$(captured '')
time_report=$(captured $'0\n')
original_stderr=$(captured $'src/lib.rs: E_SOURCE_EXTENSION source files use .lkj\n')
observation() {
    local route=$1 stderr=$2 entry=$3 cmd=$4 signal=${5:-0}
    printf '{"command":%s,"entry_bytes":%d,"entry_sha256":"%s","exit_status":1,"launcher_signal":%s,"route":"%s","stderr":%s,"stdout":%s,"time_report":%s,"upstream_entry":"src/%s.rs"}' \
        "$cmd" "$source_bytes" "$source_sha" "$signal" "$route" "$stderr" "$empty" "$time_report" "$entry"
}
original() {
    local entry=${1:-lib}
    observation original_suffix "$original_stderr" "$entry" "[\"/compiler\",\"check\",\"src/$entry.rs\",\"--source-dir\",\"/captured/source\"]"
}
projected() {
    local stream=$1 entry=${2:-lib} cmd=${3:-"[\"/compiler\",\"check\",\"src/${2:-lib}.lkj\",\"--source-dir\",\"/captured/projected\"]"}
    observation suffix_projection "$stream" "$entry" "$cmd" "${4:-0}"
}
receipt() {
    local IFS=,
    printf '{"compiler":{"path":"/compiler","sha256":"%s"},"kind":"unmodified_real_program_root_observations","observations":[%s],"package":{"commit":"%s"},"projection_manifest_sha256":"%s","schema":1,"verification_after":{"manifest_sha256":"%s"},"verification_before":{"manifest_sha256":"%s"}}' \
        "$compiler_sha" "$*" "$commit" "$manifest_sha" "$manifest_sha" "$manifest_sha"
}
d0='src/lib.lkj:1:11: E_UNSUPPORTED unsupported construct: impl'
d1='src/lib.lkj:2:9: E_PARSE expected an identifier'
d2='src/lib.lkj:1:16: E_UNSUPPORTED where predicate on a nonparameter'
d3='src/lib.lkj:1:1: E_BLOCKED remaining semantic analysis: source errors'
d4='src/lib.lkj:3:1: E_UNSUPPORTED unsupported construct: impl'
diag="$d0"$'\n'"$d1"$'\n'"$d2"$'\n'"$d3"$'\n'
new_diag="$diag$d4"$'\n'
nom_receipt() { receipt "$(original)" "$(projected "$(captured "$diag")")"; }
nom_table() {
    printf '{"accounting":{"unique_primary_sites":2},"causes":[{"id":"impl_trait","owner":"Language","rank":1,"sites":[["src/lib.rs",1,11,null,false]],"title":"impl Trait"},{"id":"qualified","owner":"Language","rank":2,"sites":[["src/lib.rs",2,9,null,false]],"title":"Qualified path"}],"current_verification":{"receipt_sha256":"%s"},"recovery_exclusions":[{"diagnostic_indices":[2],"id":"known_recovery","parent_diagnostic_indices":[0]}]}' "$(hash case/before.json)" > case/baseline.json
}
nom_receipt > case/before.json
nom_table
cp case/before.json original-receipt
cp case/baseline.json original-table
make_after() { receipt "$(original)" "$(projected "$@")" > case/after.json; }
with_diagnostics() { make_after "$(captured "$1")"; }

# The independent predecessor's whole-byte oracle is stronger than checking
# individual counters. Each row is: case ID, stdout SHA-256, report SHA-256.
declare -A stdout_sha report_sha
while read -r id out report; do
    stdout_sha[$id]=$out
    report_sha[$id]=$report
done <<'GOLDENS'
0 fb3999f62374c7726aff9e355edcfcac869f96e20d4ad341b36ba26cc0e73226 02e77ae526f0b91d366adb1125cf73800c06938907325fd16c5b81bb18e1e8bf
1 b7e5b8bebde9740a5e3e774f2c50da569b14d76b1e73bc8c3ac93af885bfc4f3 48c7719ea19b97764fb0c2a84549a1a93c912737622df90f3b25e0717e67b59b
2 fe5533999c7c8eabe5b9b707a207511cb985b469a2b7307f609373ba10848d4d 8e6887d32c005f25f7a01e994ef5ae7b54fb3151a25b939267f4e5833f67d687
9 763019c17551402e6b0245d6b49cea203955536aea4f6416f68e12ffdb172ec3 1058c24f5f7fa6218b7117aa74ece8f9d6a14f098d7c4119f02a3e702d26df60
10 2ef2dd617efacb58e58dd326df5caab4d13f345912b1da5b10cc3c3e98a0fe40 33879a882236ed8bb5837d96dc076a7be553890c116d954ef08fae7f1db3f797
11 7916452e0b9b943aacecc7fcb10c069af761ed251688af8f847d65bb681d3ecd ae982ce73f862760565e72296e82be85b9d941f3c4ed11c789421aff7edd789c
12 ac0eaa8d4522b072f2defed2be6c8086004473a748c5aedbd1b195ca02bf1520 6b4f680afb87ca06c97d3f397c6848547e0cbb12f5b695c70394cd3abe0f377b
13 38ca4fec7201cbb68a54af7824bb4c1abb95b7d6e4cf19b5e4e1f77de8aa0e69 47a432abe9d01f452ee513e112a8d43d033b29d2ef7be5a3a374ff1763ac8d30
14 6678e5040a1b090cb361edc9f25387bf2c64201d0200b3ad9e29903731c49517 5ea0234c73f1b33ba4754be0f51a38f41f3c40f0e16137b10fe5a1839dbb8e3b
27 8dc4ef33e0c30d19f1e20e4586413b9efe03cef4e6cb9dac480b5e9aea2e4736 94ddf9e7f3dcf60f5e919bf6ad6e26451df0787fd9fad1f0631fd65bb2741f59
28 5484818efec9811c55efd9766c7f8496ba3ebaefa57da5db2de463a6e4e34ae2 4e0ceda46d05fd0d478a188d3db8795e063196e4737d5314a338d73c2ccee99d
29 6e776701f2d91fbafe0b9555428e0552aad0aad46fea239af388c974528fa2c1 209ab5d52ebf672e15f4461bbfe3dd29161a4145b105b32f0fd0f4797b35035b
30 0ba64046424f294bb8f9924f0bf2325753be39d7ef172002850991461fe15a90 02e77ae526f0b91d366adb1125cf73800c06938907325fd16c5b81bb18e1e8bf
GOLDENS
count=0
invoke() {
    local id=$1 label=$2 expected=$3; shift 3
    local status=0
    local args=(--baseline case/baseline.json --before case/before.json --after case/after.json --projection case/projection.json --output "case/report-$id.json")
    if [[ $id != 30 ]]; then args+=(--source case/source); fi
    timeout --kill-after=2s 20s "$work/rerank" "${args[@]}" "$@" >stdout 2>stderr || status=$?
    if [[ $expected == pass ]]; then
        [[ $status == 0 && ! -s stderr && -f case/report-$id.json ]] ||
            { printf '%s: expected successful report (status %s)\n' "$label" "$status" >&2; exit 1; }
        [[ $(hash stdout) == "${stdout_sha[$id]}" && $(hash "case/report-$id.json") == "${report_sha[$id]}" ]] ||
            { printf '%s: stdout/report byte mismatch\n' "$label" >&2; exit 1; }
        [[ $(stat -c %a "case/report-$id.json") == 600 ]] ||
            { printf '%s: report is not private\n' "$label" >&2; exit 1; }
    else
        [[ $status == 255 && ! -s stdout && ! -e case/report-$id.json && ! -L case/report-$id.json ]] &&
            printf 'real-programs-rerank: %s\n' "$expected" | cmp -s - stderr ||
            { printf '%s: incorrect refusal (status %s; expected %s)\n' "$label" "$status" "$expected" >&2; exit 1; }
    fi
    count=$((count + 1))
}
cp case/before.json case/after.json
invoke 0 'unchanged nom; primary, recovery and E_BLOCKED' pass
invoke 30 'default manifest source' pass
make_after "$(captured "$diag"$'src/lib.lkj:3:1: E_PARSE é 🦀\nLKJSCRIPT_EDITS {"x":"é"}\n' base64)"
invoke 1 'non-ASCII source and diagnostics, base64, edit annotation' pass
absolute=''
for d in "$d0" "$d1" "$d2" "$d3"; do absolute+="/captured/projected/$d"$'\n'; done
with_diagnostics "$absolute"
invoke 2 'authenticated absolute projected aliases' pass
with_diagnostics '/captured/projected-other/src/lib.lkj:1:11: E_UNSUPPORTED unsupported construct: impl'$'\n'
invoke 3 'adjacent absolute root' 'diagnostic is outside the tracked original source: /captured/projected-other/src/lib.lkj'
with_diagnostics '/captured/projected/../projected/src/lib.lkj:1:11: E_UNSUPPORTED unsupported construct: impl'$'\n'
invoke 4 'traversal' 'diagnostic is outside the tracked original source: ../projected/src/lib.lkj'
with_diagnostics '/captured/source/src/lib.rs:1:11: E_UNSUPPORTED unsupported construct: impl'$'\n'
invoke 5 'absolute original path' 'diagnostic is outside the tracked original source: /captured/source/src/lib.rs'
with_diagnostics '/captured/projected/src/untracked.lkj:1:11: E_UNSUPPORTED unsupported construct: impl'$'\n'
invoke 6 'untracked absolute alias' 'diagnostic is outside the tracked original source: src/untracked.lkj'
make_after "$(captured "$absolute")" lib '["/compiler","check","src/lib.lkj","--source-dir","/captured/forged-projection"]'
invoke 7 'after receipt cannot replace authenticated source-dir' 'probe command changed beyond compiler path'
no_root='["/compiler","check","src/lib.lkj"]'
make_after "$(captured "$diag")" lib "$no_root"
cp case/after.json case/before.json
nom_table
make_after "$(captured "$absolute")" lib "$no_root"
invoke 8 'legacy receipt cannot authenticate absolute paths' 'diagnostic is outside the tracked original source: /captured/projected/src/lib.lkj'
cp original-receipt case/before.json
cp original-table case/baseline.json
with_diagnostics "$d1"$'\n'"$d3"$'\n'
invoke 9 'missing diagnostic is not observed, not admitted' pass
with_diagnostics "$d1"$'\n'"$d2"$'\n'"$d3"$'\n'
invoke 10 'recovery without surviving parent is unclassified' pass
with_diagnostics 'src/lib.lkj:1:11: E_TYPE different frontier'$'\n'"$d1"$'\n'"$d3"$'\n'
invoke 11 'changed diagnostic at old coordinate is unclassified' pass
with_diagnostics "$new_diag"
invoke 12 'same words at new site do not inherit cause' pass
review() {
    local sha=$1 disposition=${2:-} parent=${3:-}
    printf '{"schema":1,"sites":[{"cause":"new_reviewed_form","code":"E_UNSUPPORTED","column":1%s,"file_sha256":"%s","line":3,"message":"unsupported construct: impl","owner":"Language"%s,"path":"src/lib.rs","reason":"Fixture source review","title":"Reviewed form"}]}' "$disposition" "$sha" "$parent" > case/review.json
}
review "$source_sha"
invoke 13 'explicit primary review' pass --review case/review.json
parent=',"parent":{"code":"E_UNSUPPORTED","column":11,"line":1,"message":"unsupported construct: impl","path":"src/lib.rs"}'
disposition=',"disposition":"recovery"'
review "$source_sha" "$disposition" "$parent"
invoke 14 'explicit recovery review' pass --review case/review.json
review "$source_sha" "$disposition" "${parent/unsupported construct: impl/unobserved parent}"
invoke 15 'invented recovery parent' 'reviewed recovery parent does not match another observed diagnostic' --review case/review.json
review "$source_sha" "$disposition" ',"parent":{"code":"E_UNSUPPORTED","column":1,"line":3,"message":"unsupported construct: impl","path":"src/lib.rs"}'
invoke 16 'self-referential recovery parent' 'reviewed recovery parent does not match another observed diagnostic' --review case/review.json
review "$source_sha" "$disposition" "$parent"
with_diagnostics "$d4"$'\n'"$diag"
invoke 17 'later recovery parent' 'reviewed recovery parent is not earlier in the same root' --review case/review.json
with_diagnostics "$new_diag"
review "$bad_sha"
invoke 18 'review unrelated source bytes' 'review source hash mismatch' --review case/review.json
good_stream=$(captured "$diag")
stream_sha=$(hash_text "$diag")
make_after "${good_stream/$stream_sha/$bad_sha}"
invoke 19 'captured stream SHA-256' 'captured stream byte/hash mismatch'
make_after "$good_stream" lib '["/compiler","check","src/lib.lkj","--source-dir","/captured/projected"]' 9
invoke 20 'launcher signal' 'root observation ended abnormally'
receipt "$(original)" > case/after.json
invoke 21 'missing observation pair' 'incomplete original/projection observation pair: src/lib.rs'
make_after "$good_stream" lib '["/compiler","check","src/lib.lkj","--source-dir","/captured/projected","--changed"]'
invoke 22 'changed command' 'probe command changed beyond compiler path'
with_diagnostics $'unknown error\n'
invoke 23 'unparsed diagnostic' 'unparsed diagnostic line (must be reviewed): unknown error'
with_diagnostics $'src/lib.lkj:1:999: E_PARSE wrong\n'
invoke 24 'invalid source coordinate' 'diagnostic is outside original source coordinates: src/lib.rs:1:999'
with_diagnostics $'other.lkj:1:1: E_PARSE wrong\n'
invoke 25 'untracked relative path' 'diagnostic is outside the tracked original source: other.lkj'
cp original-receipt case/after.json
printf '// changed' >> case/source/src/lib.rs
invoke 26 'changed original source' 'original source bytes changed: src/lib.rs'
printf '%s' "$source" > case/source/src/lib.rs
duplicate='src/lib.lkj:1:11: E_UNSUPPORTED unsupported construct: impl item'
with_diagnostics "$d0"$'\n'"$d1"$'\n'"$duplicate"$'\n'"$d3"$'\n'
cp case/after.json case/before.json
printf '{"causes":[{"cause":"impl Trait","id":"impl_trait","owner":"Language","rank":1},{"cause":"Qualified path","id":"qualified","owner":"Language","rank":2}],"counts":{"unique_primary_sites":2},"current_verification":{"receipt_sha256":"%s"},"primary_sites":[["src/lib.rs:1:11","impl_trait","E_UNSUPPORTED","observed_cause"],["src/lib.rs:2:9","qualified","E_PARSE","observed_cause"]],"recovery_duplicate_messages":[{"coordinate":"src/lib.rs:1:11","primary_code":"E_UNSUPPORTED","primary_message":"unsupported construct: impl","secondary_code":"E_UNSUPPORTED","secondary_message":"unsupported construct: impl item"}]}' "$(hash case/before.json)" > case/baseline.json
invoke 27 'DOM primary/recovery split' pass
with_diagnostics "$d1"$'\n'"$duplicate"$'\n'"$d3"$'\n'
invoke 28 'DOM duplicate without former primary is not hidden' pass
cp case/source/src/lib.rs case/source/src/other.rs
projection multiple
stream=$(captured "$d0"$'\n'"$d1"$'\n')
receipt "$(original)" "$(projected "$stream")" "$(original other)" "$(projected "$stream" other)" > case/before.json
cp case/before.json case/after.json
printf '{"current_verification":{"receipt_sha256":"%s"},"groups":{"impl_trait":{"diagnosed_sites":1,"diagnostic_kinds":["E_UNSUPPORTED: unsupported construct: impl"],"independent_gap_sites":1,"owner":"Language","rank":1,"sites":[["src/lib.rs",1,11,0,"src/lib.rs"]]},"parse_recovery_or_unclassified":{"diagnosed_sites":1,"diagnostic_kinds":["E_PARSE: expected an identifier"],"independent_gap_sites":0,"owner":"Unknown","rank":2,"sites":[["src/lib.rs",2,9,0,"src/lib.rs"]]}}}' "$(hash case/before.json)" > case/baseline.json
invoke 29 'Sozu repeated-root sites and prior unclassified counts' pass

# Additional fail-closed controls, including malformed base64 that the
# predecessor's permissive Buffer decoder would silently accept.
projection
cp original-receipt case/before.json
cp original-table case/baseline.json
make_after 'null'
invoke 31 'missing stream' 'missing captured stream'
make_after '[]'
invoke 32 'array stream' 'missing captured stream'
make_after '{}'
invoke 33 'no text or base64' 'captured stream has neither text nor base64'
make_after '{"text":9}'
invoke 34 'non-string text' 'invalid UTF-8 diagnostic stream'
make_after '{"base64":9}'
invoke 35 'non-string base64' 'invalid base64 diagnostic stream'
base=$(captured "$diag" base64)
make_after "${base/\"base64\":\"/\"base64\":\"!}"
invoke 36 'malformed base64 with unchanged claimed digest' 'invalid base64 diagnostic stream'
make_after "{\"base64\":\"Zh==\",\"bytes\":1,\"sha256\":\"$(hash_text f)\"}"
invoke 37 'nonzero unused padding bits' 'invalid base64 diagnostic stream'
make_after "{\"base64\":\"/w==\",\"bytes\":1,\"sha256\":\"$(printf '\377' | sha256sum | cut -d ' ' -f 1)\"}"
invoke 38 'invalid UTF-8 decoded stream' 'invalid UTF-8 diagnostic stream'
make_after "${good_stream/\"bytes\":244/\"bytes\":245}"
invoke 39 'wrong stream byte length' 'captured stream byte/hash mismatch'
for field in projection_manifest_sha256 verification_before verification_after; do
    # Replace just the intended digest, not the other two copies.
    case "$field" in
        projection_manifest_sha256) from="\"$field\":\"$manifest_sha\""; to="\"$field\":\"$bad_sha\"";;
        *) from="\"$field\":{\"manifest_sha256\":\"$manifest_sha\""; to="\"$field\":{\"manifest_sha256\":\"$bad_sha\"";;
    esac
    before=$(cat original-receipt)
    printf '%s' "${before/"$from"/"$to"}" > case/after.json
    invoke "digest-$field" "$field mismatch" 'receipt/projection identity mismatch'
done
make_after "$good_stream" lib '["/compiler","check","src/lib.lkj","--source-dir"]'
# A malformed command in before must refuse before being used as a prefix.
cp case/after.json case/before.json
nom_table
invoke 40 'missing source-dir value' 'captured source-dir is missing its value'
make_after "$good_stream" lib '["/compiler","check","src/lib.lkj","--source-dir","/captured/projected","--source-dir","/other"]'
cp case/after.json case/before.json
nom_table
invoke 41 'ambiguous source-dir' 'ambiguous captured source-dir'
cp original-receipt case/before.json
cp original-table case/baseline.json
cp original-receipt case/after.json
printf '\377' > case/baseline.json
invoke 42 'invalid UTF-8 JSON' 'invalid JSON case/baseline.json: real-programs-rerank: invalid UTF-8 case/baseline.json'
cp original-table case/baseline.json
# Reject malformed command/entry identity and authenticate every captured stream.
make_after "$good_stream"
after=$(cat case/after.json)
command='["/compiler","check","src/lib.lkj","--source-dir","/captured/projected"]'
printf '%s' "${after/"\"command\":$command"/"\"command\":{}"}" > case/after.json
invoke 43 'non-array projected command' 'missing captured probe command'
printf '%s' "${after/\"entry_bytes\":67/\"entry_bytes\":68}" > case/after.json
invoke 44 'wrong entry byte length' 'probe entry does not match a tracked original file'
printf '%s' "${after/\"entry_sha256\":\"$source_sha\"/\"entry_sha256\":\"$bad_sha\"}" > case/after.json
invoke 45 'wrong entry digest' 'probe entry does not match a tracked original file'
printf '%s' "${after/"\"stdout\":$empty"/"\"stdout\":null"}" > case/after.json
invoke 46 'missing stdout capture' 'missing captured stream'
printf '%s' "${after/"\"time_report\":$time_report"/"\"time_report\":{}"}" > case/after.json
invoke 47 'missing time_report capture' 'captured stream has neither text nor base64'
with_diagnostics "$diag"$'LKJSCRIPT_EDITS []\n'
invoke 48 'array edit annotation' 'malformed diagnostic edit annotation'
with_diagnostics $'src/lib.lkj:1:+11: E_PARSE wrong\n'
invoke 49 'non-decimal coordinate' 'unparsed diagnostic line (must be reviewed): src/lib.lkj:1:+11: E_PARSE wrong'
# A before-receipt digest mismatch must refuse even if the table binds it.
cp original-receipt case/after.json
before=$(cat original-receipt)
printf '%s' "${before/\"projection_manifest_sha256\":\"$manifest_sha\"/\"projection_manifest_sha256\":\"$bad_sha\"}" > case/before.json
nom_table
invoke 50 'before projection binding' 'receipt/projection identity mismatch'
cp original-receipt case/before.json
cp original-table case/baseline.json
# Invalid source UTF-8 in a line without a diagnostic is also forbidden.
printf '%s\377' "$source" > case/source/src/lib.rs
source_sha=$(hash case/source/src/lib.rs)
source_bytes=$(wc -c < case/source/src/lib.rs)
projection
nom_receipt > case/before.json
nom_table
cp case/before.json case/after.json
invoke 51 'invalid UTF-8 anywhere in verified original source' 'invalid UTF-8 src/lib.rs'
printf '%s' "$source" > case/source/src/lib.rs
source_sha=$(hash case/source/src/lib.rs)
source_bytes=$(length "$source")
projection
cp original-receipt case/before.json
cp original-table case/baseline.json
cp original-receipt case/after.json
# A BOM is not stripped before JSON parsing.
{ printf '\357\273\277'; cat original-table; } > case/baseline.json
status=0
"$work/rerank" --baseline case/baseline.json --before case/before.json --after case/after.json --projection case/projection.json --output case/report-bom.json >stdout 2>stderr || status=$?
[[ $status == 255 && ! -s stdout && ! -e case/report-bom.json ]]
grep -Fq 'real-programs-rerank: invalid JSON case/baseline.json:' stderr
count=$((count + 1))
cp original-table case/baseline.json
# Existing entries of every kind refuse without opening or replacing them.
for kind in file symlink fifo; do
    output="case/existing-$kind"
    case "$kind" in
        file) printf 'keep' > "$output";;
        symlink) ln -s no-such-target "$output";;
        fifo) mkfifo "$output";;
    esac
    status=0
    timeout --kill-after=2s 5s "$work/rerank" --baseline case/baseline.json --before case/before.json --after case/after.json --projection case/projection.json --output "$output" >stdout 2>stderr || status=$?
    [[ $status == 255 && ! -s stdout ]]
    printf 'real-programs-rerank: output already exists\n' | cmp -s - stderr
    case "$kind" in
        file) printf 'keep' | cmp -s - "$output";;
        symlink) [[ -L $output && ! -e $output ]];;
        fifo) [[ -p $output ]];;
    esac
    count=$((count + 1))
done
printf 'ok rerank: %d offline exact-byte and refusal controls\n' "$count"
