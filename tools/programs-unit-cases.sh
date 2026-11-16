#!/usr/bin/env bash
# Fresh checking precedes closed execution reuse. The native cases and an
# independent interpreter harness are ordinary no-authority images. The latter
# embeds the exact checked IR: replaying a World-owning compiler is not a proof.
set -euo pipefail
cd "$(dirname "$0")/.."
repo=$(pwd)
compiler=${1:?compiler required}
if [[ "$compiler" != /* ]]; then compiler="$repo/$compiler"; fi
source=${2:?source required}
[[ "$source" != *$'\n'* && "$source" != *$'\t'* ]] || { echo 'unit source path contains a control separator' >&2; exit 1; }
work=$(mktemp -d target/programs-unit-cases.XXXXXX)
active=
cleanup() {
    local status=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$active" ]]; then kill "$active" 2>/dev/null || :; wait "$active" 2>/dev/null || :; fi
    rm -rf "$work"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
run() {
    local status=0
    "$@" & active=$!
    wait "$active" || status=$?
    active=
    return "$status"
}
reuse=()
store_path=${LKJSCRIPT_STORE:-target/store}
key_path=${LKJSCRIPT_STORE_KEY:-}
if [[ "$store_path" != /* ]]; then store_path="$repo/$store_path"; fi
if [[ -n "$key_path" ]]; then
    if [[ "$key_path" != /* ]]; then key_path="$repo/$key_path"; fi
    reuse=(--store "$store_path" --store-key "$key_path")
fi
run "$compiler" ir "$source" --library-dir library "${reuse[@]}" -o "$work/checked.lkjir"
# The source frontend fixes unit at type zero. Refuse unsupported signatures
# instead of silently dropping an observation from this closed-only helper.
awk '
    $1 == "fn" {
        test=0
        for (field=5; field<=NF; field++) if ($field == "test") test=1
        if (!test) next
        name=$2; gsub(/^"|"$/, "", name)
        if (name !~ /^[a-zA-Z0-9_:]+$/ || $3 != 0 || $4 != 0) exit 1
        print name; count++
    }
    END { if (!count) exit 1 }
' "$work/checked.lkjir" >"$work/entries"
mapfile -t entries <"$work/entries"
count=${#entries[@]}
mkdir "$work/native" "$work/model"
artifacts=()
for index in "${!entries[@]}"; do artifacts+=(--artifact "native-$index=${entries[$index]}"); done
run "$compiler" build-entries "$work/checked.lkjir" --library-dir library \
    -o "$work/native" "${artifacts[@]}" "${reuse[@]}"

# Keep module/source names stable across fresh directories and producers. Only
# these independent model sources are needed; no frontend/checker is linked.
cp src/diagnostic.lkj src/ir.lkj src/ir_text.lkj src/runtime_memory.lkj src/interpreter.lkj "$work/model/"
cp -R src/interpreter "$work/model/"
cp "$work/checked.lkjir" "$work/model/checked.lkjir"
printf '%s\n' "$count" >"$work/model/count"
cat >"$work/model/units.lkj" <<'SOURCE'
pub mod diagnostic;
pub mod ir;
pub mod ir_text;
pub mod runtime_memory;
pub mod interpreter;

pub fn main() {
    let program = ir_text::read(include_str!("checked.lkjir")).unwrap();
    let expected = std::str::trim_ascii(include_str!("count")).parse::<usize>().unwrap();
    let mut observed = 0usize;
    for (entry, function) in program.functions.iter().enumerate() {
        if !function.test { continue; }
        assert_eq!(function.params, 0usize);
        assert!(matches!(program.types[function.result].kind, ir::TypeKind::Unit));
        let mut output = Vec::<u8>::new();
        let result = match interpreter::run(&program, entry, &[0u64; 0], &mut output) {
            Ok(result) => result,
            Err(error) => panic!("{}: {}: {}", function.name, error.code(), error.message),
        };
        assert!(matches!(result.value, interpreter::Scalar::Unit));
        assert!(output.is_empty());
        observed += 1usize;
    }
    assert_eq!(observed, expected);
}
SOURCE
# Native entry reports below require no grants/reachable effects, so the model
# receives no host capabilities. Its Vec host captures output; all other effects
# are unavailable. Normal unit completion and empty output are explicit checks.
run bash -c 'cd "$1"; shift; exec "$@"' programs-unit-model "$work/model" "$compiler" \
    build-entries units.lkj --library-dir "$repo/library" -o . --artifact interpreted=main "${reuse[@]}"
run "$compiler" build src/gate_reuse_main.lkj --library-dir library "${reuse[@]}" -o "$work/runner"
reused=0
rerun=0
observe() {
    local image=$1 cases=$2 entry=$3 tag cached fresh proof extra
    run "$work/runner" "$image" "$image.authority" tools/programs-unit-cases.sh \
        "$cases" "$store_path" "$key_path" "$entry" >"$work/observation" 2>&1 \
        || { cat "$work/observation" >&2; return 1; }
    [[ $(wc -l <"$work/observation") -eq 1 ]] || { cat "$work/observation" >&2; return 1; }
    read -r tag cached fresh proof extra <"$work/observation"
    [[ "$tag" == LKJSCRIPT_CLOSED_GATE && "$proof" == proof=authenticated-exact-runner-sealed-ELF-and-input-contract && -z "$extra" ]] \
        || { cat "$work/observation" >&2; return 1; }
    if [[ "$cached" == "reused=$cases" && "$fresh" == rerun=0 ]]; then
        reused=$((reused + cases))
    elif [[ "$cached" == reused=0 && "$fresh" == "rerun=$cases" ]]; then
        rerun=$((rerun + cases))
    else
        cat "$work/observation" >&2
        return 1
    fi
    cat "$work/observation"
}
for index in "${!entries[@]}"; do observe "$work/native/native-$index" 1 "${entries[$index]}"; done
observe "$work/model/interpreted" "$count" main
[[ $((reused + rerun)) -eq $((count * 2)) ]]
printf 'LKJSCRIPT_UNIT_GATE cases=%s observations=%s reused=%s rerun=%s source=%s\n' \
    "$count" "$((count * 2))" "$reused" "$rerun" "$source"
