#!/usr/bin/env bash
set -euo pipefail
out=/home/coder/workspace/lkjscript-compiler-scalar-20261116/target/compiler-scalar-gate-acceptance-1
test ! -e "$out/gate-captures.tar.gz"
test ! -L "$out/gate-captures.tar.gz"
tar --format=ustar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --no-recursion \
    -C "$out/captures" --null -T "$out/archive-members.nul" -cf - |
    gzip -n -c > "$out/gate-captures.tar.gz"
