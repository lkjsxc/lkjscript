# AGENTS.md

This file governs the entire `lkjsxc/lkjscript` repository.

A deeper `AGENTS.md` may add rules for a genuine ownership boundary, but it may not weaken any
applicable rule in this file.

Use English for maintained code, tests, diagnostics, protocol fields, machine output,
specifications, documentation, examples, benchmark labels, commit messages, revision metadata,
evidence, and handoffs.

## Mission

Build `lkjscript` as a meaning-first, agent-native semantic software platform that can produce
ordinary, useful, high-performance applications rather than only language demonstrations.

The primary editable authority for an lkjscript program is one validated typed meaning graph with
exact immutable development history.

Coding agents and humans must be able to discover, inspect, change, validate, test, build, run,
package, diagnose, recover, and evolve useful software through the public `lkjscript` CLI without
maintaining a custom graph-construction program.

Humans remain first-class for intent, product judgment, governance, security policy, explanation,
review, operations, and acceptance.

Coding agents are first-class program authors and maintainers.

The current representative ordinary application is `lkjedit`: a Vim-like, mouse-capable, tiled text
editor whose explorer, search results, semantic tools, output views, and text editors all participate
in one ordinary tab-and-tile model.

`lkjedit` is a forcing function, not permission to special-case one product in every layer.

Optimize jointly for:

- semantic correctness;
- complete useful products;
- one coherent authority model;
- direct CLI-native authoring;
- compact exact agent interactions;
- weak-model success;
- low correction depth;
- low provider and operator cost when measured;
- deterministic artifacts;
- explicit authority;
- recoverable publication;
- bounded resource use;
- interactive responsiveness;
- maintainable ownership;
- independently checkable evidence;
- simple correctness oracles;
- long-term execution performance;
- deletion of superseded mechanisms.

Do not optimize for novelty, feature count, benchmark theater, syntax fashion, roadmap inertia,
historical compatibility, sunk cost, or preservation of an implementation merely because it was
difficult to create.

## North star

A coding agent should be able to enter an ordinary project directory and use public `lkjscript`
commands to:

1. discover the exact semantic project and selected revision;
2. obtain compact orientation without loading the whole graph, schema, prompt, or history;
3. request only the typed subgraph, constraints, examples, and target facts relevant to one task;
4. prepare one bounded exact semantic change;
5. validate through the same owner that would publish it;
6. publish exactly one immutable revision and canonical revision record;
7. receive enough exact continuation state to avoid rediscovering unchanged meaning;
8. inspect semantic diff, impact, diagnostics, history, and target status;
9. build, test, run, and package a named target derived from the selected revision;
10. recover after interruption, stale state, output loss, or unknown publication without unsafe retry;
11. continue without reading or regenerating a large construction script;
12. finish with compact verification output and separately retained detailed logs.

An ordinary user should be able to launch an lkjscript application without understanding semantic
graph internals, artifact identities, internal workspace paths, or host adapter plumbing.

The meaning graph is the center.

Text documents, command streams, JSON, rendered source-like views, TUI views, generated bindings,
compiled forms, indexes, caches, artifacts, logs, and Git diffs are proposals, views, distribution
objects, operational records, or derived state according to explicit contracts.

None silently becomes a second editable source of truth.

## Authority and precedence

When active artifacts disagree, use this order:

1. The active user task.
2. This root `AGENTS.md`.
3. An explicitly selected active campaign prompt.
4. Accepted normative files under `docs/spec/`.
5. Executable contracts and focused invariant tests.
6. The accepted semantic development repository and its exact revision records.
7. Generated descriptions mechanically derived from one executable owner.
8. `docs/status.md`.
9. `docs/architecture.md`.
10. Current structured evidence and `docs/performance.md`.
11. `docs/roadmap.md`.
12. `README.md`.
13. Comments, examples, historical prompts, branches, pull requests, commits, issues, and
    discussions.

Newer verified checkout state outranks older plans, remembered repository state, and the baseline
recorded in a campaign prompt.

A campaign prompt owns one campaign's objectives, hypotheses, gates, and handoff. It does not become
permanent semantic authority.

An old prompt is historical evidence unless the active task explicitly selects it.

When accepted behavior changes, update the owning specification and executable contract in the same
verified milestone.

Do not let generated documentation, a checked artifact, a test fixture, a benchmark receipt, or a
commit message silently outrank the semantic owner that produced it.

## Autonomy and decision responsibility

Resolve ordinary engineering decisions from the checkout, active requirements, complete product
workflows, bounded prototypes, and measured evidence.

Do not ask the user to choose between implementation details that the repository can answer.

Do not stop at a report when a safe dependency-closed implementation is authorized and feasible.

Do not scatter speculative partial architecture.

A bold change is acceptable when it converges the repository on one stronger design and completes
the consumer that justifies it.

A large rewrite is not automatically bold or correct. Preserve independently valuable invariants,
or replace them with stronger verified contracts.

## Repository safety

Before editing, inspect the actual checkout:

```sh
git status --short
git branch --show-current
git rev-parse HEAD
git log -5 --oneline
git remote -v
git status --branch --short
find .. -name AGENTS.md -print
```

Read every applicable instruction file.

Preserve unrelated modified and untracked work.

Reading in-scope files, editing in-scope files, and running non-destructive validation are authorized
for implementation tasks unless the active task says otherwise.

Do not reset, clean, overwrite unrelated files, amend, rebase, merge, force-push, publish a release,
close a pull request, or alter unrelated remote state without explicit authorization for that exact
action.

Repository permissions are not user authorization.

When an active task authorizes staging, commits, or pushes:

- inspect the complete staged and unstaged diff first;
- stage only exact in-scope paths with explicit path arguments;
- never use `git add .`, `git add -A`, or `git add --all`;
- preserve unrelated work;
- use coherent commit boundaries;
- never amend or force-push;
- push only the intended current branch to its configured or explicitly selected remote branch;
- verify the resulting local commit and remote ref;
- report every requested publication action not completed.

Never commit credentials, secrets, private transcripts, hidden model reasoning, personal data, raw
provider events, unrelated user files, disposable benchmark payloads, or generated corpora with
unclear licensing.

Keep scratch state, destructive experiments, unsanitized measurements, downloaded research, and
losing prototypes outside the repository unless a retained artifact has a named consumer.

Report partial completion, unavailable tools, failed verification, uncertain outcomes, and
irreproducible observations explicitly.

## Backward compatibility

Backward compatibility is absent unless the active user explicitly requires it.

Use incompatible-change freedom to converge on one coherent design.

After cutover, delete old readers, writers, aliases, fallbacks, compatibility tests, migration-only
code, stale examples, dormant flags, duplicate protocols, superseded applications, and superseded
documentation.

Do not introduce editions, dual success paths, hidden fallback, automatic old-format adoption,
silent migration, or renamed aliases as insurance.

A direct break still requires:

- one complete replacement;
- exact rejection of predecessors;
- focused negative tests;
- updated normative documentation;
- updated current-state documentation;
- a verified current first-party application;
- deletion of the old normal path.

Incompatible-change freedom is not permission for an unverified rewrite.

Immutable historical development snapshots may retain the exact validator needed to inspect their
own history only when that route cannot produce, publish, or execute a predecessor current artifact.

History reconstruction is not compatibility.

## Meaning graph as development authority

Each accepted project revision has one authoritative typed meaning graph.

