# Documentation-origin candidate

`site.lkj` is the lkjscript replacement candidate for the loopback documentation
origin. It seals the exact published snapshot into the executable at build time
and uses the shared readiness-based HTTP engine. Its entry grants are output,
listener, clock, readiness and Stop. It has no runtime Directory, World,
outbound connection, process or entropy authority.

The live origin is `lkjscript-site` on `127.0.0.1:8798`. Its unit starts
`/opt/lkjscript-site/current/lkjscript-site --listen 127.0.0.1:8798`.
The release is a Rust executable with embedded content. Its pages identify
snapshot `2e72b47897452627e41c48e6311373927110e9c7`, rather than the current
language documentation. The candidate deliberately preserves those page bytes,
including historical examples and the “Built in Rust” footer.

No live service, live unit or file beneath /opt was changed while preparing this
candidate. Inspection read the unit and release metadata and sent GET/HEAD
requests to the origin.
The separately linked public Markdown blobs were read from this repository's
Git object database; the .generation-one tree was not accessed.

## Published content and provenance

The lkjscript crawler follows same-origin href, src and form-action attributes,
resolves relative links and removes fragments. Its one capture found twelve
published paths. External GitHub links are retained in HTML but never followed
by the numeric-address Connector.

| Path | Captured body bytes |
| --- | ---: |
| / | 4,506 |
| /robots.txt | 41 |
| /assets/site.css | 7,239 |
| /docs/start | 9,313 |
| /docs/direction | 17,143 |
| /docs/status | 34,811 |
| /search | 1,502 |
| /docs/web | 17,325 |
| /docs/http | 13,672 |
| /docs/forms | 25,033 |
| /docs/architecture | 60,803 |
| /docs/security | 16,989 |

`site-content/manifest.json` records the paths, statuses, six stable response
headers and numbered body filenames. Files `0.body` through `12.body` hold
those exact bodies and the separate 1,188-byte missing-page response. Date,
Connection and Content-Length are generated per response rather than archived.

Search uses the exact Markdown sources linked by the published pages:

| Search file | Blob at the published commit |
| --- | --- |
| search-0.md | docs/guides/native-command.md |
| search-1.md | docs/guides/native-web.md |
| search-2.md | docs/guides/native-http.md |
| search-3.md | docs/guides/native-forms.md |
| search-4.md | docs/direction.md |
| search-5.md | docs/architecture.md |
| search-6.md | docs/status.md |
| search-7.md | docs/security.md |

These eight blobs total 159,133 bytes. They are search data, not executable
programs or newly supported historical language forms. Search framing, card
titles, descriptions and catalogue order come from the captured homepage and
empty search response. Title matches come first; other summary/source matches
retain catalogue order. Query responses are computed, not served from a finite
list of captured queries.

The candidate uses exact raw spellings for static routes and decodes the single
slug beneath literal /docs/. For example, /docs/%73tart names the start page,
while /%73earch, /assets/%73ite.css and /docs%2fstart remain missing.
The compiler includes the named archive files through its source Directory.
The resulting service has no filesystem grant; request paths select sealed
pages. Refreshing the published snapshot requires rebuilding the executable.

## Observed HTTP behavior

The covered GET and HEAD responses retain the origin's Content-Type,
Cache-Control, CSP, nosniff, Referrer-Policy and Permissions-Policy.
The origin uses `no-cache`, including
for CSS, and provides no ETag or Last-Modified. Range and conditional requests
return the full 200 response. HEAD returns the matching status and headers with
Content-Length zero and no body. A 65,537-byte header value is accepted.

Missing paths, literal or encoded traversal and trailing slashes receive the
same constant 404 page. These are observed compatibility behaviors, including
the unusual HEAD length; the candidate does not silently substitute different
HTTP policies. Parser, framing and unsupported-method errors still use the
shared engine's error responses; parity does not claim their bodies and headers
match the origin.

Search form decoding maps plus to space, decodes valid percent escapes, leaves
malformed percent text literal and replaces malformed UTF-8 subsequences.
Duplicate or unknown query fields return the origin's 400 text. The decoded
query must be at most 128 UTF-8 bytes and contain no control character before
Unicode whitespace trimming. Display text is escaped for HTML independently
of matching.

The source library now supplies `std::text::utf8_lossy`, `form_decode`,
`escape_html`, `trim` and `lowercase_chars`. Lowercase maps Unicode 17.0.0
scalars independently, including expansions; it does not implement contextual
final sigma. The pinned corpus has no cased non-ASCII text, so this gives the
same matches as the origin's full-string lowercase, including Kelvin-sign
queries. All 1,112,064 valid scalar mappings were compared with Rust's independent
Unicode tables; the native output digest matched exactly. The case table is
source data, with no Rust, Node or WebAssembly execution dependency.

## Build and verify

