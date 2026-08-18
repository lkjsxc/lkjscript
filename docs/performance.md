# Current evidence and architecture decisions

This file interprets current reproduced measurements and reversal evidence. Exact machine-readable
campaign facts are in
[`evidence/20260818-stateful-instance.json`](evidence/20260818-stateful-instance.json). Older
evidence files remain historical baselines, not current status.

## Measurement boundary

The campaign started on branch `main` at audited commit
`18740edc55bd31162085c027d9e965163f1e119f`. The checkout equalled, rather than preceded or
followed, that baseline. Pre-existing work was a modified root `AGENTS.md` and untracked active
campaign prompt `prompts/202608181036.md`; both remain user-owned and preserved. The effective root
policy SHA-256 is `0ff2bf906987f31937c11d12e3e50e60f8e10f5dd814d0f381c777b7e432e501`
and is 21,881 bytes, below its 24 KiB budget.

The observation host is Linux 7.0.0-29-generic x86-64 on ZFS, AMD Ryzen 9 9955HX, 32 logical CPUs,
32 GiB memory, rustc/cargo 1.96.0, and stable Rust edition 2024. `Cargo.lock` SHA-256 is
`d23b75fc162e485b7149d92f1e3349f3cca39f00420a9fef68f8abea6c405620`.
Times below are single observations unless a sample count is stated. They are not distributions.
`/usr/bin/time` is absent, so maximum RSS is unavailable. Provider input, cached-input, output, and
reasoning tokens, provider calls, pricing, and exact monetary cost were not exposed. Bytes are not
tokens.

At the starting checkout, 234 tests were discovered: 230 passed and four manual measurements were
ignored. `src/` plus `tests/` contained 2,474,799 bytes / 70,489 lines. Optimized binaries were
7,920,072 bytes for `lkjscript` and 4,252,144 bytes for `lkjscriptd` (12,172,216 combined). The
reusable-release workflow used 56 processes, nine Engine opens, 33,370 action bytes, and 84,805
observation bytes.

The current tree has 2,562,705 bytes / 72,900 lines across 58 files under `src/` and `tests/`:
2,303,117 source bytes / 65,738 lines and 259,588 integration-test bytes / 7,162 lines. That is an
87,906-byte (3.6%) and 2,411-line increase after adding the complete instance vertical while deleting
daemon/transport code and tests. Cargo metadata now exposes one installed binary. The optimized
`lkjscript` is 8,312,104 bytes; compared with the two-binary baseline installation this removes
3,860,112 bytes (31.7%), although the remaining binary itself grew 392,032 bytes (4.9%).

One incremental optimized LTO build after source changes took 112.389 seconds. One all-target,
all-feature test run including incremental compilation took 17.391 seconds. These single values are
observations, not clean-build or stable performance claims.

## Selected stateful architecture

The retained design is pure one-command suspension with full canonical state records and a separate
trusted host executor. State publishes before host execution; an immutable typed outcome is resumed
by another pure transition. The production capability is one exact activation slot. A disjoint
executor-bound fake host is the independent lifecycle oracle. One store-wide lock gives canonical
serial order.

### Serious alternatives

| candidate | workload result | cost and safety result | disposition / reversal |
|---|---|---|---|
| pure `(state,event)->(state,response)` only | closed state persistence but moved validation, activation phase, retry, and reconciliation intent into Python/Rust orchestration | smallest language surface, but host became the real controller | rejected; reconsider if all retained applications remain state-only |
| transition plus command batch | could encode the controller | added collection, cursor, partial completion, and ordering rules while the workload always has one causal command | rejected and no batch form retained; reopen for measured independent commands |
| one-command suspension/resume | expresses validation, activation, unknown visibility, reconciliation, retry, cancellation, and terminal states as ordinary semantic data | no opaque continuation, direct effect, collection, scheduler, or second semantic authority | selected; reopen if ordinary state duplicates control unacceptably |
| typed algebraic effects/handlers | effect signatures could describe host intent | does not solve authority, crash publication, or unknown result; adds type/lowering/handler/continuation surface for one action | boundedly rejected before broad implementation under the stop rule |
| direct capability values and host calls | concise call syntax | makes pure evaluation host-dependent and introduces resource/capability lifetime into interpreter and artifact semantics | rejected; reopen only if command data makes the host application-specific |
| external Rust/Python orchestration | useful as prototype/oracle | split authority and could not keep the transferable application as the workflow owner | retained only as driver/oracle, not product semantics |
| database/general workflow engine | could supply transactions/replay services | imports a scheduler/service/storage TCB far larger than five- and nine-record example histories | rejected; measure compaction/replay thresholds before reconsidering |