The graph may include program declarations, function bodies, tests, build targets, release
projections, application composition, interface-role declarations, and other development meaning
only when their authority domain and consumer are explicit.

Names, formatting, source positions, file paths, command order, generated indexes, renderings, and
deployment layout are not semantic identity.

A human-readable document is a proposal or view. It may be convenient and editable, but accepted
meaning never depends on preserving, reparsing, or diffing its formatting.

A command stream is a proposal and may be retained as an exact recipe or audit fact. It is not
current program authority.

A checked application artifact may be immutable distribution authority under its specification. It
is not maintained development authority.

A custom Python, shell, Rust, macro, build-script, generated source, serialized fixture, or opaque
blob that reconstructs the graph is not maintained semantic source.

Temporary migration code must be isolated, independently checked, and deleted after direct cutover.

Every maintained first-party lkjscript application must have a public-CLI-native path from its
tracked semantic project to validated targets and distribution artifacts.

Derived views may be regenerated or discarded without changing accepted meaning.

## Ordinary applications as platform proof

A platform capability is not complete merely because a unit test or synthetic sample can invoke it.

For substantial language, runtime, terminal, filesystem, build, or authoring changes, select one
complete ordinary user workflow and close it end to end.

An ordinary application:

- has a user-valued purpose independent of testing lkjscript;
- runs through supported public packaging and deployment paths;
- owns its domain policy in lkjscript meaning;
- uses host adapters only for explicit external mechanics and authority;
- has black-box acceptance tests;
- has measured representative workloads;
- has exact error and recovery behavior;
- can be reproduced from a fresh checkout;
- does not require a custom semantic builder;
- does not require users to know internal IDs or artifact paths for normal launch.

A representative product may reveal a general capability. Generalize only after identifying the
shared semantic obligation and at least one concrete consumer.

Do not make a product generic by moving its policy into native code.

Do not make the platform product-specific by naming editor concepts in unrelated universal
contracts when a narrower interactive or text contract is sufficient.

## Self-hosting gradient

Prefer building lkjscript development tools as lkjscript applications when the current language and
runtime can own their policy.

Use native code only for bootstrap, operating-system adaptation, terminal integration, selected
filesystem authority, explicit resource handling, artifact embedding or deployment, and
independently justified performance boundaries.

A native shell must not become the hidden owner of application state transitions, layout policy,
edit policy, mode policy, key bindings, tab behavior, search ordering, project selection policy,
undo semantics, or domain decisions.

A first-party semantic tool must be authored and evolved through the same public CLI that ordinary
agents use.

A bootstrap exception is temporary. Record why it exists, what exact capability blocks removal, and
the cutover condition.

Self-hosting is a gradient, not a slogan.

Do not move a boundary into lkjscript merely to increase a percentage.

The winning boundary minimizes duplicate authority while preserving exact validation, performance,
recovery, and independent testing.

## Semantic development repository

A semantic development repository owns one project continuity and immutable accepted development
history.

It exposes exact workspace identity, selected revision, immutable revision objects, canonical
revision records, parent continuity, durable allocation, tombstones, semantic diff facts, named
targets, validation, history, diagnosis, backup, and reconstruction.

Git owns collaboration and distribution of repository files.

The semantic repository owns accepted evolution of lkjscript meaning.

Neither is inferred from the other.

Do not require agents to reverse-engineer semantic history from binary Git diffs.

Do not use a Git commit hash as workspace, entity, revision, release, application, instance,
authorization, capability, buffer, tab, view, job, file-origin, or terminal-session identity.

Tracked first-party semantic repositories must be portable, bounded, deterministic under their trust
model, and usable from a fresh checkout through public commands.

A project path is a locator. It is not semantic identity.

Branching, merging, rebasing, distributed synchronization, and conflict-free replication require
current consumers and exact semantics.

Do not copy Git features speculatively.

## Public semantic CLI

The public `lkjscript` CLI is the primary development interface.

Raw constructors, private library calls, test-only builders, custom generators, and direct store
mutation are not acceptable first-party authoring paths.

Human mode provides deterministic bounded help, orientation, status, inspection, history, diff,
validation, build, test, run, recovery, and actionable errors.

Machine mode provides one strict versioned typed contract with exact framing, request correlation,
stable error classes, explicit omissions, and no progress contamination.

A caller-owned foreground session may reuse validated state, contexts, and local handles.

It is not authority, a daemon, a queue, or a scheduler.

Ordinary commands discover a project from the current directory or an explicit relative or absolute
path.

Users must not supply workspace IDs, current revisions, schema digests, internal state paths,
artifact IDs, or generated binding constants for ordinary work when the CLI can discover them
exactly.

Convenience never weakens exactness.

The CLI must preserve all of these invariants:

- project discovery resolves to one exact workspace;
- reads report the exact selected revision;
- mutations bind an exact expected revision;
- stale state rejects;
- no mutation is silently retried;
- selectors reject ambiguity;
- session-local handles never persist as semantic identity;
- all accepted changes pass the same typed validator;
- validate-only and commit use the same normalization and candidate path;
- output preflight happens before publication where output size can reject;
- output failure cannot retroactively undo accepted authority;
- compact output never omits failure classification or continuation facts.

## Quiet, exact command output

Passing commands must default to compact output.

A command that executes hundreds or thousands of successful checks must not print one line per
passing check unless the caller explicitly requests full detail.

Default successful output should contain:

- command or profile identity;
- exact selected project or target when relevant;
- total passed, failed, skipped, exhausted, unavailable, and indeterminate counts;
- elapsed time and directly observed resource totals when available;
- exact artifact, revision, or result identity when relevant;
- one stable log or receipt locator when retained.

Default failure output should contain:

- stable failure class;
- failing check identifiers;
- bounded actionable diagnostics;
- bounded context around the first or selected failures;
- exact path to complete retained logs;
- explicit truncation and continuation facts.

Detailed logs may be written to an ignored artifact directory or an explicitly selected path.

A compact terminal summary and complete log are separate projections of one result.

Never require a coding agent to ingest full passing output to determine success.

Machine output must remain closed, typed, deterministic, and independently parseable.

A `--verbose`, `--details`, or equivalent explicit mode may expose complete per-case output.

Do not suppress warnings, skipped cases, exhaustion, cancellation, unavailable tools, or
indeterminate outcomes merely to be concise.

## Human, machine, and interactive surfaces

One semantic owner may have human, machine, and interactive projections, but those projections must
not define competing semantics.

Human output is bounded, escaped, deterministic, and useful without requiring machine-field
archaeology.

Machine output is closed, versioned, correlated, deterministic, and complete about omissions and
continuations.

Interactive views are derived from exact revisions or explicit ephemeral state.

A TUI must never make rendered rows, cursor coordinates, pane order, tab order, cached labels, or
terminal colors semantic project identity.

A source-like editor view must carry exact base and context facts and normalize through the
transaction owner.

Formatting-only changes that normalize to the same meaning publish no semantic revision.

Interactive convenience must expose stale state, conflicts, unknown outcomes, and validation
failures rather than silently hiding them.

## Project discovery and paths

Relative paths are normal public input.

Resolve paths against one documented base, canonicalize and revalidate authority boundaries, reject
unsafe traversal and substitution, and keep paths out of semantic identity.

Discovery must find exactly one strict marker under a bounded parent-walk policy.

Reject symlinked markers, unsafe traversal, ambiguous nested authorities, nonregular files, foreign
workspace bindings, malformed locators, and path substitution.