```sh
sh tools/http-site-test.sh --offline
sh tools/http-site-test.sh
sh tools/http-site-test.sh --systemd
```

All wrappers accept `--compiler PATH`. The first command is the permanent
weight-two gate check and does not connect to the live origin. It verifies
twelve published bodies and the archived 404 byte-for-byte, then 51 search
queries and thirteen protocol probes against frozen live observations:
status, Content-Length, stable headers and the hash of the entire body.

The default live proof independently walks every published link again and
compares the origin and candidate. Before either connection is opened, it requires
51 unique search-query probes in `site-content/paths.txt`; a missing, shortened,
duplicate or non-search list fails. This is test configuration, not server input.
The shared thirteen protocol requests cover the required missing, traversal,
huge-header, HEAD, range and conditional cases, plus route spelling boundaries.
Date and Connection are intentionally excluded from equality; all six stable
headers and every response body byte are compared.

The recorder and frozen oracle are documented in
`../tests/fixtures/site/README.md`. The snapshot program is
`../tools/http-site-snapshot.lkj`; its explicit grants are output, origin and
root. Capture into a new empty directory, then supply the eight exact source
blobs above from the commit named in the observed pages. Existing archive
filenames and oracle files are never silently overwritten.

## Verified candidate

The sealed candidate passed the 12 published paths, 51 search queries and
13 protocol probes under the origin's effective systemd restrictions. Live
parity compared 274,818 walked body bytes plus each protocol response exactly.
The startup/client-build/parity/shutdown phase took 0.730 seconds, with load
5.40/5.35/4.80 -> 5.21/5.31/4.79. Process RSS and high-water RSS were 1,376 KiB;
cgroup current/peak memory were 2,072,576/2,097,152 bytes under MemoryMax=128 MiB.

The native executable was 863,424 bytes: 383 IR functions and 38,257 IR
instructions. Compilation took 744.114 ms, with load 5.40/5.35/4.80 unchanged,
and compiler peak RSS 59,367,424 bytes. The focused parity-source checks passed
16 tests in 36.241 seconds, load 4.35/4.71/4.60 -> 3.81/4.52/4.54. The offline
site gate includes those checks and compares 316,466 response body bytes;
the complete site job took 37.083 seconds, one-minute load 4.19 -> 3.36.

The final four-worker gate, including bootstrap fixed points, both executor
routes, explicit inlining, semantic contracts and mutation, passed in 262.690
seconds, one-minute load 2.93 -> 7.22.

SIGTERM produced the graceful stopped marker and successful exit. Launcher
1355335, candidate 1355339 and port 55087 were gone; transient unit
lkjscript-programs-http-site-systemd-Y7ntx6 was collected, and its /run stage
and private work directory were removed. The live origin remained active at
PID 256. All ignored proof binaries, logs and generated case directories were
removed after the final gate.

## Hardening and activation

`lkjscript-site.service` preserves the live unit's sandbox and resource
directives, including DynamicUser, ProtectSystem=strict, ProtectHome,
NoNewPrivileges, MemoryDenyWriteExecute, an empty capability bounding set,
128 MiB memory, 32 tasks and 512 descriptors. The listener is a checked build
grant, so the candidate needs no --listen runtime argument. Its fixed deadlines
are 60 seconds idle, two seconds for a request/write, and five seconds to drain.

The first file-backed candidate failed during startup: Directory.open returned
ENOSYS under RestrictSUIDSGID. The [systemd 255.4 filter](https://github.com/systemd/systemd-stable/blob/v255.4/src/shared/seccomp-util.c#L2289-L2303)
blocks every openat2 call because seccomp cannot inspect the indirect open_how
mode bits; DynamicUser implies this restriction. The final candidate follows
the live origin's embedded-content design. The disk-loading path and runtime
Directory grant were removed; the confined Directory implementation and all
unit protections remain intact.

The user systemd manager is unavailable on this host. The systemd proof uses a
unique transient system-manager unit and root-owned staging under /run, with an
ephemeral loopback port and Restart=no. It reads effective settings, verifies
parity, sends SIGTERM, requires successful graceful exit, and removes the unit,
port and staging files. The live origin PID must remain unchanged.

This host's global LXC drop-in disables ProtectKernelTunables and
ProtectControlGroups and leaves ProtectProc=default, ProcSubset=all and
PrivateNetwork=no. It also assigns NoNewPrivileges=no, but the effective value
is yes because DynamicUser implies it. The proof reports effective settings
rather than claiming those overridden restrictions were active.

The only activation instructions are in `site-switch.md` (25 lines).
Claude performs the switch. A new release keeps a copy of the old unit and a
link to the old release before replacing current by atomic rename. A separate
attempt pointer keeps rollback tied to that backup even after current is restored;
rollback can be retried after either staging or rename. Required preparation and
reload commands fail before later activation steps. The candidate preparation
does not execute those commands.
