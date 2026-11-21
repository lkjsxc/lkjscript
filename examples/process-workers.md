# Isolated process workers

This is the runtime specimen for parallel work, bounded queues, shared mutation
through one owner, worker supervision, and immutable shared payloads. It is a
class of program, not a service to deploy. The parent retains its listener and
directory; an admitted worker starts with exactly one Channel.

## Boundary and implemented profile

Image, Worker, Channel and ReadRegion are opaque affine IR types. They are
neither Copy nor Data. The checker validates every constructor and operation
from IR alone; a source struct, producer-local type number or frontend promise
is not evidence of authority. The selected parallelism form is separate
processes. Each process owns its allocator and mutable memory. There is no
shared mutable heap, scoped thread spawn, Send/Sync rule, language mutex, or
atomic operation in this profile. Software needing one address space pays
process startup, message transfer and serialization costs.

An Image entry grant contains exact authenticated compiler-produced bytes,
manifest identity and image hash. The admitted shape is one owned Channel slot.
The launcher verifies a private sealed memfd and executes that same descriptor.
The child imports only its startup packet, result descriptor and channel;
ambient descriptors close on exec. Parent death is armed before exec and
checked against the captured parent PID. Spawn includes its startup handshake
in the caller's absolute deadline. Worker drop kills and reaps its child.
There is no detached form.

The entry report says what the child starts with and that its channel may later
receive a moved File from the parent. send_file consumes that File value on
success and failure; receive_file produces one owner. Explicit preexisting
File duplicates may still exist. Directory, Listener, Image, Worker, Channel
and arbitrary aggregate transfer shapes are unsupported. The current primitive
channel is bytes plus File transfer; the 17-byte frame in std::workers is an
ordinary library convention. It does not establish semantic message identity
or generated codec agreement. The compiler handoff is
[programs_semantic_message_identity.lkj](../tests/diagnostics/programs_semantic_message_identity.lkj).

Image.buffer copies initial bytes into a memfd and seals write, grow, shrink and
seal changes. File.map_readonly requires all four seals, retains its own
descriptor and maps PROT_READ. ReadRegion.bytes returns a shared byte slice
whose checked loan is rooted in the region owner. Dropping the original File
does not invalidate the region; moving or dropping the region while its view
is live is rejected. A sealed File travels through send_file; the receiver maps
the same kernel object without copying its payload. There is no mutable shared
mapping or generic typed-record layout in this first profile.

The admission key is a host provisioning secret and must remain outside every
program grant, including aliases. Image has no runtime bytes-to-Image or
File-to-Image constructor. Directory/ReadDirectory key exclusion is not yet an
enforced compiler rule: the retained
[key overlap diagnostic](../tests/diagnostics/programs_image_key_grant_overlap.sh)
demonstrates direct-root and hardlink alias exposure without printing the key.
A lexical path test cannot close that gap.

## Protocol specimens

[std::workers](../library/workers.lkj) supplies ordered_map and Supervisor.
ordered_map keeps at most one accepted job per worker, so its input queue has
the caller's width as its capacity. It collects private results in input order,
then records EOF, drains, sends shutdown, joins every worker and returns the
result. An error exposes no partial result. The selected admitted image
implements the pure function; function values and their captures are not
implicitly transferred.

The [pool specimen](../tests/fixtures/process/pool.lkj) records actual Dispatch,
Complete, Eof, Shutdown, Exit and Commit events. The independent
[protocol model](../library/process_model.lkj) checks its histories for widths
one through four and nineteen jobs. The same specimen has four producers
contend on eight map keys: one parent owns and serializes all 128 mutations.
The map model independently recomputes each reply from its preceding history.
This is the processes-only equivalent of a shared map under contention and a
bounded producer/consumer channel with shutdown.

The [parser supervisor](../tests/fixtures/process/supervisor.lkj) retains a
listener and file root, accepts eight requests, and moves only each accepted
connection File to one of two channel-only parser workers. Five requests
succeed, one malformed request is rejected, one worker traps and one exceeds
its request deadline. The library restarts each failed slot for a later
request, subject to its caller-supplied restart bound. A moved request is never
implicitly replayed. Exit status and private trap output remain values; the
parent writes the located trap bytes into its own granted root.

The interpreter independently executes the parent's checked operations and
owned-cell loans. Its host launches native children through its syscall broker;
it does not interpret those child instructions or control the Linux scheduler.
Its ReadRegion implementation uses an independently checked immutable
snapshot rather than a native mapping. Deterministic schedule exploration
therefore validates observable process protocols, not language-thread races.
The retained model explores 64 successful schedules, 64 death schedules and 64
map histories, including adversarial traces. Native and broker OS leaf code
share implementation structure, so agreement between them is supplemented by
an independent C witness.

The compiler consumer handoff is
[programs_compiler_pool_adoption.lkj](../tests/diagnostics/programs_compiler_pool_adoption.lkj).
It checks the pool call contract; compiler-driver adoption for function checking
and emission remains pending and is not a performance claim here.

## Measurements

