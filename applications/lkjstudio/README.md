# lkjstudio

`lkjstudio` is the checked terminal semantic workbench for `lkjscript`. Its editor state,
commands, action selection, host-outcome transitions, and frame content are maintained lkjscript
meaning in `applications/lkjstudio/.lkjscript`. The native `lkjstudio` binary owns artifact loading,
terminal mechanics, exact project adaptation, and an optional selected-filesystem adapter. It does
not own editor or semantic-edit policy.

The checked project is workspace `6ee361b40e2ce5041d64321d79c3db0d`. Revision 48 is snapshot
`12898095ee151d9d0c6f46fdbd17838ed88febd17533c6c6badb731b1f4cf83e`; its revision record is
`79837c3208dadc34e941192c54c1fcb2252260fe9b3d657055e09ee9ed1a3961`. Public target build
reproduces `lkjstudio.lkja`. No builder, generated source, or generated binding file constructs the
meaning graph.

## Build and run

```sh
cargo build --release --locked
target/release/lkjscript doctor --project applications/lkjstudio --deep
target/release/lkjscript target test lkjstudio --project applications/lkjstudio
target/release/lkjscript target build lkjstudio \
  --project applications/lkjstudio --output /tmp/lkjstudio.lkja
cmp /tmp/lkjstudio.lkja applications/lkjstudio/lkjstudio.lkja

# Ambient project discovery, without file authority.
cd applications/lkjstudio
../../target/release/lkjstudio --artifact ./lkjstudio.lkja

# Explicit semantic project plus one selected filesystem root.
../../target/release/lkjstudio --artifact ./lkjstudio.lkja \
  --project . --root ../..
```

The live command requires a trusted local terminal. `--project` is an ordinary relative or absolute
project locator; when omitted, normal bounded project discovery applies. `--root` creates one
process-local selected-root grant. Neither locator enters application meaning.

Deterministic headless replay uses the same semantic initialize, update, resume, and render owners:

```sh
target/release/lkjstudio headless --artifact applications/lkjstudio/lkjstudio.lkja \
  < replay.json
python3 applications/lkjstudio/acceptance.py --binary target/release/lkjstudio
```

## Interaction contract

Ordinary unmodified character, paste, navigation, selection, delete, and resize events update the
active semantic buffer. Editor shortcuts are Ctrl-A select all, Ctrl-N new buffer, Ctrl-W close,
Ctrl-Z undo, Ctrl-Y redo, and Ctrl-Q request exit. Undo and redo are bounded ephemeral editor state;
they never move semantic-project HEAD and never roll back a file.

Workbench and host actions use Alt so they cannot shadow those editor commands:

| area | keys |
|---|---|
| project explorer | Alt-O orient; Alt-E children; Alt-I function; Alt-U callers; Alt-D callees; Alt-T targets; Alt-B blockers |
| proposal | Alt-P open; Alt-V validate; Alt-X apply |
| review and targets | Alt-W diff; Alt-H history; Alt-N record; Alt-K test; Alt-L build; Alt-Z run; Alt-Y target list |
| selected filesystem | Alt-J list; Alt-F open; Alt-S save; Alt-R reconcile |

The active buffer is also the bounded command argument for actions that require a selector,
revision, proposal document, target, relative file path, or reconciliation token. An action result
reenters application meaning through the typed resume function. At most one action may be pending;
a second input rejects as `authority_busy`.

The semantic editor uses Unicode scalar indexes over canonical UTF-8 text. It deliberately does not
claim grapheme-cluster editing. Combining sequences may be split by movement or selection. Newlines
are scalar LF values; tabs and controls remain semantic text and are escaped by terminal rendering.
Each edit is one undo unit, a new edit after undo discards redo, buffer IDs are monotonically
allocated and not reused, and restart loses every unsaved buffer because the selected topology is a
pure foreground session. Flat immutable scalar sequences remain the representation oracle;
first-class checked slice and concatenate remove element-copy loops without introducing a rope,
piece table, gap buffer, or second text authority.

## Semantic project workflow

Alt-P asks the public project owner for a generated function proposal bound to one exact revision.
The document is an editable proposal, not authority. Alt-V validates it without publication and
returns diagnostics or a semantic diff. Alt-X applies it once. A successful apply publishes one
project revision and record and returns an exact bounded continuation. A stale draft is rejected and
remains in the buffer; the workbench performs no implicit refresh, retry, or merge.

The project adapter uses the same `Project` owner as `lkjscript query`, `proposal`, `change`,
history, diff, and target commands. It carries an exact workspace grant and expected revision and
has no direct persistence mutation path. Integration tests compare its results with public project
commands.

## Selected filesystem workflow

The initial Linux adapter pins one canonical directory and uses validated UTF-8 relative path
components. It rejects symlinks, magic links, mount crossings, `.`/`..`, empty components, excessive
depth, and nonregular files. Directory pages are byte-ordered and digest-bound. Reads return exact
size, content digest, inode/device, and modification/change-time facts.

Save either creates without replacement or atomically replaces a regular file whose complete
observed base still matches. The adapter writes and synchronizes a temporary file, renames, then
synchronizes the directory. A failure after possible rename is `unknown_visibility`; it is never
automatically retried. Alt-R independently reconciles the action as present, absent, conflicting,
or indeterminate. File origin tokens are deployment observations and are never rendered into
semantic buffer content.

## Bounds and limitations

- 100 buffers; 130,752 Unicode scalars per editor buffer after reserving the fixed 320-scalar
  workbench chrome within the global sequence/frame bound.
- 32 undo entries and 2,097,152 retained undo scalars.
- 1,000 rows by 1,000 columns; 131,072 frame/sequence scalars; 65,536 paste scalars.
- 10,000 headless events/actions; 8 MiB headless input and terminal frame output.
- Selected paths: 32 components, 255 bytes/component, 4,096 bytes total.
- Directory observation: 4,096 entries total, 256 entries/page.
- Selected regular file: 8 MiB; workbench text content: 1 MiB and 130,048 Unicode scalars.
- The checked interactive target uses 100,000,000 deterministic fuel and 128 call frames; failure
  before candidate-frame validation preserves the prior ephemeral state and pending action.

The retained loop is synchronous and sequential. There is no daemon, background worker, async
runtime, persistent editor recovery, syntax highlighting, project-wide search, clipboard interface,
mouse contract, general process interface, broad filesystem authority, binary editor, merge, or
cross-platform terminal claim. Full-frame rendering is the correctness and production route; no
cell-patch protocol or raw ANSI exists in application meaning.
