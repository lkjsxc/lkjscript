# Executable consumers

Run these commands from the repository root after `sh tools/bootstrap.sh`.

## Hello

`hello.rs` writes through an owned `Stdout` derived from its `World` entry:

```sh
target/lkjscript run examples/hello.rs
target/lkjscript run examples/hello.rs --engine interpreter
```

## HTTP documents

`http.rs` serves files below a granted `Directory`. Its entry declares the six
capabilities it needs:

```rust
pub fn main(output: Stdout, mut root: Directory, listener: Listener,
    clock: Clock, ready: Readiness, stop: Stop)
```

The output stream publishes the bound port, the directory supplies document
bytes, the listener accepts connections, the clock supplies deadlines and UTC time,
readiness interleaves owned connections, and Stop supplies the SIGTERM request.
The executable has no `World`, process capability or ambient filesystem access.

Inspect authority from checked source or from its checked IR:

```sh
target/lkjscript authority examples/http.rs
target/lkjscript ir examples/http.rs -o target/http.lkjir
target/lkjscript authority target/http.lkjir
```

Build with each entry grant supplied explicitly. Port zero asks the kernel to
choose a free local port; the program prints `port NUMBER` after startup.

```sh
target/lkjscript build examples/http.rs --profile \
    --grant output --grant root=docs \
    --grant listener=127.0.0.1:0 --grant clock --grant ready --grant stop -o target/http
target/http
```

From another terminal, substitute the printed port:

```sh
target/lkjscript build tools/http-get.rs --grant output --grant input \
    --grant target=127.0.0.1:NUMBER -o target/http-get
printf 'GET /language.md HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n' | target/http-get
```

The server runs until its owner stops it. Profiling is a separate, explicit build
grant: `--profile` writes `target/http.samples.PID`, where PID has twenty decimal
digits. The file contains its function map and remains readable after termination.

```sh
target/lkjscript profile target/http.samples.PID
```

### Request behavior

The server supports GET and HEAD with HTTP/1.0 or HTTP/1.1 request lines. It sends
an HTTP/1.1 response with Content-Length. HTTP/1.1 connections stay open by default;
HTTP/1.0 requires `Connection: keep-alive`. `Connection: close` always wins.
Pipelined requests retain their exact body boundaries. HEAD reports the GET length
without a body. The root path serves `language.md`; query text does not select a
different file. Response headers include a UTC HTTP Date and elapsed monotonic nanoseconds.
Accepted sockets use `accept_nodelay()` so separate header and body writes do not
wait for Nagle batching. `std::buffer::ReadBuffer` snapshots files in owned chunks
before any response bytes are sent; its chunk size does not limit file size.

Header bytes accumulate in growable storage using fixed read chunks. There is no
header-size acceptance cap. Content-Length request bodies are drained before the
response; transfer encodings return 501. Expect headers return 417 before body
reading, so a sender waiting for 100 Continue receives an immediate final reply.
Malformed framing, headers or path
encoding return 400, unsupported methods return 405 with `Allow: GET, HEAD`,
and an unsupported HTTP version returns 505.

Percent-decoded paths remain relative to the directory capability. The directory
operation enforces confinement, including symlinks: an inward link can be opened,
while a path or symlink escaping the granted root returns 403. Missing documents
return 404. Each connection has explicit owned state and nonblocking I/O. A silent
connection gets a 60-second idle deadline. Its first request byte starts a fixed
two-second header/body deadline; a response gets a fixed two-second write deadline.
Neither deadline extends when a peer makes tiny progress. A request-side half-close
still receives replies to complete buffered requests.

An optional `.http-policy` file beneath the granted root sets application timeouts:
three decimal lines give idle, whole-request and whole-write seconds, respectively.
An optional fourth line gives the shutdown drain duration; it defaults to five seconds.
Existing three-line policy files retain their meaning. Values must be positive and fit the monotonic nanosecond representation. Without
this file the defaults above apply; malformed policy stops startup with a located
message. The interpreted functional fixture explicitly uses `60`, `120`, `120`
because interpreting the large-header case exceeds the native two-second request
policy. Native functional and hostile-peer measurements retain the defaults.

