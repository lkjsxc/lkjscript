# Executable consumers

Run these commands from the repository root after `sh tools/bootstrap.sh`.

## Hello

`hello.rs` writes through an owned `Stdout` derived from its `World` entry:

```sh
target/lkjscript run examples/hello.rs
target/lkjscript run examples/hello.rs --engine interpreter
```

## HTTP documents

`http.rs` serves files below a granted `Directory`. Its entry declares the five
capabilities it needs:

```rust
pub fn main(mut output: Stdout, mut root: Directory, mut listener: Listener,
    mut clock: Clock, mut ready: Readiness)
```

The output stream publishes the bound port, the directory supplies document
bytes, the listener accepts connections, the clock sets deadlines, and readiness
interleaves the owned connections.
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
    --grant listener=127.0.0.1:0 --grant clock --grant ready -o target/http
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
different file. Response headers include the elapsed monotonic nanoseconds.
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
Values must be positive and fit the monotonic nanosecond representation. Without
this file the defaults above apply; malformed policy stops startup with a located
message. The interpreted functional fixture explicitly uses `60`, `120`, `120`
because interpreting the large-header case exceeds the native two-second request
policy. Native functional and hostile-peer measurements retain the defaults.

Network work is bounded per connection turn. An indexed heap keeps one deadline
per live slot; repeated requests do not accumulate stale timers. Closed slots are
reused only after the current event batch is consumed. File snapshots are still
read synchronously before the response; response transmission yields under
backpressure through `ReadBuffer::write_some` and an owned `WriteCursor`.

`http_protocol.rs` is the parser used by the server. Its native test functions
cover request syntax and incremental header recognition through both execution
routes:

```sh
target/lkjscript test examples/http_protocol.rs --jobs 1
```

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
