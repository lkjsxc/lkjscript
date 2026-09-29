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

## Mainline continuation: complete the retained integration

The next owner request again delegated all engineering decisions and normal mainline
completion. Main was independently observed at `98b3c1f5`; the retained site branch
was `74bccdef`. The latter's one commit is merged, not recreated or allowed to
replace the newer multi-resource implementation. Existing untracked files, stash,
other worktrees, the running site origin and unrelated services remain untouched.
The original site worktree later had its own release-tool build in progress; that
process and its evidence are not owned or cancelled by this continuation.

A new test reproduced a real integration gap: modifying the embedded
`docs/status.md` selected only `rust_only_tooling` and `diff_check`, omitting execution.
The failing result is retained in
`.artifacts/20260929-rust-integration/selection-before.log` (0 passed, 1 failed).
The repair gives the site and checker two compile-time projections of one Rust
catalog in `tools/lkjscript-site/src/documents.rs`. The checker does not link the
HTTP server or maintain a second publication path list. Embedded-document edits
select the existing `workspace_tests` obligation and its dependencies; no new
verification gate or release receipt schema is introduced. Tests cover every
catalog path as untracked, modified, staged and deleted input, both rename sides,
exact-prefix nonmatches and nonembedded history remaining lightweight.

The roadmap and one stale contributor-guidance sentence are reconciled with the
explicit Rust implementation decision. Language-core priorities remain ownership,
lifetimes, generics/effects and genuinely shared execution. Neither this merge nor
the documentation server implements general memory borrowing or a new runtime.
The blocked capability-reference migration is not retried, and its original failure
record is preserved. Full acceptance, live copied-site evidence and delivery remain
unclaimed until the observations below.

Focused feedback completed on the combined working source: all 7 changed-selection
tests, all 9 policy tests (including the retained independent language oracle), and
all 8 site library tests plus its CLI test passed. Logs are `selection-after.log`,
`policy-after.log` and `site-after.log` under the same integration artifact directory.
The first site invocation piped output to a terminal-owned `tee` and retained only
incomplete compilation output; it is not acceptance evidence. The later direct-log
invocation above completed. Formatting and staged/unstaged whitespace checks passed.