Canonical paths are deployment facts and may change without changing semantic identity.

When an exact file is selected, bind the selected authority or content facts required to prevent
time-of-check/time-of-use substitution.

Build outputs, backups, imported documents, selected filesystem roots, and installed application
resources remain explicit deployment inputs.

## Context and semantic graph queries

Context budget is a correctness, latency, and provider-cost constraint.

Do not make routine agents request a global graph, full schema, full history, full prompt, or full
target closure.

Provide compact orientation, typed task-scoped context, on-demand expansion, stable projections,
exact digests, continuations, and explicit omissions.

A context object is a disposable observation bound to one exact project, revision, schema, query
plan, bounds, and result digest.

A context alias or handle is session-local and revision-bound.

It is never durable entity identity.

Known-digest reuse may return unchanged only when every bound fact agrees.

Queries must state traversal direction, ordering, page bounds, projection, truncation, and
continuation.

Graph queries must use a closed bounded vocabulary rather than an ambient general database language
unless a complete consumer proves the need.

After exact owners are identified, stop broad discovery unless evidence invalidates the ownership
map.

Context response growth, repeated rediscovery, correction depth, source bytes opened, commands,
processes, and elapsed time are measured on complete tasks.

## CLI-native change model

A semantic change may be one high-level command, one bounded atomic bundle, or one request in a
foreground session.

The change model must support multi-entity atomicity when no valid intermediate revision exists.

Draft state is not accepted meaning.

If an interactive draft exists, define its owner, lifetime, bounds, crash behavior, identity domain,
validation points, and retention.

Use exact base-bound selectors.

Friendly names resolve only when unambiguous in the selected revision.

Function-local references remain function- and revision-bound unless a concrete continuity consumer
justifies more.

A local edit selector may use exact base-local identity or a structural precondition without
granting durable cross-revision identity.

Prefer bounded subgraph edits and declaration-level operations over resending a large application
when they materially reduce context and correction risk.

Retain whole-function replacement as a simple independent oracle and escape hatch.

Validate-only and commit share parsing, normalization, semantic validation, target validation,
artifact preflight, response preflight, and relevant resource checks.

A successful apply may return a bounded continuation or context delta only when it is exact,
preflighted, idempotency-bound, and measurably useful.

Do not preserve a second edit protocol merely for compatibility.

## Automatic development history

Every successful public semantic mutation publishes exactly one immutable revision and one canonical
revision record.

Validation-only, rejection, semantic no-change, stale input, failed prepublication, and read-only
work publish no revision.

A revision record binds workspace, exact base and result, parent and result snapshots, accepted
change digest, exact semantic diff facts, durable entity changes, function-body changes, target
changes, and publication outcome.

Optional intent, actor, tool, or observed time is bounded untrusted metadata and cannot affect
semantic identity unless a future specification deliberately assigns that role.

Do not store hidden chain of thought, provider transcripts, prompts, credentials, or secrets as
revision metadata.

Normal accepted changes record themselves without requiring a manually authored commit message.

History is append-only.

Revert or restoration publishes a new validated revision and never rewrites accepted history.

Product undo and editor undo are application semantics and must not move semantic project HEAD
backward.

## Build targets and deterministic derivation

Build configuration for maintained lkjscript software belongs in an exact versioned target graph.

Do not hide build meaning in Python dictionaries, shell sequences, Cargo build scripts, private Rust
code, generated manifests, or undocumented command arguments.

Target identity and edges use exact semantic identities.

Target names are lookup metadata.

A build selects one exact project revision and exact dependency artifacts.

No target resolves `latest`, mutable registry coordinates, ambient files, or unverified paths at the
semantic boundary.

Release and application artifacts remain separate immutable authority domains.

Target declarations do not silently become runtime grants, instance state, deployment authority, or
executable identity.

Generated bindings are derived views.

Prefer direct validated artifact descriptors when they are simpler.

A checked derived artifact is allowed only when packaging needs it and a public target build
reproduces it exactly or deliberately replaces it under a new specified identity.

Build, test, and run never publish a development revision.

Selective validation, caching, incremental lowering, bytecode, and native tiers are optimizations
and require an independent full oracle.

## Prohibition on semantic build scripts

Do not retain `build.py`, `generate.py`, shell heredocs, private Rust builders, macros, `build.rs`,
generated source, opaque binary fixtures, or similar programs as the primary way to construct or
evolve maintained meaning.

Do not replace one graph builder with another language, a larger fixture, generated source, or
opaque serialized graph.

Acceptance, workload, fault-injection, and measurement scripts may remain when they exercise public
boundaries and do not own application meaning or build configuration.

## Application-first closure

Every substantial platform campaign selects a valuable complete application or user workflow.

The application owns domain state, validation, ordering, decisions, commands, layout, input policy,
and typed outcomes in lkjscript semantics.

A host client may own transport, terminal adaptation, rendering mechanics, selected resource
authority, process lifecycle, bounded background execution, and independent assertions.

A host client may not own hidden business state, edit policy, mode policy, layout policy, tab
behavior, ordering, query semantics, or domain decisions.

Build the smallest complete product slice first.

Add language, runtime, storage, interface, or tooling mechanisms only for an exact blocker revealed
by the slice.

Return to the product immediately after closing each blocker.

A capability is incomplete when the host reconstructs private state, suppresses invalid requests,
parses opaque responses for domain meaning, or remains the real workflow controller.

Run the completed product from a fresh checkout through public release binaries.

Dogfood the semantic CLI on a real maintained-application change before completion.

Delete productless infrastructure, losing prototypes, stale examples, and intermediate artifacts
without a retained consumer.

## Interactive application architecture

Interactive software separates:

- accepted semantic program meaning;
- ephemeral foreground application state;
- durable application state;
- external resource authority;
- background operational work;
- rendered presentation;
- deployment state.

Do not publish a durable application revision for every key or mouse event merely because a durable
instance mechanism exists.

Do not claim crash durability for ephemeral state.

Select pure foreground, durable, or hybrid interaction topology from a complete latency, recovery,
and authority comparison.

A foreground interactive session is caller-owned and disappears on process exit unless an explicit
recovery authority exists.

An application-defined update function owns state transition and action intent.

An application-defined render function owns semantic frame content.

A native runner owns terminal acquisition, event decoding, frame projection, safe differential
emission, signal handling, and cleanup.

The runner must not interpret application domain state to decide behavior.

External actions are closed typed requests with explicit outcomes.

Possible external visibility stops automatic retry and enters an explicit reconciliation state.

Input queues, action queues, render queues, and background work are bounded.

Event ordering, coalescing, cancellation, stale results, shutdown, and restart are specified.

A responsive foreground session may continue processing input while one bounded external action is
running only when the action has an exact ephemeral identity and application-owned pending state.

## Universal tab-and-tile model

A tab is one ordinary layout item.

Explorer, search, text editor, semantic explorer, proposal, history, output, diagnostics, and help
views use the same tab movement, ordering, focus, close, drag, and split operations.

A special content kind may have different domain behavior.

It must not bypass the common tab-and-tile lifecycle.

Keep these domains distinct:

- layout tree identity;
- tile identity;
- tab identity;
- view identity;
- buffer identity;
- file-origin identity;
- search-job identity;
- project-query identity;
- terminal hit region;
- rendered position.

A tab references one view.

An editor view references one buffer and owns viewport-local state.

Multiple editor views may reference one buffer.

