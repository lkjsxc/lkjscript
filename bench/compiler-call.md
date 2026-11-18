# Replacing one authored call through a checked transaction

This slice adds `replace-call`: an agent can replace one existing authored call or
method-call expression using its queried semantic identity. It no longer has to
submit the entire owning body for that change. The complete candidate still goes
through the existing scope resolution, independent checked IR boundary and
authenticated transactional writer. Text remains the stored projection.

The controlled duplicate-call experiment produces the exact desired source,
checked meaning and result 43 on both execution engines. The focused unfamiliar
trials are mixed: Astra selects the intended inner call and reaches the desired
meaning; Luna selects the enclosing call, checks successfully and misses the
meaning target. These outcomes establish an expressible checked operation and one
successful unfamiliar use. They do not establish reliable agent selection, a
population success rate or a large-program performance improvement.

## Inputs and scope

| Input | Identity |
| --- | --- |
| Pre-change published source | `d6401d43812c224ceb03d9775057386375444d0e` |
| Baseline fixed-point producer SHA-256 | `77695785a3c15da46bd9a3a794e81a2edf4c0155a891ff0cef2084d7bc3659d8` |
| First implementation | `2bdb58eaf616185b2dbdb0518a287b409975ce4e` |
| Delimiter/comma correction and nested-call guidance | `8fdfe776546a04577686b36719fb7cecd840ec5e` |
| Integrated published Programs parent | `739c8cde24f8df4644274451d5f39c23302a8af6` |
| Integration merge used for the predeclared unfamiliar pair | `fdd5531022fdbf2b4f499ad37ed3d761745a948c` |
| Raw-expression parser correction | `21a54f132c21dab2298fdefdc0aa5f850a97a433` |
| Final source input, with supported diagnostic access | `ecd0ff6d0e2210be5b7eaed9770b675b7c070554` |
| Published parent seed gzip SHA-256 | `9327f64a3c9a5c37146ca6c91a6926a8707d9fef3d9fd67398125909f4b5f612` |

All work is local to the `compiler-semantic-operations-20261118` worktree. The
canonical tree and its uncommitted measurement artifacts were read-only. The
published Programs parent was merged without refreshing the seed. No push or CI
result is claimed. The final producer and completed full-gate outcome are recorded in the
verification section below.

Production changes are limited to the transaction planner, edit/query help and a
crate-private parser entry point. There are no changes to name resolution,
checker trust, checked IR, optimizer, interpreter, backend, catalog identity
format or writer recovery. The narrow parser API calls the existing value
expression parser; it introduces no new language syntax.

## Why this operation

The starting point was the retained files-only versus command-only attempts,
including both frozen help checkpoints. Current help already describes request
JSON, so this is not another schema-help implementation.

Two retained Luna command attempts obtained all of the requested owner text but
submitted no changed candidate:

| Retained command attempt | Owner | Text returned | Completion |
| --- | --- | ---: | --- |
| Schema-help case 6 | host `exchange` | 3,466 UTF-8 bytes in four text chunks | `cursor=none` |
| Operation-help case 14 | `driver::structure::edit` | 7,777 UTF-8 bytes in eight text chunks | `cursor=none` |

The [schema query](compiler-call-data/retained-schema-luna-06-query.json) and
[operation query](compiler-call-data/retained-operation-luna-14-query.json) retain
the actual tool responses. Retained Astra
[schema](compiler-call-data/retained-schema-astra-06-edit.json) and
[operation](compiler-call-data/retained-operation-astra-14-edit.json) controls used
the existing `replace-item` route, submitting source strings of 4,026 and 4,023
bytes in the former request and 7,936 bytes in the latter. Whole-item editing was
therefore available; the gap was an addressable checked operation for the local
expression change, not an inability to retrieve source or edit at all. These
observations motivate the slice but do not prove why an agent stopped. Historical
checker or candidate results are not promoted to whole-task or full-gate success.

