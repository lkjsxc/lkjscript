#!/bin/bash
set -euo pipefail
witness_root=/home/coder/workspace/lkjscript-native-public-20260922
cd "$witness_root"
run="$witness_root/run-product.sh"
"$run" work 001-version --version
"$run" work 002-capabilities capabilities --generate-docs guides
"$run" work 003-change-capabilities capabilities change
"$run" work 004-create new packing --template minimal --name native-public-packing
"$run" work 005-empty-status --project packing status
base=$(awk '$1 == "revision" { sub(/^id=/,"",$2); print $2 }' transcripts/005-empty-status.log)
[[ "$base" =~ ^rev_[0-9a-f]+$ ]]
# Copy the supplied literal units unchanged; only public fresh status supplies the request base.
{ printf 'request base=%s\n' "$base"; tail -n +2 original-inputs/predecessor-create.lkjc; } > work/create-packing.lkjc
"$run" work 006-create-plan --project packing change plan --input-file create-packing.lkjc --output create-plan.txt
token=$(awk '$1 == "plan" { sub(/^token=/,"",$2); print $2 }' transcripts/006-create-plan.log)
[[ "$token" =~ ^plan_[0-9a-f]+$ ]]
"$run" work 007-create-apply --project packing change apply --input-file create-packing.lkjc --plan "$token"
"$run" work 008-initial-check --project packing check
"$run" work 009-initial-build --project packing build --output initial.lkja
./check-table.sh initial
"$run" work 010-initial-status --project packing status
"$run" work 011-initial-owners --project packing query owners --limit 200 --bytes 131072
"$run" work 012-find-module --project packing query find module packing
"$run" work 013-find-target --project packing query find target packing-advice
module_id=$(awk '$1 == "owner" { sub(/^id=/,"",$2); print $2 }' transcripts/012-find-module.log)
"$run" work 014-find-function-parent --project packing query find declaration recommend --parent "$module_id"
function_id=$(awk '$1 == "owner" { sub(/^id=/,"",$2); print $2 }' transcripts/014-find-function-parent.log)
[[ "$function_id" =~ ^decl_[0-9a-f]+$ ]]
"$run" work 015-find-parameter --project packing query find parameter parcel --parent "$function_id"
"$run" work 016-initial-definition --project packing inspect owner pure_function "$function_id" --detail definition --limit 1000 --bytes 1048576
# The original literal input and review output leave every later consumer mount.
mv work/create-packing.lkjc original-inputs/create-packing.lkjc
mv work/create-plan.txt original-outputs/create-plan.txt
mv work/initial.lkja detached/old.lkja
find work -maxdepth 1 -type f -print > transcripts/017-after-sequester.out
test ! -s transcripts/017-after-sequester.out
module_id=$(awk '$1 == "owner" { sub(/^id=/,"",$2); print $2 }' transcripts/012-find-module.log)
target_id=$(awk '$1 == "owner" { sub(/^id=/,"",$2); print $2 }' transcripts/013-find-target.log)
[[ "$module_id" =~ ^mod_[0-9a-f]+$ ]]
[[ "$target_id" =~ ^target_[0-9a-f]+$ ]]
"$run" work 018-canonical-draft --project packing change draft --owner "$module_id" --owner "$target_id" --output draft-untouched.lkjc
"$run" work 019-untouched-plan-no-output --project packing change plan --input-file draft-untouched.lkjc
rg -F 'semantic-change=false owner-recreation=false publication=none' transcripts/019-untouched-plan-no-output.log
"$run" work 020-after-untouched-status --project packing status
cmp <(awk '$1 == "revision"' transcripts/010-initial-status.log) <(awk '$1 == "revision"' transcripts/020-after-untouched-status.log)
printf 'Creation, four independent cases, graph check, input sequestration, canonical draft and unchanged planning passed.\n'