Application format 3 directly replaces format 2 because the invocation profile now owns exact
stateful event/resume/decision targets. Release format 1 remains sufficient: all required nominal
types and functions were already expressible. No new language value, operation, generic result,
sequence, map, text, clock, capability value, or host effect was added.

## Durable-controller observation

One `examples/durable-controller/run.sh` observation produced:

| measure | observed value |
|---|---:|
| authoring workspace RPC calls | 2 |
| Engine opens | 1 |
| total processes | 41 |
| action bytes | 56,595 |
| observation plus diagnostic bytes | 26,419 |
| summed command/RPC boundary time | 485,881,456 ns |
| controller application | 19,669 B |
| activated payload application | 16,479 B |
| primary retained revisions | 5 (revision 0 through 4) |
| primary retained record bytes | 6,312 B |
| secondary retained revision before tombstone | 8 |

The process count includes one workspace session and 40 short-lived release/application/instance
commands, including deliberate rejection and corruption probes. The session reduces two dependent
workspace RPCs to one Engine/process open; instance operations remain process-per-command. This is
an operational count, not a provider-cost proxy.

The workflow proves source-workspace and standalone-release deletion, exact embedded application
replay, process restart between instance calls, validate/apply parity, duplicate event/result
replay, stale rejection, two instance/grant domains, denied cross-executor use, production slot
activation, deterministic fake unknown outcome, reconciliation absent, retry, known failure,
cancellation, bounded history, corrupt record/outcome rejection, tombstone deletion, and no identity
reuse. The exact slot bytes equal the validated payload artifact.

The current record counts and sub-20-KiB applications do not cross any journal, database,
compaction, external bundle store, executable cache, or instance-session threshold.

## Architecture compression decisions

| domain | retained result and measurement | rejection/deletion and reversal |
|---|---|---|
| global workspace contract | retained 153,227-byte `contract.rs` for workspace schema digest, root projection, help, and strict client consumers | instance family uses an 18,838-byte command-local CLI owner plus its semantic owner; macro/proc-macro generation rejected because it adds expanded/derived review surface without displacing the workspace owner; rerun frozen changes if broad discovery recurs |
| daemon/transport | deleted binary, client, socket framing, public exports, docs, and tests | unique lock, disconnect, shutdown/flush, and restart behavior moved to direct Engine/session tests; reopen only for a measured multi-client authority |
| binary topology | one 8,312,104-byte binary versus 12,172,216 bytes across two baseline binaries | no worker added; add a binary only for a proved security/lifecycle boundary |
| managed immutable bytes | retained with allocate-new differential oracle; durable controller does not expose a contrary copy bottleneck | no new planner feature; reopen if broader stateful values reverse absolute benefit or verifier maintenance dominates |
| workspace storage | full snapshots and deterministic scans retained | release/controller workloads show no restart/query bottleneck; prototype one delta/object design only after two measured thresholds cross |
| instance storage | full canonical state record per revision plus HEAD; five primary and nine secondary records | journal/database/compaction rejected; unresolved evidence and event receipts must survive any later cutover |
| application distribution | embedded exact graph retained at 16,479–19,669 bytes in controller workload | mutable store rejected; reopen for measured repeated-bundle cost across current deployments |
| compiler/interpreter | decode, graph validation, lowering, Core verification, and explicit-frame execution remain one exact route | no serialized IR, bytecode, JIT, native cache, or second runtime; optimize only a measured dominant term |
| module ownership | cohesive `instance.rs` is 87,510 bytes with command CLI separate at 18,838 bytes | no forwarding split; split when a subsequent change repeatedly crosses an independently nameable codec/persistence/host boundary |
| active prompts | two tracked superseded campaign prompts deleted; Git retains history | the current untracked 320,321-byte user-supplied campaign is preserved rather than overwritten/deleted; remove only with explicit ownership authorization or a later commit decision |