Closing one view does not close the buffer while another view retains it.

A buffer owns text, dirty state, edit history, file origin, and save/conflict state.

A file path is not a buffer, view, or tab identity.

The layout is a bounded normalized split tree.

An internal split has one orientation, ordered children, and bounded size allocation.

A leaf has one ordered tab stack and one selected tab.

Empty leaves collapse deterministically.

Single-child splits normalize away.

Size ratios or weights use exact bounded integer or fixed-point semantics, never host floating-point
accident.

Layout normalization is idempotent.

Keyboard and mouse operations invoke the same application-owned layout commands.

A tab drag has explicit source, pointer origin, threshold, current target, preview, cancellation, and
drop semantics.

Dropping on a tab strip reorders or moves the tab.

Dropping on a content center joins the target stack.

Dropping on a content edge creates a split in the corresponding direction.

Dropping outside valid regions cancels without changing layout.

Splitter dragging is clamped by minimum tile dimensions and deterministic rounding.

Host hit testing may expose terminal coordinates only.

The application owns the semantic mapping from current layout geometry to a command.

## Buffer, view, and file semantics

Buffer identity is distinct from file path, content digest, tab position, view identity, and display
name.

View state includes cursor, selection, preferred column, viewport, mode-local state, and transient
search state only when those are deliberately per-view.

Buffer state includes content, line-ending policy, dirty status, edit history, file origin, and
conflict state.

Two views of one buffer observe the same accepted edit sequence.

Per-view cursor and viewport state remain independent.

Buffer allocation is monotonic within its application session and identities are not silently
reused.

Closing a dirty buffer requires an application-owned decision.

Force close is explicit and never confused with save.

Opening an already-open exact file selects or creates a view according to one documented policy.

A file origin is a deployment locator plus exact observed base facts.

It is not buffer identity.

External modification produces an application-visible decision: reload, keep, compare, save-as,
overwrite under an explicit new base, or cancel.

No path spelling, digest, inode, timestamp, or tab label becomes buffer identity.

## Text and Unicode semantics

Never call byte offsets character offsets.

For each operation, name the exact indexing unit:

- UTF-8 byte boundary;
- Unicode scalar index;
- extended grapheme cluster index;
- logical line index;
- line-local scalar or grapheme column;
- terminal cell column.

Every accepted text value is valid UTF-8.

Every text edit preserves valid UTF-8 or returns a typed rejection.

Text equality remains exact UTF-8 byte equality unless a narrower operation explicitly specifies
another comparison.

Normalization, case folding, locale collation, and canonical equivalence are absent unless a current
consumer specifies them exactly.

A user-facing editor should treat extended grapheme clusters as the default cursor and deletion unit
when the retained segmentation implementation, Unicode version, performance, and terminal projection
are specified and differentially tested.

Byte-oriented operations remain available for file identity, offsets in external protocols, and
exact search evidence.

Line break handling is explicit.

Preserve the file's observed LF or CRLF convention where representable.

Define behavior for mixed line endings and a final missing line terminator.

Do not silently normalize line endings during ordinary open and save.

Cursor, anchor, selection, replacement, end-of-buffer, preferred column, wrapping, movement, and
line-boundary behavior are exact.

Selection direction and collapsed-selection behavior are explicit.

Undo and redo define retention, bounds, grouping, branching after undo, external action interaction,
and crash behavior.

Editor undo is not semantic project restoration and is not filesystem rollback.

Search defines exact matching unit, overlap, ordering, case behavior, normalization behavior,
bounds, and continuation.

Syntax highlighting, diagnostics, line indexes, grapheme indexes, and search indexes are derived and
disposable.

Text representation is unobservable.

Retain a rope, piece tree, gap buffer, line tree, chunk table, or structural sharing only when the
complete editor workload beats the simple canonical oracle and cache miss or reconstruction remains
correct.

## Vim-like interaction policy

`lkjedit` is Vim-like.

Do not claim full Vim compatibility unless an explicit conformance corpus proves it.

Modes, counts, operators, motions, registers, command-line input, search, undo grouping, and
selection behavior belong to application meaning.

The native host decodes keys and mouse events.

It does not implement Vim commands.

At minimum, a useful retained product should specify and test:

- Normal mode;
- Insert mode;
- characterwise Visual mode;
- linewise Visual mode;
- command-line mode;
- forward search input;
- counts on supported motions and edits;
- insertion before and after the cursor;
- line opening above and below;
- character, word, line, document, and viewport movement;
- delete, change, yank, paste, undo, and redo;
- buffer write, close, quit, split, tab, explorer, and search commands;
- exact dirty-buffer and external-conflict behavior;
- keyboard equivalents for every essential mouse-only layout action.

Unsupported Vim syntax must reject or remain literal according to an exact policy.

Do not silently implement a near-match with materially different destructive behavior.

## Terminal input boundary

Applications do not emit raw terminal escape sequences.

Terminal input is decoded into one closed bounded event vocabulary.

Key code, modifiers, press, repeat, release, paste, focus, mouse button, mouse action, coordinates,
scroll direction, and resize are distinct when supported.

Unknown or malformed escape sequences do not become arbitrary application bytes.

Mouse support uses one explicit negotiated encoding and capture policy.

Enable capture only after terminal acquisition succeeds.

Disable every acquired mouse mode during cleanup.

Coordinates are bounded terminal cells.

The terminal adapter does not decide which tab, splitter, editor position, or command a coordinate
means.

Resize dimensions are explicit, bounded, and tested at zero, minimum, odd, typical, and excessive
sizes.

Frames use a closed cell, row, span, or style model with exact clipping and cursor semantics.

Display width, combining marks, wide characters, control characters, invalid terminal responses, and
clipping have explicit behavior.

Terminal output is escaped by construction.

Acquire, raw mode, alternate screen, paste, mouse capture, focus capture, cursor visibility, signal
handling, suspension, EOF, panic, output error, and normal close all have tested cleanup behavior.

A terminal process boundary is not a sandbox.

Do not add a general terminal framework without a complete retained application.

## Frame and rendering policy

Application meaning owns the complete logical frame or an equivalent exact bounded presentation
model.

The host may retain a derived prior frame and emit only changed cells, rows, or spans.

The full logical frame remains the correctness oracle.

A differential renderer must reconstruct exactly the full-frame result after:

- first render;
- resize;
- style change;
- wide or combining text;
- clipping;
- cursor movement;
- scroll;
- tab drag preview;
- cache loss;
- output interruption;
- restart.

Style is a bounded closed semantic palette or style descriptor.

Applications never supply ANSI.

Missing or corrupt render cache falls back to full rendering.

Differential output never changes application state, publication state, input order, or
acknowledgment.

Measure complete input-to-visible-frame latency, not only renderer execution.

## Selected filesystem boundary

Ambient broad filesystem authority is forbidden.

A filesystem grant selects one exact root and explicit operation classes under a documented trust
model.

Semantic paths are ordered validated relative components, not unchecked host strings.

Define encoding, separator, dot, dot-dot, empty component, reserved name, symlink, hard-link, mount,
case-sensitivity, and normalization behavior.

Directory listing defines ordering, pagination, metadata, truncation, races, and inaccessible
entries.

Recursive traversal defines root inclusion, depth, entry, file, byte, result, time, cancellation, and
continuation bounds.

File read binds exact observed type, size, content digest or equivalent version fact, and maximum
bytes.

File write uses explicit expected-base semantics and no-clobber or atomic-replace publication.

