# Fingerprint indexing integration — prepared only

One four-operation request is rooted at `src/semantic_main.lkj`. No compiler, native program, semantic edit or tracked source write has run during this preparation.

1. Replace the body of `elaborate::incremental::fingerprint` with the reviewed indexed body.
2. Add `mod incremental_fingerprint;` to the existing `semantic_tests` module and project its complete source to `src/semantic_tests/incremental_fingerprint.lkj`.
3. Replace `semantic_tests::core`'s body with the same body plus the four-group registration.
4. Replace `closed_core`'s body with the count 192 → 196.

The actual parser and documentation allow `projection: {path, source}` on add-item. The existing physical parent and both absent loader candidates are recorded. No optional insertion override, guessed item `expect`, or guessed semantic `snapshot` is supplied. The compiler must resolve all identities and check the combined candidate.

## Exact golden correspondence

The new module is **15,640 bytes**, SHA `ccaad8644eddaab9f4ae344648922ef7e3df032751a732e8923dcdbe1df7d956`. Only the original `expected(case)` item is replaced in a separate copy of frozen module `5b5a7c73…`. Prefix/suffix hashes and a full rollback comparison prove all other bytes unchanged, including `record`, `dump`, mutations, groups and `run`.

Each match arm contains its exact complete record from raw dump `34634088…`: case name, outcome, shape and every body slot, including empty slots. All seventeen records join exactly to that raw dump with its original header/footer. String-literal and request-JSON round trips were checked. No candidate calculation supplies an expected value. Original golden source, controls and captures remain untouched.

The fingerprint body payload removes only the frozen body's single terminal LF, fitting the existing brace-delimited body span. The full original body SHA and normalized payload SHA are explicit in `correspondence.json`; no other replacement-body byte changes.

## Guards and separate host-tool change

The before inventories cover **1,221 tracked .lkj files** and **all 1,743 src/library/tests files**, with direct Git-blob equality. All bytes were rechecked unchanged after preparation. The source/library trees remain 626c9c1f… / 25bdb8ff…. Metadata-only HEAD movement is permitted; the guard checks actual path populations and bytes.

`guard.pl` is read-only. It verifies the full manifests, request, module, canonical b01 producer, host-runner before bytes, existing physical parent identity and both negative module paths. It never opens a key. `commands.json` records the pending exact edit argv and existing global store-key path from previously committed operation argv. Root supplies its existing bounded execution/capture wrapper and controls launch.

After the language transaction succeeds, root separately applies the guarded `tools/closed-semantics.sh` proposal: only `if [[ "$entry" == main ]]; then count=192; fi` changes to count196. Before SHA9ff51ad8… → proposed SHAebbd18fb…. This shell edit has no language-operation credit.

## Later adoption check

Retain actual operation stdout/status and full before/after manifests. Require four planned operations and only three existing .lkj files plus the new module. Verify the new file against the filled-module SHA; verify all replaced bodies and the declaration against request payloads. Account for compiler-chosen declaration placement/framing from actual file facts, rather than asserting a guessed full-file after hash. Every other tracked language/input file must remain exact. Derive retained line counts from the actual applied/Git delta only.

Preparation completed in 0.29 s, user0.19/system0.05, RSS51,644 KiB, load1.98/1.92/2.29 unchanged. A tool-side string assembly error occurred before any remote call; records/tool-assembly-refusal.json preserves it. The corrected Perl preparation passed syntax and ran offline once. There was no compiler/native/operation refusal or partial source publication.
