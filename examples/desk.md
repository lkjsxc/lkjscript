# Desk command and isolated service

This is the bounded consumer from the second programs note: the state-changing
commands and a service without external board, handoff or wake behavior. It is
not the live desk replacement. The installed Python desk remains the owner and
agents' tool until a later handoff.

## Command contract

The command supports note, ask, close, inbox, inbox --peek and path. Arguments
remain exact bytes until the command requires UTF-8. Named environment grants
provide DESK_STATE and TZ; there is no environment enumeration. The state
directory must already exist and DESK_STATE must name it. There is no implicit
read of HOME and no fallback to the live state directory.

The additional --stdin option appends UTF-8 standard input to note or ask text.
Normal exit status is an affine capability; choosing a status does not skip
destructors. Timestamps use an explicitly chosen fixed offset: UTC by default,
JST-9 or +/-HH:MM. Regional timezone transition files are a future library need.

Build the command from the repository root:

~~~sh
target/lkjscript build examples/desk.lkj --library-dir library -o target/desk \
  --grant args --grant state=env:DESK_STATE --grant state_env=DESK_STATE \
  --grant timezone=TZ --grant output --grant errors --grant input \
  --grant clock --grant entropy --grant exit
~~~

For an isolated invocation, create a private directory inside this worktree and
set DESK_STATE to its absolute path. Do not point test commands at the live desk.

## Shared state and crash behavior

The two implementations use the same .lock, notes.jsonl, inbox.jsonl, seen, ping
and alive names. A blocking flock serializes writers across processes. IDs use
realtime nanoseconds and kernel entropy. The JSON field order and whitespace may
differ; decoded fields, row order and references agree after normalizing IDs and
times. Each implementation reads the other's exact stored bytes, including
Japanese text, without conversion.

Publication writes a fresh exclusively created temporary file, syncs it, renames
it over the intended name, and syncs the directory. Existing raw log bytes remain
unchanged. Temporary names and alive never follow an existing in-directory
symlink when creating output. A killed publication cannot expose a torn JSON line
to the Python reader. Appending refuses an incomplete last line without changing
the file. The old prefix is copied, so append cost still grows with log size.

Owner-message and answer retries return the original row. If a crash published
an inbox row before ping, retry repairs ping. Retrying an already-read row does
not recreate an empty ping. As in the Python command, inbox without --peek commits
seen before printing; interruption in that interval can suppress a repeated
display, while the stored rows remain present. Agents use --peek.

## Service contract

The separate serve entry implements the original page bytes, GET state, and
POST say, including proxy prefixes, X-Desk, first-answer and request-id retries.
The shared HTTP engine supplies readiness, fixed request/write deadlines,
keep-alive handling and graceful Stop cleanup. jobs is empty and detail is empty.
The original page still polls. Duplicate security/framing headers or JSON keys, and structured values
where the page sends strings are rejected explicitly.

~~~sh
target/lkjscript build examples/desk.lkj --entry serve --library-dir library \
  --grant output --grant state=env:DESK_STATE --grant clock --grant id_clock \
  --grant entropy --grant listener=127.0.0.1:0 --grant ready --grant stop \
  -o target/desk-service
~~~

The command and service have separate entry grants so a note never binds a
listening port. Combining them behind one serve subcommand needs a deferred
listener grant or reviewed entry dispatch. No production unit or switch procedure
is offered at this boundary.

## Why external programs stop this item

An exact executable path and chosen argv do not restrict the executable's
filesystem, network, signals or descendants. Existing Process execution runs
under the host identity. Closing descriptors and clearing environment variables
do not prevent reopening host files.

The actual cxloop is Bash and exposes run, relay, cancel and clean, reads /proc
and other job/session directories, and starts git, codex and claude. desk listen
also takes arbitrary commands. An authority report saying only "may run cxloop"
would hide these effects and would not meet the required child-authority subset.

The next child facility should admit an immutable authenticated lkjscript image,
check its entry's authority against explicitly transferred parent capabilities,
and start from those handles. Merely checking a path, ELF header or supplied
manifest is insufficient. External Bash/Python helpers need broader declared host
trust or OS confinement. No narrow external-process grant has been added here.

There is also a second read root in the Python service: handoff files beneath its
owner's memory directory. It is not part of the state-directory grant and is not
silently acquired by this candidate.

## Verification and cost

Run the short isolated regression with:

~~~sh
bash tools/http-desk-test.sh --compiler target/lkjscript
~~~

The recorded oracle campaign uses only fresh worktree state and the original
Python executable as an oracle:

