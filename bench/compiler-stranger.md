# Fresh agents changing a compiler

This experiment gives twenty fixed historical compiler changes to a new agent in
each of two arms. The cases are the first twenty successful native-.lkj rows in
compiler-self-history.tsv. Their revision IDs, intent and affected paths are
frozen in compiler-stranger-cases.json. A trial sees an ordinary change request,
not the desired patch or a replay operation sequence.

The files arm can list, search, read and edit source text. The commands arm can
call only the compiler's own query/edit commands and its printed help. Both can
submit a candidate to the same case checker and can finish with a reason. The
stdio MCP adapter exposes only that arm's tools; shell, unified exec, apps,
browser, computer use and host skill discovery are disabled in the fresh Codex
process. The process uses an empty temporary working directory and an ephemeral
conversation. Existing normal project trust and automatic approval review stay
enabled. A setup approval refusal is infrastructure failure, not language evidence.

## Baseline discovery

The complete gpt-6-luna baseline in compiler-stranger-initial.json contains twenty
fresh attempts per arm with the frozen b4594275-era producer. Every row keeps its
actual calls, bytes, time, submitted source, checker and target comparison results.

| Measurement | Files arm | Commands arm |
| --- | ---: | ---: |
| Tool calls | 410 | 304 |
| UTF-8 payload bytes returned | 3,699,592 | 954,192 |
| Agent wall seconds | 3,306.150 | 1,824.132 |
| Active setup-tail seconds | 0.076 | 2,079.338 |
| Changed first proposals | 15 | 0 |
| Unchanged first submissions | 1 | 1 |
| First checker pass / fail / not-run | 13 / 1 / 6 | 1 / 0 / 19 |
| First canonical target equality | 0 | 0 |

Both case 17 arms first submitted the unchanged parent. The files arm edited
afterward. Twelve changed first proposals passed the checker, one failed, and two
were submitted after the tool allowance so no checker ran. Full repository gates
remain not-run: the seventeen submissions form sixteen distinct exact gate inputs.
No complete implementation success is established by a static checker pass; exact
historical equality is stricter than behavior, and case 1 files comparison was
unavailable because of an early selector-path bug.

The command agents made 58 request-bearing invocations. Fifty reached the binary
and all were refused: 12 VERSION, 33 REQUEST and 5 OPERATION diagnostics. Eight
were wrapper-option refusals across cases 1, 3, 6, 11 and 13. The adapter documents
raw JSON in its request field; literal --request/--against options are refused.
That wrapper friction is a separate experimental limitation.

Manual overlapping coding of final explanations finds operation/request discovery
or construction in 16 cases, followed by body, identity or dependent-context
discovery in 14. Their companion files arms produced changed checker-passing first
proposals in eleven and seven cases respectively. These contrasts motivate help
work but do not establish completed-task or full-gate success. Eleven files arms
encountered the explicit tool allowance; none of the commands arms exhausted it.

The first six pairs already established missing request schema and body discovery,
so query/edit/compare help was implemented while the frozen baseline continued.
It prints exact operation objects, identity lookup, body/text continuation,
source/library pairing, and diagnostic recovery pointers. A later unsupported-op
observation added the OPERATION recovery pointer. Focused tests execute the first
complete printed request and check native/interpreter agreement.

The intermediate 9db99295 producer and later 2a761133 producer are frozen separately;
their complete twenty-case repeats are still pending and are not claimed by this
baseline. The fresh gpt-6-astra control uses the original baseline producer.
Shorter time before giving up is not a successful-workflow speedup. The recorded
847.343-second setup pause is separate; the original cohort lacks complete
preparation timing, so its active setup-tail measurement is not end-to-end setup.

## Run the fixed population

Use an already built compiler. The runner freezes its exact bytes once per output
directory. A later invocation with a different compiler refuses that directory.

~~~sh
perl bench/compiler-stranger.pl campaign \
  /path/to/lkjscript /path/to/compiler \
  /tmp/lkjscript-stranger-before gpt-6-luna all files,commands 900 24
~~~

