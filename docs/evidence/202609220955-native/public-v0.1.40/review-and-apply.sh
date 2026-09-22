#!/bin/bash
set -euo pipefail
witness_root=/home/coder/workspace/lkjscript-native-public-20260922
cd "$witness_root"
run="$witness_root/run-product.sh"
# edit-packing.lkjc and tampered-packing.lkjc are explicit human-readable edits of the fresh product draft.
# This harness never synthesizes declarations or writes accepted project storage.
test -f work/edit-packing.lkjc
test -f work/tampered-packing.lkjc
module_id=$(awk '$1 == "owner" { sub(/^id=/,"",$2); print $2 }' transcripts/012-find-module.log)
"$run" work 021-find-edited-test --project packing query find declaration fragile-chilled-parcel --parent "$module_id"
"$run" work 022-review-edit --project packing change plan --input-file edit-packing.lkjc --output edit-plan.txt
function_id=$(awk '$1 == "owner" { sub(/^id=/,"",$2); print $2 }' transcripts/014-find-function-parent.log)
test_id=$(awk '$1 == "owner" { sub(/^id=/,"",$2); print $2 }' transcripts/021-find-edited-test.log)
# Affected dependents are not changed stored owners when their before/after objects are equal.
awk -v function_id="$function_id" -v test_id="$test_id" '
$1 == "logical-plan.owner" {
  delete fields
  for (i = 2; i <= NF; i++) { split($i, pair, "="); fields[pair[1]]=pair[2] }
  owner=fields["owner"]
  if (owner !~ /^expr_/) {
    affected++
    if (fields["before"] != fields["after"]) {
      changed++; print "changed", owner, fields["before"], fields["after"]
      if (owner != function_id && owner != test_id) unexpected++
      if (fields["before-present"] != "true" || fields["after-present"] != "true") recreated++
    } else { preserved++; print "unchanged-dependent", owner, fields["before"] }
  }
}
END { print "affected=" affected, "changed=" changed, "preserved=" preserved, "unexpected=" unexpected+0, "recreated=" recreated+0; exit changed != 2 || unexpected != 0 || recreated != 0 }
' work/edit-plan.txt | tee transcripts/023-reviewed-owner-assertions.out
token=$(awk '$1 == "plan" { sub(/^token=/,"",$2); print $2 }' transcripts/022-review-edit.log)
[[ "$token" =~ ^plan_[0-9a-f]+$ ]]
set +e
"$run" work 024-tampered-preacceptance --project packing change apply --input-file tampered-packing.lkjc --plan "$token"
tampered_status=$?
set -e
test "$tampered_status" -ne 0
rg -F "code=change_request_commitment_mismatch" transcripts/024-tampered-preacceptance.log
"$run" work 025-after-rejection-status --project packing status
cmp <(awk '$1 == "revision"' transcripts/010-initial-status.log) <(awk '$1 == "revision"' transcripts/025-after-rejection-status.log)
"$run" work 026-after-rejection-run --project packing run packing-advice --arguments '[{"fragile":true,"chilled":true}]'
rg -F 'value="\"Use an insulated carton.\""' transcripts/026-after-rejection-run.log
# This is the first acceptance attempt of the unchanged reviewed request, not an accepted retry.
"$run" work 027-accept-reviewed-edit --project packing change apply --input-file edit-packing.lkjc --plan "$token"
"$run" work 028-edited-check --project packing check
"$run" work 029-edited-build --project packing build --output edited.lkja
./check-table.sh edited
"$run" work 030-edited-status --project packing status
"$run" work 031-edited-owners --project packing query owners --limit 200 --bytes 131072
"$run" work 032-edited-definition --project packing inspect owner pure_function "$function_id" --detail definition --limit 1000 --bytes 1048576
for phase in initial edited; do
  case "$phase" in initial) owner_log=011-initial-owners.log; definition_log=016-initial-definition.log ;; edited) owner_log=031-edited-owners.log; definition_log=032-edited-definition.log ;; esac
  awk '$1 == "owner" && /named=true/ { for (i=2;i<=NF;i++) if ($i ~ /^(id|kind|named|name|class|parent)=/) printf "%s ", $i; print "" }' "transcripts/$owner_log" | sort > "transcripts/$phase-named-identities.out"
  awk '$1 == "definition.parameter"' "transcripts/$definition_log" > "transcripts/$phase-parameters.out"
done
test -s transcripts/initial-named-identities.out
test -s transcripts/initial-parameters.out
cmp transcripts/initial-named-identities.out transcripts/edited-named-identities.out
cmp transcripts/initial-parameters.out transcripts/edited-parameters.out
printf 'All named identities and the function parameter contract remained equal.\n' > transcripts/033-identities-assertions.out
mv work/edited.lkja detached/new.lkja
# Detach authoring authority completely: future execution mounts detached/, never retained/ or work/.
mv work/packing retained/packing
find detached -maxdepth 1 -type f -printf '%f\n' | sort > transcripts/034-detached-files.out
"$run" detached 035-detached-old run --deployment old.deployment.json --arguments '[{"fragile":true,"chilled":true}]'
"$run" detached 036-detached-new run --deployment new.deployment.json --arguments '[{"fragile":true,"chilled":true}]'
rg -F 'value="\"Use an insulated carton.\""' transcripts/035-detached-old.log
rg -F 'value="\"Use a padded insulated carton.\""' transcripts/036-detached-new.log
for name in 035-detached-old 036-detached-new; do
  rg -F 'execution-mode=production verification=not-performed' "transcripts/$name.log"
  rg -F '\"remaining_tasks\":0,\"cleanup_failures\":[]' "transcripts/$name.log"
done
env -i PATH=/usr/bin:/bin docker ps --filter label=lkjscript.native-public=20260922 --format '{{.ID}} {{.Status}}' > transcripts/037-owned-running-containers.out
test ! -s transcripts/037-owned-running-containers.out
printf 'Reviewed mismatch rejected; original HEAD and behavior preserved; reviewed edit accepted; graph/four-case/identity checks and detached old/new production runs passed; no owned containers remain.\n'
