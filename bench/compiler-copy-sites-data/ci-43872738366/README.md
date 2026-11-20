# Retained CI acceptance at f9e4581a

CI43872738366, attempt 1, completed successfully for both required native gate
and optional Rust witness. The complete downloaded logs, final run response,
final job response and derived summary are available directly in this directory.
The [original capture note](capture-README.md) describes their interpretation.

`raw-ci.tar.gz` preserves the complete original capture directory, including
all API snapshots, exact commands/statuses, workflow/bootstrap/gate sources,
the pinned mutation-command source, the inspection warning and separate
Git-seed identity. Extract it into a fresh directory to use `raw-manifest.sha256`
there. That manifest describes the original archive layout, not this directory's
few duplicated convenience views.

The archive is 54,190 bytes, SHA-256
`12197424fe9194642cae806776e8100498dd8c19d50470ce7784416c0b2f4ab5`.
Its 140 regular files total 306,068 bytes, including the original 139-record
manifest `1d3710033c86e8062f67bb0bf45313cd43f060ccf7a7b5a50509e07dcddcf4c8`.
Root independently compared every archived file with its manifest entry and
live original bytes before acceptance; [the result](https://github.com/lkjsxc/lkjscript/blob/589e11b5a5ede874d05917c2c8b9227c7af62fc5/bench/compiler-copy-sites-data/ci-43872738366/archive-acceptance.json)
records that verification. The language source is historical evidence inside
the archive, not an additional compiled specimen or adopted source change.

Native gate: 1,677.588 s, load 1.18→2.15, 5,448 fresh case requests and zero skips.
The witness reports 276 value/output cases, 344 verdicts and compiler byte
identity. Detailed mutation outcome counts and a native image digest are not
printed by this compact CI log. The source-contract and separate Git-seed
identity qualifications remain explicit in summary.json and the capture note.
