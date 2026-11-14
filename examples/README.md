# Executable consumers

Run these commands from the repository root after `sh tools/bootstrap.sh`.

## Hello

`hello.rs` writes through an owned `Stdout` derived from its `World` entry:

```sh
target/lkjscript run examples/hello.rs
target/lkjscript run examples/hello.rs --engine interpreter
```

## HTTP documents

`http.rs` serves files below a granted `Directory`. Its entry declares the four
capabilities it needs:

```rust
pub fn main(mut output: Stdout, mut root: Directory, mut listener: Listener, mut clock: Clock)
```

The output stream publishes the bound port, the directory supplies document
bytes, the listener accepts connections, and the clock measures request handling.
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
    --grant listener=127.0.0.1:0 --grant clock -o target/http
target/http
```

From another terminal, substitute the printed port:

```sh
curl --noproxy '*' http://127.0.0.1:NUMBER/language.md
curl --noproxy '*' --head http://127.0.0.1:NUMBER/ir.md
```

The server runs until its owner stops it. Profiling is a separate, explicit build
grant: `--profile` writes `target/http.samples.PID`, where PID has twenty decimal
digits. The file contains its function map and remains readable after termination.

```sh
target/lkjscript profile target/http.samples.PID
```

### Request behavior

The server supports GET and HEAD with HTTP/1.0 or HTTP/1.1 request lines. It sends
an HTTP/1.1 response with Content-Length and closes that connection afterward.
The process keeps serving subsequent connections. HEAD reports the GET length
without a body. The root path serves `language.md`; query text does not select a
different file. Response headers include the elapsed monotonic nanoseconds.

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
return 404. This example handles one request at a time and has no request timeout;
a production concurrency policy is future language work.

`http_protocol.rs` is the parser used by the server. Its native test functions
cover request syntax and incremental header recognition through both execution
routes:

```sh
target/lkjscript test examples/http_protocol.rs --jobs 1
```

## Generated traffic and the hour run

The load generator in `tools/http-test.rs` is a native lkjscript program. Its
test-only `World` authority opens the explicitly supplied curl executable and
spawns it with owned argument and stream files. Curl configuration files and
proxies are disabled for the local test.

`tools/http-test.sh` builds a profiled server and the generator, starts the server
on a fresh loopback port, and grants a private copy of the documentation. It
checks GET/HEAD bytes and headers, missing files, path traversal and symlinks,
malformed requests, a multi-chunk body and a 65,537-byte header. After warmup, it
generates deterministic mixed traffic for one hour by default. The native client
records verified requests and completed batches once per minute.

```sh
sh tools/http-test.sh
# Short integration check using the same functional cases:
sh tools/http-test.sh --seconds 2
# Functional requests through the independent interpreter; no repeated load:
sh tools/http-test.sh --engine interpreter --seconds 0
```

The supervisor owns the server PID and samples its RSS, virtual size and peak RSS
every thirty seconds. It reports post-warmup first/last/min/max measurements and
the verified request rate, then prints the CPU profile. Measurements do not
impose a memory or throughput limit. Test request watchdogs and the selected load
duration belong to the harness; they do not change the server's behavior.

The supervisor stops and reaps its server, and the native generator waits for
each curl child. An interrupted run stops the server first so the active client
can finish and reap curl. Temporary documents and executables are removed.
A completed run preserves raw memory samples under
`target/http-profiles/http.memory.PID`; native runs also keep their self-contained
CPU samples as `target/http-profiles/http.samples.PID`.
`--out DIRECTORY` selects another location.
`--compiler PATH` and `--curl PATH` select explicit executables.
