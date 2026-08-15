# Nominal data and exhaustive match

This example drives the release `lkjscriptd` daemon exclusively through the production strict
version-4 generic JSON CLI. JSON is transport; canonical immutable `.lkjscript` revisions in the
daemon's private temporary state are the only program authority.

In one structured transaction it creates:

- `Reading`, a product with `value: i64` and `valid: bool`;
- `Input`, a closed sum with `sample(Reading)`, `missing`, and `override(i64)` variants;
- functions that construct and project `Reading`, construct every `Input` variant, and exhaustively
  and lazily match `Input`;
- `main`, initially blocked by one `Reading`-typed hole.

The driver obtains the schema manifest, six task-relevant sections, and a known-digest `unchanged`
response. It requests only selected semantic IDs, reads nominal repair context, proves a wrong
identity-keyed product repair rejects without publication, applies a valid identity-preserving
repair, and queries its semantic diff. Public Run checks cover scalar arguments/results, exact
nominal input and output, every sum arm, and both unselected and selected overflow. It then performs
a typed shutdown, restarts, checks both retained revisions and stable IDs, repeats all Run oracles,
and shuts down through the typed protocol again.

Run from the repository root:

```sh
./examples/nominal-match/run.sh
```

The script builds the release binaries and uses a mode-0700 temporary state directory created and
removed by Python's standard library. It never points the daemon at user state and deletes no
caller-owned state. It requires Rust, a POSIX shell, and Python 3; the Python driver has no third-party
dependencies.
