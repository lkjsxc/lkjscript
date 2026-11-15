#!/bin/sh
# Execute the documented switch/rollback only against a private fake service.
set -eu
cd "$(dirname "$0")/.."
work=$(mktemp -d target/site-switch-test.XXXXXX)
work="$PWD/$work"
trap 'rm -rf -- "$work"' EXIT HUP INT TERM
before=$(cut -d ' ' -f 1-3 /proc/loadavg)
begin=$(date +%s%N)
mkdir -p "$work/bin" "$work/target" "$work/tools" "$work/examples" "$work/site/releases/original" "$work/site/releases/programs-reused"
awk -v root="$work" '
    /^~~~sh$/ {section++; inside=1; next}
    /^~~~$/ {inside=0; next}
    inside {print > (root "/" (section == 1 ? "switch.raw" : "rollback.raw")); count++}
    END {if(section != 2 || count > 25) exit 1}
' examples/site-switch.md
for kind in switch rollback; do
    sed -e "s|/opt/lkjscript-site|$work/site|g" \
        -e "s|/etc/systemd/system/lkjscript-site.service|$work/live.service|g" \
        "$work/$kind.raw" >"$work/$kind.sh"
    if grep -E '/opt/|/etc/' "$work/$kind.sh"; then exit 1; fi
done
cat >"$work/bin/sudo" <<'SH'
#!/bin/sh
exec "$@"
SH
cat >"$work/bin/git" <<'SH'
#!/bin/sh
printf 'reused\n'
SH
cat >"$work/bin/systemctl" <<'SH'
#!/bin/sh
exit 0
SH
cat >"$work/target/lkjscript" <<'SH'
#!/bin/sh
printf 'candidate\n' >target/lkjscript-site
SH
cat >"$work/tools/http-site-test.sh" <<'SH'
#!/bin/sh
exit 0
SH
chmod +x "$work/bin/"* "$work/target/lkjscript"
printf 'original unit\n' >"$work/live.service"
cp "$work/live.service" "$work/original.service"
cp examples/lkjscript-site.service "$work/examples/lkjscript-site.service"
printf 'failed attempt retained\n' >"$work/site/releases/programs-reused/marker"
ln -s "$work/site/releases/original" "$work/site/current"
ln -s "$work/site/releases/programs-reused" "$work/site/.programs-attempt"
run() { (cd "$work"; PATH="$work/bin:$PATH" sh "$1"); }
restored() {
    test "$(readlink -f "$work/site/current")" = "$work/site/releases/original"
    cmp "$work/live.service" "$work/original.service"
    test "$(cat "$work/site/releases/programs-reused/marker")" = 'failed attempt retained'
}
run ./switch.sh
first=$(readlink -f "$work/site/current")
test "$first" != "$work/site/releases/programs-reused"
test "$(readlink -f "$work/site/.programs-attempt")" = "$first"
cmp "$work/live.service" examples/lkjscript-site.service
run ./rollback.sh
run ./rollback.sh
restored
run ./switch.sh
second=$(readlink -f "$work/site/current")
test "$second" != "$first"
run ./rollback.sh
restored
# Interrupt after the attempt pointer is complete and after activation, then
# execute exactly the same rollback twice from independent shells.
for lines in 6 7; do
    head -n "$lines" "$work/switch.sh" >"$work/partial.sh"
    run ./partial.sh
    run ./rollback.sh
    run ./rollback.sh
    restored
done
elapsed=$(($(date +%s%N) - begin))
after=$(cut -d ' ' -f 1-3 /proc/loadavg)
awk -v elapsed="$elapsed" -v before="$before" -v after="$after" \
    'BEGIN {printf "ok site switch: failed release retained, unique retries, interrupted activation and repeated rollback (%.3f s, load %s -> %s)\n", elapsed / 1000000000, before, after}'
