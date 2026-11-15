#!/usr/bin/env bash
# The timed operation itself; scale.sh verifies unchanged projection bytes/meaning.
set -euo pipefail
(($# == 6)) || { echo 'usage: scale-operation COMPILER TREE STORE KEY KIND STORE_BYTES' >&2; exit 2; }
compiler=$1 tree=$2 store=$3 key=$4 kind=$5 store_bytes=$6
request=$(mktemp "$(dirname "$tree")/scale-operation.XXXXXX")
trap 'rm -f -- "$request"' EXIT
trap 'exit 130' HUP INT TERM
case "$kind" in
    body)
        cat > "$request" <<'JSON'
{"version":1,"operations":[{"op":"replace-body","id":"edit::target","source":"{\nvalue ^ 2u64\n}"}]}
JSON
        ;;
    signature)
        cat > "$request" <<'JSON'
{"version":1,"operations":[{"op":"change-signature","id":"edit::target","parameters":["edit::target::param:value",{"name":"extra","type":"u64","argument":"1u64"}]},{"op":"replace-body","id":"edit::target","source":"{\nvalue ^ extra ^ 2u64\n}"}]}
JSON
        ;;
    rename)
        cat > "$request" <<'JSON'
{"version":1,"operations":[{"op":"rename","id":"edit::target","name":"target_renamed"}]}
JSON
        ;;
    *) printf 'unknown operation: %s\n' "$kind" >&2; exit 2 ;;
esac
"$compiler" edit main.lkj --source-dir "$tree" --store "$store" --store-key "$key" \
    --store-bytes "$store_bytes" --stats --request "$request"
