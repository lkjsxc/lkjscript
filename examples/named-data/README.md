# Named records, variants, and complete handling

This focused example drives the release `lkjscriptd` local background service exclusively through
the production strict version-5 generic JSON CLI. JSON is transport; immutable `.lkjscript`
revisions in the service's private temporary state remain the authoritative program.

One structured transaction creates:

- `Reading`, a named immutable record with `value: i64` and `valid: bool` fields;
- `Input`, a variant type with `sample(Reading)`, `missing`, and `override(i64)` alternatives;
- functions that construct and project `Reading`, construct every `Input` alternative, and handle
  every alternative lazily;
- `main`, initially blocked by one `Reading` typed placeholder.

The driver obtains the machine-contract manifest, 12 operational endpoint roots with complete
JSON-envelope, error, ID, and limit definitions, and the compact unchanged response for a known
fingerprint. It requests only selected persistent IDs, reads exact repair context, proves a
field/type-invalid record repair publishes nothing, and fills the
placeholder without changing its stable identity. A semantic diff records the refinement. Public
Run checks cover scalar arguments/results, exact named input and output, every variant, and both an
unselected and selected overflow path. Typed shutdown and restart then prove both saved revisions,
their IDs, and their behavior remain available.

Run from the repository root:

```sh
./examples/named-data/run.sh
```

The script builds optimized release binaries and uses a mode-0700 temporary state directory created
and removed by Python's standard library. It never points the service at user state and deletes no
caller-owned state. It requires Rust, a POSIX shell, and Python 3; the driver has no third-party
Python dependencies.
