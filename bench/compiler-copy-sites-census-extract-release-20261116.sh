#!/usr/bin/env bash
set -euo pipefail
test "$PWD" = /home/coder/workspace/lkjscript
test ! -e target/compiler-copy-sites-census-evidence.20261116-1
test ! -e target/compiler-copy-sites-census-output.20261116-1
for path in bench/compiler-copy-sites-extract.pl bench/compiler-copy-sites-extract-capture.sh bench/compiler-copy-sites-census-plan.md bench/compiler-copy-sites-census-extract-release-20261116.sh bench/compiler-copy-sites-census-extract-release-20261116.md bench/compiler-profile-copy-sites-cohort-2-acceptance/acceptance.json bench/compiler-profile-copy-sites-cohort-2-data/06-copy-detail/stdout; do
    git show "HEAD:$path" | cmp - "$path"
done
sha256sum -c <<'PINS'
9ca679c6f24cfe1ad9fcfd14a54f27be0edef56a1c9a0faeb7c455ab76536517  bench/compiler-copy-sites-extract.pl
85dcbc04be568238d8b7f079292b5e4475567428fc7d1c8be326ed9e10eb5109  bench/compiler-copy-sites-extract-capture.sh
8a1f3e3d5354b3eca7fd9adb68bfee892f94360130a19b265f3f9ffb73c4dc15  bench/compiler-copy-sites-census-plan.md
adfb9a9837ec044cc1a03cec186a7ada643300bab7efe79b324df6e597bddcbe  bench/compiler-profile-copy-sites-cohort-2-acceptance/acceptance.json
7849d4dd69906491d07664f493eb0f4fd8340cfde3760f6e4f168dbbde95f302  target/compiler-copy-sites-run.20261116-2/producer-profile64.samples.00000000000002892325
32c0f43dbde125e43f2ca5a0c6d8548ba0f1774ce3b51bc62d94a6488af0fdca  target/compiler-copy-sites-run.20261116-2/selected-native.lkjir
06093752531ea752f7081548166f33cc95c71aad381e72d2713e35f5bcaab25d  bench/compiler-profile-copy-sites-cohort-2-data/06-copy-detail/stdout
3b11dec50514a8473e9f6efa7a34d584d0657538c09988f61b72d38ad4991a10  /usr/bin/time
17064f67e650d6152a6902b013aab496b54c87587c8eea6f5023aafee6154069  /usr/bin/prlimit
12690a043dfd555a6c14ccc1564f5649c18f1762c0d6ff66729948130898ec52  /usr/bin/timeout
fa36b75b21e94f424b61aa058b743cc62c94e7fa45a085fdeffb2bae687ed925  /usr/bin/perl
af4c78b51063bce8a24e2a35bd364a48cbb3a888d01c1fd6219faa3d5983d4f8  src/ir_text.lkj
40f7ae357e53524e0e2e89aa5e0b697613705b7342fef10346f78673679c9655  src/ir.lkj
f88d1ff9a50ce796d2fc10bfa04cd4e8d73a30d8284d5a9b3bcd3488f724daf0  src/native/layout.lkj
71a23af143af92e8134f8453f34dd53f934b41d25736419f296c7458f5016e78  src/check.lkj
961d59858c83afd8c5e8e2f9ec00d0f3a90cc91953ba8903528589f41a9da705  src/optimizer.lkj
333d6ec927416917fa32269730a9a507fcfe31072a2191403a24161b5cf6c412  src/driver.lkj
4a42f7705edf85ad31641b0afec6f89192585b18ff0eca9e75b835ae623b64e3  src/reuse.lkj
029b2ebc01b3aa7fe87c06b663c51ff9f7533fd95d687c0ad10f3c37f844fd26  docs/ir.md
PINS
bash bench/compiler-copy-sites-extract-capture.sh "$PWD/target/compiler-copy-sites-census-evidence.20261116-1" /usr/bin/prlimit --as=4294967296 -- /usr/bin/timeout --kill-after=10s 300s /usr/bin/perl "$PWD/bench/compiler-copy-sites-extract.pl" --companion "$PWD/target/compiler-copy-sites-run.20261116-2/selected-native.lkjir" --detail "$PWD/bench/compiler-profile-copy-sites-cohort-2-data/06-copy-detail/stdout" --output "$PWD/target/compiler-copy-sites-census-output.20261116-1"
