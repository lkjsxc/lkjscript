# Conditional reference capture: prepared semantic integration

Status: prepared, unapplied. No compiler/native command, semantic publication or production-source write was performed by this preparation. Root owns sequencing, capture and execution release.

## Frozen inputs

Language baseline: main 0b0b27803d27b932a5b91cde2d766bd3c8bb82b9. Later bench/cleanup commits and the separately installed host tools do not require an otherwise clean working tree; the language-file baseline is checked directly.

Desired root: /home/coder/workspace/lkjscript/target/compiler-reference-capture-desired-1. Root froze it after preliminary attempt 4. Its 1,743-record source manifest is e34cdb687a9e7195e686f38bf53a6f55c1b38f7f1769f60605ba36361b6119b7; before and after copies agree. Producer record 180a4bf01f9f8b8b141604133ca790c46b91d9041f1d888f68e0c582beb2cc84 identifies canonical 6cd52308aba91d6f84396e6f999ffba6d97eb7ccbacbb6bec654c31fee7bdacc.

The final definition_types file is 35aede47461e4b7204ae257d17bdfcd3476dbfb0e8335893818b584a665b418b; the complete new public test module is 694b8163ad7a1216b22941187b0f740b5f46d46c37500f84017b6cb9742091c5. freeze.json pins all ten affected files and their nine baseline counterparts.

Root reported preliminary old-emitter validation: 728 closed contracts, zero failures; build 18.22 s and run 61.26 s. That is retained preliminary validation, not a new execution or final candidate/gate claim by this preparation.

## Two source transactions

### 1. Core replay under src/main.lkj

core-apply.sh contains the exact resource-bounded command. commands.json also retains its argv and an optional dry-run variant. Both entries are relative src/main.lkj; actual and desired source/library roots are explicit. Do not replace the desired entry with an absolute pathname under its explicit grant: that previously produced E_IO errno 18 before candidate reading.

The replay target contains changes to exactly these seven existing files:

- src/driver.lkj
- src/elaborate.lkj
- src/elaborate/definition_types.lkj
- src/elaborate/definition_types/pattern_equations.lkj
- src/elaborate/diagnostic_inference.lkj
- src/elaborate/generic_check.lkj
- src/elaborate/incremental.lkj

The three private assertion items are part of the same core projection: elaborate::assert_reference_capture; elaborate::definition_types::assert_reference_capture_recording; elaborate::definition_types::assert_reference_capture_deferred. They are not direct tracked-file edits or a separate uncredited copy step.

The compiler derives the finite replay operations. Do not predict its operation count from textual replacements. Retain the actual emitted operation targets, sites, file before/after hashes, authority deltas, current-check result, desired/candidate checked-meaning digests and verified=true. An unsupported framing/import/file correspondence remains a recorded refusal; do not weaken replay policy or replace source manually.

### 2. Explicit test-root request under src/semantic_main.lkj

tests-request.json is a concrete version-1 request, SHA 3721803e8ed288cbe6687f35cee818f6bf403310bac9c234adf1eeb650acbd37, with exactly three operations:

1. add-item to semantic_tests, before semantic_tests::optimizer: mod reference_capture; with projection path src/semantic_tests/reference_capture.lkj and the entire frozen module bytes.
2. replace-body semantic_tests::core, adding the one reference_capture::run() call.
3. replace-body closed_core, changing its expected count from 185 to 192.

test-apply.sh has the exact command. It uses the actual semantic test root and a request file, with no --against. This avoids the known test-root replay file-bijection refusal on include-only test inputs. It does not alter or bypass that refusal rule.

The physical parent src/semantic_tests already exists; the new module was absent during preparation. Compiler projection planning must still prove current absence of selected/alternate loader candidates, parent identity and selected-module provenance. No directory creation, source copy, key rewrite or raw .lkj write is proposed.

The request builder checks that each registration file differs from Git baseline by only the declared module/run/count changes. It preserves the new module's full UTF-8 bytes and rejects additional root-file changes.

## Execution and capture obligations

No command runs implicitly. Root may use its existing capture wrapper around the exact scripts/argv. Each released compiler command gets one worker, 4 GiB AS, 900-second timeout and 10-second grace; record actual producer digest/size, argv, cwd, UTC/ns, load, GNU time, complete stdout/stderr and exit. Preserve every preliminary/refused/dry-run attempt separately.

Before each command, capture the complete relevant actual source inventory and verify the desired 1,743-record freeze. Afterward capture both again. An optional dry run is not a publication; if root chooses one, keep its outcome distinct. Only run the second source transaction after accepting the first.

The existing private key path appears only as a path in argv, copied from the prior accepted transaction. It is outside all explicit source/library grants. This preparation neither reads its bytes nor creates, replaces, hashes or removes it. All other credentials, global store contents and foreign keys remain outside preparation. Durable edit metadata is owned by the compiler's transaction protocol.

Root separately installed semantic-query-test.sh and closed-semantics.sh (185 to 192); their host-tools-integration.json remains a separate shell-tool record, not additional .lkj adoption.

## Preparation record

materialize.pl ran only as an offline request generator. It checked the ten frozen desired files, nine current language files against the exact Git baseline, new-module absence, registration deltas and UTF-8 round trip. No compiler or source mutation occurred. The full source-manifest population is copied unchanged; independent full untouched-file verification belongs to the later adoption audit.

The initial bounded inspection mistakenly looked for prior scalar transaction directories under main and got two missing-path diagnostics. The actual records were then read at their known isolated scalar-worktree paths. That setup-only exit 2 is retained under reads/inspection-1; it is not a compiler or semantic refusal.