The last two arguments are explicit experiment allowances: 900 seconds per
agent and 24 tool calls. They are not compiler resource policies. The runner
records over-allowance tool errors too. Cases may be selected by comma-separated
IDs, and an arm may be selected as files or commands. Two workers require
disjoint case selections and separate output directories.

After a change to the commands, run the same complete population with the new
compiler and a new output directory. The desired historical programs stay fixed:

~~~sh
perl bench/compiler-stranger.pl campaign \
  /path/to/lkjscript /path/to/new-compiler \
  /tmp/lkjscript-stranger-after gpt-6-luna all files,commands 900 24
~~~

Run all twenty changes in both arms for gpt-6-luna and gpt-6-astra, with separate
before/after output directories for each model. Both use the same fixed medium
reasoning effort, normal automatic approval review, and the same explicit
allowances. Check the normal advertised model configuration before starting; a
CLI/model setup failure is not language evidence. Completed arms are retained
and skipped on resume. An unfinished arm requires inspection; the command does
not silently start another agent on an uncertain edit.

## Pause for the shared host

There is one pause flag: COHORT/pause. Use the coordinator commands so an already
running cohort also stops at a boundary:

~~~sh
perl bench/compiler-stranger.pl pause /tmp/lkjscript-stranger-before
perl bench/compiler-stranger.pl resume /tmp/lkjscript-stranger-before
~~~

Pause stops only this experiment's coordinator and lets its current compiler or
agent finish. It returns drained=true after the active children have exited. The
agent's own finish time is preserved; the next case does not start until resume.
A paused active agent still has its original experimental wall allowance. The
coordinator identity is checked before signals are sent. If pause drains an index
build before the agent starts, resume records that idle interval in a separate
coordinator-pauses.json sidecar. Reports keep the raw setup_seconds and show
setup_pause_seconds and setup_active_seconds separately; the old coordinator's
historical timing is never silently rewritten.

## What is measured

Each arm retains its task text, model event trace, exact tool requests and replies,
first-candidate snapshots, final source changes and result metadata. The desired
tree is available only to the coordinator. Its contents are not exposed as a task
solution.

* payload_bytes counts the exact UTF-8 text returned in MCP results, including
  help, diagnostics and refusal messages. response_bytes additionally counts
  the MCP result envelope, excluding the JSON-RPC transport wrapper. task_bytes
  is separate. Text is encoded once; byte counts use UTF-8, not character counts.
* calls counts requests that reached the adapter, including refusals.
  attempted_tool_calls also counts model calls that failed before execution.
  Any tool call outside the experiment is recorded as outside_tool_calls.
* wall_seconds runs from preparation completion to the agent's finish.
  setup_seconds is separate: the original metric starts after archive/key/hash
  preparation and covers index setup and its bookkeeping. New preparations also
  record preparation_seconds from entry, including those earlier operations;
  historical missing values remain unavailable. setup_active_seconds subtracts only
  separately recorded idle setup pauses; preparation_active_seconds does the same
  when a complete preparation measurement exists. Commands need a prebuilt index;
  the frozen compiler publishes one only through build --index.
* submission records the saved first proposal, including an empty source delta,
  explicit tool-allowance refusals and the final finish request even if the
  allowance refused it. An unchanged parent can pass the checker without
  implementing the requested change; keep those outcomes distinct.
* checker_first says whether the first explicit gate candidate passed the existing
  independent checker. A missing submission is not-run, not a failed check.
* edit_metrics separates help/usage discovery from request-bearing invocations,
  retaining the first request exit, diagnostic, operation names and dry-run kind.
  An edit transaction checks its candidate before publication. A later explicit
  gate pass does not mean that the first edit request was accepted. Rejected edit
  requests and their exact refusals remain in the raw tool log; they have no
  published source candidate to submit to the full repository gate.
* target_first reports exact canonical checked-meaning equality to the historical
  target. This is stricter than behavioral equivalence: a different valid
  algorithm or diagnostic text can differ. An oracle setup failure is unavailable.