An editor replacement preserves the observed ordinary permission bits unless an explicit save policy
says otherwise.

A new file uses one explicit deterministic creation mode under the deployment policy.

Do not silently replace an executable file with mode `0600`.

Conflict, absence, permission denial, invalid type, excessive input, I/O failure, known success,
known failure, and unknown visibility are distinct.

A possibly visible write is never silently repeated.

Reconciliation determines present, absent, conflicting, or indeterminate state from independently
observed facts.

Temporary files, synchronization, rename, directory synchronization, cleanup, and crash points are
tested.

The filesystem adapter cannot invent editor state, project meaning, layout, search policy, or user
intent.

## Search boundary

Current-buffer search may remain pure application or runtime text semantics.

Selected-root search is an explicit filesystem observation.

A root search request binds:

- selected root grant;
- validated relative start path;
- exact literal or accepted pattern contract;
- case and normalization policy;
- file-size and total-byte limits;
- maximum directories, files, matches, and snippet bytes;
- deterministic traversal and result order;
- continuation identity;
- cancellation or abandonment behavior.

A search result binds relative path, exact observed file facts, byte range, line facts when
available, and escaped bounded preview.

Opening a search result revalidates the file.

A stale result never authorizes an edit or save.

Search results are ordinary tabs.

A search tab may outlive its job.

Closing the tab does not imply that an already visible external operation was rolled back.

Do not add a persistent project-wide index until a complete workload proves that rebuilding,
invalidation, storage, corruption, and fallback complexity pays rent.

## Semantic project host boundary

An lkjscript application that operates on another semantic project needs one explicit project grant.

The grant binds exact workspace identity, locator policy, allowed read and mutation classes, limits,
and applicable authority revision.

Project reads return exact revision-bound typed results.

Project mutations carry exact expected revision and idempotency facts.

The application may propose project actions.

The project owner alone validates and publishes them.

Cross-authority atomicity between application state, project history, files, terminal output, and
background jobs is absent unless explicitly proved.

A pending cross-authority action records enough facts to avoid duplicate execution and to reconcile
after interruption.

The project adapter does not expose private store mutation, raw lock manipulation, or unvalidated
graph insertion.

A project path is deployment state and cannot substitute for the granted workspace identity.

Semantic project explorer, proposal, history, diff, diagnostics, and target output may be ordinary
tabs in an editor.

They do not receive special layout authority.

## Host interfaces and grants

Pure deterministic computation remains the default.

Ambient host authority is forbidden.

Applications declare exact host-interface requirements but never grants.

Instances or foreground runners bind requirements to exact grants.

A grant binds sharing domain, interface identity, adapter kind, bounded descriptor, applicable
authority revision, and limits.

Host requests and outcomes are closed typed values.

Opaque bytes are permitted only when the interface deliberately defines and bounds them.

An adapter cannot invent semantic state, application response, command intent, layout, or authority.

Live resources require explicit acquire, use, transfer, consume, close, cancellation, timeout,
crash, and cleanup semantics.

Expected workflow outcomes may be nominal data.

Corruption, denial, exhaustion, infrastructure failure, stale state, conflict, and unknown
visibility remain distinguishable.

Non-idempotent work is never silently retried after possible partial execution.

Time, randomness, scheduling observations, terminal input, filesystem observations, and host
observations are explicit when observable.

## Bounded background work

A foreground terminal loop may use one or more bounded native workers only for explicit external
actions whose domain policy remains in application meaning.

Do not add a general async runtime, executor, scheduler, actor system, or daemon merely to prevent
one editor action from blocking input.

A worker design states:

- exact job identity domain;
- application request and result types;
- queue capacity;
- worker count;
- admission and overload behavior;
- ordering;
- cancellation points;
- non-cancellable visibility boundaries;
- panic containment;
- stale-result handling;
- tab or view closure behavior;
- terminal shutdown behavior;
- process-exit behavior;
- test oracle.

A bounded synchronous channel is preferable to an unbounded hidden queue.

A save that may already be visible is not cancelled or retried merely because the user closed a tab.

A result reenters application meaning through one typed route.

The host does not update editor state directly.

Local key and mouse input may continue while a read-only search, listing, build, test, or query runs
when application state can represent the pending job exactly.

## Mutation and query separation

Mutations and observations are separate semantic contracts.

A project mutation may reject, validate without publication, or publish exactly one revision and
record.

An application mutation may decline, report unchanged, publish one completed state, or publish one
suspended state and command under its profile.

A pure query returns a typed value without publishing semantic or durable state.

A query must not be implemented as a no-op event.

A product client must not decode private state as a second query authority.

Query output failure has no rollback meaning because no semantic publication occurred.

Pagination, ordering, truncation, omissions, revision binding, and result digests are exact and
bounded.

## Identity and continuity

Assign durable identity only for a concrete continuity, reference, repair, history, sharing, import,
export, target, instance, product, or operational consumer.

Names, formatting, positions, order, paths, hashes, compiler indexes, artifact offsets, storage keys,
runtime handles, queue positions, process IDs, and addresses are not semantic identity unless a
closed contract assigns a narrower role.

Workspace, revision, revision record, change, build target, release, application, instance, product
entity, command, outcome, grant, interface, adapter, deployment, executable, checkpoint, backup,
cache entry, session handle, buffer, view, tab, tile, layout node, file origin, search job, terminal
session, and runtime handle are distinct domains.

A digest is never implicitly continuity, provenance, authorization, signature, freshness, or
capability identity.

Identity-preserving change requires an explicit validated rule.

Deleted durable identities are not silently reused.

Multiple exact versions may coexist only when references remain unambiguous.

Function-local identities remain bound to one function and exact revision unless a real continuity
consumer justifies more.

## Publication and durability

Published revisions, records, releases, applications, instance records, host outcomes, authoritative
checkpoints, backups, and other declared durable objects are immutable within their domains.

Every durable namespace has one publication authority.

One successful publication creates exactly one accepted durable outcome.

Rejection and validate-only publish nothing and consume no durable identity.

Semantic no-change does not consume a revision merely to return a response.

Success is acknowledged only after the documented synchronization boundary.

A possibly visible but unconfirmed outcome is reported as unknown and never silently retried.

Recovery, replay, retention, checkpointing, compaction, corruption, backup, restore, deletion, and
garbage collection are explicit and validated.

Semantic state publication and externally visible host work remain separate unless atomicity is
proved.

Output failure cannot retroactively undo accepted authority.

Related durable objects must not become independently visible in conflicting combinations.

## Values, language, and representation

Add a language or value capability only for a complete current application or the semantic
development CLI itself.

Text and variable-length collections require exact validation, canonical encoding, bounds,
deterministic equality and ordering, and a current consumer.

Do not add a conventional source language merely to replace a graph-construction script.

Do not add generics, traits, macros, maps, sets, iterators, mutable builders, normalization,
formatting, reflection, or operator syntax without an exact blocker and complete end-to-end use.

Representation, sharing, allocation, reclamation, line aggregates, grapheme indexes, checkpoints,
caches, IR, bytecode, profiles, and native code are unobservable or derived unless deliberately
promoted by specification.

A simple independent allocation, execution, reconstruction, and build route remains the oracle for
optimized values, storage, caches, compaction, and execution tiers.

Cache miss, eviction, missing derived state, and process restart remain correct.

