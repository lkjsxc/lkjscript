#!/bin/sh
# cause: bad message.
# pending: mutate --no-store must honor the flag or name it as unsupported,
# not request a value. Generic compiler help currently advertises a switch.
set -eu
compiler=${1:-target/lkjscript}
if output=$("$compiler" mutate 2701 1 --no-store 2>&1); then
    printf 'pending mutation flag is now accepted; update this diagnostic case\n' >&2
    exit 1
fi
if [ "$output" != 'E_USAGE --no-store needs a value' ]; then
    printf 'pending mutation flag diagnostic changed: %s\n' "$output" >&2
    exit 1
fi
printf 'ok pending mutation --no-store diagnostic\n'
