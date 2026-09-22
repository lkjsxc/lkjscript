#!/bin/bash
set -euo pipefail
witness_root=/home/coder/workspace/lkjscript-native-public-20260922
phase=$1
case "$phase" in initial|edited) ;; *) exit 64 ;; esac
while IFS=$'\t' read -r fragile chilled initial edited; do
  expected=$initial
  if test "$phase" = edited; then expected=$edited; fi
  name="$phase-$fragile-$chilled"
  arguments=$(printf '[{"fragile":%s,"chilled":%s}]' "$fragile" "$chilled")
  "$witness_root/run-product.sh" work "$name" --project packing run packing-advice --arguments "$arguments"
  expected_record=$(printf 'value="\\"%s\\""' "$expected")
  grep -F "$expected_record" "$witness_root/transcripts/$name.log" > /dev/null
  grep -F 'differential=equal' "$witness_root/transcripts/$name.log" > /dev/null
  printf 'ASSERT phase=%s fragile=%s chilled=%s expected=%s passed\n' "$phase" "$fragile" "$chilled" "$expected"
done < "$witness_root/expected.tsv"
