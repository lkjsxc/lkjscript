set -euo pipefail
repo=$1
out=$2
cd "$repo"
gzip -n -9 -c target/lkjscript > "$out/candidate.gz"
gzip -n -9 -c target/lkjscript > "$out/repeated.gz"
cmp "$out/candidate.gz" "$out/repeated.gz"
gzip -dc "$out/candidate.gz" > "$out/expanded"
cmp "$out/expanded" target/lkjscript
test "$(sha256sum target/lkjscript | cut -d ' ' -f 1)" = b01eabfa60015ad93d86dd991f5f64a59754811062b578e12737e2feb4e0d5db
test ! -e seed/compiler-reference-capture-seed.tmp
test ! -L seed/compiler-reference-capture-seed.tmp
cp "$out/candidate.gz" seed/compiler-reference-capture-seed.tmp
chmod 644 seed/compiler-reference-capture-seed.tmp
mv -T seed/compiler-reference-capture-seed.tmp seed/lkjscript.gz
cmp seed/lkjscript.gz "$out/candidate.gz"
sha256sum "$out/candidate.gz" "$out/repeated.gz" "$out/expanded" seed/lkjscript.gz target/lkjscript > "$out/outputs.sha256"
stat -c '%n %s' seed/lkjscript.gz target/lkjscript > "$out/sizes"