Full snapshots, deltas, journals, object stores, databases, collectors, bytecode, JITs, schedulers,
and supervisors must beat simpler safe designs on a representative complete workload.

## Execution, scheduling, and concurrency

One simple executable route remains the correctness oracle.

Faster tiers are differential until direct cutover is justified.

Values, traps, order, state transitions, host requests, resource semantics, and diagnostics remain
stable across tiers unless accepted semantics deliberately change.

Operational scheduling and language semantics are separate.

Deterministic fuel is not wall-clock time.

A scheduler may use time, priority, quota, or load only when those choices do not silently change
semantic results.

Observable interruption, yield, cancellation, or timeout requires an accepted contract.

Per-project and per-instance mutations remain serial unless a stronger model is specified and
proved.

Pure queries may run concurrently only after snapshot isolation, lifetime, ordering, admission,
shutdown, and mutation interaction are proved.

Cross-project, cross-instance, or background-job parallelism requires exact isolation, bounded
queues, deterministic per-authority order, overload behavior, safe shutdown, and differential tests
against serial execution.

## Fuel and work accounting

One scalar fuel number is permitted only when it remains a predictable semantic work bound rather
than a substitute for every resource class.

Keep these separate when they differ:

- executed instruction work;
- traversed text bytes or scalars;
- allocated logical result bytes;
- retained backing bytes;
- live values and objects;
- call frames;
- collection elements;
- output cells and bytes;
- host actions;
- queued jobs;
- wall-clock operational deadlines.

A persistent or structurally shared text operation should charge touched semantic work and newly
created logical structure.

It must not automatically charge the entire unchanged document merely because the result denotes the
whole document.

Representation sharing cannot evade retained logical accounting.

A fixed enormous fuel budget that exists only to make one common edit pass is a reversal signal.

Expose bounded stage or resource breakdowns when they materially improve diagnosis without flooding
normal output.

## Resource governance

Semantic limits and operational limits are separate.

Semantic limits include fuel, frames, value depth, item count, text bytes, collection elements,
graph expansion, transaction operations, revision record size, target closure, state size, query
work, response size, layout nodes, tabs, buffers, views, undo entries, and render cells.

Operational limits include queue capacity, concurrent compilation, cache bytes, aggregate memory,
open files, terminal events, adapter operations, process count, CPU share, temporary publication
bytes, selected roots, project count, and deployment quotas.

Each limit states accounting unit, owner, reservation and release points, peak and retained
accounting, source, rejection class, retryability, observability, restart behavior, and publication
interaction.

Check lengths, counts, depth, and reservations before allocation or corresponding work.

Do not present allocator observations as exact semantic accounting.

Shared backing storage, checkpoints, caches, embedded artifacts, target outputs, undo roots, and
render caches must not bypass limits.

Overload rejects, queues within a bound, or sheds work under an explicit policy.

Never create an unbounded hidden queue.

OS controls may strengthen containment but do not replace semantic or runtime accounting.

## Long-term performance

Correctness is necessary and not sufficient for an interactive product.

Measure complete workflows in optimized builds.

Separate:

- startup;
- artifact validation;
- application initialization;
- input decode;
- semantic update;
- host action;
- render construction;
- frame diff;
- terminal encode;
- terminal write and flush;
- file publication;
- search traversal;
- project open;
- compilation and execution.

Do not optimize a stage that does not dominate the complete workflow.

Preserve an independent simple route for every optimized representation or execution tier.

Prefer changes that improve asymptotic behavior before micro-optimizing constants.

Avoid full-document copying, full-history decode, full-target rebuild, full-frame terminal output,
and per-command process startup when representative evidence shows they dominate.

A bytecode, specialized interpreter, JIT, native tier, persistent index, render cache, or worker
must have:

- a named complete consumer;
- a reproduced baseline;
- an independent oracle;
- a measurable target;
- bounded failure and fallback behavior;
- exact cache invalidation;
- a deletion or reversal gate.

Do not weaken semantic limits or inflate fuel merely to hide an algorithmic defect.

## Security and trust

Accepted semantics cannot express unchecked memory access.

User-controlled depth does not consume unbounded native stack.

No local unsafe Rust is permitted unless the active user explicitly authorizes a replacement after a
concrete need, isolated ownership, safe public contract, and independent tests are recorded.

Memory safety, exhaustion, stack safety, cleanup, aliasing, concurrency, permissions, path safety,
terminal safety, crash consistency, supply-chain trust, and hostile-code isolation are separate
contracts.

Treat bytes, text, paths, locators, filesystem metadata, semantic repositories, revision records,
artifacts, instance files, checkpoints, backups, caches, profiles, IPC peers, terminal events, paste,
mouse coordinates, search patterns, and adapter outcomes as hostile input.

A process boundary creates neither semantic authority nor a sandbox.

A runtime supervisor authenticates and authorizes every request under its deployment model.

A path, socket, UID, environment variable, or connection is not sufficient semantic authority by
itself.

Write the threat model before multi-user access, untrusted native code, broad filesystem or network
access, child processes, secrets, terminal control beyond trusted local use, or remote
synchronization.

Large input, work, state, history, graph, output, queue, cache, diagnostics, and metrics are bounded,
paginated, chunked, streamed, or policy-controlled.

Human terminal output is escaped and bounded.

Machine output is framed separately and never contaminated by progress text.

Compactness never weakens validation, authorization, identity, durability, diagnostics, or
verification.

## Agent and provider economy

Context budget is part of correctness, latency, and cost.

This root policy has no arbitrary byte ceiling, but every durable instruction pays recurring context
rent.

Keep permanent rules here and volatile campaign facts in the active prompt, status, evidence, or
generated orientation.

State durable principles once.

Prefer compact orientation, task-scoped graph queries, exact on-demand expansion, bounded review,
stable diagnostics, validate/apply parity, delta receipts, digest reuse, session-local handles,
quiet passing checks, and explicit omissions.

Build a compact campaign ledger instead of repeatedly reading a long prompt.

Expose only relevant schema roots and tools.

Add prompt rules, examples, schemas, or context only for a measured failure mode.

A larger response is justified when it prevents more expensive rediscovery or correction.

Byte minimization alone is not the objective.

Compare equal tasks using semantic success, unintended changes, correction depth, repeated discovery,
action bytes, observation bytes, request count, process count, files opened, source bytes opened,
schema bytes, build invocations, elapsed time, and failure quality.

Record provider model identity, token classes, cache classes, dated prices, and monetary cost only
when directly exposed.

Bytes are not tokens.

Never infer provider cost from bytes.

Do not claim API-cost savings without exact comparable telemetry.

Measured reductions in bytes, calls, processes, files opened, correction depth, or elapsed time may
be claimed precisely.

## Code ownership and dependencies

`docs/spec/` owns accepted contracts.

`docs/architecture.md` owns components and trust boundaries.

`docs/status.md` owns implemented reality.

`docs/performance.md` and structured evidence own measurements.

`docs/roadmap.md` owns unresolved consumer-driven reversal gates.

`README.md` owns concise orientation.

Application READMEs own product-specific user contracts.

Campaign prompts are temporary execution artifacts.

Keep one executable owner for every type, field, operation, query, error, limit, format, interface,
grant, resource, command, target, record, machine descriptor, input event, layout command, and job
outcome.

Derive views only when staleness cannot be silent.

Organize code around stable ownership and changed-together behavior.

Split large files when bounded review, agent context, test isolation, compile locality, or ownership
clarity improves without duplicating invariants.

