#!/bin/sh
# This key is compiler authority, kept outside source/library/store grants.
# Cache entries are disposable; key rotation merely makes old entries miss.
set -eu
umask 077
state_root=${XDG_STATE_HOME:-"$HOME/.local/state"}
state_directory="$state_root/lkjscript"
mkdir -p "$state_directory"
[ ! -L "$state_directory" ] || exit 1
[ "$(stat -c %u "$state_directory")" = "$(id -u)" ] || exit 1
chmod 700 "$state_directory"
key="$state_directory/store-key"
if [ ! -e "$key" ]; then
    temporary=$(mktemp "$state_directory/.store-key.XXXXXX")
    trap 'rm -f "$temporary"' EXIT HUP INT TERM
    head -c 32 /dev/urandom > "$temporary"
    chmod 600 "$temporary"
    ln "$temporary" "$key" 2>/dev/null || true
    rm -f "$temporary"
    trap - EXIT HUP INT TERM
fi
[ -f "$key" ] && [ ! -L "$key" ] || exit 1
[ "$(stat -c %u "$key")" = "$(id -u)" ] || exit 1
[ "$(wc -c < "$key")" -eq 32 ] || exit 1
chmod 600 "$key"
printf '%s\n' "$key"
