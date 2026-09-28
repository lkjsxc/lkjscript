# Inspect recent accepted changes

Development v0.1.53 adds this read-only command. It is absent from v0.1.51 and
v0.1.52; check `capabilities --section inspection` and [current status](../status.md).

```sh
lkjscript --project site inspect history
lkjscript --project site inspect history --limit 1
lkjscript --project site inspect history --limit 100
```

Use an existing accepted project, such as the [web starter](native-web.md) or
[durable editor](native-web-editor.md). The command does not require either
application to be running and does not open its operational `.lkjdata` directory.

## Read the result

The `history` header identifies one observed revision and its record. Entries
follow newest-first, with exact parent links, recorded change intent, changed-owner
and dependency counts, and separate test-selection, execution and pass counts.
`intent-present=false` distinguishes absent intent from an explicitly empty string.
Intent is an author's nonsemantic explanation, not proof of the actual change;
the recorded counts and exact bindings have separate meanings.

The default returns at most 20 entries; `--limit` accepts 1 through 100. When
`truncated=true`, the next parent is committed by the last displayed record but
has **not been read**. Its identity is not a continuation token or a promise that
the rest of the chain is intact. Increasing the limit can expose older entries
within the advertised bound. There is no arbitrary historical revision selector
or paging beyond 100 in this increment.

`current-validation=not-run` is deliberate. A historical record saying that a
change was accepted is not a claim that the old or current program passes today's
validator. Selected tests are not executed tests, and executed tests are not
necessarily passed tests. Run the independent current check when needed:

```sh
lkjscript --project site check
```

## Keep the editing workflow explicit

An accepted edit creates a new revision and appears at the front. Merely exporting
a draft, making a plan, failing a stale request or retrying an already accepted
idempotent request does not add an accepted revision. Named drafting remains
available for the next edit:

```sh
lkjscript --project site change draft --declaration web::page-title --output title.lkjc
```

Edit that absent-output draft, inspect the complete `change plan`, then apply only
the reviewed exact request and token as described in the [web guide](native-web.md).
The history command cannot apply an edit, replay old input or select an older
deployment. Reusing an old runtime/artifact pair also does not roll back data.

## Boundaries

The traversal reads at most 200 revision/receipt objects and 8 MiB under shared
pre-consumption admission. Its reported read counters exclude normal repository
opening and first-open reconstruction of disposable lock/catalog files. It neither
scans every pack for history nor reconstructs old executable graphs. A damaged
visited link fails the complete response rather than looking like an empty suffix.
No second history database, hidden project copy, timestamp or author attribution
is synthesized. Existing accepted objects are the only history authority.

See the [precise contract](../spec/semantic-cli.md#recent-recorded-project-history)
and [verification record](../campaigns/202609281010.md).
The [original prototype](../campaigns/202609280123.md) remains a separate,
unaccepted historical attempt, not this continuation's acceptance evidence.
