# lkjedit

`lkjedit` is the maintained ordinary terminal editor built with lkjscript. Its typed meaning graph
owns buffers, views, tabs, the normalized split tree, Vim-like modes and commands, key and mouse
interpretation, explorer/search state, save decisions, pending-job identity, outcome integration,
and logical frames. The Rust host owns the checked package, terminal mechanics, selected-root and
semantic-project adapters, one bounded worker, process lifecycle, and independent assertions.

The sole maintained application authority is `applications/lkjedit/.lkjscript`. It is workspace
`6ee361b40e2ce5041d64321d79c3db0d`, revision 174, snapshot
`7e037b9e97e2f04cd2243899a30a3721f31faf30c55aa9d7d97f050d22004aa4`, and revision record
`aa634314469c3712fd4fca1976d07ce20073c5162a4109abaec94c6dabfe107b`.
Public target derivation produces the checked `lkjedit.lkja`; no builder or generated semantic
source reconstructs the graph.

## Build, verify, and launch

```sh
cargo build --workspace --release --locked
target/release/lkjscript doctor --project applications/lkjedit --deep
target/release/lkjscript target test lkjedit --project applications/lkjedit
target/release/lkjscript target build lkjedit-application \
  --project applications/lkjedit --output /tmp/lkjedit.lkja
cmp /tmp/lkjedit.lkja applications/lkjedit/lkjedit.lkja
python3 applications/lkjedit/acceptance.py --binary target/release/lkjedit
```

Ordinary launch never needs an artifact locator:

```text
lkjedit [PATH]
lkjedit --root ROOT [PATH]
lkjedit --project PROJECT [PATH]
```

A directory becomes the exact selected root and opens an explorer tab. A file selects its parent as
root and opens that file. With `--root`, `PATH` is resolved below the explicit root. `--project`
adds one exact semantic-project grant and opens semantic orientation when no path is given. An
absent file is an unsaved buffer and is not created before a write. `headless --artifact` is a
conformance-only override; the normal binary embeds and validates the checked application.

## Editor and layout model

Every content surface is an ordinary tab in an ordinary tile: editor view, explorer, root search,
semantic explorer/proposal/history, target output, diagnostics, and help. A tab references one view;
an editor view references one buffer. Two views of one buffer share exact content, dirty state,
origin, and undo history while retaining independent cursor, selection, and viewport state. Closing
one view does not close a multiply viewed buffer. Closing the final dirty view requires explicit
save, discard, or cancel policy.

The application stores one bounded canonical split tree. Leaves contain ordered tab stacks; internal
nodes have horizontal or vertical axis and positive integer weights. Empty leaves collapse,
single-child splits disappear, and normalization is deterministic and idempotent. Tab, view,
buffer, tile, layout-node, file-origin, and job identities are separate monotonic session domains;
paths and rendered coordinates are never those identities.

Keyboard layout commands include `:split`, `:vsplit`, `:tabnew`, `:tabclose`, tab cycling and
reordering, directional focus/move, and bounded resize commands. SGR mouse events drive the same
application owner: click selects, tab-strip drag reorders, center drop moves between tiles, edge
drop creates a split, invalid drop cancels, and splitter drag uses clamped integer geometry. A resize
during tab drag cancels it.

## Vim-like editing subset

The product deliberately claims a useful subset, not Vim compatibility. It has Normal, Insert,
Visual-character, Visual-line, command-line, and forward-search modes. Retained commands include:

- movement: `h j k l`, `0 ^ $`, `w b e`, `gg G`, counts, and Ctrl-B/Ctrl-D/Ctrl-F/Ctrl-U;
- insertion: `i a I A o O`, text, Enter, Backspace, Delete, arrows, and Escape;
- editing: `x X`, `dd D`, `cc C`, `yy`, `p P`, `u`, Ctrl-R, and visual `d y c`;
- search: `/`, `n`, and `N`, using exact case-sensitive overlapping literal matches with wrap;
- commands: `:w [PATH]`, `:q`, `:q!`, `:wq`, `:x`, `:e PATH`, `:split [PATH]`,
  `:vsplit [PATH]`, `:tabnew [PATH]`, `:tabclose`, `:buffers`, `:explore [PATH]`,
  `:search PATTERN`, `:help`, and exact semantic-project commands when a grant is present.

One unnamed register is retained. Insert sessions and complete commands form undo groups. Undo is
buffer-local, bounded ephemeral state; a new edit after undo discards redo. It never changes a file
origin, republishes a save, or moves semantic-project history.

