#!/usr/bin/env bash
# The same checked IR snapshot supplies both the authority report and image.
# This trusted caller supplies a fresh matched pair; the digest footer catches swaps.
# This script is part of the closed execution identity; compiler verdicts are fresh.
set -euo pipefail
cd "$(dirname "$0")/.."
compiler=${1:-target/lkjscript}
action=${2:-run}
closed_directory=${3:-target/gate/closed}
reuse=()
if [[ -n "${LKJSCRIPT_STORE_KEY:-}" ]]; then
    reuse=(--store "${LKJSCRIPT_STORE:-target/store}" --store-key "$LKJSCRIPT_STORE_KEY")
fi
if [[ "$action" == build ]]; then
    mkdir -p "$closed_directory"
    "$compiler" build src/gate_reuse_main.rs -o "$closed_directory/runner" "${reuse[@]}"
    "$compiler" ir src/closed_semantics.rs -o "$closed_directory/cases.lkjir" "${reuse[@]}"
    "$compiler" authority "$closed_directory/cases.lkjir" "${reuse[@]}" > "$closed_directory/authority"
    "$compiler" build "$closed_directory/cases.lkjir" -o "$closed_directory/cases" "${reuse[@]}"
    read -r image_digest _ < <(sha256sum "$closed_directory/cases")
    printf 'closed-image-sha256 %s\n' "$image_digest" >> "$closed_directory/authority"
elif [[ "$action" == run ]]; then
    "$closed_directory/runner" "$closed_directory/cases" "$closed_directory/authority" \
        tools/closed-semantics.sh 5 "${LKJSCRIPT_STORE:-target/store}" "${LKJSCRIPT_STORE_KEY:-}"
else
    echo "closed-semantics: expected build or run" >&2
    exit 1
fi