On the unchanged `const_function_calls.lkj` fixture, the baseline can query two
identically spelled `self.get()` calls separately. However, addressing either
call ID through `replace-body` or `replace-item` refuses `E_EDIT_IDENTITY`, because
those paths capture declarations only. `replace-call` is unknown to the baseline
and refuses `E_EDIT_OPERATION`. A whole-body control passes and reaches the
intended checked meaning. These are a reproducible operation gap and its existing
larger replacement alternative; see the retained
[new-operation refusal](compiler-call-data/baseline-call-refusal.txt),
[body-at-call refusal](compiler-call-data/baseline-body-call-refusal.txt),
[item-at-call refusal](compiler-call-data/baseline-item-call-refusal.txt),
[selected call](compiler-call-data/baseline-selected-call.txt),
[outline](compiler-call-data/baseline-outline.txt) and
[whole-body control](compiler-call-data/baseline-whole-body-control.txt).

The paired files-only Luna submissions did produce changed first candidates for
these cases, and their ordinary checker results passed. Both failed equality to
the hidden historical target meaning. The command companions submitted no
candidate. The [four original result rows](compiler-call-data/retained-companions.json)
retain that files/commands distinction together with their source-result-file
hashes. Neither side establishes a completed task or a comparative speed result.

Whole-program metadata retention remains a real scale issue.
 It is not what
causes this four-instance call-ID refusal, and this work does not claim to fix it.
The 10M-line experiment and the full stranger matrix were not repeated.

## Mechanism and checking frontier

A request uses the existing body token epoch and call ordinal:

```json
{"version":1,"operations":[
  {"op":"replace-call","id":"FUNCTION::body:BODY_DIGEST::call:N",
   "source":"value + 2","expect":"SELECTED_CALL_ITEM_DIGEST"}
]}
```

`expect` is the selected call's `item-sha256`, not its function's digest. The
optional request `snapshot` remains the complete input digest. A body token change
invalidates its call IDs. Trivia alone preserves the token epoch; exact call-text
expectations can still reject a changed comment or spacing at the selected call.
All operations in one request address the original catalog. Disjoint sites can be
combined; nested overlapping selections refuse before publication.

The planner requires an authored call-site item inside a validated authored
function body in the same source file. It validates the old recorded span as one
complete call or method call and the replacement as one complete value expression.
Delimiters must balance inside the raw supplied fragment. The new parser entry
point uses `expr(true)`, refuses recovered diagnostics and requires end of input.
It retains macro invocations without expanding them and uses ordinary authored
parser flags, so `self`, local names, generics and macros are resolved later in
the actual candidate scope.

Only after raw validation does the planner project `(`, newline, supplied source,
newline, `)`. This preserves precedence and prevents a trailing line comment from
absorbing surrounding syntax. The candidate then takes the ordinary full compile,
independent checking and source-revalidation path. Replacement may intentionally
change behavior or authority; the existing receipt reports those changes. No
equivalence to the old program is promised.

Generated macro or derive anchors remain refused. Parenthesized callees and
receivers sometimes have incomplete written spans in the current grouping-free
AST, for example `helper)(x)` from `(helper)(x)`. Those spans conservatively refuse
with `E_EDIT_CALL_SITE`; replacing the authored owning body remains available.
This feature does not redesign span provenance.

Call identities require full catalog capture. Both original and candidate are
still compiled, with existing authenticated instance reuse. Editing a selected
library call promotes validation to the entire explicit library, including
undemanded functions. There is no new unit-local cost bound, independent
per-definition checking boundary or proof reuse mechanism.

## Exact controlled result

The experiment selects the second `self.get()` in `Number::doubled`, leaving the
first identically spelled call and every other source byte untouched except for
the selected replacement span. The requested expression is
`add(self.get(), 1i64)`: **21 UTF-8 bytes**. The original and desired programs have
different behavior by design; the operation is compared against a separately
constructed desired whole-body control.

