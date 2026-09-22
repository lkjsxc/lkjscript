#!/bin/bash
set -u
witness_root=/home/coder/workspace/lkjscript-native-public-20260922
stage=$1
name=$2
shift 2
case "$stage" in work|detached) ;; *) exit 64 ;; esac
log="$witness_root/transcripts/$name.log"
if test -e "$log"; then printf 'Refusing to overwrite transcript %s\n' "$log" >&2; exit 65; fi
container_cmd=(docker run --rm --label lkjscript.native-public=20260922 --network none --read-only --cap-drop ALL --security-opt no-new-privileges --user "$(id -u):$(id -g)" --tmpfs /tmp:rw,nosuid,nodev -v "$witness_root/installation:/installation:ro" -v "$witness_root/$stage:/work:rw" -w /work debian@sha256:70509c95d1857a3704c0a5d92ee2e0adac95f612a9386889d70760bfd7c1ebba env -i PATH=/usr/bin:/bin HOME=/tmp LC_ALL=C /installation/bin/lkjscript "$@")
printf 'Command: env -i PATH=/usr/bin:/bin ' > "$log"
printf '%q ' "${container_cmd[@]}" >> "$log"
printf '\n\n' >> "$log"
env -i PATH=/usr/bin:/bin "${container_cmd[@]}" >> "$log" 2>&1
result=$?
printf '\nExit status: %s\n' "$result" >> "$log"
cat "$log"
exit "$result"