Network work is bounded per connection turn. An indexed heap keeps one deadline
per live slot; repeated requests do not accumulate stale timers. Closed slots are
reused only after the current event batch is consumed. File snapshots are still
read synchronously before the response; response transmission yields under
backpressure through `ReadBuffer::write_some` and an owned `WriteCursor`.

`http_server.rs` shares the connection engine through an ordinary generic handler;
`http.rs` supplies the static-file handler and `journal.rs` supplies the journal.
`http_protocol.rs` parses requests with the handler's allowed method list. Its native test functions
cover request syntax and incremental header recognition through both execution
routes:

```sh
target/lkjscript test examples/http_protocol.rs --jobs 1
```

### Graceful stop

The server publishes `stop-target PID` beside its port. Send SIGTERM to that PID
to request shutdown. For a native executable it is the executable's own PID.
For an interpreted execution it is the owned syscall broker's PID: signaling the
compiler process itself remains an abrupt interpreter termination. The explicit
endpoint avoids pretending that a frozen-seed host has acquired a new signal handler.

The first observed stop event closes the listener and idle peers. Each active
connection freezes the bytes already read into its owned input buffer. The server
finishes its current request/response and each pipelined request whose first byte
was in that buffer. Later bytes may complete those admitted requests, including
a partial final body; they cannot admit another request. The last response closes
the connection. Headers transmitted before shutdown may already advertise
keep-alive; the connection still closes after its final admitted response. A peer
that sends unadmitted bytes may observe a TCP reset after that complete response.

The drain has one absolute deadline, five seconds by default. Existing request
and write deadlines remain in force and are shortened when the drain ends sooner.
Progress and repeated SIGTERM never extend it. When connections finish or expire,
the journal syncs its log and directory, then the executable prints `stopped`
and exits 0. Sync failure prints an error and exits unsuccessfully. Synchronous
handler work and the final filesystem sync still depend on the filesystem making
progress; the deadline bounds network draining.

`stopping DEADLINE` reports the absolute monotonic drain deadline for supervisors.
Stop claims SIGTERM for the execution lifetime; its requested state is sticky
after consumption. A repeated signal during final cleanup does not change exit 0.

```sh
sh tools/http-stop.sh
# Select a compiler:
sh tools/http-stop.sh --compiler target/lkjscript
# Tiny interpreted consumer smoke using the published broker endpoint:
sh tools/http-stop.sh --engine interpreter
```

This native check uses an explicit two-second fourth policy line. It tests a
backpressured pipeline, an admitted incomplete body, idle closure, refusal of
new connections and requests after stop, a nonreader, an unfinished header,
a repeated SIGTERM, HTTP Date headers, and successful exit at the fixed deadline.
The interpreted smoke uses only a three-byte document and checks GET, completion
of an admitted partial request, idle closure, repeated SIGTERM to the verified
broker child, and exit 0. Both checks run in the regular gate with weight two.
Their clients, server/broker, port, FIFO and temporary files are removed afterwards.

## Outbound clients and persistent traffic

`Connector` grants one numeric IPv4 TCP destination. Its `connect()` method returns
an owned `File` stream; the caller cannot substitute another destination.
`tools/http-get.rs` accepts a raw request on its granted input and writes one
framed response to its granted output. It recognizes HEAD and Content-Length.

`tools/http-test.rs` connects directly through lkjscript and checks GET/HEAD bytes
and headers, missing files, traversal and symlinks, malformed requests, request
bodies, a 65,537-byte header, persistent requests and pipelining.
`tools/http-load.rs` holds one TCP connection open, pipelines requests, and checks
every response's status, framing and exact body against its granted fixture.
It reports elapsed monotonic nanoseconds, requests/second and the validation mode.
The default `--verification full` compares every body byte. `--verification framing`
checks every status, length and connection policy and drains every body, while
skipping byte comparisons to measure beyond a verifying client's CPU limit.
Client startup and connection setup are outside the sustained measurement.

`tools/http-test.sh` builds these programs and a profiled server, grants a private
copy of the documentation, and chooses a fresh loopback port. No external HTTP
client is involved.