| Property | Recorded result |
| --- | --- |
| Original fixture SHA-256 | `873487978dc8d0891080042a90d5148f51181eb99da889b1b5720f5ced44df4c` |
| Selected original byte span | `[237,247)` |
| Selected call ordinal | `1` (`0` is its equal-text sibling; `2` is the enclosing `add`) |
| Selected call item digest | `1cf0a9ca71a6ca86882b1649e0d493667549af2ee2492a73d023b1e36bb0b715` |
| Exact edited file SHA-256 | `6d51fab3f0db5a3cc435ad54a99b9dba6c864fa3429765cbec57673508602e6c` |
| Original checked meaning | `e207108def576597aaa549a750e6f45129202ce28987e0be9790605d4442d1c6` |
| Desired and edited checked meaning | `250cfd42b58054221c7774132ebc0b95892b5cdedd57170751eba7a274fac4c2` |
| Native / interpreter result | `43` / `43` |
| Applied receipt frontier | four checked instances; two freshly checked, two reused |
| Resumed frontend instances | four on the warm apply after the dry-run |
| Authority delta | none |

The [request](compiler-call-data/call.request.json),
[dry-run](compiler-call-data/controlled-dry-run.txt),
[apply receipt](compiler-call-data/controlled-apply.txt),
[comparison](compiler-call-data/controlled-compare.txt),
[native output](compiler-call-data/controlled-native.txt) and
[interpreter output](compiler-call-data/controlled-interpreter.txt) retain the
primary evidence. The response to the old authenticated query pin remains byte-identical after
the edit; the reviewer also checked the pinned outline independently.

On the first implementation producer
`bab2b7ed2a94a36838e623a253f3378badd677cacd72b6672917a580cc112201`,
the controlled dry-run took 0.37 s wall, 0.36 s user and 32,448 KiB maximum RSS;
the following warm apply took 0.37 s wall, 0.34 s user and 32,704 KiB. The sampled
host load was 6.40 before and after. These are single observations on a four-instance
fixture, with a prior indexed baseline and the apply warmed by the dry-run. They
are not final-producer timing, an isolated checking cost or a speedup measurement.
The exact [time](compiler-call-data/controlled-dry-run.time)
[records](compiler-call-data/controlled-apply.time) and load samples are retained.

## Shape defects found and corrected before the full gate

Review was substantive, and the rejected candidates remain identifiable:

1. Adding projection parentheses before validating raw source allowed
   `1i64) + 2i64 + (3i64` to borrow the wrapper's delimiters. The old dry-run accepted
   it. Independent delimiter balancing fixed that boundary; the
   [request](compiler-call-data/fragment-boundary.request.json) and
   [pre-fix receipt](compiler-call-data/fragment-boundary.pre-fix.txt) are retained.
2. Parenthesized validation could turn bare `40i64, 2i64` into a tuple expression.
   Independent review found this. Validation moved to the raw source; explicit
   `(40i64, 2i64)` remains a positive control.
3. A synthetic function tail uses statement-position grammar and rejected valid
   block-, `if`- and `match`-led arithmetic, plus a block cast. The final narrow
   API calls the existing value-expression parser directly and requires EOF.
   Recovered malformed blocks also refuse. An initial build of that helper used
   unsupported `Vec::remove`; it was corrected to the already-used
   `first().cloned()` form. That failed bootstrap log is retained and is not
   reported as a compiler fixed point.

The [focused suite](../tools/structure-call-test.sh) is part of `tools/check.sh`.
It covers exact multi-file source projection and desired checked meaning, both
engines, selected duplicate calls, disjoint writes, dry-run, stale expectations,
stale source snapshots and body epochs, trivia, unknown fields, non-call targets,
empty or statement source, delimiter escape, bare comma versus explicit tuple,
block/if/match arithmetic, block casts, inferred closure calls, recovered syntax
errors, nested overlap, grouped spans, generic/method scopes, real-scope macro
replacement, generated macro/derive refusals and undemanded-library promotion.
Every refusal snapshots all source files and `.lkjscript-edit.generation`, requires
an empty success receipt and verifies that no pending transaction remains.

