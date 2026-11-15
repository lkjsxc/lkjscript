#!/bin/sh
# cause: bad message.
# Regression: --no-store is a value-less switch and overrides store options
# regardless of their position; neither coordinator nor workers publish entries.
set -eu
compiler=${1:-target/lkjscript}
mkdir -p target
mutation_directory=$(mktemp -d target/mutation-no-store.XXXXXX)
trap 'rm -rf "$mutation_directory"' EXIT
trap 'exit 1' HUP INT TERM
mkdir "$mutation_directory/store"
(umask 077; head -c 32 /dev/urandom > "$mutation_directory/key")
"$compiler" mutate 2701 1 --no-store -o "$mutation_directory/plain" > "$mutation_directory/plain.log"
"$compiler" mutate 2701 1 --store "$mutation_directory/store" --store-key "$mutation_directory/key" \
    --no-store -o "$mutation_directory/last" > "$mutation_directory/last.log"
"$compiler" mutate 2701 1 --no-store --store "$mutation_directory/store" --store-key "$mutation_directory/key" \
    -o "$mutation_directory/first" > "$mutation_directory/first.log"
for result in plain last first; do
    grep -Fq 'ok mutation: 1 mutants' "$mutation_directory/$result.log"
    if grep -Eq 'checked reuses|emitted reuses|store rejects' "$mutation_directory/$result.log"; then
        echo 'mutation --no-store retained a store request' >&2
        exit 1
    fi
done
test -z "$(find "$mutation_directory/store" -mindepth 1 -print -quit)"
printf 'ok mutation --no-store: value-less, order independent, no published store entries\n'