```sh
# Functional requests plus a short persistent measurement:
sh tools/http-test.sh --seconds 2
# Framing-only throughput; functional cases still compare exact bodies:
sh tools/http-test.sh --seconds 30 --verification framing
# Functional requests through the independent interpreter:
sh tools/http-test.sh --engine interpreter --seconds 0
```

The harness records the host load average beside elapsed measurements and samples
server RSS, virtual size and peak RSS. It prints the source CPU profile after
stopping and reaping its server. The watchdog, pipeline size and duration are
test settings, not language acceptance limits.

Temporary documents and executables are removed on success and interruption.
`--out DIRECTORY` selects the destination for memory and CPU samples;
`--compiler PATH` selects the compiler under test.

## Many clients and hostile peers

`tools/http-concurrency.rs` drives ten persistent connections concurrently, with
32 requests in flight per connection and a 40,108-byte response. Every response
checks status, length and connection policy and drains the full body. It brackets
the silent-peer measurement with unloaded measurements, then exercises a header
sent one byte per second, a client that stops reading a 32 MiB response, and a
client that disconnects mid-header while useful traffic continues.

```sh
sh tools/http-concurrency.sh
# Small integration check, without the final 1,000-connection measurement:
sh tools/http-concurrency.sh --seconds 1 --idle 2
```

The default run holds 1,000 peers open: 990 silent and ten active. Its three
measurement phases each last ten seconds. The client receives a separate Directory
grant for the test server's descriptor listing, so it can verify actual server
handle release at the deadline even while TCP bytes remain queued for a nonreader.
The supervisor reports process RSS per silent peer, excluding kernel socket memory,
records load averages, and checks that every client handle, process, temporary
directory and listener is gone. `--seconds`, `--idle` and `--compiler` select the
test workload and compiler; they do not alter language resource rules.

## Durable HTTP journal

`journal.rs` accepts JSON entries, retains the latest version of each entry in an
in-memory index, and stores every mutation in an append-only log beneath its granted
`Directory`. Its entry grants are output, root directory, listener, clock, readiness, Stop and
Entropy. The latter initializes the durable session-signing key. The granted root is writable
and should be a private directory used by one journal.

```sh
mkdir -p target/journal-data
target/lkjscript build examples/journal.rs \
    --grant output --grant root=target/journal-data \
    --grant listener=127.0.0.1:0 --grant clock --grant ready --grant stop --grant entropy -o target/journal
target/journal
```

Startup prints recovery progress, then `ready REVISION ENTRIES`, `port NUMBER`
and `stop-target PID`. After that first startup, issue a write token from a separate
program that receives the signing-key directory:

```sh
target/lkjscript build examples/journal-token.rs \
    --grant output --grant errors --grant input --grant root=target/journal-data \
    --grant entropy --grant clock -o target/journal-token
journal_token=$(printf '3600\n' | target/journal-token)
target/lkjscript build tools/http-get.rs --grant output --grant input \
    --grant target=127.0.0.1:NUMBER -o target/journal-get
printf 'POST /entries HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer %s\r\nContent-Type: application/json\r\nContent-Length: 16\r\nConnection: close\r\n\r\n{"text":"hello"}' "$journal_token" | target/journal-get
```

### Routes and JSON

| Request | Result |
| --- | --- |
| `POST /entries` with `{"text":"hello"}` | 201 and the newly appended entry. |
| `GET /entries/ID` | 200 and the latest version, or 404 if absent. |
| `PUT /entries/ID` with `{"text":"replacement"}` | 200 and the replacement; an absent ID returns 404. |
| `GET /entries/FIRST/END` | 200 with `{"entries":[...]}`, including IDs from FIRST up to but excluding END. |
| `GET /entries` | 200 with all current entries in increasing ID order. |
| `GET /status` | 200 with `entries`, `next_id` and `revision` numeric fields. |
| `HEAD` on a read route | The same status and representation length, with no body. |