Do not preserve arbitrary file-size or directory-count limits.

Prefer the standard library and existing dependencies.

A new dependency must repay supply-chain, build, binary, audit, operational, and maintenance cost on
the complete workflow.

Vendor or patch a dependency only for a reproduced defect with exact scope, upstream relation,
retention reason, and differential tests.

Git history is the archive for deleted repository material.

Delete stale active-tree copies and losing generated paths.

## Testing and verification

Acceptance tests have exact immutable input, oracle, policy, selection, order, and result.

Skipped, exhausted, cancelled, unavailable, or indeterminate tests do not pass.

For changed boundaries, cover every applicable case:

- canonical success;
- repeated success;
- validate-only parity;
- semantic no-change;
- no-publication outcomes;
- pure-query no-write behavior;
- revision-record atomicity;
- stale and future base;
- duplicate and idempotency conflict;
- wrong identity domain;
- malformed input;
- truncated input;
- trailing input;
- oversized input and output;
- exact and one-over limits;
- ambiguous selectors;
- foreign authority;
- corruption;
- restart;
- interrupted publication;
- unknown visibility;
- reconciliation;
- output failure;
- cleanup;
- concurrent access;
- authority busy;
- overload;
- replay;
- cache miss, hit, eviction, and corruption;
- checkpoint and reconstruction differential;
- build-target determinism;
- generated-view stale detection;
- artifact reproduction;
- backup and restore;
- relative-path and discovery safety;
- terminal resize and cleanup;
- mouse enable, decode, drag, and cleanup;
- layout normalization;
- buffer/view/tab identity separation;
- Unicode and line-ending behavior;
- filesystem substitution and conflict;
- background job ordering and stale results;
- public CLI workflows;
- first-party application dogfooding;
- compact passing output and bounded failure output.

Use a simple independent reference model where semantics become substantial.