## Unfamiliar command-only replay

The [runner](compiler-call-replay.pl) uses the unchanged retained
`compiler-stranger.pl serve` MCP adapter, with fresh ephemeral `gpt-6-luna` or
`gpt-6-astra` agents, medium reasoning effort, a 24-call allowance and a 900-second
agent-execution watchdog. Setup time is separate. The adapter records requests
above the allowance and refuses them; it does not cap the recorded count at 24.
The watchdog has a one-second termination grace.

The adapter exposes compiler, gate and finish tools; help, query and edit are
compiler routes. The CLI disables shell, unified execution, apps, browser,
computer use, skill search and web access. The prompt directs the agent to use
only the experiment tools and forbids other filesystem or external access. These
settings do not establish an operating-system filesystem sandbox. All three
recorded trials made zero outside-tool calls. The private store key is outside
both granted source roots and is excluded from retained archives.

The task names the second `self.get()` and the desired nested addition but does
not name `replace-call`. The hidden target is constructed separately by the
runner. The first submitted source snapshot is retained and checked against that
target. Success at the ordinary checker and equality to the desired checked
meaning are separate fields. A successful runner exit alone does not imply task
success.

One exploratory trial preceded the generic nested-call-ID help revision. Then,
before outcomes were known, one fresh Luna and one fresh Astra trial were declared
on the integrated producer. Neither final trial was retried. Their source and
binary precede the later raw-parser grammar correction; they are not relabeled as
runs on the final gate producer.

| Trial | Producer | Calls / outside calls | Submitted / applied source bytes | First checker | Desired meaning | Raw wall s |
| --- | --- | ---: | ---: | --- | --- | ---: |
| Exploratory Luna | `8dca1f43…6431e` | 10 / 0 | 42 / 21 | pass | fail | 88.780368 |
| Predeclared Luna | `f76ae439…ea6d4b` | 11 / 0 | 42 / 21 | pass | fail | 81.776432 |
| Predeclared Astra | `f76ae439…ea6d4b` | 11 / 0 | 21 / 21 | pass | pass | 61.521430 |

Submitted bytes count UTF-8 `operation.source` strings, including repeated
requests. Luna's wrapper-option refusal followed by an applied request is
retained and counted twice. Both Luna trials selected the enclosing `add` call,
not the requested second `self.get()`. Thus a checked program was produced, but
the desired edit was not accomplished. Astra inspected the nested call, selected
its ID, submitted one 21-byte replacement and reached the exact target meaning.

The predeclared Luna and Astra tool payload totals are 24,801 and 26,717 bytes,
respectively. These include help/query interaction and are not replacement-source
size. The trial summaries explicitly mark the full repository gate and separate
native/interpreter execution as **not run within those trials**. The separate
controlled and focused suites supply engine evidence; the repository gate below
is also separate. All trial prompts, traces, requests, responses and first/final
snapshots are retained under `compiler-call-data/replay-*` with producer, adapter
and runner identities in the replay manifest. No comparison with the entire
historical cohort or claim of statistically established improvement is made.

## Compiler self-edit blocker and operation credit

Applying the implementation itself through the baseline compiler's authenticated
edit path fails before candidate planning: `E_IDENTITY` reports duplicate
`syntax::macros::expand`. A source module and function legally share that name in
different namespaces, while the current catalog key conflates them. The original
source remains unchanged and no success receipt is emitted. Lead approved guarded
source-file edits for this feature, so **zero implementation lines are credited
as operation-produced**.

The [original refusal](compiler-call-data/self-edit-refusal.txt) and a
[three-line legal namespace example](compiler-call-data/namespace-collision.lkj)
with its [request](compiler-call-data/namespace-collision.request.json),
[check stderr](compiler-call-data/namespace-collision.check.err) and
[edit refusal](compiler-call-data/namespace-collision.edit.err) preserve a small
reproduction. Namespace-aware catalog identities are a follow-up, not folded into
this call-expression change.