* full_repository_gate remains not-run unless the actual tools/check.sh gate has
  been run on that saved candidate. A successful checker or comparison is never
  described as a full repository gate pass.

The case oracle checks the complete selected historical compiler root. It does
not establish behavior outside that root or replace the fixed point, mutants or
CI required when a command change lands. Native/interpreter and full-gate evidence
must be named separately. CLI bootstrap probes, including approval/configuration failures, are outside the
measured population.

Read results.json with:

~~~sh
perl bench/compiler-stranger.pl report /tmp/lkjscript-stranger-before
~~~

Compare completed changes before comparing time. An arm that gives up sooner is
not evidence of a faster successful workflow. Rank command-only blockers against
files-arm successes, retaining unavailable and ambiguous outcomes rather than
silently scoring them as success or failure.

## Run the repository gate on the first submitted candidate

The agent's first explicit gate attempt saves its current source before allowance
checking. An attempt beyond the allowance still refuses and runs no checker, but
its proposed source is retained for later external validation. Use that snapshot,
not the agent's corrected final files, for the separate full-gate metric:

~~~sh
perl bench/compiler-stranger.pl full-gate \
  /path/to/lkjscript /tmp/lkjscript-stranger-before/01-files \
  /tmp/lkjscript-stranger-before-full-gate-01-files
~~~

This creates a detached historical worktree with tests and bootstrap seed from the
target revision, restores the pre-change src/library tree, and applies the exact
saved first candidate. Its own tools/check.sh builds the candidate compiler and
runs the historical fixed point, mutants and suites. The frozen trial compiler is
not used as the compiler under test. The command reserves four gate workers and
must run while the campaign and other builds are paused.

List distinct gate inputs before reserving a gate window:

~~~sh
perl bench/compiler-stranger.pl gate-inputs \
  /path/to/lkjscript /tmp/lkjscript-stranger-gate-plan.json \
  /tmp/lkjscript-stranger-before
~~~

The plan groups identical source deltas with the same source parent and historical
test/tool/seed revision, and provides exact full-gate argument arrays. It records
unsubmitted and unfinished trials separately. An input identity is bookkeeping;
it makes no claim that a gate passed.

Historical gates share REPOSITORY/target/stranger-history-store by default and use
the private key returned by the normal tools/store-key.sh. Optional STORE and KEY
arguments select another external persistent store and existing private key.
Only the infrastructure paths change: normal producer, runner, artifact, closure
and closed-input authentication still decides reuse. The store and key must stay
outside the ephemeral gate worktree, and the key outside the store. The same
actual tools/check.sh runs with LKJSCRIPT_GATE_JOBS=4 for every candidate.

The gate evidence records both revisions, the candidate snapshot and resulting
compiler hashes, exit status, duration and raw output. A failed historical
baseline or a test-harness failure needs separate attribution; neither is silently
counted as an agent regression. No attempted gate means not-run. Early text-arm
snapshots are recoverable by replaying the exact recorded successful writes up to
their first gate attempt; the recovery record identifies those calls and whether
the experiment actually ran its checker. Case 8 files reached the gate after its
24-call allowance: its refusal remains recorded, and its source was recovered
from successful writes 23 and 24 before gate attempt 26. A later external full-gate
result does not turn this into an in-allowance workflow success.

## Adapter regressions

~~~sh
perl bench/compiler-stranger-adapter-test.pl
~~~

This invokes no model or compiler. It verifies non-ASCII source survives the MCP
JSON transport with exact UTF-8 byte counts, distinguishes successful help from
request acceptance, and pauses/resumes only its synthetic private coordinator.

The first 202 actual baseline tool replies (cases 1--6 and drained 7-files) were
inspected after discovering a double-encoding bug: all were ASCII, so none was
corrupted or undercounted. The adapter was corrected before the next arm. The
recorded first edit invocation in the initial evidence is help or usage discovery,
not candidate acceptance. The drained seventh files arm had a successful finish
timestamp, so its measurement did not acquire idle pause time. Later adapters
record their loaded script hash on every tool result.