Run narrow checks first, then the full repository gates.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --release --locked
git diff --check
```

Run every affected retained public workflow and every selected complete application.

Use Miri, sanitizers, mutation tests, property tests, fuzzing, model checking, crash injection,
filesystem fault injection, pseudo-terminal tests, or cross-platform execution when they target a
real risk and are available.

State scope and limitations.

Do not weaken an invariant test to make implementation pass.

Change specification, implementation, and oracle together when behavior deliberately changes.

## Verification output contract

Provide a repository-owned verification entry point with at least focused, product, and full
profiles when doing so materially reduces repeated command and output cost.

The entry point must:

- execute exact documented commands;
- preserve each command's exit status;
- retain complete stdout and stderr separately or with exact framing;
- print a compact deterministic summary on success;
- print only bounded failure excerpts by default;
- report the complete log paths;
- support an explicit full-detail mode;
- reject unknown profiles and arguments;
- never transform skipped or unavailable checks into success;
- run without network unless a profile explicitly declares network use;
- remain an operational harness rather than semantic authority.

A passing full profile should normally fit within a small terminal screen.

Do not achieve compactness by piping through a filter that discards the only copy of diagnostics.

## Evidence and claims

Evidence is not semantic authority.

Record exact environment, commit, command, input corpus, output, raw observations, units, sample
selection, warm or cold classification, and unavailable data.

Do not infer tokens from bytes.

Do not infer cost without exact model-specific token and dated price telemetry.

Do not call a warm-host sample cold.

Do not call summed command waits wall-clock time.

Do not call logical accounting RSS enforcement.

Do not call a digest provenance, signature, authorization, or freshness.

Do not call a process, worker, container, private directory, or project locator a sandbox.

Do not call compile success cross-platform support.

Do not claim full-history validation from a current-state check.

Do not call a terminal smoke test a production editor.

Do not call mouse decoding draggable tabs until application-owned hit testing, drag state, movement,
split drop, cancellation, and PTY acceptance all pass.

Do not call a text editor Vim-compatible merely because it has modes and `hjkl`.

Retain bounded evidence for serious losing alternatives, not only the selected result.

Claims must be no stronger than the checkout and reproduced evidence.

## Decision standard

Treat every historical mechanism as provisional except enduring semantic, safety, and authority
contracts in the effective instructions.

Do not preserve a mechanism because it was difficult, planned, shipped recently, or heavily tested.

Reproduce relevant evidence before reversing working behavior.

Prefer complete useful verticals over isolated features.

Prefer one exact path over parallel convenience paths.

Prefer explicit domains over overloaded names.

Prefer direct semantic CLI operations over custom construction code.

Prefer automatic immutable history over manual reconstruction.

Prefer one topology-neutral owner over duplicated process adapters.

Prefer local features over platforms built for one consumer.

Prefer deletion over permanent deprecation.

Prefer bounded prototypes over speculative architecture.

Prefer high-leverage corrections over accumulated exceptions.

Prefer asymptotic corrections over larger limits.

Prefer one generic tab lifecycle over special sidebar behavior.

Prefer one typed input event vocabulary over native policy.

Prefer one retained editor product over parallel `lkjstudio` and `lkjedit` products when the latter
is the direct successor.

Every retained abstraction, dependency, process, artifact, identity, schema, cache, optimization,
service, scheduler, worker, framework, source form, or generator needs a named current consumer.

Complexity must pay rent in a representative end-to-end workflow.

Before retaining a substantial choice, record:

- consumer;
- obligations;
- reproduced baseline;
- serious alternatives, including deletion;
- expected benefit;
- measured result;
- semantic and operational costs;
- independent oracle;
- direct-cutover deletions;
- stop rule;
- reversal condition.

Delete losing prototypes completely.

Current absences are not permanent prohibitions without semantic reason.

## Change workflow

1. Inspect checkout, instructions, branch, commit, remotes, and unrelated work.
2. Identify authoritative owners and active format identities.
3. Select the valuable complete application workflow and freeze an independent oracle.
4. Reproduce current public authoring, build, run, recovery, product, and performance paths.
5. Create a compact campaign ledger with outcomes, non-goals, alternatives, stop rules, and
   verification state.
6. Build the smallest complete semantic-CLI slice.
7. Use it immediately on a real maintained application.
8. Prototype uncertain questions in the smallest dependency-closed form.
9. Select one coherent design from evidence.
10. Implement the full vertical across semantic model, persistence, protocol, CLI, targets, product,
    tests, documentation, and operational tooling.
11. Publish semantic changes through the public path and inspect automatic history.
12. Cut over directly and delete builders, duplicate surfaces, compatibility paths, and stale facts.
13. Run focused, full, representative, restart, corruption, publication, interaction, and product
    checks.
14. Measure equal authoring and product tasks and record only supported savings.
15. Dogfood from a fresh checkout.
16. Inspect the final diff and staged scope.
17. Perform only the explicitly authorized Git publication actions.
18. Leave a compact exact handoff.

Do not stop at a report when a safe complete implementation is authorized and feasible.

Do not scatter partial architecture.

Do not ask the user to decide ordinary engineering details that checkout evidence can resolve.

## Completion and handoff

A semantic-development capability is complete only when it is:

- discoverable from an ordinary project directory;
- usable through supported public human and machine CLI contracts;
- exact-base-bound;
- automatically recorded as immutable revision history;
- validated by one semantic owner;
- boundedly inspectable and diffable;
- buildable through first-class target declarations;
- reproducible from a fresh checkout without a custom graph builder;
- covered by independent oracles;
- restart, corruption, limit, path, and publication tested;
- resource-accounted where applicable;
- documented by one owner;
- exercised by a useful first-party application;
- measured end to end;
- free of superseded paths.

An interactive editor product is complete only when:

- normal launch does not require internal artifact plumbing;
- keyboard editing is useful;
- mouse selection, tab drag, and split drop are real product workflows;
- explorer and search are ordinary tabs;
- all essential layout actions have keyboard equivalents;
- buffer, view, tab, tile, and file identities remain distinct;
- dirty, conflict, save, unknown visibility, and reconciliation behavior are explicit;
- Unicode and line-ending behavior are documented and tested;
- local input remains responsive under representative file and layout workloads;
- terminal acquisition and cleanup are fault-tested;
- the application meaning, not native glue, owns editor policy;
- a fresh checkout reproduces the checked product.

Before finishing, report:

- exact starting and ending state;
- selected design and serious rejected alternatives;
- changed authority and format contracts;
- automatic-history behavior;
- CLI workflows;
- interactive-session behavior;
- terminal, filesystem, project, and job grants;
- layout, tab, view, buffer, and file-origin behavior;
- text representation and Unicode contract;
- build-target and packaging behavior;
- first-party application migration or creation;
- deleted builders, products, and stale paths;
- validation commands and exact results;
- representative product and authoring-economy evidence;
- provider telemetry only when directly exposed;
- known limits and trust assumptions;
- reversal gates;
- Git commits and remote publication outcome when authorized;
- every requested action not performed.

Claims must be no stronger than the checkout and reproduced evidence.

## Durable identity audit table

### workspace

- Continuity: semantic project continuity.
- It is not interchangeable with: path or Git commit.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### revision

- Continuity: one accepted project state.
- It is not interchangeable with: timestamp or sequence position outside its owner.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### revision record

- Continuity: canonical facts about one accepted transition.
- It is not interchangeable with: log line.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### target

- Continuity: one exact build derivation root.
- It is not interchangeable with: target name alone.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### release

- Continuity: one reusable semantic closure.
- It is not interchangeable with: workspace identity.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### application

- Continuity: one runnable exact release world.
- It is not interchangeable with: artifact path.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### instance

- Continuity: durable product-state continuity.
- It is not interchangeable with: process.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### layout node

- Continuity: one ephemeral split-tree node.
- It is not interchangeable with: screen rectangle.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### tile

- Continuity: one tab-stack container.
- It is not interchangeable with: pane coordinates.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### tab

- Continuity: one movable layout item.
- It is not interchangeable with: buffer.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### view

- Continuity: one content presentation and local interaction state.
- It is not interchangeable with: tab label.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### buffer

- Continuity: one editable text continuity.
- It is not interchangeable with: file path.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### file origin

- Continuity: one observed external file base.
- It is not interchangeable with: buffer.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### search job

- Continuity: one bounded external observation request.
- It is not interchangeable with: search tab.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### host action

- Continuity: one explicit external request.
- It is not interchangeable with: keyboard event.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### terminal session

- Continuity: one acquired live terminal lifecycle.
- It is not interchangeable with: application state.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

### render cache

- Continuity: one disposable prior-frame optimization.
- It is not interchangeable with: logical frame.
- Define allocation, equality, retention, deletion, nonreuse, and diagnostic spelling.
- Reject foreign-domain values even when their bytes or display names match.
- Keep deployment locators and rendered positions outside identity.

## Durable layout invariants

1. Every reachable tab belongs to exactly one live tile.
2. Every live tile contains at least one tab after normalization.
3. Every selected-tab reference names a tab in its own tile.
4. Every non-root layout node has exactly one parent.
5. No split node has fewer than two children after normalization.
6. No split node directly contains a child split of the same orientation after canonical flattening unless weights require a documented distinction.
7. No layout operation duplicates or loses a tab.
8. Moving a tab preserves its tab and view identity.
9. Splitting with a tab moves or clones only according to an explicit command; drag-drop defaults to move.
10. Closing a tab never destroys a buffer retained by another view.
11. Closing the final view of a dirty buffer enters an explicit decision state.
12. A cancelled drag leaves the canonical layout unchanged.
13. A failed render leaves application layout state unchanged under transactional interactive-step semantics.
14. Resize recomputes geometry without changing layout identity or weights.
15. Minimum dimensions clamp geometry but do not silently delete tabs.
16. Hit regions are derived from the exact rendered geometry for the same state.
17. Mouse and keyboard variants of one command pass through one semantic owner.
18. A tab strip overflow policy is deterministic and keyboard-accessible.
19. Focus always names one reachable tile and one selected tab.
20. Layout normalization is deterministic and idempotent.

## Durable editor invariants

1. Content is valid UTF-8 at every accepted state.
2. Byte, scalar, grapheme, line, and terminal-cell units are never conflated.
3. A cursor is always at a valid boundary for the active movement contract.
4. An anchor and cursor define one exact directed selection.
5. A replacement validates both range and inserted text before changing state.
6. One user edit creates one documented undo group unless command semantics group more.
7. A new edit after undo discards or branches redo according to one explicit policy.
8. Undo never changes file-origin evidence by itself.
9. Save success updates file-origin evidence only from the host's exact published observation.
10. Unknown save visibility blocks automatic retry and preserves reconciliation data.
11. Reload replaces content only after the user-visible policy accepts external state.
12. Two views of one buffer observe one content and undo history.
13. View-local cursor movement does not dirty the buffer.
14. Formatting or render cache changes do not dirty the buffer.
15. Search highlights are derived and do not edit content.
16. Line-ending policy is preserved across ordinary open and save.
17. A file without a final terminator is not silently given one.
18. Unsupported non-UTF-8 input produces a typed product outcome.
19. Oversized content rejects before unbounded allocation.
20. Closing a dirty final view requires explicit save, discard, or cancel.

## Durable failure classification examples

### `proposal_rejected`

- Meaning: The request is well-framed but semantically invalid.
- Required behavior: No publication.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `malformed_input`

- Meaning: The input violates its closed syntax or encoding.
- Required behavior: No domain work starts.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `stale_base`

- Meaning: The exact expected authority revision no longer matches.
- Required behavior: No implicit refresh or retry.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `authority_busy`

- Meaning: The bounded owner or queue cannot admit the request.
- Required behavior: Reject or expose a retryable operational outcome.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `resource_exhausted`

- Meaning: An exact semantic or operational bound is exceeded.
- Required behavior: Report unit, limit, and observed request.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `filesystem_conflict`

- Meaning: The selected file or directory no longer matches observed base facts.
- Required behavior: Require user policy.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `unknown_visibility`

- Meaning: External publication may have become visible.
- Required behavior: Never retry automatically; reconcile.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `terminal_decode`

- Meaning: Host input cannot be decoded under the active contract.
- Required behavior: Preserve application state and clean up if terminating.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `terminal_output`

- Meaning: Logical frame projection or write failed.
- Required behavior: Do not roll back external publications.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `terminal_cleanup`

- Meaning: A live terminal stage is not known restored.
- Required behavior: Attempt all remaining cleanup and report precedence.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `job_stale`

- Meaning: A background result no longer matches its application pending state.
- Required behavior: Discard or present explicitly; never mutate unrelated state.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.

### `corrupt_authority`

- Meaning: Retained authority fails canonical validation.
- Required behavior: Fail closed and preserve evidence.
- Keep domain failure distinct from transport, output, and cleanup failure.
- Include bounded exact identity and continuation facts when applicable.