An entry is `{"id":1,"revision":1,"text":"hello"}`. IDs start at 1 and append
contiguously; replacement retains the ID. Every mutation increments a single
global revision, so the revision identifies the entry's last successful log record.
The request must be a JSON object with exactly one string field named `text`.
Malformed JSON, duplicate keys, wrong field types and extra fields return 400.
Strings retain Unicode and escaped control characters, including embedded NUL.
JSON numbers preserve their exact lexical value in the shared library; entry IDs
and revisions do not pass through floating-point values.

The shared HTTP engine handles persistence, pipelining, Content-Length bodies,
HEAD and fixed deadlines. POST/PUT bodies are assembled only after exact framing
identifies their boundaries and header-time authentication succeeds. GET/HEAD
bodies are drained without retaining a full body copy. Transfer encoding and Expect retain the static server's 501 and
417 behavior. The journal enables POST and PUT; the static server still allows
only GET and HEAD. Query text is ignored, so range bounds are path components.
There is no application record-size cap. A malformed request cannot become a
partial visible entry.

### Signed write sessions

GET and HEAD remain public. Every POST and PUT requires a valid
`Authorization: Bearer TOKEN`, including requests for missing or malformed
routes. Authentication follows header parsing and precedes body retention, route
and JSON validation. Missing, invalid or expired credentials receive 401 with
`WWW-Authenticate: Bearer realm="journal"` and a closed connection, without waiting
for a declared body. Authorization is checked again before mutation, so a token
that expires while its body arrives cannot authorize the write. Duplicate
Authorization fields return 400 during HTTP parsing.

The token is `v1.EXPIRY.NONCE.HMAC`: an exact decimal Unix-seconds expiry,
32 random bytes represented by 64 lowercase hex digits, and a 32-byte HMAC-SHA256
represented the same way. The MAC binds a fixed journal-session domain and the
preceding token text. The shared cryptographic library compares MAC bytes without
an early byte mismatch exit. A token expires when the current UTC seconds reach
its expiry. The issuer reads an optional positive lifetime from stdin; empty input
means 3,600 seconds. There is no public token-issuing HTTP route.

The server creates `session.key` as exactly 32 kernel-random bytes while holding
the journal writer lock. The staging file is created exclusively with mode 0600 before secret bytes are
written; explicit permission repair also handles a restrictive umask. It is then
synced, renamed and its directory synced. Existing keys are
read exactly and survive restart; a malformed key stops startup without replacing
it. The separately granted token issuer may read the key and issue tokens; access
to that directory is signing authority.

### Commit and recovery

The durable record protocol uses four names beneath its granted root; the session
key described above is stored alongside them:

| File | Purpose |
| --- | --- |
| `lock` | An owned File holds a nonblocking exclusive advisory lock for the server's lifetime. It is never unlinked or renamed. |
| `journal.log` | Checksummed append-only records; the source of truth for entries and revisions. |
| `head` | A 96-byte durable-head manifest identifying a complete log boundary. |
| `.head-next` | The exclusive-create staging name for atomic manifest replacement. |

A record contains a 40-byte header, the raw UTF-8 text, and a 32-byte SHA-256
digest of header and text. The header begins with `LKJLOG01` and contains four
little-endian u64 fields: text length, ID, revision, and operation (1 append,
2 replace). The implementation uses `std::sha256`, shared with the compiler.

For each mutation, the server writes the complete log record and syncs the log.
It then creates `.head-next` exclusively, writes and syncs it, renames it over
`head`, and syncs the directory. Only after those operations succeed does it
queue the 201 or 200 response. The manifest starts with `LKJHEAD1`, followed by
log end offset, revision and next ID as little-endian u64 values, the last record's
32-byte digest, and SHA-256 of those first 64 bytes. Manifest replacement never
exposes a partly written manifest to a reader opening `head`.

Startup obtains the writer lock before inspecting or repairing state. It streams
the log in fixed read chunks, validates checksums, UTF-8, revision order and entry
IDs, and rebuilds the index. Declared lengths do not trigger an up-front allocation.
An EOF-short final header, text or checksum is truncated to the previous complete
record only after recovery has matched the durable `head` boundary. A head
claiming a boundary beyond the complete log prefix stops recovery without repair.
A complete record with a bad checksum or invalid sequence also stops recovery;
corruption is never silently discarded.

