# Rust implementation direction and public documentation origin

Date: 2026-09-29 (Asia/Tokyo).

## Subsequent owner correction — target was lkjstr

The owner subsequently stated: "すみません。lkjstrのつもりがlkjscriptとしていました".
The all-Rust implementation and `lkjstr.lkjsxc.com` request therefore applies to
`lkjsxc/lkjstr`, not this repository. The direction selection described below is
superseded by that correction; lkjscript retains its earlier eventual self-hosting
goal. The experimental site is not the requested Nostr client, and its deployment
or tests cannot establish lkjstr progress. Existing code, commits, tests, deployed
bytes and independent language development are preserved rather than broadly
reverted. Current guidance and status are corrected separately. The following
original mandate, implementation record, failed runs and accepted observations
remain historical evidence, not present authority for a Rust-only lkjscript rewrite.

## Literal owner mandate

> [https://github.com/lkjsxc/lkjscript](https://github.com/lkjsxc/lkjscript) について進めるようよろしくお願いします。必要であれば大幅なスタイルの変更も許容します。あらゆる判断において、過去ではなく、今のあなたに委ねます。全ての判断をあらかじめ許可します。最も筋の良い方向に進むために、限界まで深く考えてほしい。超長期的な視点からお願いしたい。どれだけ時間がかかっても構いません。リポジトリ全体をすべてRustになるようにお願いします。できればlkjstr.lkjsxc.comでホストしてほしい。

## Scope and baseline

The explicit repository URL selects `lkjsxc/lkjscript`, not a similarly named
repository. Baseline main was `e1a1059869baeac61a84d73c86d565a7472b7c35`.
A separate `rust-site-20260929` worktree protects already-in-progress kernel,
compiler, evaluator and public resource-test edits in the shared main checkout.
No unrelated application service, stash, working change or archived evidence is
selected for deletion. Normal non-force publication is the only integration path.

The new owner mandate supersedes the earlier goal of eventually removing Rust
through implementation self-hosting. The selected host implementation language is
Rust. Native standard-library/application programs, graph transports, compiler test
inputs, configuration and historical shell evidence are not deleted to improve a
language-percentage statistic. This is an implementation migration, not a claim that
every repository byte or transitive platform dependency is Rust.

## Implemented change

The no-Python gate now classifies extensions and 512-byte first-line shebangs directly
in Rust. It retains ASCII case behavior, extension precedence, input ordering,
relative-path admission, bounded reads and final-symlink nonfollowing. Production
classification no longer prepares an interpreter or serializes observations through
JSON/Base64. The unchanged native bundle remains a test-only independent oracle:
1,607 byte-prefix cases and 11 extension cases are compared with the Rust result.

The independent `lkjscript-site` crate supplies a real read-only documentation origin:
an explicitly embedded eight-document catalog, Rust Markdown rendering and search,
HTML escaping, local/source link resolution, unique heading identifiers, restrained
responsive styling, a script-free Content Security Policy, bounded query admission,
HEAD/error behavior, health metadata and loopback-first CLI configuration. It does
not link the compiler, read a runtime content directory or offer an execution API.
The systemd example makes deployment boundaries explicit without claiming a sandbox.

## Uncompleted Rust migration

The capability-reference generator at `platform/contract/native_guides` still
executes its language bundle. A proposed Rust replacement write was rejected by the
tool safety check before application. That write was not retried through another
path; its owned empty scaffolding was removed. The existing generator, its bundles,
its eight generated documents and its independent tests are unchanged. The public
website renders Markdown; it does not replace this capability-reference generator.

The overall request for complete Rust migration is therefore not complete. Root
guidance, direction, architecture and current status distinguish this fact from the
Rust classifier and new site. Remaining generated installation/workflow shell glue,
configuration and archived scripts are also not represented as Rust source.

## Verification and publication boundary

The first focused classifier run completed 9 tests with no failures, including the
retained native oracle, raw-byte and filesystem cases. The first site run completed
7 library tests and 1 CLI test with no failures. A subsequent regression case covers
collisions between generated heading suffixes and literal heading names. These are
focused observations, not claims of a completed full gate, release, public deployment
or requested-hostname routing. Final source, validation and deployment observations
are recorded below only after execution.

## Continuation: initial aggregate failure and current-main integration

The first aggregate run did finish, but **failed**: 21 of 26 gates passed fresh,
with stable source inputs, at source `74bccdef22f587eb0a71e3abb14b0d8e2c1fb0ad`.
The original receipt is
`.artifacts/lkjscript-dev/check/1790649836244123742-790315-0/receipt.json`.
The invoker was mistakenly the `--profile test` verifier at `target/debug`:
its 609,826,528 bytes exceeded the 402,653,184-byte admission bound. Distributed
HTTP, outbound HTTP, offline packages, pure-tail and stateful HTTP therefore
rejected the verifier before exercising their workloads. Workspace tests, Clippy,
release compilation and the other selected gates passed. Neither the size bound
nor the failed receipt is changed to convert these failures into successes.

During the interruption, parallel language work advanced main to
`98b3c1f59701e151a77acf5473ae856a8dffb7aa`, including the accepted v0.1.59
multiple-resource suffix implementation. A normal merge into this isolated
worktree incorporated that complete history without conflicts. The new aggregate
must use the maintained release verifier built for the merged source, not the
oversized test-profile executable or an old source label. The original shared
checkout remains separate and no unpublished work there is selected for staging.

## Accepted merged source and deployment

The merged implementation source is
`2e72b47897452627e41c48e6311373927110e9c7`, tree
`90af92aeebaee1d110072825684fae9fa86b9fef`. Its maintained
`target/release/lkjscript-dev check full --fresh --jobs 2 --machine` run completed
successfully on 2026-09-29 at 04:12:37 UTC: **26 selected, 26 passed fresh, zero
reused, none unrun**, with `input_stable=true` and no failure. Elapsed execution
was 1128.034704193 seconds, excluding the preceding verifier build. The
36,329,672-byte release verifier satisfied the unchanged admission boundary.
The original 64,836-byte receipt and individual logs remain at
`.artifacts/lkjscript-dev/check/1790654029765622330-977573-0/`.
This is correctness acceptance of the merged source, not a performance comparison.
The earlier oversized-verifier aggregate remains a failed run and is not reused.
Full-profile acceptance does not assert that every ignored or opt-in probe ran.

Separately, `cargo test --release --locked -p lkjscript-site`, using a separate
site target directory and the same source label, passed all eight library tests
and the CLI test. The installed site executable was built from that clean merged
source and compared byte-for-byte with its root-owned installed copy. It resides
at `/opt/lkjscript-site/releases/2e72b47897452627e41c48e6311373927110e9c7/`;
`current` points to this revision, with the preceding installed revision retained
for rollback. The source label is an explicit build label, not independent attestation.

The read-only origin is enabled as `lkjscript-site.service` on `127.0.0.1:8798`.
Its dynamic user, read-only system view, inaccessible homes and 128 MiB memory
ceiling are deployed service settings, not a hostile-code sandbox claim. A second
owned test instance served the same source on port 18998, stopped normally with
exit status zero after SIGTERM, and left no listener there. The public origin
remained running throughout that lifecycle probe.

Public preview: https://symposium-tips-twins-hotel.trycloudflare.com/

At 03:54 UTC, HTTPS requests through that public front door from the development
workspace returned 200 for the home page, all eight document pages, search, CSS
and health (12 routes). Health reported the exact merged source above. A private
file path returned 404, POST to the home page returned 405, a repeated search field
returned 400, and the script-free CSP and other documented response headers were
present. These are real HTTP observations, not browser rendering acceptance.
Optional visual-browser checks and a separate automated internal-link audit were
not completed because the execution tools blocked them; no pass is claimed and
those blocked operations were not retried by an alternate route.

The preview tunnel is the separate transient
`lkjscript-site-preview-tunnel.service`. It is development infrastructure, not a
persistent custom-domain deployment or an uptime guarantee. Workspace/tunnel
restarts can invalidate its URL. The requested `lkjstr.lkjsxc.com` resolved to
`92.202.56.95`, but connecting to port 443 from this workspace timed out. An
existing authorized connection for changing the relevant DNS/TLS/edge routing
was not available. No DNS record, existing edge configuration, unrelated service,
credential or GitOps repository was changed. Custom-domain hosting remains undone.

This closing campaign update and the native-guide README correction are
narrative-only descendants of the tested source. Neither is in the site's embedded
catalog, and neither changes the compiler, policy classifier, site implementation,
lockfile, generated capability pages or deployed executable. Normal mainline
publication and its independent ref read are recorded in the execution return;
these notes do not relabel earlier proof with a later reporting commit. No release
tag, compiler distribution, forced update or protection change belongs to this work.
The separate worktree and ignored original evidence remain available for diagnosis;
unrelated shared-checkout changes and services remain preserved.