The command-local contract experiment compared four ownership shapes for the instance family. Manual
global metadata would enlarge the already-consumed workspace catalogue; `macro_rules!` would still
require separate cross-field validators; a proc macro would add a build dependency and expanded
diagnostic surface; strict local serde plus explicit semantic validation provides one reviewable
owner and passed unknown/duplicate/trailing tests. The local owner was selected; no losing generator,
schema root, or generated file remains.

## Agent-facing economy

The instance command help is 1,777 bytes. The global full workspace schema response is 136,796 bytes
and its manifest is 1,245 bytes. Instance creation, operation, diagnosis, unknown reconciliation,
history, and deletion use only command-local help plus exact inspection; they do not require the
136-KiB catalogue. The root repeated policy remained byte-identical at 21,881 bytes. Two historical
tracked prompts totalling 13,147 lines were removed from active discovery.

Deterministic task oracles cover creation, adding state/event transitions, stale base, denied grant,
unknown result, minimum resume facts, corruption, exact application binding, review via inspection,
and tombstone deletion. The public driver completed the operational subset with zero correction
rounds. This is not an independent model trial. Model/version/token/cost telemetry and multiple
provider attempts were unavailable, so no claim is made about weak-model success, prompt-cache
tokens, or monetary savings. A provider-backed equal-task comparison remains an explicit roadmap
gate rather than an estimated result.

## Verification and adversarial evidence

- Generic instance envelopes reject every truncation point, deterministic one-bit mutation, wrong
  version/digest, and trailing byte, with byte-identical re-encode.
- Exact and one-over policy, event-key, and path grammar values are tested. Application format 2,
  contract 2, and old string profiles reject.
- Explicit state-machine tests enumerate creation-directory, immutable-object, HEAD, and activation
  boundaries before/after write, file sync, link/rename visibility, cleanup, and directory sync.
- Public integration covers two processes/instances, restart, response replay, stale bases,
  capability denial, unknown/reconciliation, deletion, source removal, and retained-byte corruption.
- The full explicit-frame interpreter and managed allocate-new differential remain applicable.
- Nightly Miri 0.1.0 passed all 11 current instance unit/fault tests in 9.84 seconds with
  filesystem isolation deliberately disabled for temporary-directory syscalls. The initial isolated run failed
  before tests because Miri forbids `mkdir`; it was not counted as a pass.
- `cargo-fuzz` is not installed. Coverage-guided fuzzing, sanitizers, external formal tools,
  cross-platform execution, and provider trials were not run. Deterministic mutation and the finite
  publication model are retained; no formal-verification claim follows.

## External comparison

Primary sources were consulted for failure dimensions rather than copied architecture:

- [Temporal documentation](https://docs.temporal.io/) informed deterministic workflow versus
  externally fallible activity separation; no service, scheduler, or opaque replay engine was used.
- [Koka's book](https://koka-lang.github.io/koka/doc/book.html) informed comparison with typed
  effects; explicit effect signatures were not confused with capability grants.
- [SQLite atomic commit](https://www2.sqlite.org/atomiccommit.html) informed file/directory sync and
  failure-point analysis; no database was imported.
- [WebAssembly Component Model concepts](https://component-model.bytecodealliance.org/design/component-model-concepts.html)
  informed closed interface comparison; no component ABI/runtime was added.
- [OpenAI model guidance](https://developers.openai.com/api/docs/guides/latest-model) informed the
  preference for stable lean instructions and representative evals; provider telemetry was absent.

## Reversal summary

Reopen the selected design for a concrete application that needs command parallelism, a live
resource, unattended time, revocable grants, hostile-code isolation, state migration, or a second
capability that justifies a shared narrower abstraction. Reopen storage only after measured state or
history thresholds. Reopen agent surfaces only with frozen deterministic tasks and actual provider
telemetry. In every case retain the current pure interpreter, full-history replay, and fake-host
oracle until direct cutover is complete.
