# Macro-origin CI watchdog diagnosis

The original type-outlives CI run 44332902680 remains failed: witness succeeded, but the macro-origin host suite was terminated in its 600-second watchdog window. All 6,575 ordinary case requests completed freshly before the aggregate failure. The final IR-mutation phase and gate success were not reached.

The preceding successful d640 CI run took 580.842 seconds for the complete macro-origin host suite under the same 600-second limit. That measurement was not an isolated interpreter duration. Its remaining margin was only 19.158 seconds.

## Controlled local observation

Both known compiler binaries ran the same complete 739 contract source files with fresh private fixtures. The archived contract-input manifests are byte-identical. All 25 malformed-origin cases, source queries, edits, native execution and interpreted execution passed.

| Producer | Whole suite | Source queries | Contract build | Native contracts | Interpreted contracts |
| --- | ---: | ---: | ---: | ---: | ---: |
| Previous macro producer 77695785 | 288.54 s | 70 s | 3 s | less than 1 s | 216 s |
| Current type-outlives producer a2d0d858 | 289.42 s | 68 s | 3 s | less than 1 s | 218 s |

Phase durations use whole seconds; the independent process timer gives the whole-suite values. The current total differs by 0.30 percent in this single serial pair. This is not a statistical benchmark, but it provides no evidence of a material type-outlives slowdown. Interpreted contract execution dominates the local duration. The original CI log cannot retrospectively distinguish its last three contract commands; the new markers make that phase visible in future failures.

## Exact correction

Source commit: 57efff2ee20d047464478a95d976d572d0ff17e4.

Only tools/check.sh and tools/macro-origin-test.sh change. The complete host suite gets a finite 1,800-second watchdog plus the existing 10-second termination allowance. This adjusts a test watchdog, not a language acceptance or performance threshold. Every original assertion and compiler/native/interpreter command is byte-identical after removing only instrumentation.

The script records each phase and preserves the original exit status during cleanup. Default fixtures are removed. Explicit LKJSCRIPT_MACRO_KEEP=1 retention removes the ephemeral key first. Fresh final-revision controls verify default failure, default TERM, and explicit-retention TERM, including no key output and no default fixture left behind. HUP, INT and TERM have their conventional distinct statuses.

The local pair ran the frozen first instrumentation revision, which is archived with its hashes. The final revision changes only cleanup/status handling and the explanatory comment; separate controls verify those final changes. Both earlier retained real fixtures have had their ephemeral keys removed.

Compiler a2d0d858 and seed 9327f64a remain unchanged. No compiler source, library, checked IR, checker, interpreter, runtime or malformed-origin fixture changes. The exact failed CI receipts, first revision, controlled phase logs and final cleanup controls remain available in this directory.

The corrective full gate and public CI are still pending. This diagnosis does not turn the original failed run into a pass.
