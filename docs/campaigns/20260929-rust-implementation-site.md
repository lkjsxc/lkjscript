# Rust implementation direction and public documentation origin

Date: 2026-09-29 (Asia/Tokyo).

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
