# Independent adoption audit plan

Prospective only. No operation success, line-count numerator or semantic acceptance is claimed until root publishes and the following independent checks pass.

## Evidence boundary

Pin language baseline 0b0b27803d27b932a5b91cde2d766bd3c8bb82b9, canonical producer 6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc, desired freeze e34cdb687a9e7195e686f38bf53a6f55c1b38f7f1769f60605ba36361b6119b7, and explicit test request 3721803e8ed288cbe6687f35cee818f6bf403310bac9c234adf1eeb650acbd37. Preserve the original proposed source, all preliminary failures and root's final private-binding-context-v5 correction; do not collapse them into the successful source claim.

For each actual command, retain exact argv/cwd/producer, status and complete diagnostics/plan, timestamps/load/time, source inventories immediately before/after, desired before/after inventories, request hash before/after where applicable, and resulting Git diff. Every file read by this audit is a source or retained text/artifact record; no private key contents or authentication secrets are needed.

## Complete source mapping

1. Enumerate the complete tracked src/library/tests population from the baseline Git tree, including non-.lkj include inputs. Compare the first actual before manifest to exact Git blobs. Do not infer completeness from a fixed guessed file count; reconcile the actual path set and the new module's initial absence. Check all unchanged library/test inputs, not only the changed files.
2. The first successful core publication may alter only the seven named core files. Every report file-before hash must equal the captured actual-before hash; every report file-after hash must equal actual-after bytes. All other inventoried inputs must remain exact. Record actual operation/site counts from the compiler, not from draft textual substitutions.
3. The complete core after inventory must equal the explicit test transaction before inventory. This prevents an unrecorded intermediate manual delta. The latter may change only src/semantic_tests.lkj, src/semantic_main.lkj and add src/semantic_tests/reference_capture.lkj.
4. Decode the exact three-op request independently. Require operation kind/identity/order, insertion anchor, projection path and complete projected source hash. Verify its two replacement bodies against the frozen desired roots, and the module's bytes against 694b8163ad7a1216b22941187b0f740b5f46d46c37500f84017b6cb9742091c5.
5. Check the test transaction file facts against actual before/after bytes, including before=absent for the new file. Verify all remaining inputs unchanged, with no unexplained added, deleted or unrepresented file. Compare final actual bytes to the eventual committed Git blobs.
6. Confirm that the desired freeze is unchanged across both commands. The candidate's physical output may differ in blank lines, insertion order or detached comments from the desired source because replay preserves untouched projection bytes. Report each such discrepancy explicitly. Exact bytes and narrowly described presentation-only differences are separate evidence; never globally strip whitespace/comments or assume an unexplained hunk harmless.

## Publication and trust

Require successful apply, current candidate checking and no pending unexplained intent. For replay, require exact printed desired/candidate checked-meaning equality and verified=true. This is canonical checked-meaning equality under the same current checker/environment, not a claim of identical serialized IR or arbitrary behavioral equivalence.

For the explicit test request, independent candidate checking is required; the request itself deliberately changes test behavior. Do not invent a desired-replay equality marker for a transaction that used no --against. Preserve authority gained/lost reporting and any compiler refusal rather than treating it as absent.

Source publication evidence does not replace final candidate-emitter regression/gate evidence. The preliminary desired closed-report pass used the canonical old emitter and remains separately labelled.

## Adoption accounting

Compute final retained .lkj added+deleted lines from baseline to actual final source. Do not count entire replaced items, removed transient trials, repeated validation copies or every requested source line. Credit a retained line only through a verified source before/after publication chain.

Report the core source delta, private assertion delta where independently separable, and explicit test module/registration delta; sum unique retained paths only. Also give the source/library denominator and the all-retained-.lkj denominator separately if any .lkj evidence specimens are committed. Shell host-tool edits, JSON requests, reports, Rust files and archives are not .lkj adoption.

Unknown or uncovered retained source changes receive zero credit and are listed. Refused attempts contribute no published lines. A count is not an independent semantic-soundness claim.

## Intended audit result

Produce one small audit JSON with pinned input paths/hashes, exact baseline/final file populations, per-transaction file chains and actual operations, final changed-path list, untouched-file counts, desired-projection discrepancies, numeric retained-line numerator/denominators, explicit limitations and refusal findings. Retain the audit's own script/argv/status/diagnostics. Do not modify source or rerun a compiler as part of the adoption audit.
