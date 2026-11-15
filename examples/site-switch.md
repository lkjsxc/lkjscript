# Switch and rollback

Claude runs these commands later, from this repository; no switch was performed here. First require `sh tools/http-site-test.sh --systemd` to pass against the live origin.
```sh
set -eu
site_release="/opt/lkjscript-site/releases/programs-$(git rev-parse --short HEAD)"; test ! -e "$site_release"
sudo ln -sfnT "$site_release" /opt/lkjscript-site/.programs-attempt
target/lkjscript build examples/site.rs --library-dir library --grant output --grant listener=127.0.0.1:8798 --grant clock --grant ready --grant stop -o target/lkjscript-site
sudo install -d -m 0755 "$site_release"
sudo install -m 0755 target/lkjscript-site "$site_release/lkjscript-site"
sudo cp /etc/systemd/system/lkjscript-site.service "$site_release/previous.service"; sudo ln -s "$(readlink -f /opt/lkjscript-site/current)" "$site_release/previous"
sudo ln -sfnT "$site_release" /opt/lkjscript-site/.programs-next; sudo mv -Tf /opt/lkjscript-site/.programs-next /opt/lkjscript-site/current
sudo install -m 0644 examples/lkjscript-site.service /etc/systemd/system/lkjscript-site.service
sudo systemctl daemon-reload; sudo systemctl restart lkjscript-site
sh tools/http-site-test.sh --systemd
```
Rollback the recorded attempt after activation; safe to retry from a fresh shell, including after the current pointer was restored:
```sh
set -eu; site_failed=$(readlink -f /opt/lkjscript-site/.programs-attempt); site_previous=$(readlink -f "$site_failed/previous")
site_current=$(readlink -f /opt/lkjscript-site/current); test "$site_current" = "$site_failed" || test "$site_current" = "$site_previous"
sudo cp "$site_failed/previous.service" /etc/systemd/system/lkjscript-site.service
sudo ln -sfnT "$site_previous" /opt/lkjscript-site/.programs-rollback
sudo mv -Tf /opt/lkjscript-site/.programs-rollback /opt/lkjscript-site/current
sudo systemctl daemon-reload; sudo systemctl restart lkjscript-site
```
