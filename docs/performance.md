# Current evidence and architecture decisions

This file interprets reproduced measurements and reversal evidence for the generalized application
world/runtime campaign. Exact structured facts are in
[`evidence/20260818-generalized-runtime.json`](evidence/20260818-generalized-runtime.json). Older
evidence remains historical baseline, not current status.

## Measurement boundary

The campaign started on `main` at `d52f7cda0cc64f988dd3f48ea454ce1647601aa1`, exactly equal to
the audited baseline. Pre-existing user work was modified root `AGENTS.md` and untracked active
`prompts/202608181408.md`; both were preserved. Their SHA-256 values are respectively
`796ba183836f961eef0c4e2304b1f20b48f93918b4ed318a9d59984df569d3dd` and
`98f293d665b4a4c3052fb7f6247fc6d24b74769ea71df74b60b09eae20b7e61f`.

The observation host is Linux 7.0.0-29-generic x86-64 on ZFS, AMD Ryzen 9 9955HX, 20 exposed
logical CPUs, 32 GiB memory, rustc/cargo 1.96.0, and stable Rust edition 2024. `Cargo.lock` SHA-256
is `d23b75fc162e485b7149d92f1e3349f3cca39f00420a9fef68f8abea6c405620`.
`/usr/bin/time` is absent, so maximum RSS is unavailable. Provider model, input/cached-input/output/
reasoning tokens, pricing, and exact provider cost were not exposed. Bytes are not tokens.

The audited/reproduced baseline had 230 passing and four ignored tests, 2,562,705 bytes / 72,900
lines under `src/` plus `tests/`, and an 8,312,104-byte optimized binary. The current measured tree
has 2,689,232 bytes / 76,324 lines in those roots: +126,527 bytes (4.9%) and +3,424 lines (4.7%).
The optimized binary is 8,821,040 bytes: +508,936 bytes (6.1%). One changed-source optimized LTO
build after the campaign changes took 120.60 seconds. This build observation is not a clean-build
distribution.

## Selected application-world and authority model

The selected model combines application-owned nominal command/outcome/decision sums with two closed
built-in immutable host-interface identities. An application import slot maps exact command and
request variants to one operation and compatible outcome variants. Resume is `(State, Outcome) ->
Decision`. Instance creation binds every slot to one immutable instance-specific grant.

| alternative | result | disposition and reversal |
|---|---|---|
| preserve activation-specific integer ABI | cannot express blob content publication without another magic family | rejected; v3 path deleted |
| generic command/outcome bytes | small runtime shape but moves type ownership to conventions/adapters | rejected; no byte envelope retained |
| application-owned nominal types | both controller and blob publisher use existing products/sums with exact local workflow vocabulary | selected |
| dedicated interface artifact/registry | independent distribution identity but no current independent publisher/resolver consumer | rejected; reopen for a third reusable-interface consumer |
| closed built-in interface identity | two exact narrow contracts, no mutable catalogue, application vocabulary remains local | selected bootstrap |
| direct interpreter host effects | concise but mixes purity, grants, live authority, and interpreter lifetime | rejected |
| general filesystem grant | implements both adapters but broadens authority and path-oriented semantics | rejected |
| immutable lifetime grant | satisfies both retained applications and keeps restart lookup exact | selected; new instance is current authority-change route |
| grant revision/mutable lookup | no rotation/revocation consumer and complicates pending unknown outcomes | deferred with no dormant format |
| dynamic plugins/workers | no untrusted external adapter consumer | rejected; no ABI/dependency/process retained |

Application format 4 and instance format 2 directly replace v3/v1. Release format 1 and workspace
semantic schema 6 remain sufficient because all request/outcome/decision types use existing nominal
language constructs. No language operation, effect system, release format, or global machine-schema
record was added.

## Complete application observations

