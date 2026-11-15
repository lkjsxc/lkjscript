# Native generators and reduction

These IR seeds are deliberately pure: the mutation preflight rejects host types
and authority-bearing operations. Their text is checked before mutation; accepted
mutants run through the independent interpreter and native backend. The checker
and emitter run in the mutation worker, with one cached runtime per worker.

Commands run from the repository root:

```sh
target/lkjscript generate values 601 100 -o target/generated
target/lkjscript generate ownership 601 100 -o target/generated
target/lkjscript generate sugar 1301 216 -o target/generated
target/lkjscript generate closures 1501 44 -o target/generated
target/lkjscript generate integers 0 0 -o target/generated
target/lkjscript test target/generated --jobs 8
target/lkjscript mutate 1701 32768 --jobs 8 -o target/mutation
target/lkjscript reduce target/mutation/finding-17.lkjir --entry probe -o target/reduced.lkjir
```

Source families are values, ownership, output, drops, sugar and closures; all
selects all six. Values builds typed random statement sequences with prior-function
calls, casts, shifts, enums, mutable and returned projections, loops, and an
order-sensitive checksum of every live value. Ownership maintains a separate model
of live Vec, Box and struct owners while generating moves, lexical loans, captures,
replacement, nested branches and loops. It emits the model's exact checksum or a
seeded explicit E_MOVE/E_BORROW contract; no compiler labels its own verdict.
Sugar cycles through 85 forms and closures through 28 forms,
with seeded operands, branch choices, captures and cleanup markers. Output, drops,
sugar and closures carry an independently written reference program. Both sources
run through both routes. Each case has its expectation in the file; generated
reference files are not separate cases. The integers family always emits the
complete 195-case boundary table, with independent expected constants.

Mutation records only findings, never an observation ledger. The command reports
rejected inputs, values, traps, timeouts, resource refusal, findings and mutants
per minute. Checked execution failures are findings, not checker rejections.
A two-second child allowance separates nontermination from safety/model failures.
The reducer removes balanced statements/items or IR instructions, reruns both
routes and preserves the same disagreement and diagnostics. Rejected candidates
cannot replace executable evidence. `--reference SOURCE` also compares against a
fixed independent expansion that agrees on both routes. Exit status and program
stderr are compared separately from compiler diagnostics.

Implementation lives in src/test_generators.lkj and src/test_generators/.
`generate --self-test` checks deterministic mutation replay, stale edit detection,
authority exclusion, outcome classification and balanced reduction boundaries.

The separate `generate scale-FAMILY 0 SIZE` command emits source-size workloads
for funcs, methods, ifs, structs, boxes, expr, match, vecs, allocvec, alloclist,
traits, fields, variants and refs. `bash tools/scale.sh` runs them without Node,
reporting median wall time/RSS and the compiler's phase statistics.
