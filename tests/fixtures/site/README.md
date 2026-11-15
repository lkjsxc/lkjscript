# Documentation-origin test oracle

`origin.json` records live GET/HEAD observations from the Rust documentation
origin at `http://127.0.0.1:8798`. It is an operational test fixture, separate
from the archive served by the candidate. The candidate never opens this file.

The recorder is `tools/http-site-record.lkj`. Its only network authority is the
granted origin connector. It reads the 51 search-query targets in
`examples/site/content/paths.txt` and the thirteen protocol probes shared with live
parity in `tools/http_site_probes.lkj`. Every record stores the request, observed
status, Content-Length presence/value, all six stable header pairs, and SHA-256
of the complete response body. Date and Connection are excluded. Header names
are lowercase; duplicate header pairs, if present, remain separate.

Request `extra_headers` values are descriptions expanded by that shared source:
`none`, `large_header_65537` (X-Large with 65,537 x bytes),
`range_0_15`, and `conditional_future` (the exact conditional headers in the
shared probe list). The large header is generated when sending the request,
rather than copied into this JSON file.

## Run without the live origin

From the repository root:

```sh
sh tools/http-site-test.sh --offline
```

The harness starts a candidate on a fresh loopback port and runs
`tools/http-site-cases.lkj` with output, repository-directory and candidate
connector grants. The checker has no live-origin connector. It verifies all
twelve published archive paths and the archived 404 byte-for-byte, then compares
the 64 recorded requests with the frozen status, framing, stable headers and
complete-body digest. It also requires the fixture request descriptions to
match the current query/protocol lists, so changing a test cannot silently reuse
an unrelated observation.

## Intentional refresh

Refresh only after reviewing an intended origin or test change. The recorder
refuses an existing `origin.json`; preserve the old file for comparison, then
remove it intentionally before executing:

```sh
target/lkjscript build tools/http-site-record.lkj --library-dir library \
  --grant output --grant origin=127.0.0.1:8798 --grant root=. \
  -o target/http-site-record
target/http-site-record
sh tools/http-site-test.sh --offline
git diff -- tests/fixtures/site/origin.json
rm -f target/http-site-record
```

The recorder gathers every response before creating the fixture and removes an
incomplete file on a returned write/sync error. CI consumes the committed
fixture; it never refreshes it and never needs port 8798.
