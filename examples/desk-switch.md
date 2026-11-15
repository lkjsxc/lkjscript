# Desk switch and rollback (Claude executes)

Do not run this until `tools/http-desk-systemd.sh`, the four-worker gate and CI
pass on the candidate commit. This session does not install or restart anything.

Before switching, cxloop must publish the board contract in `desk.md` and honor
`CXLOOP_WAKE_ID` / `CXLOOP_WAKE_COUNT` as a durable idempotence key when invoked
by the delivered wake wrapper. The wake unit has broader host authority than
desk: the reviewed relay can read the home directory, write agent work/state,
and use the network. That authority is in its unit, not hidden in desk's report.

Before switching, Claude must adapt any cxloop/caller use of `desk listen CMD...`
to the delivered `listen-begin` / `listen-end` marker contract. The external
caller holds `flock "$DESK_STATE/listening"` continuously from before begin
until after end, closes its lock descriptor in CMD, and owns the child's
termination, reaping and cleanup. This adaptation is a switch prerequisite.

The installed command is the reviewed binding wrapper `desk-command`: it binds
the literal default state root and UTC0, then execs the native CLI. It preserves
arguments, standard streams and exit status. Defaults apply only when `DESK_STATE`
or `TZ` is unset; explicit empty values remain empty, and the native entry rejects
an empty state selector. Defaults do not inspect HOME. The isolated
binding-wrapper proof is included in the short desk suite.

The state directory stays at `/home/coder/.local/state/desk`; there is no state
migration or copy. The new service reads the existing cxloop and handoff roots.
Keep the original Python executable and original unit until the switch is
verified. Claude may then delete the Python implementation; rollback remains
possible only while its saved command target is retained.

The following switch and rollback blocks contain 24 executable lines in total.
The release and backup directory belongs to this switch, not to either state
root. Check the installed command's path is the expected `~/.local/bin/desk`
before replacing it.

## Switch

~~~sh
cd /home/coder/workspace/lkjscript-programs-20261115
desk_release="$PWD/target/desk-switch-$(git rev-parse --short HEAD)"; mkdir -p "$desk_release"
test "$(command -v desk)" = /home/coder/.local/bin/desk
cp -a examples/desk "$desk_release/program"; sed -i 's/127.0.0.1:0/127.0.0.1:8911/' "$desk_release/program/program.lkj"
target/lkjscript build "$desk_release/program" --library-dir library -o "$desk_release/desk-service"
target/lkjscript build examples/desk/main.lkj --library-dir library -o "$desk_release/desk" --grant args --grant state=env:DESK_STATE --grant state_env=DESK_STATE --grant timezone=TZ --grant output --grant errors --grant input --grant clock --grant entropy --grant exit
cp -a /home/coder/.local/bin/desk "$desk_release/desk-command.before"
sudo cp "$(systemctl show desk -p FragmentPath --value)" "$desk_release/desk.service.before"
install -d /home/coder/.local/libexec/lkjscript; install -m755 "$desk_release/desk" "$desk_release/desk-service" examples/desk-command examples/desk-wake /home/coder/.local/libexec/lkjscript/
sudo systemctl stop desk.service
sudo install -m644 examples/desk.service examples/desk-wake.service examples/desk-wake.path /etc/systemd/system/
ln -s /home/coder/.local/libexec/lkjscript/desk-command /home/coder/.local/bin/.desk-next; mv -Tf /home/coder/.local/bin/.desk-next /home/coder/.local/bin/desk
sudo systemctl daemon-reload; sudo systemctl start desk.service
curl -fsS http://127.0.0.1:8911/state | jq -e '.board_status and .cursor'
sudo systemctl enable --now desk-wake.path
desk note "lkjscript deskへの切替と状態の読出しを確認しました。" --from claude
~~~

## Rollback

Use the exact `desk_release` selected above. This restores the original service
and command without altering `notes.jsonl`, `inbox.jsonl`, `seen` or `ping`.

~~~sh
test -f "$desk_release/desk.service.before"
sudo systemctl disable --now desk-wake.path
sudo systemctl stop desk-wake.service desk.service
sudo install -m644 "$desk_release/desk.service.before" /etc/systemd/system/desk.service
cp -a "$desk_release/desk-command.before" /home/coder/.local/bin/.desk-restore
mv -Tf /home/coder/.local/bin/.desk-restore /home/coder/.local/bin/desk
sudo systemctl daemon-reload; sudo systemctl start desk.service
curl -fsS http://127.0.0.1:8911/state >/dev/null
~~~

A failed check after stopping the service requires this rollback immediately.
The wrapper and new binaries can remain unused; do not remove the preserved
Python target until the switch is accepted. No operation above replaces the
live state directory.