One optimized durable-controller run after cutover used 41 processes and one workspace `Engine`,
57,880 action bytes, 42,197 observation/diagnostic bytes, and 449,545,031 ns summed command/RPC
boundary time. Its controller/payload applications were 19,890 / 14,486 bytes. The primary ended at
revision 4 with five records / 9,006 bytes; the fake lifecycle reached revision 8 before tombstone.

The immutable-blob publisher application is 17,609 bytes. It proves exact 4,096-byte application
business acceptance and 4,097-byte terminal handling, content-addressed publication, exact
already-present behavior, unknown-to-inspection reconciliation, fake known failure/retry,
production/fake and cross-instance denial, corrupt outcome rejection, bounded history, restart,
tombstone, and no reuse. Its primary history is five records / 8,956 bytes and retained content is
25 bytes.

Both workflows build through public authoring/release/application commands and delete their source
workspace and standalone release before instance operation. Python supplies inputs and assertions;
the lkjscript nominal state machines own workflow decisions.

## One-shot versus foreground session

The predeclared foreground gate was at least 20% complete-workflow latency improvement or at least
50% process reduction, without semantic duplication or weaker failure boundaries. The resident
supervisor gate additionally required an unresolved multi-client/aggregate-admission consumer.

The same optimized blob workflow ran once as unrecorded warm-up and then five measured times per
topology. Every sample used a fresh temporary workspace, release/application build, object
namespaces, and instance store; the binary and OS page cache were warm and were not cleared. Thus
these are fresh-authority/warm-host samples, not cold-OS measurements. `boundary_elapsed_nanoseconds`
is the sum of monotonic command/RPC waits recorded by the driver, not an external wall-clock/RSS
measurement. The topology samples ran sequentially; no measured workflow competed with the other
topology.

| topology | processes | samples | min ns | median ns | max ns | action bytes | observation bytes |
|---|---:|---:|---:|---:|---:|---:|---:|
| one-shot | 36 | 5 | 330,054,595 | 330,890,204 | 336,399,529 | 59,082 | 42,843 |
| foreground session | 5 | 5 | 288,461,787 | 292,133,309 | 316,441,803 | 61,945 | 49,845–49,852 |

The session improves median summed boundary time by 11.71%, below the latency gate, but removes 31
processes (86.1%), crossing the process gate. Its extra bytes include one detailed runtime
inspection. It retains because the caller owns lifecycle, malformed lines recover independently,
one common kernel owns semantics, and shutdown releases the exact store lock.

One-shot remains a thin supported adapter for shell composition. A Unix-socket supervisor was not
prototyped: after the session there is no demonstrated multi-client, durable-queue, or centralized
admission consumer. Therefore there is no socket protocol, daemon, auto-spawn, stale-socket path, or
fallback topology to retain.

## Stage evidence, cache, and execution tier

The foreground session exposes aggregated stages for the same 29 admitted instance requests. Across
five samples, median totals were:

| stage | median total ns | share of complete median boundary |
|---|---:|---:|
| state publication | 102,709,767 | 35.16% |
| deterministic instance replay | 78,268,409 | 26.79% |
| outcome publication | 24,041,683 | 8.23% |
| adapter preparation | 8,652,783 | 2.96% |
| application envelope decode | 9,905,500 | 3.39% |
| closure flattening | 8,486,937 | 2.91% |
| transition preparation | 8,534,732 | 2.92% |
| host action | 4,049,997 | 1.39% |
| record-chain validation | 2,014,862 | 0.69% |
| interpreter execution | 949,845 | 0.33% |
| lowering | 723,277 | 0.25% |
| Core verification | 100,089 | 0.03% |
| public-value validation/materialization | 30,821 | 0.01% |

The predeclared in-memory-cache gate required repeated validation/lowering to be material and at
least 20% complete-workflow improvement. Decode plus flattening is about 6.3% of the measured complete
median even before subtracting cache lookup/invalidation cost. Persistent cache adds hostile format,
target/compiler binding, atomic publication, eviction, and disk accounting without a restart benefit
large enough to repay it. No cache is retained; cache budgets/counters are explicitly zero.