## Text, Unicode, and line endings

Files and application text must be valid UTF-8. Invalid UTF-8 is a typed unsupported-text outcome;
replacement characters are never invented. Runtime text uses an unobservable persistent UTF-8
piece treap with bounded chunks, structural sharing, byte/scalar/newline aggregates, and derived
extended-grapheme boundaries from `unicode-segmentation` 1.13.3. Public values remain canonical flat
UTF-8, and randomized differential tests compare the persistent route with that flat oracle.

Cursor movement, deletion, and Visual-character selection use extended grapheme clusters. Exact
UTF-8 bytes remain equality and save authority; Unicode normalization, case folding, and locale
collation are absent. Terminal cell width is presentation only. Tabs and controls are rendered
safely and never become terminal control bytes.

Opening records LF, CRLF, mixed/lone-CR, and final-terminator facts. Pure CRLF buffers insert CRLF;
other buffers insert LF. Unedited separators and the final-terminator fact remain exact. Mixed input
is preserved byte-for-byte except at the explicit splice and is never silently normalized globally.

## Files, conflicts, and jobs

Selected-filesystem contract 2 pins one Linux directory descriptor and validates UTF-8 relative
components with `openat2` beneath/no-symlink/no-magiclink/no-mount-crossing confinement. Directory
pages and recursive literal search are deterministic and bounded. Stable reads return content plus
origin evidence. New files are deterministically mode 0644; atomic replacement preserves the
observed ordinary permission bits.

`:w` is an expected-base write. External change returns conflict rather than overwriting. Save-as
uses create-without-replacement. Explicit overwrite first observes a new base; it is not an alias for
blind `:w!`. A failure after publication may have become visible returns an opaque reconciliation
token. The application blocks duplicate publication until independent reconciliation reports
present, absent, conflicting, or indeterminate.

The terminal runner has one capacity-one worker request channel and one capacity-one result channel.
Application state owns the exact pending job. Local editing, navigation, layout, resize, and help
continue during read-only root search or semantic target work. A second host action reports busy;
there is no hidden queue or retry. Closing a read-only job's tab abandons its result by identity.
Possibly visible save work cannot be cancelled by dropping a tab.

## Semantic tabs and development

With a project grant, ordinary tabs expose orientation, owner/child/function navigation,
callers/callees, targets/blockers, proposals, validation, apply, history, records, diff, and target
test/build/run. Proposal text is editable but remains a base-bound proposal. Validation publishes
nothing. Apply publishes at most one revision; stale drafts remain visible and are never refreshed,
merged, or retried implicitly.

Revision 172 is the retained user-visible dogfood change. Public orientation, summary inspection, refactor
context, and generated proposal selected `open_content_tab_model`; validation reported one scalar
change and no publication; one apply changed the status to `ordinary tab opened`, published one
revision/record, returned exact continuation state, and passed the affected target.

Revisions 173 and 174 are subsequent public-CLI performance cutovers. Revision 173 replaced three
application-level scalar construction loops with bulk text/sequence operations. Revision 174
replaced full-line cell fitting with `text_cell_prefix_boundary`. Both validated without
publication, applied once, produced one immutable record, and passed all 12 target cases. The final
artifact is 471,096 bytes with SHA-256
`95cb525cea6440164e9eac58383fc194d79fc2b6df9baeadfc75309083e8338a`.

## Bounds and trust

- runtime text values: 16 MiB UTF-8; selected regular files: 8 MiB;
- terminal dimensions: 1,000 by 1,000 cells; paste: 65,536 scalars;
- headless replay: 20,000 transitions, 10,000 actions, 8 MiB request;
- selected paths: 32 components, 255 bytes each, 4,096 bytes total;
- directory observation: 4,096 entries, 256 per page;
- root search: 4,096 directories, 8,192 files, 64 MiB read, 4,096 matches, 256-byte previews;
- terminal encoded frame: 8 MiB; one executing host job and one bounded result.

The verified product scope is Linux x86-64, one trusted operator and OS account, trusted first-party
Rust, one validated package, one selected root, and an optional selected project. Artifacts,
projects, files, paths, terminal input, outcomes, and logs are hostile bounded data. There is no
network, plugin/native-code loading, shell or child-process editor interface, binary-file mode,
persistent unsaved-session recovery, syntax highlighting, clipboard, file watch, or cross-platform
product claim.
