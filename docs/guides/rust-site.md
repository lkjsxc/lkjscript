# Rust documentation origin

`lkjscript-site` is an independent, read-only Rust HTTP server for the project's
public documentation. It is not a native-language application, a compiler service,
a playground, a replacement capability-reference generator or a new product CLI
operation. It was added before the owner corrected the 2026-09-29 request to
target `lkjsxc/lkjstr`. This retained experiment is not the requested Nostr client
or a Rust-only implementation mandate for lkjscript.

## Build and run

From a clean, committed checkout:

```sh
LKJSCRIPT_SITE_REVISION="$(git rev-parse HEAD)" cargo build --release --locked -p lkjscript-site
./target/release/lkjscript-site --listen 127.0.0.1:8798
```

The revision is an explicit build label, not build attestation. A missing or malformed
40-hex label displays `unversioned`; it does not manufacture a successful publication.
The default listener is also `127.0.0.1:8798`. `--listen` accepts a literal IP and
port, including bracketed IPv6, and rejects duplicate/conflicting options. `--help`
and `--version` do not start a listener. SIGINT and, on Unix, SIGTERM request graceful
shutdown. There is no runtime repository path or content-directory option.

Only this crate and its Rust dependencies are linked. The compiler, an installed
lkjscript executable, Node.js, Python, a template process and a database are not
runtime dependencies. Documents, styles and rendered pages are embedded or prepared
at startup. A source rebuild is required to publish changed documentation.

## Publication boundary

`src/catalog.rs` is the compile-time allowlist: first command, native web, HTTP,
forms, direction, architecture, status and security. Adding a file elsewhere in the
checkout cannot publish it. The server does not read workspace files, start programs,
modify projects or make outbound HTTP requests. Linked source pages are GitHub
navigation links, not server-side fetches.

GET and HEAD expose `/`, `/docs/{slug}`, `/search`, `/assets/site.css`, `/robots.txt`
and `/healthz`. Unknown paths return 404; unsupported methods on known GET routes
return 405. HEAD responses have no body, including errors. Health is plain text with
site version and source label; it contains no host paths, environment, process IDs,
credentials or unrelated service state.

Search has one optional `q` field. Duplicate or unknown fields reject. Decoded
queries are limited to 128 UTF-8 bytes and cannot contain control characters; the
encoded request URI is limited to 4096 bytes. Search only examines the embedded
allowlist. The query is escaped before appearing in HTML. Search URLs are visible to
the browser and deployment proxy, so do not use them to transmit secrets.

Raw Markdown HTML is displayed as escaped text, images do not produce network
fetches, heading anchors are deterministic and unique, and only HTTP/HTTPS or
validated repository/fragment links are emitted. Published relative document links
stay on this site; other repository references point to the labeled GitHub source.
Application responses carry a script-free Content Security Policy, `nosniff`,
`no-referrer`, restricted browser permissions and `no-cache`. There is no analytics
script, external font, user-content upload or authentication cookie.

## Deploy behind an HTTPS front door

The origin is plaintext HTTP. DNS, TLS certificates, public hostname routing,
connection/header/body/time limits and front-door traffic protection belong to a
managed reverse proxy or tunnel. Memory-safe implementation and the settings below
are not a hostile-code sandbox or an availability guarantee.

A Linux systemd example lives at
[`tools/lkjscript-site/lkjscript-site.service`](../../tools/lkjscript-site/lkjscript-site.service).
Install the tested executable in a root-owned revision directory, point
`/opt/lkjscript-site/current` at it, and install that unit as
`/etc/systemd/system/lkjscript-site.service`. The example uses a dynamic unprivileged
identity, a read-only system view, inaccessible homes, no capabilities, a private
`/tmp`, and explicit memory, task and file-descriptor limits. Adapt these settings
only to actual platform support; do not silently describe an unconfined process as
confined. Stop or restart this unit, not unrelated workspace services.

A managed hostname such as the owner's requested `lkjstr.lkjsxc.com` must be routed
by infrastructure authorized for that hostname. A temporary provider-assigned tunnel
URL is only a preview: it is not that hostname and may change when the tunnel or
workspace restarts. Do not create a custom-hostname CNAME to an arbitrary temporary
tunnel and assume that its certificate or routing will work.

## Verify changes

```sh
cargo fmt --all --check
cargo clippy --locked -p lkjscript-site --all-targets -- -D warnings
cargo test --locked -p lkjscript-site
```

The tests cover actual embedded documents, links, HTML escaping, heading collisions,
query admission, publication boundaries, methods, security headers, health and
loopback-first argument parsing. These are Rust tests over the real application;
there are no fake documentation responses or browser-execution placeholders. Use
ordinary HTTP checks against the installed binary and the HTTPS front door as well;
a router test alone does not establish live hostname reachability.