The bytecode/native/JIT gate required execution to dominate and at least 30% complete-workflow
improvement after compile cost. Lowering, Core verification, and execution together are under 1% of
the complete median. The explicit-frame interpreter remains the sole tier/oracle; no bytecode,
compiler backend, executable-memory path, profile, or cache format was prototyped or retained.

Full records also remain: current five-record histories are small, and replay is the largest measured
stage but no second representative history crosses a service/retained-byte threshold. A later
replay optimization must preserve corruption traversal and the full-history differential rather
than infer a database requirement from this one workload.

## Runtime resource decision

The kernel separates semantic policy from deployment admission. Existing exact owners retain
application/graph/test bytes, public values, fuel, frames, cells, managed visible/backing bytes,
state, event, response, host evidence, history, replay, and blob object/count bytes. Runtime policy
adds request/response/application bytes plus one store, transition, host-operation, and compilation
slot.

Queue, compiled-unit, cache, and profile limits are exactly zero; a nonzero policy rejects. Runtime
inspection reports reservation release, request/application/replay/adapter counters, all named stage
slots, and explicit RSS/open-file/temporary-publication omissions. This is logical admission and
observation, not exact resident-memory enforcement. A synchronous store-wide lock stays because the
workload proves process amortization but not cross-instance throughput requiring per-instance locks
or a scheduler.

## Agent-facing economy

The measured development/runtime cost improvement is process count: the exact same complete blob
task falls from 36 to 5 processes. Runtime orientation is 1,194 bytes and runtime help is 582 bytes;
bounded expansion uses 988-byte application help, 1,558-byte instance help, and exact inspection.
The unrelated full workspace schema remains 136,796 bytes. Agents can discover the runtime without
opening that global catalogue or Rust source, and application authoring no longer requires magic
integer tags.

Deterministic validators cover interface routing, completed/suspended typing, grant binding, denied
authority, unknown/reconciliation, malformed outcome compatibility, exact/one-over resource policy,
cache-zero inspection, direct incompatible format rejection, and both complete workflows. These
tests are frozen machine oracles, not an independent model trial. Provider telemetry was absent, so
no token, weak-model, cached-input, or monetary claim is made.

## External primary-source orientation

Primary sources were accessed 2026-08-18 to sharpen tradeoffs, not import architecture:

- [WebAssembly Component Model worlds](https://component-model.bytecodealliance.org/design/worlds.html)
  reinforced explicit directional imports/exports and host fulfillment. lkjscript retained its own
  nominal semantics and did not add WIT, live resources, WASI, or a component runtime.
- [Wasmtime `Config`](https://docs.wasmtime.dev/api/wasmtime/struct.Config.html) distinguishes
  deterministic fuel from nondeterministic epoch interruption and notes that guest interruption does
  not solve blocking host calls. lkjscript kept deterministic semantic fuel and did not add async,
  epochs, Wasmtime, or a native tier.
- [V8 Sparkplug](https://v8.dev/blog/sparkplug) emphasizes real workloads and the compile-speed/
  execution-speed tradeoff of a baseline tier. Here interpreter execution is below 1%, so no tier
  pays rent.
- [Erlang/OTP runtime options](https://www.erlang.org/doc/apps/erts/erlang.html) informed the
  distinction between runtime scheduling and language meaning. No actor/mailbox or mid-transition
  consumer exists, so no scheduler was copied.
- [systemd transient settings](https://systemd.io/TRANSIENT-SETTINGS/) show process lifecycle and
  cgroup resource controls available at deployment. They do not replace semantic/runtime admission,
  and no service-manager integration was added.

## Reversal summary

Reopen interface identity for an independent reusable-interface consumer; grant revision for a
current rotation/revocation need; supervisor/per-instance scheduling for measured multi-client or
concurrency demand; cache only above the 20% complete-workload gate; and execution tier only when
execution dominates above the 30% gate. Preserve the pure interpreter, exact application validator,
full replay, and deterministic fake adapters until any replacement directly cuts over.