~~~sh
bash tools/http-desk-test.sh --compiler target/lkjscript \
  --oracle /home/coder/workspace/agent-toolkit/bin/desk --measure
~~~

It covers every state command, Japanese text, two simultaneous writers, UTC/JST
display, mutually readable files, standard input, normal status and missing
environment, all three HTTP routes, retry after reading, and safe publication
when stale temporary names or alive are symlinks. Capability regressions also
compare standalone, native-run and interpreted execution; long campaigns stay
outside the ordinary gate.

### Recorded cost

The four-worker cold gate passed in **534.967 s**, load **8.50 -> 5.92**.
Its six consumer suites total **34.039 s**; the desk suite, including capability
checks on three execution routes, takes **4.283 s**, load **8.12 -> 7.87**.
The cold command/service builds take **752.396/786.849 ms** within that interval
(load **8.12/7.15/5.50 -> 7.87/7.12/5.50**), with **656 KiB** service RSS.
The suite table is in [status.md](../docs/status.md). Long campaigns stay outside
the gate; the stored seed remains unchanged.

The final command uses 581 lines of Rust-shaped source, including comments and
tests, and the unchanged 186-line page: 767 direct lines. The Python original is
249 command/service lines plus the same page, or 435 lines. The candidate also
uses 1,241 shared HTTP lines and the 384-line JSON library. Those shared sources
are not included in the direct desk count. Relative to the origin repair commit,
the command-capability plumbing adds 604 compiler-source lines and removes 32,
excluding semantic-test sources. The new command helper module has 26 lines;
the fixed-offset time helpers add 49 library lines. Fixtures, harnesses and
documentation are separate from these counts.

The command build with reuse disabled took **700.379 ms**, within a measurement
interval whose load was **2.09/2.87/4.34 -> 2.40/2.92/4.35**. The command is
**299,635 bytes** and the service **498,357 bytes**. After the isolated route
checks the service used **660 KiB RSS**. The complete oracle/measurement session
took **16.179 s**, load **2.09/2.87/4.34 -> 2.34/2.89/4.32**.

Each latency below is the arithmetic mean of 100 separate end-to-end note
commands, including process startup, output redirection and storage work. Each
implementation starts from a copy of the same fixture and adds 100 records.

| Initial records | Implementation | Mean note time | Load before -> after |
| ---: | --- | ---: | --- |
| 0 | lkjscript | 29.914 ms | 2.40/2.92/4.35 -> 2.37/2.90/4.34 |
| 0 | Python | 18.767 ms | 2.37/2.90/4.34 -> unchanged |
| 10,000 | lkjscript | 27.613 ms | 2.37/2.90/4.34 -> unchanged |
| 10,000 | Python | 18.695 ms | 2.37/2.90/4.34 -> 2.34/2.89/4.32 |

On this run the candidate costs 1.48-1.59 times the Python command's latency and
has more direct source. Different filesystem/cache conditions mean the lower
10,000-record sample is not evidence that larger logs are faster. A profile
identified reparsing the entire old log as the first structural cost; append now
copies the original bytes without reparsing them. It still performs durable
whole-file publication and safely replaces alive, whereas Python's append path
does not fsync. These measurements favor keeping the installed tool until the
missing authority boundary and handoff are solved; they do not claim a speed win.

The gains demonstrated here are independently checked, explicit authority;
standalone execution without a language host; mutually readable state; and a
publication protocol with durable acknowledgements. Full declared capabilities
and every reachable effect for both entries are in [authority.txt](desk/authority.txt).
The report states the invocation bindings separately because the authority
command currently describes capability types, not grant payload values.

### Crash campaign

The optional campaign remains outside the gate:

~~~sh
bash tools/http-desk-crash.sh --compiler target/lkjscript \
  --oracle /home/coder/workspace/agent-toolkit/bin/desk
~~~

The recorded run delivered **96 SIGKILLs** in 107 attempts, including ten kills
after an acknowledgement was printed. All **128 acknowledged IDs** survived,
all 10,132 published rows parsed, and 107 normal recovery appends succeeded. The
original 910,000-byte prefix of 10,000 records remained byte-identical. Four
additional records were durable without having been acknowledged, which is a
permitted interrupted-command outcome. The Python oracle read all surviving rows
and returned the same referenced-answer inbox output.

That campaign took **6.243 s**, load **1.74/2.71/4.04 -> 2.32/2.82/4.07**; its
command build took **0.745 s**, load **1.74/2.71/4.04 unchanged**. All temporary
state and owned children were removed.