Complete unacknowledged records may be adopted after a restart. Before serving,
recovery syncs the validated log and any repaired tail, removes an abandoned
`.head-next` under the lock, and publishes the recovered head through the same
atomic replacement sequence. Killing recovery at any point can be handled by the
same procedure again. A malformed existing head stops startup. Startup failures
print `journal error: ...` before exiting; an uncertain write or sync failure
during service prints the operation error and stops before a successful response.

Progress messages distinguish actual recovery from serving:
`recovery begin`, `recovery record RECORDS OFFSET` after the first and every
128 validated records, an optional `recovery repaired OFFSET`, and
`recovery done RECORDS OFFSET`. These markers accompany real work; no sleep
creates an artificial recovery window.

Network reads and writes remain nonblocking with the static server's fair dispatch
and timeout policy. JSON parsing, hashing and local storage writes/syncs run
synchronously when a complete request is ready; a stalled filesystem can delay
the event loop. The optional `.http-policy` file sets the same idle, request and
write and shutdown deadlines described above. SIGTERM drains admitted work, syncs,
and exits as described in the graceful-stop section.

### Focused journal checks

`tools/http-journal-cases.rs` uses the lkjscript HTTP client to check append,
replace, exact Unicode/control-character roundtrips, pipelining, reads, ranges,
HEAD and strict JSON input. Its shell supervisor checks single-writer refusal,
three incomplete-tail boundaries, abandoned staging files, adoption of a complete
unacknowledged record, corruption rejection and refusal to truncate below a durable
head. They also build the separate token issuer, prove distinct random nonces and
restart-stable authorization, reject expired/forged/duplicate credentials before
body retention, and test private creation, permission repair, abandoned key staging
and malformed keys without replacement. Credentials stay in private test files,
never command arguments or logs. These cases use fresh loopback ports.

```sh
sh tools/http-journal-cases.sh
# Small interpreted CRUD, pipeline and restart smoke:
sh tools/http-journal-cases.sh --engine interpreter
# Select the compiler under review:
sh tools/http-journal-cases.sh --compiler target/lkjscript
```

The wrapper kills and reaps its crash-test servers, then asks the final server
to stop through its published endpoint and requires final sync plus exit 0.
It verifies released ports and removes temporary state on success or failure. Its watchdogs bound test
execution; they do not restrict journal data size or language execution.

### Random crash campaign

Run the final stress campaign separately from the ordinary gate:

```sh
sh tools/http-journal-test.sh --cycles 1000
```

The lkjscript supervisor creates 128 entries, then runs 1,000 rounds. Each round
first receives a new acknowledged mutation, starts pipelined append/replace load,
and sends SIGKILL at a seeded random deadline with a complete request still in
flight. It next kills recovery after a randomly selected observed replay
checkpoint. A progress marker must precede the signal and the completed output
must contain no recovery-done marker; missed windows are retried and counted.
Recovery is real log validation, without an artificial delay.

After each pair of kills, a fresh server must start. An independent reader checks
every complete log frame, checksum, operation, ID, revision and exact issued text.
It checks all historical acknowledgements, including versions later replaced,
and compares the full HTTP entry list and status against that replay. Complete
unacknowledged writes may survive; incomplete records and missing acknowledged
versions fail. The supervisor reports acknowledgement counts, verified frames,
observed tail repairs, missed windows, elapsed time and all child starts/reaps.
The shell records host load and removes the private data and executables.

`--cycles 3` is a small integration check; `--seed N` selects another reproducible
schedule. A process crash exercises commit ordering and repeatable recovery.
The stable-storage promise also requires a filesystem and storage device that
honor successful sync operations; this campaign does not emulate a host power cut.

## Documentation-origin candidate

`site.rs` embeds the exact twelve-path public documentation snapshot and computes
dynamic search, with explicit Listener, Clock, Readiness, Stop and output grants. [site.md](site.md) explains the captured content, text helpers,
independent live parity, offline regression checks and temporary systemd proof.
The candidate unit is [lkjscript-site.service](lkjscript-site.service); the
[24-line switch and rollback procedure](site-switch.md) is for Claude to execute.
The live origin and /opt are left untouched during preparation.