## Verification and reproduction

The final source input is `ecd0ff6d0e2210be5b7eaed9770b675b7c070554`. Its
40,989,463-byte producer has SHA-256
`103557eddf30d24e046cb09f6a881e549829283495c8648c0ac9dcd1d72a5d35`
and reproduces itself byte for byte. This bootstrap reused its store after the
recorded failed helper build; the full gate below starts with a separate empty
store. The recorded bootstrap took 110.74 s wall,
107.75 s user and 1,560,032 KiB maximum RSS. The final call-edit plus help tests
passed in 19.28 s wall, 18.25 s user and 153,848 KiB maximum RSS. These are complete
script timings, not isolated checking costs.

The [independent Astra review](compiler-call-data/reviewer/review.md) passed on
that exact source and binary: all 13
whole-body versus call-site expression pairs matched exact projected source and
checked meaning, including the four formerly rejected value expressions. Its
separate transaction controls passed the edited both-engine result 43, historical
pins, epochs, generated/grouped refusal, macro and closure use, malformed and
recovered syntax, stale generation and two-file atomic refusal. The reviewer did
not run the repository gate.

The serialized four-worker **full repository gate passed in 1,260.372 s** on that
same source and producer, starting from an absent store. It reran **6,575 case
requests with zero case-request reuse** and reran the host suites. This counter
concerns whole test-request skips; ordinary authenticated instance/object reuse
inside commands remains enabled. Both ordinary and optimized conformance report
1,357 cases, 2,712 route checks and **two pending entries**, which remain pending.
The mutation stage ran 128 mutants: 104 rejected, 22 values, one trap and one
nontermination, with zero resource outcomes and **zero findings**. Its ordinary
internal reuse counters were 12 checked and five emitted reuses.

The [full log](compiler-call-data/gate/full-gate.log),
[exact input](compiler-call-data/gate/input.txt),
[provenance](compiler-call-data/gate/provenance.txt),
[output hashes](compiler-call-data/gate/output.sha256),
[time record](compiler-call-data/gate/time.txt) and
[machine-readable verification](compiler-call-data/verification.json) retain the
result. Native output is byte-identical to the reviewed 40,989,463-byte producer,
and the public seed is unchanged. GNU time reports 3,160.62 s user, 82.25 s system
and 1,576,788 KiB maximum RSS for the script; that RSS metric is not a measurement
of combined simultaneous memory across four workers. Host load was 2.69 at start
and 3.79 at finish. No speed comparison is inferred from this single shared-host
run.

The exact source, library, tools, tests, witness and seed inputs remained identical
to the tested source commit throughout. The documentation diff and report hashes
at gate start were recorded separately; the later report/evidence/benchmark-runner
commit does not change those compiler or gate inputs. The shared gate lock was
released, and the native gate process exited zero.

The real-programs suite passed **inside** this full gate; its
[receipt](compiler-call-data/gate-details/real-programs.log) retains the original
library/provenance controls, eight tests per engine and 7,239 native corpus rows.
There was no extra standalone real-programs rerun, no independent Rust-witness run
on this final producer and no CI run for this unpublished slice. Lead will decide
a separate landing and seed handoff after the public parent's CI; this change is
not bundled with the type-defaults publication.

The normal reproduction in this checkout is:

```sh
bash tools/bootstrap.sh
bash tools/structure-call-test.sh target/lkjscript
bash tools/structure-help-test.sh target/lkjscript
LKJSCRIPT_GATE_JOBS=4 LKJSCRIPT_STORE=target/call-reproduction-store bash tools/check.sh
```

Use a new empty gate store and the shared crew gate slot. The source/seed/producer
identities and actual run times in the final verification record specify the
measured input. The regression suite creates its own temporary source roots and
private key and removes them on exit. The replay runner can reproduce a new trial
in a new output directory when explicitly wanted; it does not overwrite earlier
trials. Neither reproduction requires the 10M build or full stranger matrix.
