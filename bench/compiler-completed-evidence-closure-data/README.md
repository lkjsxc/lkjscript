# Completed compiler evidence: prospective closure

This bundle prepares one cooperative, exact-path cleanup of completed compiler
evidence. It does not authorize execution. Root must first review the successful
preparation, copy this complete directory byte for byte to
`bench/compiler-completed-evidence-closure-data`, commit all three pending
retention bundles, independently audit that copy outside this bundle, and release
one invocation against that exact commit.

## Scope and recovery

The 25 completed evidence roots contain exactly 2,175 regular files, 201
directories including their roots, and 480,049,506 logical file bytes. Their
literal paths are compiled into `closure-lib.pl` and frozen in `manifest.json`.
The manifest also covers the query and native closure-proposal directories and
the standalone eleven-scope proposal. It records each complete path set, file
digest, file and directory metadata, and recovery location.

Only 34 files are disposable:

- The 32 explicitly reviewed native `preparation_intermediate` files total
  39,832,656 bytes.
- The two held ordinary/profile compiler images total 101,407,191 bytes. Their
  exact digests and production evidence remain retained; no future consumer
  requires the executable files.

Every other regular file has an exact recovery from a committed Git blob,
an exact archive member, a gzip stream, or a byte range in a committed blob.
The complete current bytes must equal that recovery before any removal.
`prepare.pl` verifies the current pending query/native retention copies.
Execution instead resolves every `revision: release` reference exclusively
through the supplied immutable commit, including those pending copies.
The original eleven-scope proposal is retained here unchanged.

The query and native proposal mirrors recover from their committed retention
bundles. This preparation directory is the fourth proposal mirror. Its exact
file and directory set comes from the final committed bundle tree; its live
metadata is frozen during the final read-only preflight. This avoids a
self-referential digest. Copy this bundle exactly: any main-only audit, CI
addition, or retention note belongs outside the bundle, unless it was included
in both copies before the commit. No preparation write may occur after release.

## Guard and results

The only released entry point will be the absolute main path, with one full
40-character release commit:

```sh
bash /home/coder/workspace/lkjscript/bench/compiler-completed-evidence-closure-data/run.sh EXACT_RELEASE_COMMIT
```

The runner exclusively claims
`bench/compiler-completed-evidence-closure-result` in main. It records its own
PID, command, working directory, UTC and nanosecond interval, load before and
after, raw stdout/stderr, exit status, and GNU time metrics. The foreground guard
has a 4 GiB address-space ceiling and 300-second deadline with 10-second grace.
These are cleanup limits, not compiler-performance measurements.

The guard verifies its own source, library and manifest against the supplied
commit and requires main HEAD to equal that commit. It then checks all 29
literal roots: the 25 old roots, three frozen proposal mirrors, and this
preparation directory. Regular files must have one link; symlinks, special
files, unexpected paths and changed identities refuse. Reads use no-follow
file descriptors with before/after metadata checks. All recoveries and the
complete second path-set check must finish before the first unlink.

Removal follows the enumerated file list, with current file-byte, descriptor,
path and parent identity rechecks. Directories are removed deepest first after
empty and identity checks. Final success requires ENOENT for every selected
root. The main retained bundle and result directory lie outside every removal
root. The guard never scans processes, file descriptors, keys, other target
siblings or services.

Cleanup is not atomic. Any refusal, timeout or I/O error stops the invocation;
already removed files remain removed and all remaining files stay in place.
The event stream records intent and completed removals, and a caught failure
records the phase and completed counts. A killed guard may have only the raw
wrapper status and its preceding events. The committed recovery records are
sufficient to recover removed nondisposable files, but this protocol performs
no automatic restoration, broadening or retry. A later action needs a new
explicit release. The checks describe cooperative ownership and are not a
hostile-race atomic filesystem transaction.

## Preparation evidence

The initial guard syntax failure is preserved as `guard.syntax-1.pl` and
`syntax-1/guard.pl.*`; it was a missing separator in a comparison expression.
The corrected guard passed `syntax-2/guard.*`. The library and preparation
helper passed their Perl syntax checks, and the runner passed Bash syntax.
No guard, runner or removal ran during those checks.

`preparation-1/` records the separately bounded, read-only recovery pass.
Its successful output, when present, must agree with `manifest.json`.
A failed preparation remains retained and does not release cleanup. Neither
preparation nor closure invokes a compiler, generator, profiler or benchmark.
