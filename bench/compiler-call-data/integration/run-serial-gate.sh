#!/usr/bin/env bash
set -euo pipefail
cd /home/coder/workspace/lkjscript-replace-call-landing-20261118
proof="$PWD/target/replace-call-landing/serial-gate"
store="$PWD/target/replace-call-landing/fresh-store"
lock=/home/coder/workspace/prompts/talk/lkjscript-gate.lock
expected_input=b38830634201d4a8974c6a2b7afc05bc76b3927a
expected_base=e414aeddbb2ec6985fd564c96a44a4f583b6ba1b
expected_reviewed=ecd0ff6d0e2210be5b7eaed9770b675b7c070554
expected_producer=103557eddf30d24e046cb09f6a881e549829283495c8648c0ac9dcd1d72a5d35
expected_seed=65408daa78849909a0a3ddcf0a2cab29120b9ddfc095ffb2095c422261f6a6ee
test "$(git rev-parse HEAD)" = "$expected_input"
test -z "$(git status --porcelain=v1 --untracked-files=all)"
test "$(git ls-remote --exit-code origin refs/heads/main | cut -f1)" = "$expected_base"
git merge-base --is-ancestor "$expected_base" HEAD
git merge-base --is-ancestor 7f87d6d7b73365a6d9976c367823d48343a6b347 HEAD
git diff --exit-code "$expected_reviewed" -- src library tests witness examples
git diff --exit-code "$expected_base" -- tools/macro-origin-test.sh
test "$(git diff "$expected_base" -- src library | sha256sum | cut -d' ' -f1)" = 9c90f2ed55346a356a919a1cedcaa6195524ddf8ba5867647aeb4f13b9b43bac
printf '%s  %s\n' "$expected_seed" seed/lkjscript.gz | sha256sum --check
test "$(gzip -dc seed/lkjscript.gz | sha256sum | cut -d' ' -f1)" = "$expected_producer"
test ! -e "$store"
test ! -e "$proof"
mkdir -p "$proof"
mkdir "$lock"
child_pid=
cleanup() {
    local result=$?
    trap - EXIT HUP INT TERM
    if [[ -n "$child_pid" ]] && kill -0 -- "-$child_pid" 2>/dev/null; then
        kill -TERM -- "-$child_pid" 2>/dev/null || true
        wait "$child_pid" 2>/dev/null || true
    fi
    date -u +%Y-%m-%dT%H:%M:%SZ > "$proof/wrapper-finished-at.txt"
    printf '%s\n' "$result" > "$proof/wrapper-exit-status.txt"
    cat /proc/loadavg > "$proof/wrapper-load-after.txt"
    rmdir "$lock" || result=1
    printf 'wrapper_exit=%s shared_lock=released\n' "$result"
    exit "$result"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
printf 'integrated_input=%s\npublic_parent=%s\nreviewed_source=%s\nexpected_producer=%s\nseed_gzip=%s\nworkers=4\nstore=%s\nstore_initial=absent\nlock=%s\n' "$expected_input" "$expected_base" "$expected_reviewed" "$expected_producer" "$expected_seed" "$store" "$lock" > "$proof/input.txt"
git status --porcelain=v1 --untracked-files=all > "$proof/worktree-at-start.txt"
git rev-parse HEAD HEAD^{tree} > "$proof/input-commit-tree.txt"
git ls-tree -r HEAD -- src library tools tests witness seed examples .github > "$proof/input-manifest.txt"
sha256sum target/replace-call-landing/run-serial-gate.sh > "$proof/runner.sha256"
printf '0\n' > "$proof/store-initial-entry-count.txt"
date -u +%Y-%m-%dT%H:%M:%SZ > "$proof/gate-started-at.txt"
cat /proc/loadavg > "$proof/gate-load-before.txt"
printf 'gate_start input=%s workers=4 fresh_store=true\n' "$expected_input"
gate_result=0
setsid /usr/bin/time -f 'wall_seconds=%e user_seconds=%U system_seconds=%S max_rss_kib=%M exit=%x' -o "$proof/gate.time" env LKJSCRIPT_GATE_JOBS=4 LKJSCRIPT_STORE="$store" bash tools/check.sh > "$proof/full-gate.log" 2>&1 &
child_pid=$!
printf '%s\n' "$child_pid" > "$proof/gate-process-group.txt"
wait "$child_pid" || gate_result=$?
child_pid=
printf '%s\n' "$gate_result" > "$proof/gate-exit-status.txt"
date -u +%Y-%m-%dT%H:%M:%SZ > "$proof/gate-finished-at.txt"
cat /proc/loadavg > "$proof/gate-load-after.txt"
cat "$proof/gate.time"
tail -n 10 "$proof/full-gate.log"
if ((gate_result)); then exit "$gate_result"; fi
printf '%s  %s\n' "$expected_producer" target/lkjscript | sha256sum --check
printf '%s  %s\n' "$expected_seed" seed/lkjscript.gz | sha256sum --check
gzip -dc seed/lkjscript.gz | cmp - target/lkjscript
printf 'bootstrap_matches_decoded_seed=byte-identical\n' > "$proof/bootstrap-equality.txt"
sha256sum target/lkjscript seed/lkjscript.gz > "$proof/output.sha256"
test "$(git rev-parse HEAD)" = "$expected_input"
git status --porcelain=v1 --untracked-files=all > "$proof/worktree-after-gate.txt"
test ! -s "$proof/worktree-after-gate.txt"
date -u +%Y-%m-%dT%H:%M:%SZ > "$proof/standalone-started-at.txt"
cat /proc/loadavg > "$proof/standalone-load-before.txt"
standalone_result=0
setsid /usr/bin/time -f 'wall_seconds=%e user_seconds=%U system_seconds=%S max_rss_kib=%M exit=%x' -o "$proof/standalone.time" bash tools/real-programs-test.sh target/lkjscript > "$proof/standalone.log" 2>&1 &
child_pid=$!
wait "$child_pid" || standalone_result=$?
child_pid=
printf '%s\n' "$standalone_result" > "$proof/standalone-exit-status.txt"
date -u +%Y-%m-%dT%H:%M:%SZ > "$proof/standalone-finished-at.txt"
cat /proc/loadavg > "$proof/standalone-load-after.txt"
cat "$proof/standalone.time"
cat "$proof/standalone.log"
if ((standalone_result)); then exit "$standalone_result"; fi
git status --porcelain=v1 --untracked-files=all > "$proof/worktree-at-finish.txt"
test ! -s "$proof/worktree-at-finish.txt"
printf 'ok integrated gate, exact bootstrap and standalone real programs\n'