All times below are native Linux x86-64 on the shared host, with the other
programs jobs paused for the four-worker measurement. Every row retains its
observed load average. No run was selected for being the fastest. The function
is a sequential xorshift recurrence; each job returns its final state. Results
and their commit traces are compared in order.

| Input | One worker | Four workers | Ratio | Load before → after |
| --- | ---: | ---: | ---: | --- |
| 32 jobs × 4,000,000 rounds | 221.669035 ms | 57.985313 ms | 3.822× | 8.24 → 8.24 |
| 32 jobs × 4,000,000 rounds | 243.400331 ms | 77.449783 ms | 3.142× | 8.24 → 8.24 |
| 32 jobs × 4,000,000 rounds | 220.285893 ms | 57.150402 ms | 3.854× | 8.24 → 8.24 |
| 4,096 jobs × 1,000,000 rounds | 6.915111572 s | 2.240069256 s | 3.087× | 6.76 → 6.03 |
| 4,096 jobs × 1,000,000 rounds | 6.759489433 s | 2.105873193 s | 3.209× | 6.03 → 5.65 |
| 4,096 jobs × 1,000,000 rounds | 6.779873284 s | 2.205594393 s | 3.073× | 5.65 → 5.34 |

The retained benchmark now uses the 4,096-job input. In those same three runs,
worker startup was 283.975, 379.464 and 220.414 µs; average small-message round
trip over 1,000 requests was 7.274, 6.779 and 7.068 µs respectively, with the same
per-row load readings. Startup measures spawn of an already admitted Image;
the parent's initial image authentication is outside that interval.

| 1 MiB logical payload, 32 requests | Elapsed | Logical MiB/s | Load before → after |
| --- | ---: | ---: | --- |
| Copy in 64 KiB packets | 22.050594 ms | 1,451 | 10.71 → 10.71 |
| Moved sealed descriptor and readonly mapping | 1.174289 ms | 27,250 | 10.71 → 10.71 |

These are logical payload transport rates with one sampled byte per 4 KiB
page, not full-memory read bandwidth. Both timed paths inspect the same 256
offsets per request and check the reply. Before timing, both paths separately
validate every byte of the 1 MiB payload and its full checksum. The shared
backing is created once outside the interval; each shared request still
duplicates, transfers, maps and drops its own File/ReadRegion. Copy sends all
payload bytes each time. The independent interpreter snapshot is deliberately
not used for the shared-memory performance number.

## Failure and checker evidence

The publication specimen publishes only after every worker joins, through a
parent-owned temporary file, file sync, rename and directory sync. Its retained
campaign kills the parent twenty times at six publication barriers and after
observing a newly forked child PID. Those eight kills need not reach user code. Every result is the complete old or complete new
128-byte value; thirty-two adopted workers are reaped. The campaign took
3.455 s, load 8.16 → 9.03. This is the atomic publication contract for this
pure-result specimen, not a sudden-power-loss experiment or a promise to roll back arbitrary File effects.

The optional independent OS witness holds a forked child before its first
instruction and proves that spawn's deadline kills and reaps it. It checks
absent, extra, truncated and credential ancillary records, malformed markers,
descriptor cleanup, caller buffers unchanged on failed byte receives, empty
buffer refusal, parent death, and inherited-descriptor closure. The combined
witness took 1.596 s, load 7.70 → 7.48.

The targeted IR campaign admits 46 controls and rejects 552 trials: 512
seeded signature trials over 128 distinct edits, 28 opaque-value attacks,
six File-use-after-send paths and six region-origin attacks. It also checks
48 Copy/Data predicates. Seed 901173 produced no checker finding;
diagnostics were E_BORROW 13, E_MOVE 14 and E_TYPE 525. It took 3.595 s,
load 10.34 → 10.15. Separate source/direct-IR controls reject live-view owner
drop/move, heap views, false static returns and forged mutable region slices.
Positive region cases cover empty maps, unsealed refusal, surviving original
File drop, independent aliases and identical errno behavior.

## Reproduce

The default gate uses fresh state and an OS-assigned loopback port, cleans its
own files, and runs beneath a private Perl subreaper that kills/reaps any
leaked test descendants. A Bash loopback client compares the exact HTTP
responses from the parser supervisor.

```sh
bash tools/programs-process-test.sh --compiler target/lkjscript
bash tools/programs-process-test.sh --compiler target/lkjscript --bench
bash tools/programs-publication-test.sh --compiler target/lkjscript
bash tools/programs-publication-test.sh --compiler target/lkjscript --campaign
bash tools/programs-process-witness.sh --compiler target/lkjscript
```

The last two campaigns use optional C witnesses; no C is linked into a language
image. Coordinate the four-worker benchmark on the shared host. Default
contracts include native, interpreter and standalone executions, authentication
refusals, actual pool/map transcripts, supervisor failure values, region
lifetime cases and the targeted mutation campaign. The runtime remains
Linux x86-64; other targets, shared mutable threads and semantically typed
messages remain explicit demand-table gaps.
