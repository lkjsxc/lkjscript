# Executed command route

Working directory: `/home/coder/workspace/lkjscript-native-public-20260922`.
The [launcher](run-product.sh) records every literal Docker command, full output and exit status
in separate external `transcripts/*.log` files and refuses to overwrite one. No publishing token,
compiler checkout or external semantic generator enters its consumer mounts.

The fresh environment probe was exactly:

```sh
docker run --rm --label lkjscript.native-public=20260922 --network none --read-only \
  --cap-drop ALL --security-opt no-new-privileges --user 1001:1001 \
  debian@sha256:70509c95d1857a3704c0a5d92ee2e0adac95f612a9386889d70760bfd7c1ebba \
  env -i PATH=/usr/bin:/bin sh -c 'for tool in python python3 node nodejs rustc cargo; do if command -v "$tool"; then exit 1; else printf "%s=absent\n" "$tool"; fi; done; uname -srm' \
  > transcripts/000-environment.out
```

Its [output](000-environment.out) records all six tools absent. Acquisition began only
after the root executor confirmed immutable official publication. [acquire.sh](acquire.sh) was
run as `bash -x ./acquire.sh > transcripts/acquisition.log 2>&1`; its
[output presentation](acquisition.log) records strict installation and the selected immutable slot.
The installed binary was then mounted at `/installation/bin/lkjscript` for all public operations.

`./create-and-draft.sh > transcripts/create-and-draft.original.log 2>&1` discovered capabilities,
generated guides, created the empty project, read fresh status, attached that base to the supplied
literal units, reviewed/applied creation, checked/built it and ran the independent four-case table.
It stopped at the parentless function lookup. The exact original script is retained externally as
`create-and-draft.original.sh`.

The corrected continuation used these discovered selectors:

```sh
./run-product.sh work 014-find-function-parent --project packing query find declaration recommend \
  --parent mod_c21c9f135464f7c2bd6ee6b6e0e21c19
./run-product.sh work 015-find-parameter --project packing query find parameter parcel \
  --parent decl_37f2292c663884a3c01f23d346f5d398
./run-product.sh work 016-initial-definition --project packing inspect owner pure_function \
  decl_37f2292c663884a3c01f23d346f5d398 --detail definition --limit 1000 --bytes 1048576
mv work/create-packing.lkjc original-inputs/create-packing.lkjc
mv work/create-plan.txt original-outputs/create-plan.txt
mv work/initial.lkja detached/old.lkja
find work -maxdepth 1 -type f -print > transcripts/017-after-sequester.out
test ! -s transcripts/017-after-sequester.out
./run-product.sh work 018-canonical-draft --project packing change draft \
  --owner mod_c21c9f135464f7c2bd6ee6b6e0e21c19 \
  --owner target_1f3fc32bfa03b95d0d1e11f692e7bb92 --output draft-untouched.lkjc
```

The first resume suffix had already stopped at the transcript-overwrite guard without product
execution; its original script and `transcripts/resume-create-and-draft.log` remain retained.
After the no-op `--output` misuse, the successful unchanged planning was:

```sh
./run-product.sh work 019-untouched-plan-no-output --project packing change plan \
  --input-file draft-untouched.lkjc
./run-product.sh work 020-after-untouched-status --project packing status
cmp <(awk '$1 == "revision"' transcripts/010-initial-status.log) \
    <(awk '$1 == "revision"' transcripts/020-after-untouched-status.log)
```

The [tracked creation script](create-and-draft.sh) includes those selector/no-op corrections for
inspection and fresh reproduction. Its earlier failure originals were not overwritten.
The fresh draft was copied and explicitly edited into [edit-packing.lkjc](edit-packing.lkjc), then copied
and changed once more into [tampered-packing.lkjc](tampered-packing.lkjc); no historical owner IDs were
replayed and no script synthesized either edited body.

`./review-and-apply.sh > transcripts/review-and-apply.original.log 2>&1` then completed successfully.
The [exact executed script](review-and-apply.sh) records the complete sequence: review the intended
edit; compare stored before/after objects; submit tampered input with the intended review token;
assert the exact commitment mismatch, unchanged HEAD and original result; accept the original
reviewed input; check/build all four edited cases; compare all named identities and the parameter
definition; move accepted authority outside the mount; execute old/new deployments. The
[table harness](check-table.sh) uses the independent literal [expected.tsv](expected.tsv).

Final cleanup checks, already included in that script, query only containers carrying this
campaign's owned label. `transcripts/037-owned-running-containers.out` was empty. A later
`find tmp -mindepth 1 -maxdepth 1 -print` was also empty. No consumer process remains. The owned
installation and retained project/bundles are intentionally preserved; no destructive cleanup or
operational-data change was performed.

Tracked `.log` and named-identity output presentations have trailing whitespace removed only.
The complete original outputs and actual edit/tamper diffs remain unchanged outside the checkout.
